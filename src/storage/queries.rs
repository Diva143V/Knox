//! Dynamic query builder and search engine for Knox knowledge graph.

use chrono::{DateTime, Utc};
use rusqlite::types::ToSqlOutput;
use rusqlite::{ToSql, params_from_iter};
use std::collections::HashMap;

use super::db::KnoxDb;
use crate::core::error::KnoxResult;
use crate::core::ids::{ActionId, SessionId};
use crate::core::types::{
    ActionRelation, DeltaSummary, KnoxAction, QueryFilter, QueryResult, RelationType, StatusReport,
};

/// Encodes pagination cursor `(timestamp_ms, id)` into a URL-safe base64 string.
pub fn encode_cursor(timestamp_ms: i64, id: &str) -> String {
    let raw = format!("{timestamp_ms}:{id}");
    const B64: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
    let bytes = raw.as_bytes();
    let mut out = String::with_capacity((bytes.len() * 4 / 3) + 4);
    let mut i = 0;
    while i < bytes.len() {
        let b0 = bytes[i];
        let b1 = if i + 1 < bytes.len() { bytes[i + 1] } else { 0 };
        let b2 = if i + 2 < bytes.len() { bytes[i + 2] } else { 0 };

        let n0 = b0 >> 2;
        let n1 = ((b0 & 3) << 4) | (b1 >> 4);
        let n2 = ((b1 & 15) << 2) | (b2 >> 6);
        let n3 = b2 & 63;

        out.push(B64[n0 as usize] as char);
        out.push(B64[n1 as usize] as char);
        if i + 1 < bytes.len() {
            out.push(B64[n2 as usize] as char);
        } else {
            out.push('=');
        }
        if i + 2 < bytes.len() {
            out.push(B64[n3 as usize] as char);
        } else {
            out.push('=');
        }
        i += 3;
    }
    out
}

/// Decodes a URL-safe base64 pagination cursor into `(timestamp_ms, id)`.
pub fn decode_cursor(cursor: &str) -> Option<(i64, String)> {
    let clean = cursor.trim().trim_end_matches('=');
    if clean.is_empty() {
        return None;
    }
    fn val(c: u8) -> Option<u8> {
        match c {
            b'A'..=b'Z' => Some(c - b'A'),
            b'a'..=b'z' => Some(c - b'a' + 26),
            b'0'..=b'9' => Some(c - b'0' + 52),
            b'-' | b'+' => Some(62),
            b'_' | b'/' => Some(63),
            _ => None,
        }
    }
    let chars = clean.as_bytes();
    let mut bytes = Vec::with_capacity(chars.len() * 3 / 4);
    let mut i = 0;
    while i < chars.len() {
        let c0 = val(chars[i])?;
        let c1 = if i + 1 < chars.len() { val(chars[i + 1])? } else { 0 };
        let c2 = if i + 2 < chars.len() { val(chars[i + 2])? } else { 0 };
        let c3 = if i + 3 < chars.len() { val(chars[i + 3])? } else { 0 };

        bytes.push((c0 << 2) | (c1 >> 4));
        if i + 2 < chars.len() {
            bytes.push(((c1 & 15) << 4) | (c2 >> 2));
        }
        if i + 3 < chars.len() {
            bytes.push(((c2 & 3) << 6) | c3);
        }
        i += 4;
    }
    let s = String::from_utf8(bytes).ok()?;
    let (ts_str, id_str) = s.split_once(':')?;
    let ts = ts_str.parse::<i64>().ok()?;
    Some((ts, id_str.to_string()))
}

/// Executes a flexible filtered query against the actions stored in Knox.
pub fn query_actions(db: &KnoxDb, filter: &QueryFilter) -> KnoxResult<QueryResult> {
    let start_instant = std::time::Instant::now();
    let conn = db.conn();
    let (where_clause, params) = build_where_clause(db, filter)?;

    // Clamp the limit to prevent resource exhaustion from callers requesting
    // very large result sets. The token budget still constrains the response.
    let limit = filter.limit.min(200);

    let fetched = fetch_query_actions(conn, &where_clause, &params, limit + 1)?;
    let has_more = fetched.len() > limit;
    let results: Vec<KnoxAction> = fetched.into_iter().take(limit).collect();
    let total_count = if !has_more && filter.cursor.is_none() {
        results.len()
    } else {
        fetch_query_count(conn, &where_clause, &params)?
    };
    let reason = if results.is_empty() {
        Some(
            crate::storage::telemetry::determine_empty_reason(
                db,
                filter.project.as_deref(),
                filter.file_path.as_deref(),
            )
            .to_string(),
        )
    } else {
        None
    };

    tracing::debug!(
        elapsed_ms = start_instant.elapsed().as_millis(),
        total_count,
        returned = results.len(),
        "query_actions completed"
    );

    let next = if has_more && !results.is_empty() {
        results.last().map(|last| {
            encode_cursor(
                last.timestamp.timestamp_millis(),
                last.id.as_str(),
            )
        })
    } else {
        None
    };

    Ok(QueryResult {
        results,
        total_count,
        has_more,
        next,
        reason,
    })
}

/// Executes COUNT query for matched actions.
fn fetch_query_count(
    conn: &rusqlite::Connection,
    where_clause: &str,
    params: &[SqlParam],
) -> KnoxResult<usize> {
    let count_sql = format!("SELECT COUNT(*) FROM actions a WHERE {where_clause}");
    let mut stmt = conn.prepare(&count_sql)?;
    let count: i64 = stmt.query_row(params_from_iter(params.iter()), |r| r.get(0))?;
    Ok(count as usize)
}

type ActionDbRow = (
    String,
    String,
    String,
    String,
    Option<f64>,
    String,
    Option<String>,
    Option<String>,
    Option<String>,
    i32,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<i32>,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
    i64,
);

/// Maps one SQLite row (in the canonical select order) to the raw action tuple.
fn map_action_db_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<ActionDbRow> {
    Ok((
        row.get::<_, String>(0)?,
        row.get::<_, String>(1)?,
        row.get::<_, String>(2)?,
        row.get::<_, String>(3)?,
        row.get::<_, Option<f64>>(4)?,
        row.get::<_, String>(5)?,
        row.get::<_, Option<String>>(6)?,
        row.get::<_, Option<String>>(7)?,
        row.get::<_, Option<String>>(8)?,
        row.get::<_, i32>(9)?,
        row.get::<_, Option<String>>(10)?,
        row.get::<_, Option<String>>(11)?,
        row.get::<_, Option<String>>(12)?,
        row.get::<_, Option<String>>(13)?,
        row.get::<_, Option<i32>>(14)?,
        row.get::<_, Option<String>>(15)?,
        row.get::<_, Option<String>>(16)?,
        row.get::<_, Option<String>>(17)?,
        row.get::<_, Option<String>>(18)?,
        row.get::<_, i64>(19)?,
    ))
}

/// Hydrates a vector of `ActionDbRow`s into complete `KnoxAction`s
/// by executing 5 batched child queries (one per child table) using SQL `IN (...)`.
fn hydrate_action_rows(
    conn: &rusqlite::Connection,
    rows: Vec<ActionDbRow>,
) -> KnoxResult<Vec<KnoxAction>> {
    if rows.is_empty() {
        return Ok(Vec::new());
    }

    let id_params: Vec<&str> = rows.iter().map(|r| r.0.as_str()).collect();
    let mut files: HashMap<String, Vec<String>> = HashMap::new();
    let mut tags: HashMap<String, Vec<String>> = HashMap::new();
    let mut relations: HashMap<String, Vec<ActionRelation>> = HashMap::new();
    let mut symbols: HashMap<String, Vec<String>> = HashMap::new();
    let mut deltas: HashMap<String, DeltaSummary> = HashMap::new();

    for chunk in id_params.chunks(500) {
        let placeholders = vec!["?"; chunk.len()].join(",");

        let mut file_stmt = conn.prepare(&format!(
            "SELECT action_id, file_path FROM action_files WHERE action_id IN ({placeholders}) ORDER BY file_path"
        ))?;
        for row in file_stmt.query_map(params_from_iter(chunk.iter()), |r| {
            Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))
        })? {
            let (action_id, value) = row?;
            files.entry(action_id).or_default().push(value);
        }

        let mut tag_stmt = conn.prepare(&format!(
            "SELECT action_id, tag FROM action_tags WHERE action_id IN ({placeholders}) ORDER BY tag"
        ))?;
        for row in tag_stmt.query_map(params_from_iter(chunk.iter()), |r| {
            Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))
        })? {
            let (action_id, value) = row?;
            tags.entry(action_id).or_default().push(value);
        }

        let mut rel_stmt = conn.prepare(&format!(
            "SELECT from_action_id, to_action_id, relation_type FROM action_relations
             WHERE from_action_id IN ({placeholders}) ORDER BY to_action_id"
        ))?;
        for row in rel_stmt.query_map(params_from_iter(chunk.iter()), |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
            ))
        })? {
            let (action_id, target, rel_type) = row?;
            relations
                .entry(action_id)
                .or_default()
                .push(ActionRelation {
                    target_id: ActionId::from(target),
                    relation_type: rel_type
                        .parse::<RelationType>()
                        .unwrap_or(RelationType::RelatesTo),
                });
        }

        let mut sym_stmt = conn.prepare(&format!(
            "SELECT action_id, symbol FROM action_symbols WHERE action_id IN ({placeholders}) ORDER BY symbol"
        ))?;
        for row in sym_stmt.query_map(params_from_iter(chunk.iter()), |r| {
            Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))
        })? {
            let (action_id, value) = row?;
            symbols.entry(action_id).or_default().push(value);
        }

        let mut delta_stmt = conn.prepare(&format!(
            "SELECT action_id, COUNT(DISTINCT file_path), COALESCE(SUM(lines_added), 0), COALESCE(SUM(lines_deleted), 0)
             FROM action_deltas WHERE action_id IN ({placeholders}) GROUP BY action_id"
        ))?;
        for row in delta_stmt.query_map(params_from_iter(chunk.iter()), |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, i64>(1)?,
                r.get::<_, i64>(2)?,
                r.get::<_, i64>(3)?,
            ))
        })? {
            let (action_id, files_changed, added, deleted) = row?;
            if files_changed > 0 || added > 0 || deleted > 0 {
                deltas.insert(
                    action_id,
                    DeltaSummary {
                        total_files: files_changed as usize,
                        lines_added: added.max(0) as u32,
                        lines_deleted: deleted.max(0) as u32,
                    },
                );
            }
        }
    }

    let mut actions = Vec::with_capacity(rows.len());
    for row in rows {
        let key = &row.0;
        actions.push(build_action_from_row(
            &row,
            files.remove(key).unwrap_or_default(),
            tags.remove(key).unwrap_or_default(),
            relations.remove(key).unwrap_or_default(),
            symbols.remove(key).unwrap_or_default(),
            deltas.remove(key),
        ));
    }
    Ok(actions)
}

/// Fetches and populates actions along with their associated files, tags, relations, and symbols.
fn fetch_query_actions(
    conn: &rusqlite::Connection,
    where_clause: &str,
    params: &[SqlParam],
    limit: usize,
) -> KnoxResult<Vec<KnoxAction>> {
    let select_sql = format!(
        "SELECT a.id, a.actor, a.action_type, a.summary, a.confidence,
                a.timestamp, a.session_id, a.parent_action_id, a.superseded_by, a.is_current,
                a.rationale, a.context_summary, a.rejected_alternatives,
                a.significance, a.is_resolved, a.resolution_note, a.commit_sha, a.project_id,
                a.dedupe_key, a.seen_count
         FROM actions a WHERE {where_clause}
         ORDER BY a.timestamp_ms DESC, a.id DESC LIMIT {limit}"
    );

    let mut stmt = conn.prepare(&select_sql)?;
    let action_rows = stmt
        .query_map(params_from_iter(params.iter()), map_action_db_row)?
        .collect::<Result<Vec<ActionDbRow>, _>>()?;

    hydrate_action_rows(conn, action_rows)
}

/// Executes full-text search against action summaries using the SQLite FTS5 table,
/// optionally scoped to a project and file path.
pub fn search_actions_scoped(
    db: &KnoxDb,
    query: &str,
    project: Option<&str>,
    file_path: Option<&str>,
    limit: usize,
) -> KnoxResult<Vec<KnoxAction>> {
    let conn = db.conn();
    let sanitized_query = sanitize_fts_query(query);

    let mut sql = "SELECT a.id FROM actions_fts fts JOIN actions a ON fts.rowid = a.rowid WHERE actions_fts MATCH ?".to_string();
    let mut params: Vec<SqlParam> = vec![SqlParam::Text(sanitized_query)];

    if let Some(p) = project {
        match crate::storage::sessions::resolve_project_identifier(db, p)? {
            Some(project_id) => {
                sql.push_str(" AND (a.project_id = ? OR a.session_id IN (SELECT s.id FROM sessions s WHERE s.project_id = ?))");
                params.push(SqlParam::Text(project_id.clone()));
                params.push(SqlParam::Text(project_id));
            }
            None => {
                sql.push_str(" AND 0=1");
            }
        }
    }

    if let Some(f) = file_path {
        sql.push_str(" AND a.id IN (SELECT af.action_id FROM action_files af WHERE af.file_path = ?)");
        params.push(SqlParam::Text(f.to_string()));
    }

    sql.push_str(" ORDER BY rank LIMIT ?");
    params.push(SqlParam::Int(limit as i64));

    let mut stmt = conn.prepare(&sql)?;
    let to_sql_params: Vec<&dyn ToSql> = params.iter().map(|p| p as &dyn ToSql).collect();
    let action_ids = stmt
        .query_map(params_from_iter(to_sql_params), |r| {
            r.get::<_, String>(0).map(ActionId::from)
        })?
        .collect::<Result<Vec<_>, _>>()?;

    get_actions_by_ids(db, &action_ids)
}

/// Executes full-text search against action summaries using the SQLite FTS5 table.
pub fn search_actions(db: &KnoxDb, query: &str, limit: usize) -> KnoxResult<Vec<KnoxAction>> {
    search_actions_scoped(db, query, None, None, limit)
}

/// Fetches a set of actions and all of their child rows using batched queries.
///
/// Order follows `ids`, so FTS rank ordering survives. This replaces the former
/// per-hit `get_action` loop (five statements per result).
pub fn get_actions_by_ids(db: &KnoxDb, ids: &[ActionId]) -> KnoxResult<Vec<KnoxAction>> {
    if ids.is_empty() {
        return Ok(Vec::new());
    }

    let conn = db.conn();
    let mut rows = Vec::with_capacity(ids.len());

    for chunk in ids.chunks(500) {
        let placeholders = vec!["?"; chunk.len()].join(",");
        let chunk_params: Vec<&str> = chunk.iter().map(|id| id.as_str()).collect();

        let select_sql = format!(
            "SELECT a.id, a.actor, a.action_type, a.summary, a.confidence,
                    a.timestamp, a.session_id, a.parent_action_id, a.superseded_by, a.is_current,
                    a.rationale, a.context_summary, a.rejected_alternatives,
                    a.significance, a.is_resolved, a.resolution_note, a.commit_sha, a.project_id,
                    a.dedupe_key, a.seen_count
             FROM actions a WHERE a.id IN ({placeholders})"
        );
        let mut stmt = conn.prepare(&select_sql)?;
        let chunk_rows = stmt
            .query_map(params_from_iter(chunk_params.iter()), map_action_db_row)?
            .collect::<Result<Vec<ActionDbRow>, _>>()?;
        rows.extend(chunk_rows);
    }

    let actions = hydrate_action_rows(conn, rows)?;
    let mut by_id: HashMap<String, KnoxAction> =
        actions.into_iter().map(|a| (a.id.to_string(), a)).collect();

    let mut ordered = Vec::with_capacity(ids.len());
    for id in ids {
        if let Some(act) = by_id.remove(id.as_str()) {
            ordered.push(act);
        }
    }
    Ok(ordered)
}

/// Builds a `KnoxAction` from a raw row plus pre-fetched child collections.
fn build_action_from_row(
    row: &ActionDbRow,
    files: Vec<String>,
    tags: Vec<String>,
    relations: Vec<ActionRelation>,
    symbols: Vec<String>,
    delta_summary: Option<DeltaSummary>,
) -> KnoxAction {
    let (
        id,
        actor,
        action_type,
        summary,
        confidence,
        ts_str,
        s_id,
        p_id,
        sup_by,
        is_curr,
        rationale,
        context_summary,
        rejected_str,
        significance,
        is_resolved,
        resolution_note,
        commit_sha,
        project_id,
        dedupe_key,
        seen_count,
    ) = row.clone();

    let timestamp = DateTime::parse_from_rfc3339(&ts_str)
        .map(|dt| dt.with_timezone(&Utc))
        .unwrap_or_else(|_| Utc::now());
    let rejected_alternatives = rejected_str
        .and_then(|s| serde_json::from_str::<Vec<String>>(&s).ok())
        .unwrap_or_default();

    KnoxAction {
        id: ActionId::from(id),
        actor,
        action_type,
        summary,
        files,
        tags,
        confidence,
        timestamp,
        session_id: s_id.map(SessionId::from),
        project_id: project_id.map(crate::core::ids::ProjectId::from),
        parent_action_id: p_id.map(ActionId::from),
        supersedes: None,
        is_current: is_curr == 1 && sup_by.is_none(),
        relations,
        symbols,
        rationale,
        rejected_alternatives,
        context_summary,
        significance: significance.or_else(|| Some("major".to_string())),
        is_resolved: is_resolved.map(|v| v == 1),
        resolution_note,
        commit_sha,
        delta_summary,
        dedupe_key,
        seen_count: seen_count.max(1) as u32,
    }
}

/// Aggregates database-wide status metrics.
pub fn get_status_report(db: &KnoxDb) -> KnoxResult<StatusReport> {
    let conn = db.conn();

    let total_actions: i64 = conn.query_row("SELECT COUNT(*) FROM actions", [], |r| r.get(0))?;
    let total_sessions: i64 = conn.query_row("SELECT COUNT(*) FROM sessions", [], |r| r.get(0))?;
    let projects_tracked: i64 =
        conn.query_row("SELECT COUNT(*) FROM projects", [], |r| r.get(0))?;

    let last_action_ms: Option<i64> = conn
        .query_row("SELECT MAX(timestamp_ms) FROM actions", [], |r| r.get(0))
        .unwrap_or(None);

    let last_action_timestamp = last_action_ms.and_then(DateTime::from_timestamp_millis);

    // Actions by actor
    let mut actor_stmt = conn.prepare("SELECT actor, COUNT(*) FROM actions GROUP BY actor")?;
    let mut actions_by_actor = HashMap::new();
    let actor_rows =
        actor_stmt.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?)))?;
    for row in actor_rows {
        let (actor, count) = row?;
        actions_by_actor.insert(actor, count as usize);
    }

    // Actions by type
    let mut type_stmt =
        conn.prepare("SELECT action_type, COUNT(*) FROM actions GROUP BY action_type")?;
    let mut actions_by_type = HashMap::new();
    let type_rows =
        type_stmt.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?)))?;
    for row in type_rows {
        let (atype, count) = row?;
        actions_by_type.insert(atype, count as usize);
    }

    Ok(StatusReport {
        total_actions: total_actions as usize,
        total_sessions: total_sessions as usize,
        projects_tracked: projects_tracked as usize,
        last_action_timestamp,
        actions_by_actor,
        actions_by_type,
    })
}

/// Dynamic SQL parameter wrapper.
enum SqlParam {
    Text(String),
    Int(i64),
}

impl ToSql for SqlParam {
    fn to_sql(&self) -> rusqlite::Result<ToSqlOutput<'_>> {
        match self {
            SqlParam::Text(s) => Ok(ToSqlOutput::from(s.as_str())),
            SqlParam::Int(i) => Ok(ToSqlOutput::from(*i)),
        }
    }
}

/// Appends file path matching clauses to the dynamic query.
///
/// Matching is segment-aware in both directions so an absolute write and a
/// relative query resolve to the same row (and vice versa). With
/// `exact == true` only the exact path is compared, which lets guardrails query
/// a directory scope deliberately.
fn append_file_filter(
    clauses: &mut Vec<String>,
    params: &mut Vec<SqlParam>,
    file_path: &str,
    exact: bool,
) {
    let ref_target = format!("file:{file_path}");
    if exact {
        clauses.push(
            "a.id IN (
                SELECT af.action_id FROM action_files af WHERE af.file_path = ?
                UNION
                SELECT e.from_action_id FROM edges e WHERE e.to_id = ?
            )"
            .to_string(),
        );
        params.push(SqlParam::Text(file_path.to_string()));
        params.push(SqlParam::Text(ref_target));
        return;
    }

    let trimmed = file_path.trim_end_matches('/');
    let escaped = trimmed
        .replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_");
    let ref_prefix_like = format!("file:{escaped}/%");

    clauses.push(
        "a.id IN (
            SELECT af.action_id FROM action_files af WHERE af.file_path = ? OR af.file_path LIKE ? ESCAPE '\\'
            UNION
            SELECT e.from_action_id FROM edges e WHERE e.to_id = ? OR e.to_id LIKE ? ESCAPE '\\'
        )"
        .to_string(),
    );
    params.push(SqlParam::Text(file_path.to_string()));
    params.push(SqlParam::Text(format!("{escaped}/%")));
    params.push(SqlParam::Text(ref_target));
    params.push(SqlParam::Text(ref_prefix_like));
}

/// Appends project matching clauses to the dynamic query.
///
/// The identifier is resolved once to a project id (path or unique name), then
/// matched against `actions.project_id` directly. Falling back to the session
/// join keeps rows written before the project column existed visible.
fn append_project_filter(
    db: &KnoxDb,
    clauses: &mut Vec<String>,
    params: &mut Vec<SqlParam>,
    project: &str,
) -> KnoxResult<()> {
    match crate::storage::sessions::resolve_project_identifier(db, project)? {
        Some(project_id) => {
            clauses.push(
                "(a.project_id = ? OR a.session_id IN (SELECT s.id FROM sessions s WHERE s.project_id = ?))"
                    .to_string(),
            );
            params.push(SqlParam::Text(project_id.clone()));
            params.push(SqlParam::Text(project_id));
        }
        None => {
            // An unknown project must return nothing, not everything: a scoped
            // query that silently widens is the fail-open pattern this audit
            // exists to remove.
            clauses.push("0=1".to_string());
        }
    }
    Ok(())
}

/// Constructs dynamic SQL WHERE clauses and bound parameters from `QueryFilter`.
fn build_where_clause(db: &KnoxDb, filter: &QueryFilter) -> KnoxResult<(String, Vec<SqlParam>)> {
    let mut clauses = Vec::new();
    let mut params = Vec::new();

    if filter.current_only {
        clauses.push("a.is_current = 1 AND a.superseded_by IS NULL".to_string());
    }
    if let Some(ref actor) = filter.actor {
        clauses.push("a.actor = ?".to_string());
        params.push(SqlParam::Text(actor.clone()));
    }
    if let Some(ref s_id) = filter.session_id {
        clauses.push("a.session_id = ?".to_string());
        params.push(SqlParam::Text(s_id.to_string()));
    }
    if let Some(ref since) = filter.since {
        clauses.push("a.timestamp >= ?".to_string());
        params.push(SqlParam::Text(since.to_rfc3339()));
    }
    if let Some(ref file_path) = filter.file_path {
        append_file_filter(&mut clauses, &mut params, file_path, filter.file_path_exact);
    }
    if let Some(ref project) = filter.project {
        append_project_filter(db, &mut clauses, &mut params, project)?;
    }
    for tag in &filter.tags {
        clauses.push(
            "a.id IN (SELECT at.action_id FROM action_tags at WHERE at.tag = ?)"
                .to_string(),
        );
        params.push(SqlParam::Text(tag.clone()));
    }
    if let Some(ref at) = filter.action_type {
        if at == "decide" || at == "decision" {
            clauses.push("(a.action_type = 'decide' OR a.action_type = 'decision')".to_string());
        } else if at == "doubt" || at == "question" {
            clauses.push("(a.action_type = 'doubt' OR a.action_type = 'question')".to_string());
        } else {
            clauses.push("a.action_type = ?".to_string());
            params.push(SqlParam::Text(at.clone()));
        }
    }
    for sym in &filter.symbols {
        clauses.push(
            "a.id IN (SELECT asy.action_id FROM action_symbols asy WHERE asy.symbol = ?)"
                .to_string(),
        );
        params.push(SqlParam::Text(sym.clone()));
    }
    if let Some(ref q) = filter.query {
        clauses.push(
            "a.rowid IN (SELECT fts.rowid FROM actions_fts fts WHERE actions_fts MATCH ?)"
                .to_string(),
        );
        params.push(SqlParam::Text(sanitize_fts_query(q)));
    }
    if let Some(ref cur) = filter.cursor {
        let (ts, id) = decode_cursor(cur).ok_or_else(|| {
            crate::core::error::KnoxError::InvalidInput(format!("Invalid pagination cursor: '{cur}'"))
        })?;
        clauses.push(
            "(a.timestamp_ms < ? OR (a.timestamp_ms = ? AND a.id < ?))"
                .to_string(),
        );
        params.push(SqlParam::Int(ts));
        params.push(SqlParam::Int(ts));
        params.push(SqlParam::Text(id));
    }

    let where_clause = if clauses.is_empty() {
        "1=1".to_string()
    } else {
        clauses.join(" AND ")
    };

    Ok((where_clause, params))
}

/// Sanitizes an input string for safe usage in SQLite FTS5 MATCH queries.
fn sanitize_fts_query(input: &str) -> String {
    let clean: String = input
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '_' {
                c
            } else {
                ' '
            }
        })
        .collect();
    let words: Vec<&str> = clean.split_whitespace().collect();
    if words.is_empty() {
        "\"\"".to_string()
    } else {
        words.join(" ")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::nodes::insert_action;

    fn make_test_action(
        actor: &str,
        action_type: &str,
        summary: &str,
        file: &str,
        tag: &str,
    ) -> KnoxAction {
        KnoxAction {
            id: ActionId::new(),
            actor: actor.to_string(),
            action_type: action_type.to_string(),
            summary: summary.to_string(),
            files: vec![file.to_string()],
            tags: vec![tag.to_string()],
            confidence: None,
            timestamp: Utc::now(),
            session_id: None,
            parent_action_id: None,
            supersedes: None,
            is_current: true,
            ..Default::default()
        }
    }

    #[test]
    fn test_query_filtering_by_tag_and_actor() {
        let db = KnoxDb::open_memory().unwrap();
        let action1 = make_test_action("agent_a", "plan", "Alpha task", "file1.rs", "backend");
        let action2 = make_test_action("agent_b", "write", "Beta task", "file2.rs", "frontend");

        insert_action(&db, &action1).unwrap();
        insert_action(&db, &action2).unwrap();

        let filter = QueryFilter {
            actor: Some("agent_a".to_string()),
            ..Default::default()
        };
        let res = query_actions(&db, &filter).unwrap();
        assert_eq!(res.results.len(), 1);
        assert_eq!(res.results[0].id, action1.id);

        let tag_filter = QueryFilter {
            tags: vec!["frontend".to_string()],
            ..Default::default()
        };
        let res_tag = query_actions(&db, &tag_filter).unwrap();
        assert_eq!(res_tag.results.len(), 1);
        assert_eq!(res_tag.results[0].id, action2.id);
    }

    #[test]
    fn test_fts_search() {
        let db = KnoxDb::open_memory().unwrap();

        let action = KnoxAction {
            id: ActionId::new(),
            actor: "developer".to_string(),
            action_type: "debug".to_string(),
            summary: "Investigating intermittent network latency spike".to_string(),
            files: vec![],
            tags: vec![],
            confidence: None,
            timestamp: Utc::now(),
            session_id: None,
            parent_action_id: None,
            supersedes: None,
            is_current: true,
            ..Default::default()
        };
        insert_action(&db, &action).unwrap();

        let results = search_actions(&db, "intermittent latency", 10).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, action.id);
    }
}
