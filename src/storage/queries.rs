//! Dynamic query builder and search engine for Knox knowledge graph.

use chrono::{DateTime, Utc};
use rusqlite::types::ToSqlOutput;
use rusqlite::{ToSql, params_from_iter};
use std::collections::HashMap;

use super::db::KnoxDb;
use crate::core::error::KnoxResult;
use crate::core::ids::{ActionId, SessionId};
use crate::core::types::{KnoxAction, QueryFilter, QueryResult, StatusReport};

/// Executes a flexible filtered query against the actions stored in Knox.
pub fn query_actions(db: &KnoxDb, filter: &QueryFilter) -> KnoxResult<QueryResult> {
    let start_instant = std::time::Instant::now();
    let conn = db.conn();
    let (where_clause, params) = build_where_clause(filter);

    let total_count = fetch_query_count(conn, &where_clause, &params)?;
    let results = fetch_query_actions(conn, &where_clause, &params, filter.limit)?;
    let has_more = total_count > results.len();

    tracing::debug!(
        elapsed_ms = start_instant.elapsed().as_millis(),
        total_count,
        returned = results.len(),
        "query_actions completed"
    );

    Ok(QueryResult {
        results,
        total_count,
        has_more,
    })
}

/// Executes COUNT query for matched actions.
fn fetch_query_count(
    conn: &rusqlite::Connection,
    where_clause: &str,
    params: &[SqlParam],
) -> KnoxResult<usize> {
    let count_sql = format!("SELECT COUNT(DISTINCT a.id) FROM actions a WHERE {where_clause}");
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
);

/// Fetches and populates actions along with their associated files and tags.
fn fetch_query_actions(
    conn: &rusqlite::Connection,
    where_clause: &str,
    params: &[SqlParam],
    limit: usize,
) -> KnoxResult<Vec<KnoxAction>> {
    let select_sql = format!(
        "SELECT a.id, a.actor, a.action_type, a.summary, a.confidence,
                a.timestamp, a.session_id, a.parent_action_id, a.superseded_by, a.is_current
         FROM actions a WHERE {where_clause} ORDER BY a.timestamp DESC LIMIT {limit}"
    );

    let mut stmt = conn.prepare(&select_sql)?;
    let action_rows = stmt
        .query_map(params_from_iter(params.iter()), |row| {
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
            ))
        })?
        .collect::<Result<Vec<ActionDbRow>, _>>()?;

    let mut file_stmt = conn.prepare_cached(
        "SELECT file_path FROM action_files WHERE action_id = ?1 ORDER BY file_path",
    )?;
    let mut tag_stmt =
        conn.prepare_cached("SELECT tag FROM action_tags WHERE action_id = ?1 ORDER BY tag")?;

    map_action_rows(action_rows, &mut file_stmt, &mut tag_stmt)
}

/// Maps raw database rows into complete KnoxAction structures.
fn map_action_rows(
    rows: Vec<ActionDbRow>,
    file_stmt: &mut rusqlite::CachedStatement,
    tag_stmt: &mut rusqlite::CachedStatement,
) -> KnoxResult<Vec<KnoxAction>> {
    let mut results = Vec::with_capacity(rows.len());
    for (id, actor, action_type, summary, confidence, ts_str, s_id, p_id, sup_by, is_curr) in rows {
        let timestamp = DateTime::parse_from_rfc3339(&ts_str)
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now());

        let files = file_stmt
            .query_map(params_from_iter([&id]), |r| r.get::<_, String>(0))?
            .collect::<Result<Vec<String>, _>>()?;

        let tags = tag_stmt
            .query_map(params_from_iter([&id]), |r| r.get::<_, String>(0))?
            .collect::<Result<Vec<String>, _>>()?;

        results.push(KnoxAction {
            id: ActionId::from(id),
            actor,
            action_type,
            summary,
            files,
            tags,
            confidence,
            timestamp,
            session_id: s_id.map(SessionId::from),
            parent_action_id: p_id.map(ActionId::from),
            supersedes: None,
            is_current: is_curr == 1 && sup_by.is_none(),
        });
    }
    Ok(results)
}

/// Executes full-text search against action summaries using the SQLite FTS5 table.
pub fn search_actions(db: &KnoxDb, query: &str, limit: usize) -> KnoxResult<Vec<KnoxAction>> {
    let conn = db.conn();
    let sanitized_query = sanitize_fts_query(query);

    let mut stmt = conn.prepare(
        "SELECT a.id
         FROM actions_fts fts
         JOIN actions a ON fts.rowid = a.rowid
         WHERE actions_fts MATCH ?1
         ORDER BY rank
         LIMIT ?2",
    )?;

    let action_ids = stmt
        .query_map(
            params_from_iter([&sanitized_query, &limit.to_string()]),
            |r| r.get::<_, String>(0).map(ActionId::from),
        )?
        .collect::<Result<Vec<_>, _>>()?;

    let mut actions = Vec::with_capacity(action_ids.len());
    for id in action_ids {
        if let Some(act) = super::nodes::get_action(db, &id)? {
            actions.push(act);
        }
    }

    Ok(actions)
}

/// Aggregates database-wide status metrics.
pub fn get_status_report(db: &KnoxDb) -> KnoxResult<StatusReport> {
    let conn = db.conn();

    let total_actions: i64 = conn.query_row("SELECT COUNT(*) FROM actions", [], |r| r.get(0))?;
    let total_sessions: i64 = conn.query_row("SELECT COUNT(*) FROM sessions", [], |r| r.get(0))?;
    let projects_tracked: i64 =
        conn.query_row("SELECT COUNT(*) FROM projects", [], |r| r.get(0))?;

    let last_action_str: Option<String> = conn
        .query_row("SELECT MAX(timestamp) FROM actions", [], |r| r.get(0))
        .unwrap_or(None);

    let last_action_timestamp = last_action_str.and_then(|s| {
        DateTime::parse_from_rfc3339(&s)
            .ok()
            .map(|dt| dt.with_timezone(&Utc))
    });

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
}

impl ToSql for SqlParam {
    fn to_sql(&self) -> rusqlite::Result<ToSqlOutput<'_>> {
        match self {
            SqlParam::Text(s) => Ok(ToSqlOutput::from(s.as_str())),
        }
    }
}

/// Appends file path matching clauses to the dynamic query.
fn append_file_filter(clauses: &mut Vec<String>, params: &mut Vec<SqlParam>, file_path: &str) {
    clauses.push(
        "EXISTS (SELECT 1 FROM action_files af WHERE af.action_id = a.id AND (af.file_path = ? OR af.file_path LIKE ?))"
            .to_string(),
    );
    params.push(SqlParam::Text(file_path.to_string()));
    params.push(SqlParam::Text(format!(
        "{}/%",
        file_path.trim_end_matches('/')
    )));
}

/// Appends project matching clauses to the dynamic query.
fn append_project_filter(clauses: &mut Vec<String>, params: &mut Vec<SqlParam>, project: &str) {
    clauses.push(
        "EXISTS (SELECT 1 FROM sessions s LEFT JOIN projects p ON s.project_id = p.id WHERE s.id = a.session_id AND (s.project_id = ? OR p.name = ? OR p.path = ?))"
            .to_string(),
    );
    params.push(SqlParam::Text(project.to_string()));
    params.push(SqlParam::Text(project.to_string()));
    params.push(SqlParam::Text(project.to_string()));
}

/// Constructs dynamic SQL WHERE clauses and bound parameters from `QueryFilter`.
fn build_where_clause(filter: &QueryFilter) -> (String, Vec<SqlParam>) {
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
        append_file_filter(&mut clauses, &mut params, file_path);
    }
    if let Some(ref project) = filter.project {
        append_project_filter(&mut clauses, &mut params, project);
    }
    for tag in &filter.tags {
        clauses.push(
            "EXISTS (SELECT 1 FROM action_tags at WHERE at.action_id = a.id AND at.tag = ?)"
                .to_string(),
        );
        params.push(SqlParam::Text(tag.clone()));
    }

    let where_clause = if clauses.is_empty() {
        "1=1".to_string()
    } else {
        clauses.join(" AND ")
    };

    (where_clause, params)
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
        };
        insert_action(&db, &action).unwrap();

        let results = search_actions(&db, "intermittent latency", 10).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, action.id);
    }
}
