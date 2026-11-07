//! Node CRUD operations for Knox actions and entities in SQLite.

use chrono::{DateTime, Utc};
use rusqlite::params;

use super::db::KnoxDb;
use crate::core::error::KnoxResult;
use crate::core::ids::{ActionId, ProjectId, SessionId};
use crate::core::types::{
    ActionDelta, ActionRelation, DeltaChangeType, DeltaSummary, KnoxAction, Project, RelationType,
    UpdateActionParams,
};

pub use super::queries::query_actions;

/// Inserts a new `KnoxAction` into the database, populating files, tags,
/// relations, symbols, and handling supersede relationships.
///
/// The whole write runs in one transaction so a failure cannot leave an action
/// row without its files, tags, relations or symbols.
pub fn insert_action(db: &KnoxDb, action: &KnoxAction) -> KnoxResult<ActionId> {
    db.with_tx(|conn| insert_action_conn(conn, action))
}

/// Inserts an action using an existing transaction/connection.
pub(crate) fn insert_action_conn(
    conn: &rusqlite::Connection,
    action: &KnoxAction,
) -> KnoxResult<ActionId> {
    let rejected_alt_json = if action.rejected_alternatives.is_empty() {
        None
    } else {
        Some(serde_json::to_string(&action.rejected_alternatives).unwrap_or_default())
    };

    let is_resolved_int = match action.is_resolved {
        Some(false) => 0,
        _ => 1,
    };
    let significance_str = action.significance.as_deref().unwrap_or("major");

    conn.execute(
        "INSERT INTO actions (
            id, actor, action_type, summary, confidence, timestamp,
            session_id, parent_action_id, superseded_by, is_current,
            rationale, context_summary, rejected_alternatives,
            significance, is_resolved, resolution_note, commit_sha,
            project_id, timestamp_ms, dedupe_key, seen_count
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21)",
        params![
            action.id.as_str(),
            action.actor.as_str(),
            action.action_type.as_str(),
            action.summary.as_str(),
            action.confidence,
            action.timestamp.to_rfc3339(),
            action.session_id.as_ref().map(|s| s.as_str()),
            action.parent_action_id.as_ref().map(|p| p.as_str()),
            None::<&str>,
            if action.is_current { 1 } else { 0 },
            action.rationale.as_deref(),
            action.context_summary.as_deref(),
            rejected_alt_json.as_deref(),
            significance_str,
            is_resolved_int,
            action.resolution_note.as_deref(),
            action.commit_sha.as_deref(),
            action.project_id.as_ref().map(|p| p.as_str()),
            action.timestamp.timestamp_millis(),
            action.dedupe_key.as_deref(),
            action.seen_count.max(1),
        ],
    )?;

    if let Some(ref old_id) = action.supersedes {
        mark_superseded_conn(conn, old_id, &action.id)?;
    }

    insert_action_files_and_tags(conn, &action.id, &action.files, &action.tags)?;
    insert_action_relations_and_symbols(conn, &action.id, &action.relations, &action.symbols)?;
    if let Some(note) = action.resolution_note.as_deref() {
        let now = chrono::Utc::now().to_rfc3339();
        for rel in &action.relations {
            if rel.relation_type == RelationType::Resolves {
                let edge_id = format!("{}:{}:resolves", action.id, rel.target_id);
                conn.execute(
                    "INSERT INTO edges (id, from_action_id, to_id, edge_type, created_at, note)
                     VALUES (?1, ?2, ?3, 'resolves', ?4, ?5)
                     ON CONFLICT(id) DO UPDATE SET note = excluded.note",
                    params![edge_id, action.id.as_str(), rel.target_id.as_str(), now, note],
                )?;
            }
        }
    }
    insert_action_state(conn, action)?;
    insert_guardrail_rules(conn, action)?;

    tracing::debug!(
        id = %action.id,
        actor = %action.actor,
        action_type = %action.action_type,
        "Inserted action into knowledge graph"
    );

    Ok(action.id.clone())
}

fn insert_action_state(conn: &rusqlite::Connection, action: &KnoxAction) -> KnoxResult<()> {
    let state = if !action.is_current {
        "superseded"
    } else if matches!(action.is_resolved, Some(false)) {
        "open"
    } else if matches!(action.action_type.as_str(), "doubt" | "question") {
        "resolved"
    } else {
        "current"
    };

    conn.execute(
        "INSERT INTO action_state (action_id, state, by_action_id, updated_at)
         VALUES (?1, ?2, NULL, ?3)
         ON CONFLICT(action_id) DO UPDATE SET
           state = excluded.state,
           updated_at = excluded.updated_at",
        params![action.id.as_str(), state, action.timestamp.to_rfc3339()],
    )?;
    Ok(())
}

fn insert_guardrail_rules(conn: &rusqlite::Connection, action: &KnoxAction) -> KnoxResult<()> {
    if action.action_type != "decide" {
        return Ok(());
    }

    let project_id = action.project_id.as_ref().map(|id| id.as_str());
    for (index, alternative) in action.rejected_alternatives.iter().enumerate() {
        let pattern = alternative.trim();
        if pattern.is_empty() {
            continue;
        }
        conn.execute(
            "INSERT OR IGNORE INTO guardrail_rules
             (id, action_id, project_id, pattern_kind, pattern, severity, active)
             VALUES (?1, ?2, ?3, 'phrase', ?4, 'advisory', 1)",
            params![
                format!("{}:rejected:{index}", action.id),
                action.id.as_str(),
                project_id,
                pattern
            ],
        )?;
    }
    Ok(())
}

/// Inserts associated files and tags for a given action.
fn insert_action_files_and_tags(
    conn: &rusqlite::Connection,
    id: &ActionId,
    files: &[String],
    tags: &[String],
) -> KnoxResult<()> {
    let mut stmt_files = conn.prepare_cached(
        "INSERT OR IGNORE INTO action_files (action_id, file_path) VALUES (?1, ?2)",
    )?;
    let mut stmt_ref = conn.prepare_cached(
        "INSERT OR IGNORE INTO refs (id, kind, value, project_id, created_at)
         VALUES (?1, 'file', ?2, (SELECT p.id FROM projects p WHERE p.id = (SELECT a.project_id FROM actions a WHERE a.id = ?3)), ?4)",
    )?;
    let mut stmt_edge = conn.prepare_cached(
        "INSERT OR IGNORE INTO edges (id, from_action_id, to_id, edge_type, created_at, provenance)
         VALUES (?1, ?2, ?3, 'relates_to', ?4, (SELECT commit_sha FROM actions WHERE id = ?2))",
    )?;
    let now = chrono::Utc::now().to_rfc3339();
    for file in files {
        stmt_files.execute(params![id.as_str(), file.as_str()])?;
        let ref_id = format!("file:{file}");
        stmt_ref.execute(params![ref_id.as_str(), file.as_str(), id.as_str(), now.as_str()])?;
        let edge_id = format!("{}:{}:relates_to", id, ref_id);
        stmt_edge.execute(params![edge_id.as_str(), id.as_str(), ref_id.as_str(), now.as_str()])?;
    }

    let mut stmt_tags =
        conn.prepare_cached("INSERT OR IGNORE INTO action_tags (action_id, tag) VALUES (?1, ?2)")?;
    for tag in tags {
        stmt_tags.execute(params![id.as_str(), tag.as_str()])?;
    }
    Ok(())
}

/// Inserts associated semantic relations and code symbols for an action.
fn insert_action_relations_and_symbols(
    conn: &rusqlite::Connection,
    id: &ActionId,
    relations: &[ActionRelation],
    symbols: &[String],
) -> KnoxResult<()> {
    let mut stmt_rel = conn.prepare_cached(
        "INSERT OR IGNORE INTO action_relations (from_action_id, to_action_id, relation_type) VALUES (?1, ?2, ?3)",
    )?;
    let mut stmt_is_action = conn.prepare_cached("SELECT 1 FROM actions WHERE id = ?1")?;
    let mut stmt_ref_edge = conn.prepare_cached(
        "INSERT OR IGNORE INTO edges (id, from_action_id, to_id, edge_type, created_at, provenance)
         VALUES (?1, ?2, ?3, ?4, ?5, (SELECT commit_sha FROM actions WHERE id = ?2))",
    )?;
    let now = chrono::Utc::now().to_rfc3339();
    for rel in relations {
        let is_action = stmt_is_action.exists(params![rel.target_id.as_str()])?;
        if is_action {
            stmt_rel.execute(params![
                id.as_str(),
                rel.target_id.as_str(),
                rel.relation_type.to_string(),
            ])?;
        } else {
            let edge_id = format!("{}:{}:{}", id, rel.target_id, rel.relation_type);
            stmt_ref_edge.execute(params![
                edge_id,
                id.as_str(),
                rel.target_id.as_str(),
                rel.relation_type.to_string(),
                now,
            ])?;
        }
    }

    let mut stmt_sym = conn.prepare_cached(
        "INSERT OR IGNORE INTO action_symbols (action_id, symbol) VALUES (?1, ?2)",
    )?;
    for sym in symbols {
        stmt_sym.execute(params![id.as_str(), sym.as_str()])?;
    }

    Ok(())
}

/// Fetches associated files and tags for an action ID.
fn fetch_action_files_and_tags(
    conn: &rusqlite::Connection,
    id: &ActionId,
) -> KnoxResult<(Vec<String>, Vec<String>)> {
    let mut file_stmt = conn.prepare_cached(
        "SELECT file_path FROM action_files WHERE action_id = ?1 ORDER BY file_path",
    )?;
    let files = file_stmt
        .query_map(params![id.as_str()], |r| r.get::<_, String>(0))?
        .collect::<Result<Vec<String>, _>>()?;

    let mut tag_stmt =
        conn.prepare_cached("SELECT tag FROM action_tags WHERE action_id = ?1 ORDER BY tag")?;
    let tags = tag_stmt
        .query_map(params![id.as_str()], |r| r.get::<_, String>(0))?
        .collect::<Result<Vec<String>, _>>()?;

    Ok((files, tags))
}

/// Fetches associated relations and symbols for an action ID.
fn fetch_action_relations_and_symbols(
    conn: &rusqlite::Connection,
    id: &ActionId,
) -> KnoxResult<(Vec<ActionRelation>, Vec<String>)> {
    let mut rel_stmt = conn.prepare_cached(
        "SELECT to_action_id, relation_type FROM action_relations WHERE from_action_id = ?1 ORDER BY to_action_id",
    )?;
    let relations = rel_stmt
        .query_map(params![id.as_str()], |r| {
            let target_str: String = r.get(0)?;
            let rel_type_str: String = r.get(1)?;
            let rel_type = rel_type_str
                .parse::<RelationType>()
                .unwrap_or(RelationType::RelatesTo);
            Ok(ActionRelation {
                target_id: ActionId::from(target_str),
                relation_type: rel_type,
            })
        })?
        .collect::<Result<Vec<ActionRelation>, _>>()?;

    let mut sym_stmt = conn
        .prepare_cached("SELECT symbol FROM action_symbols WHERE action_id = ?1 ORDER BY symbol")?;
    let symbols = sym_stmt
        .query_map(params![id.as_str()], |r| r.get::<_, String>(0))?
        .collect::<Result<Vec<String>, _>>()?;

    Ok((relations, symbols))
}

/// Retrieves an action by its unique identifier, including associated files, tags, relations, symbols, commit SHA, and delta summary.
pub fn get_action(db: &KnoxDb, id: &ActionId) -> KnoxResult<Option<KnoxAction>> {
    let conn = db.conn();

    let mut stmt = conn.prepare(
        "SELECT id, actor, action_type, summary, confidence, timestamp,
                session_id, parent_action_id, superseded_by, is_current,
                rationale, context_summary, rejected_alternatives,
                significance, is_resolved, resolution_note, commit_sha, project_id,
                dedupe_key, seen_count
         FROM actions WHERE id = ?1",
    )?;

    let mut rows = stmt.query(params![id.as_str()])?;
    if let Some(row) = rows.next()? {
        let action_id = ActionId::from(row.get::<_, String>(0)?);
        let actor: String = row.get(1)?;
        let action_type: String = row.get(2)?;
        let summary: String = row.get(3)?;
        let confidence: Option<f64> = row.get(4)?;
        let timestamp_str: String = row.get(5)?;
        let session_id: Option<String> = row.get(6)?;
        let parent_action_id: Option<String> = row.get(7)?;
        let superseded_by: Option<String> = row.get(8)?;
        let is_current_int: i32 = row.get(9)?;
        let rationale: Option<String> = row.get(10)?;
        let context_summary: Option<String> = row.get(11)?;
        let rejected_str: Option<String> = row.get(12)?;
        let significance: Option<String> = row.get(13)?;
        let is_resolved_int: Option<i32> = row.get(14)?;
        let resolution_note: Option<String> = row.get(15)?;
        let commit_sha: Option<String> = row.get(16)?;
        let project_id: Option<String> = row.get(17)?;
        let dedupe_key: Option<String> = row.get(18)?;
        let seen_count: i64 = row.get(19)?;

        let timestamp = DateTime::parse_from_rfc3339(&timestamp_str)
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now());

        let (files, tags) = fetch_action_files_and_tags(conn, id)?;
        let (relations, symbols) = fetch_action_relations_and_symbols(conn, id)?;
        let delta_summary = fetch_action_delta_summary(conn, id)?;

        let rejected_alternatives = rejected_str
            .and_then(|s| serde_json::from_str::<Vec<String>>(&s).ok())
            .unwrap_or_default();

        Ok(Some(KnoxAction {
            id: action_id,
            actor,
            action_type,
            summary,
            files,
            tags,
            confidence,
            timestamp,
            session_id: session_id.map(SessionId::from),
            project_id: project_id.map(ProjectId::from),
            parent_action_id: parent_action_id.map(ActionId::from),
            supersedes: None,
            is_current: is_current_int == 1 && superseded_by.is_none(),
            relations,
            symbols,
            rationale,
            rejected_alternatives,
            context_summary,
            significance: significance.or_else(|| Some("major".to_string())),
            is_resolved: is_resolved_int.map(|v| v == 1),
            resolution_note,
            commit_sha,
            delta_summary,
            dedupe_key,
            seen_count: seen_count.max(1) as u32,
        }))
    } else {
        Ok(None)
    }
}

/// Marks an existing action as superseded by a newer action.
pub fn mark_superseded(db: &KnoxDb, old_id: &ActionId, new_id: &ActionId) -> KnoxResult<()> {
    mark_superseded_conn(db.conn(), old_id, new_id)
}

/// Connection-scoped variant of [`mark_superseded`], safe to call inside a transaction.
pub(crate) fn mark_superseded_conn(
    conn: &rusqlite::Connection,
    old_id: &ActionId,
    new_id: &ActionId,
) -> KnoxResult<()> {
    conn.execute(
        "UPDATE actions SET superseded_by = ?1, is_current = 0 WHERE id = ?2",
        params![new_id.as_str(), old_id.as_str()],
    )?;
    conn.execute(
        "UPDATE action_state
         SET state = 'superseded', by_action_id = ?1, updated_at = datetime('now')
         WHERE action_id = ?2",
        params![new_id.as_str(), old_id.as_str()],
    )?;
    Ok(())
}

/// Inserts a project record into the database.
pub fn insert_project(db: &KnoxDb, project: &Project) -> KnoxResult<ProjectId> {
    let clean_path = project
        .path
        .strip_prefix(r"\\?\")
        .unwrap_or(&project.path)
        .trim_end_matches(['\\', '/']);

    db.conn().execute(
        "INSERT INTO projects (id, name, path, remote, created_at) VALUES (?1, ?2, ?3, ?4, ?5)
         ON CONFLICT(path) DO UPDATE SET
             name = excluded.name,
             remote = COALESCE(excluded.remote, projects.remote)",
        params![
            project.id.as_str(),
            project.name.as_str(),
            clean_path,
            project.remote.as_deref(),
            project.created_at.to_rfc3339()
        ],
    )?;
    Ok(project.id.clone())
}

/// Retrieves a project record by its unique identifier.
pub fn get_project(db: &KnoxDb, id: &ProjectId) -> KnoxResult<Option<Project>> {
    let conn = db.conn();
    let mut stmt = conn.prepare("SELECT id, name, path, remote, created_at FROM projects WHERE id = ?1")?;
    let mut rows = stmt.query(params![id.as_str()])?;

    if let Some(row) = rows.next()? {
        let proj_id = ProjectId::from(row.get::<_, String>(0)?);
        let name: String = row.get(1)?;
        let path: String = row.get(2)?;
        let remote: Option<String> = row.get(3)?;
        let created_at_str: String = row.get(4)?;
        let created_at = DateTime::parse_from_rfc3339(&created_at_str)
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now());

        Ok(Some(Project {
            id: proj_id,
            name,
            path,
            remote,
            created_at,
        }))
    } else {
        Ok(None)
    }
}

/// Retrieves a project record by its canonical root path.
pub fn get_project_by_path(db: &KnoxDb, path: &str) -> KnoxResult<Option<Project>> {
    let clean = path
        .strip_prefix(r"\\?\")
        .unwrap_or(path)
        .trim_end_matches(['\\', '/']);
    let verbatim = format!(r"\\?\{clean}");

    let conn = db.conn();
    let mut stmt = conn
        .prepare("SELECT id, name, path, remote, created_at FROM projects WHERE path = ?1 OR path = ?2")?;
    let mut rows = stmt.query(params![clean, verbatim])?;

    if let Some(row) = rows.next()? {
        let proj_id = ProjectId::from(row.get::<_, String>(0)?);
        let name: String = row.get(1)?;
        let mut p: String = row.get(2)?;
        if let Some(stripped) = p.strip_prefix(r"\\?\") {
            p = stripped.to_string();
        }
        let remote: Option<String> = row.get(3)?;
        let created_at_str: String = row.get(4)?;
        let created_at = DateTime::parse_from_rfc3339(&created_at_str)
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now());

        Ok(Some(Project {
            id: proj_id,
            name,
            path: p,
            remote,
            created_at,
        }))
    } else {
        Ok(None)
    }
}

/// Retrieves a project record by its normalized Git remote URL.
pub fn get_project_by_remote(db: &KnoxDb, remote: &str) -> KnoxResult<Option<Project>> {
    let conn = db.conn();
    let mut stmt = conn.prepare(
        "SELECT id, name, path, remote, created_at FROM projects WHERE remote = ?1 ORDER BY created_at ASC LIMIT 1",
    )?;
    let mut rows = stmt.query(params![remote])?;

    if let Some(row) = rows.next()? {
        let proj_id = ProjectId::from(row.get::<_, String>(0)?);
        let name: String = row.get(1)?;
        let mut p: String = row.get(2)?;
        if let Some(stripped) = p.strip_prefix(r"\\?\") {
            p = stripped.to_string();
        }
        let remote_val: Option<String> = row.get(3)?;
        let created_at_str: String = row.get(4)?;
        let created_at = DateTime::parse_from_rfc3339(&created_at_str)
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now());

        Ok(Some(Project {
            id: proj_id,
            name,
            path: p,
            remote: remote_val,
            created_at,
        }))
    } else {
        Ok(None)
    }
}

/// Retrieves all projects ordered by creation time.
/// Normalizes paths by stripping Windows verbatim prefix `\\?\` if present.
pub fn list_projects(db: &KnoxDb) -> KnoxResult<Vec<Project>> {
    let conn = db.conn();
    let mut stmt =
        conn.prepare("SELECT id, name, path, remote, created_at FROM projects ORDER BY created_at ASC")?;
    let mut rows = stmt.query([])?;
    let mut projects = Vec::new();

    while let Some(row) = rows.next()? {
        let proj_id = ProjectId::from(row.get::<_, String>(0)?);
        let name: String = row.get(1)?;
        let mut path: String = row.get(2)?;
        if let Some(stripped) = path.strip_prefix(r"\\?\") {
            path = stripped.to_string();
        }
        let remote: Option<String> = row.get(3)?;
        let created_at_str: String = row.get(4)?;
        let created_at = DateTime::parse_from_rfc3339(&created_at_str)
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now());

        projects.push(Project {
            id: proj_id,
            name,
            path,
            remote,
            created_at,
        });
    }

    Ok(projects)
}

/// Marks an existing doubt action as resolved with an explanation, optionally linking to a resolver action.
pub fn resolve_doubt(
    db: &KnoxDb,
    doubt_id: &ActionId,
    resolution_note: &str,
    resolver_action_id: Option<&ActionId>,
) -> KnoxResult<()> {
    let conn = db.conn();
    let rows_affected = conn.execute(
        "UPDATE actions SET is_resolved = 1, resolution_note = ?1 WHERE id = ?2",
        params![resolution_note, doubt_id.as_str()],
    )?;

    if rows_affected == 0 {
        return Err(crate::core::error::KnoxError::ActionNotFound(
            doubt_id.to_string(),
        ));
    }

    if let Some(resolver_id) = resolver_action_id {
        conn.execute(
            "INSERT OR IGNORE INTO action_relations (from_action_id, to_action_id, relation_type)
             VALUES (?1, ?2, ?3)",
            params![resolver_id.as_str(), doubt_id.as_str(), "resolves"],
        )?;
        let edge_id = format!("{}:{}:resolves", resolver_id, doubt_id);
        let now = chrono::Utc::now().to_rfc3339();
        conn.execute(
            "INSERT INTO edges (id, from_action_id, to_id, edge_type, created_at, note)
             VALUES (?1, ?2, ?3, 'resolves', ?4, ?5)
             ON CONFLICT(id) DO UPDATE SET note = excluded.note",
            params![edge_id, resolver_id.as_str(), doubt_id.as_str(), now, resolution_note],
        )?;
    }

    Ok(())
}

/// Updates an existing `KnoxAction` and its associated files, tags, symbols, and relations.
pub fn update_action(db: &KnoxDb, params: &UpdateActionParams) -> KnoxResult<KnoxAction> {
    let existing = match get_action(db, &params.id)? {
        Some(a) => a,
        None => {
            return Err(crate::core::error::KnoxError::ActionNotFound(
                params.id.to_string(),
            ));
        }
    };

    let summary = params
        .summary
        .as_deref()
        .unwrap_or(existing.summary.as_str());
    let action_type = params
        .action_type
        .as_deref()
        .unwrap_or(existing.action_type.as_str());
    let actor = params.actor.as_deref().unwrap_or(existing.actor.as_str());
    let rationale = params
        .rationale
        .as_deref()
        .or(existing.rationale.as_deref());
    let context_summary = params
        .context_summary
        .as_deref()
        .or(existing.context_summary.as_deref());
    let significance = params
        .significance
        .as_deref()
        .or(existing.significance.as_deref())
        .unwrap_or("major");

    let is_resolved_bool = params.is_resolved.or(existing.is_resolved).unwrap_or(true);
    let is_resolved_int = if is_resolved_bool { 1 } else { 0 };

    let resolution_note = params
        .resolution_note
        .as_deref()
        .or(existing.resolution_note.as_deref());

    let parent_action_id_str = match &params.parent_action_id {
        Some(Some(parent_id)) => Some(parent_id.as_str().to_string()),
        Some(None) => None,
        None => existing
            .parent_action_id
            .as_ref()
            .map(|p| p.as_str().to_string()),
    };

    let rejected_alts = params
        .rejected_alternatives
        .as_ref()
        .unwrap_or(&existing.rejected_alternatives);
    let rejected_alt_json = if rejected_alts.is_empty() {
        None
    } else {
        Some(serde_json::to_string(rejected_alts).unwrap_or_default())
    };

    let conn = db.conn();

    // 1. Update primary actions row
    conn.execute(
        "UPDATE actions SET
            summary = ?1,
            action_type = ?2,
            actor = ?3,
            rationale = ?4,
            context_summary = ?5,
            significance = ?6,
            is_resolved = ?7,
            resolution_note = ?8,
            parent_action_id = ?9,
            rejected_alternatives = ?10
         WHERE id = ?11",
        params![
            summary,
            action_type,
            actor,
            rationale,
            context_summary,
            significance,
            is_resolved_int,
            resolution_note,
            parent_action_id_str,
            rejected_alt_json,
            params.id.as_str(),
        ],
    )?;

    // 2. Update files if provided
    if let Some(ref new_files) = params.files {
        conn.execute(
            "DELETE FROM action_files WHERE action_id = ?1",
            params![params.id.as_str()],
        )?;
        let mut stmt_files = conn.prepare_cached(
            "INSERT OR IGNORE INTO action_files (action_id, file_path) VALUES (?1, ?2)",
        )?;
        for file in new_files {
            stmt_files.execute(params![params.id.as_str(), file.as_str()])?;
        }
    }

    // 3. Update tags if provided
    if let Some(ref new_tags) = params.tags {
        conn.execute(
            "DELETE FROM action_tags WHERE action_id = ?1",
            params![params.id.as_str()],
        )?;
        let mut stmt_tags = conn
            .prepare_cached("INSERT OR IGNORE INTO action_tags (action_id, tag) VALUES (?1, ?2)")?;
        for tag in new_tags {
            stmt_tags.execute(params![params.id.as_str(), tag.as_str()])?;
        }
    }

    // 4. Update symbols if provided
    if let Some(ref new_symbols) = params.symbols {
        conn.execute(
            "DELETE FROM action_symbols WHERE action_id = ?1",
            params![params.id.as_str()],
        )?;
        let mut stmt_sym = conn.prepare_cached(
            "INSERT OR IGNORE INTO action_symbols (action_id, symbol) VALUES (?1, ?2)",
        )?;
        for sym in new_symbols {
            stmt_sym.execute(params![params.id.as_str(), sym.as_str()])?;
        }
    }

    // 5. Update relations (replace all) if provided
    if let Some(ref new_relations) = params.relations {
        conn.execute(
            "DELETE FROM action_relations WHERE from_action_id = ?1",
            params![params.id.as_str()],
        )?;
        let mut stmt_rel = conn.prepare_cached(
            "INSERT OR IGNORE INTO action_relations (from_action_id, to_action_id, relation_type) VALUES (?1, ?2, ?3)",
        )?;
        for rel in new_relations {
            stmt_rel.execute(params![
                params.id.as_str(),
                rel.target_id.as_str(),
                rel.relation_type.to_string(),
            ])?;
        }
    }

    // 6. Add specific relations if provided
    if let Some(ref add_rels) = params.add_relations {
        let mut stmt_rel = conn.prepare_cached(
            "INSERT OR REPLACE INTO action_relations (from_action_id, to_action_id, relation_type) VALUES (?1, ?2, ?3)",
        )?;
        for rel in add_rels {
            stmt_rel.execute(params![
                params.id.as_str(),
                rel.target_id.as_str(),
                rel.relation_type.to_string(),
            ])?;
        }
    }

    // 7. Remove specific relations if provided
    if let Some(ref rem_rels) = params.remove_relations {
        let mut stmt_rem = conn.prepare_cached(
            "DELETE FROM action_relations WHERE from_action_id = ?1 AND to_action_id = ?2",
        )?;
        for target_id in rem_rels {
            stmt_rem.execute(params![params.id.as_str(), target_id.as_str()])?;
        }
    }

    get_action(db, &params.id)?
        .ok_or_else(|| crate::core::error::KnoxError::ActionNotFound(params.id.to_string()))
}

/// Inserts associated delta records for an action into SQLite.
pub fn insert_deltas(db: &KnoxDb, deltas: &[ActionDelta]) -> KnoxResult<()> {
    db.with_tx(|conn| insert_deltas_conn(conn, deltas))
}

/// Connection-scoped variant of [`insert_deltas`], safe inside a transaction.
pub(crate) fn insert_deltas_conn(
    conn: &rusqlite::Connection,
    deltas: &[ActionDelta],
) -> KnoxResult<()> {
    let mut stmt = conn.prepare_cached(
        "INSERT OR REPLACE INTO action_deltas (
            id, action_id, file_path, change_type, old_path,
            lines_added, lines_deleted, diff_hunk, deleted_code,
            deletion_reason, created_at
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
    )?;

    for delta in deltas {
        let delta_id = if delta.id.is_empty() {
            uuid::Uuid::new_v4().to_string()
        } else {
            delta.id.clone()
        };
        stmt.execute(params![
            delta_id,
            delta.action_id.as_str(),
            delta.file_path.as_str(),
            delta.change_type.to_string(),
            delta.old_path.as_deref(),
            delta.lines_added as i64,
            delta.lines_deleted as i64,
            delta.diff_hunk.as_deref(),
            delta.deleted_code.as_deref(),
            delta.deletion_reason.as_deref(),
            delta.created_at.to_rfc3339(),
        ])?;
    }
    Ok(())
}

/// Inserts an action and its deltas atomically.
pub fn insert_action_with_deltas(
    db: &KnoxDb,
    action: &KnoxAction,
    deltas: &[ActionDelta],
) -> KnoxResult<ActionId> {
    db.with_tx(|conn| {
        let id = insert_action_conn(conn, action)?;
        insert_deltas_conn(conn, deltas)?;
        Ok(id)
    })
}

/// Aggregates code change metrics for a given action ID.
pub fn fetch_action_delta_summary(
    conn: &rusqlite::Connection,
    id: &ActionId,
) -> KnoxResult<Option<DeltaSummary>> {
    let mut stmt = conn.prepare_cached(
        "SELECT COUNT(DISTINCT file_path), COALESCE(SUM(lines_added), 0), COALESCE(SUM(lines_deleted), 0)
         FROM action_deltas WHERE action_id = ?1",
    )?;
    let mut rows = stmt.query(params![id.as_str()])?;
    if let Some(row) = rows.next()? {
        let files_changed: i64 = row.get(0)?;
        let total_added: i64 = row.get(1)?;
        let total_deleted: i64 = row.get(2)?;
        if files_changed > 0 || total_added > 0 || total_deleted > 0 {
            return Ok(Some(DeltaSummary {
                total_files: files_changed as usize,
                lines_added: total_added.max(0) as u32,
                lines_deleted: total_deleted.max(0) as u32,
            }));
        }
    }
    Ok(None)
}

/// Retrieves all code deltas recorded for a specific action.
pub fn get_deltas_by_action(db: &KnoxDb, action_id: &ActionId) -> KnoxResult<Vec<ActionDelta>> {
    let conn = db.conn();
    let mut stmt = conn.prepare(
        "SELECT id, action_id, file_path, change_type, old_path,
                lines_added, lines_deleted, diff_hunk, deleted_code,
                deletion_reason, created_at
         FROM action_deltas WHERE action_id = ?1 ORDER BY created_at ASC",
    )?;
    let rows = stmt.query_map(params![action_id.as_str()], map_delta_row)?;
    let mut deltas = Vec::new();
    for row in rows {
        deltas.push(row?);
    }
    Ok(deltas)
}

/// Retrieves recent code deltas affecting a specific file path.
pub fn get_deltas_by_file(
    db: &KnoxDb,
    file_path: &str,
    limit: usize,
) -> KnoxResult<Vec<ActionDelta>> {
    let conn = db.conn();
    let mut stmt = conn.prepare(
        "SELECT id, action_id, file_path, change_type, old_path,
                lines_added, lines_deleted, diff_hunk, deleted_code,
                deletion_reason, created_at
         FROM action_deltas WHERE file_path = ?1 ORDER BY created_at DESC LIMIT ?2",
    )?;
    let rows = stmt.query_map(params![file_path, limit as i64], map_delta_row)?;
    let mut deltas = Vec::new();
    for row in rows {
        deltas.push(row?);
    }
    Ok(deltas)
}

/// Finds an action by commit hash (exact match or prefix match).
pub fn find_action_by_commit(db: &KnoxDb, commit_sha: &str) -> KnoxResult<Option<KnoxAction>> {
    let conn = db.conn();
    let prefix = format!("{}%", commit_sha.trim());
    let mut stmt = conn.prepare(
        "SELECT id FROM actions
         WHERE commit_sha = ?1 OR commit_sha LIKE ?2
         ORDER BY COALESCE(timestamp_ms, 0) DESC LIMIT 1",
    )?;
    let mut rows = stmt.query(params![commit_sha.trim(), prefix])?;
    if let Some(row) = rows.next()? {
        let action_id = ActionId::from(row.get::<_, String>(0)?);
        return get_action(db, &action_id);
    }
    Ok(None)
}

/// Parses the line range from a unified diff hunk header (`+start,count`).
pub fn parse_hunk_line_range(hunk: &str) -> Option<(u32, u32)> {
    for line in hunk.lines() {
        if line.starts_with("@@") {
            if let Some(plus_idx) = line.find('+') {
                let rest = &line[plus_idx + 1..];
                let end_idx = rest
                    .find(' ')
                    .or_else(|| rest.find('@'))
                    .unwrap_or(rest.len());
                let range_str = &rest[..end_idx];
                let mut parts = range_str.split(',');
                if let Some(start_str) = parts.next() {
                    if let Ok(start) = start_str.trim().parse::<u32>() {
                        let count = parts
                            .next()
                            .and_then(|c| c.trim().parse::<u32>().ok())
                            .unwrap_or(1);
                        return Some((start, count));
                    }
                }
            }
        }
    }
    None
}

/// Prunes action deltas older than `max_age_days` days to prevent unbounded database growth.
pub fn prune_action_deltas(db: &KnoxDb, max_age_days: i64) -> KnoxResult<usize> {
    let cutoff = chrono::Utc::now() - chrono::Duration::days(max_age_days);
    let cutoff_str = cutoff.to_rfc3339();
    let deleted = db.conn().execute(
        "DELETE FROM action_deltas WHERE created_at < ?1",
        params![cutoff_str],
    )?;
    Ok(deleted)
}

fn map_delta_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<ActionDelta> {
    let id: String = row.get(0)?;
    let action_id_str: String = row.get(1)?;
    let file_path: String = row.get(2)?;
    let change_type_str: String = row.get(3)?;
    let old_path: Option<String> = row.get(4)?;
    let lines_added: i64 = row.get(5)?;
    let lines_deleted: i64 = row.get(6)?;
    let diff_hunk: Option<String> = row.get(7)?;
    let deleted_code: Option<String> = row.get(8)?;
    let deletion_reason: Option<String> = row.get(9)?;
    let created_at_str: String = row.get(10)?;

    let change_type = change_type_str
        .parse::<DeltaChangeType>()
        .unwrap_or(DeltaChangeType::Modified);

    let created_at = DateTime::parse_from_rfc3339(&created_at_str)
        .map(|dt| dt.with_timezone(&Utc))
        .unwrap_or_else(|_| Utc::now());

    Ok(ActionDelta {
        id,
        action_id: ActionId::from(action_id_str),
        file_path,
        change_type,
        old_path,
        lines_added: lines_added.max(0) as u32,
        lines_deleted: lines_deleted.max(0) as u32,
        diff_hunk,
        deleted_code,
        deletion_reason,
        created_at,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_insert_and_get_action() {
        let db = KnoxDb::open_memory().unwrap();
        let action = KnoxAction {
            id: ActionId::new(),
            actor: "cursor".to_string(),
            action_type: "write".to_string(),
            summary: "Implemented node CRUD".to_string(),
            files: vec!["src/storage/nodes.rs".to_string()],
            tags: vec!["storage".to_string(), "crud".to_string()],
            confidence: Some(0.9),
            timestamp: Utc::now(),
            session_id: None,
            parent_action_id: None,
            supersedes: None,
            is_current: true,
            ..Default::default()
        };

        insert_action(&db, &action).unwrap();
        let fetched = get_action(&db, &action.id)
            .unwrap()
            .expect("action should exist");

        assert_eq!(fetched.id, action.id);
        assert_eq!(fetched.summary, action.summary);
        assert_eq!(fetched.files, action.files);
        let mut expected_tags = action.tags.clone();
        expected_tags.sort();
        assert_eq!(fetched.tags, expected_tags);
        assert!(fetched.is_current);
    }

    #[test]
    fn test_supersede_action() {
        let db = KnoxDb::open_memory().unwrap();
        let action1 = KnoxAction {
            id: ActionId::new(),
            actor: "human".to_string(),
            action_type: "plan".to_string(),
            summary: "Old architecture plan".to_string(),
            files: vec!["plan.md".to_string()],
            tags: vec!["plan".to_string()],
            confidence: None,
            timestamp: Utc::now(),
            session_id: None,
            parent_action_id: None,
            supersedes: None,
            is_current: true,
            ..Default::default()
        };
        insert_action(&db, &action1).unwrap();

        let action2 = KnoxAction {
            id: ActionId::new(),
            actor: "antigravity".to_string(),
            action_type: "decide".to_string(),
            summary: "New architecture plan".to_string(),
            files: vec!["plan.md".to_string()],
            tags: vec!["plan".to_string()],
            confidence: Some(1.0),
            timestamp: Utc::now(),
            session_id: None,
            parent_action_id: None,
            supersedes: Some(action1.id.clone()),
            is_current: true,
            ..Default::default()
        };
        insert_action(&db, &action2).unwrap();

        let fetched1 = get_action(&db, &action1.id).unwrap().unwrap();
        let fetched2 = get_action(&db, &action2.id).unwrap().unwrap();

        assert!(!fetched1.is_current);
        assert!(fetched2.is_current);
    }

    #[test]
    fn test_list_projects() {
        let db = KnoxDb::open_memory().unwrap();
        let proj1 = Project {
            id: ProjectId::new(),
            name: "Knox".to_string(),
            path: r"\\?\D:\Knox".to_string(),
            remote: None,
            created_at: Utc::now(),
        };
        let proj2 = Project {
            id: ProjectId::new(),
            name: "hongli".to_string(),
            path: r"D:\hongli".to_string(),
            remote: None,
            created_at: Utc::now(),
        };

        insert_project(&db, &proj1).unwrap();
        insert_project(&db, &proj2).unwrap();

        let list = list_projects(&db).unwrap();
        assert_eq!(list.len(), 2);
        assert_eq!(list[0].name, "Knox");
        assert_eq!(list[0].path, r"D:\Knox"); // verbatim prefix stripped
        assert_eq!(list[1].name, "hongli");
        assert_eq!(list[1].path, r"D:\hongli");
    }

    #[test]
    fn test_update_action_flow() {
        let db = KnoxDb::open_memory().unwrap();
        let target = KnoxAction {
            id: ActionId::new(),
            actor: "cursor".to_string(),
            action_type: "plan".to_string(),
            summary: "Initial architectural plan".to_string(),
            files: vec!["plan.md".to_string()],
            tags: vec!["v1".to_string()],
            timestamp: Utc::now(),
            is_current: true,
            ..Default::default()
        };
        insert_action(&db, &target).unwrap();

        let action = KnoxAction {
            id: ActionId::new(),
            actor: "gemini".to_string(),
            action_type: "write".to_string(),
            summary: "Original summary".to_string(),
            files: vec!["src/old.rs".to_string()],
            tags: vec!["old".to_string()],
            timestamp: Utc::now(),
            is_current: true,
            ..Default::default()
        };
        let action_id = insert_action(&db, &action).unwrap();

        // Update action
        let updated = update_action(
            &db,
            &UpdateActionParams {
                id: action_id.clone(),
                summary: Some("Updated summary with new features".to_string()),
                action_type: Some("decide".to_string()),
                rationale: Some("Design was simplified".to_string()),
                files: Some(vec!["src/new.rs".to_string(), "src/lib.rs".to_string()]),
                tags: Some(vec!["refactor".to_string(), "core".to_string()]),
                add_relations: Some(vec![ActionRelation {
                    target_id: target.id.clone(),
                    relation_type: RelationType::Implements,
                }]),
                ..Default::default()
            },
        )
        .unwrap();

        assert_eq!(updated.summary, "Updated summary with new features");
        assert_eq!(updated.action_type, "decide");
        assert_eq!(updated.rationale.as_deref(), Some("Design was simplified"));
        assert_eq!(updated.files, vec!["src/lib.rs", "src/new.rs"]);
        assert_eq!(updated.tags, vec!["core", "refactor"]);
        assert_eq!(updated.relations.len(), 1);
        assert_eq!(updated.relations[0].target_id, target.id);
        assert_eq!(updated.relations[0].relation_type, RelationType::Implements);

        // Remove relation
        let updated2 = update_action(
            &db,
            &UpdateActionParams {
                id: action_id,
                remove_relations: Some(vec![target.id.to_string()]),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(updated2.relations.len(), 0);
    }

    #[test]
    fn test_insert_and_query_deltas() {
        let db = KnoxDb::open_memory().unwrap();
        let action = KnoxAction {
            id: ActionId::new(),
            actor: "antigravity".to_string(),
            action_type: "write".to_string(),
            summary: "Added delta support".to_string(),
            files: vec!["src/storage/nodes.rs".to_string()],
            commit_sha: Some("abcdef1234567890".to_string()),
            ..Default::default()
        };
        insert_action(&db, &action).unwrap();

        let deltas = vec![ActionDelta {
            id: "delta-1".to_string(),
            action_id: action.id.clone(),
            file_path: "src/storage/nodes.rs".to_string(),
            change_type: DeltaChangeType::Modified,
            old_path: None,
            lines_added: 25,
            lines_deleted: 5,
            diff_hunk: Some("@@ -10,5 +10,25 @@\n+added code".to_string()),
            deleted_code: Some("old code".to_string()),
            deletion_reason: Some("Refactored".to_string()),
            created_at: Utc::now(),
        }];

        insert_deltas(&db, &deltas).unwrap();

        let fetched_deltas = get_deltas_by_action(&db, &action.id).unwrap();
        assert_eq!(fetched_deltas.len(), 1);
        assert_eq!(fetched_deltas[0].file_path, "src/storage/nodes.rs");
        assert_eq!(fetched_deltas[0].lines_added, 25);
        assert_eq!(fetched_deltas[0].lines_deleted, 5);
        assert_eq!(fetched_deltas[0].change_type, DeltaChangeType::Modified);

        let fetched_action = get_action(&db, &action.id).unwrap().expect("action exists");
        assert_eq!(
            fetched_action.commit_sha.as_deref(),
            Some("abcdef1234567890")
        );
        let summary = fetched_action.delta_summary.expect("delta summary exists");
        assert_eq!(summary.total_files, 1);
        assert_eq!(summary.lines_added, 25);
        assert_eq!(summary.lines_deleted, 5);

        // Test delta retention pruning
        let pruned = prune_action_deltas(&db, 90).unwrap();
        assert_eq!(pruned, 0); // newly inserted delta is within 90 days
    }

    #[test]
    fn test_decision_creates_state_and_guardrail_rules() {
        let db = KnoxDb::open_memory().unwrap();
        let action = KnoxAction {
            id: ActionId::new(),
            actor: "cursor".to_string(),
            action_type: "decide".to_string(),
            summary: "Use WAL transactions".to_string(),
            rejected_alternatives: vec!["autocommit writes".to_string(), "flat files".to_string()],
            timestamp: Utc::now(),
            ..Default::default()
        };

        insert_action(&db, &action).unwrap();
        let state: String = db
            .conn()
            .query_row(
                "SELECT state FROM action_state WHERE action_id = ?1",
                rusqlite::params![action.id.as_str()],
                |row| row.get(0),
            )
            .unwrap();
        let rules: i64 = db
            .conn()
            .query_row(
                "SELECT COUNT(*) FROM guardrail_rules WHERE action_id = ?1",
                rusqlite::params![action.id.as_str()],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(state, "current");
        assert_eq!(rules, 2);
    }

    #[test]
    fn test_superseding_action_updates_derived_state() {
        let db = KnoxDb::open_memory().unwrap();
        let old = KnoxAction {
            id: ActionId::new(),
            actor: "cursor".to_string(),
            action_type: "decide".to_string(),
            summary: "Use flat files".to_string(),
            timestamp: Utc::now(),
            ..Default::default()
        };
        let new = KnoxAction {
            id: ActionId::new(),
            actor: "cursor".to_string(),
            action_type: "decide".to_string(),
            summary: "Use SQLite".to_string(),
            supersedes: Some(old.id.clone()),
            timestamp: Utc::now(),
            ..Default::default()
        };
        insert_action(&db, &old).unwrap();
        insert_action(&db, &new).unwrap();

        let state: String = db
            .conn()
            .query_row(
                "SELECT state FROM action_state WHERE action_id = ?1",
                rusqlite::params![old.id.as_str()],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(state, "superseded");
    }
}
