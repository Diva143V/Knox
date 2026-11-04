//! Project memory export and import (the escape hatch for project-local DBs).
//!
//! Export writes newline-delimited JSON: one project, then its sessions, then
//! every action in causal order. Import is idempotent and preserves IDs so a
//! round trip is lossless.

use std::collections::HashSet;
use std::fs;
use std::io::{BufRead, BufReader, Read, Write};
use std::path::Path;

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::core::error::{KnoxError, KnoxResult};
use crate::core::types::KnoxAction;
use crate::storage::db::KnoxDb;
use crate::storage::nodes::insert_action_conn;
use crate::storage::sessions::resolve_project_identifier;

/// Counts produced by an export or import run.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TransferReport {
    pub projects: usize,
    pub sessions: usize,
    pub actions: usize,
    pub skipped: usize,
}

/// Resolves a `--project` argument to a project id. Relative paths are
/// canonicalized first so `--project .` works from inside the repository.
fn resolve_project_arg(db: &KnoxDb, project: Option<&str>) -> KnoxResult<Option<String>> {
    let Some(project) = project else {
        return Ok(None);
    };
    let trimmed = project.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }

    let path = Path::new(trimmed);
    let canonical = fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    let canonical_str = canonical.to_string_lossy().to_string();

    let resolved = resolve_project_identifier(db, &canonical_str)?
        .or(resolve_project_identifier(db, trimmed)?);
    match resolved {
        Some(id) => Ok(Some(id)),
        None => Err(KnoxError::InvalidInput(format!(
            "project '{project}' is not registered in this database; run 'knox init' first"
        ))),
    }
}

/// Writes a project's memory as JSONL.
pub fn cmd_export<W: Write>(
    db: &KnoxDb,
    project: Option<&str>,
    writer: &mut W,
) -> KnoxResult<TransferReport> {
    let project_id = resolve_project_arg(db, project)?;
    let conn = db.conn();
    let mut report = TransferReport::default();

    let mut project_stmt = conn.prepare(
        "SELECT id, name, path, created_at FROM projects
         WHERE (?1 IS NULL OR id = ?1)
         ORDER BY created_at ASC",
    )?;
    let projects = project_stmt
        .query_map(rusqlite::params![project_id.as_deref()], |row| {
            Ok(json!({
                "type": "project",
                "id": row.get::<_, String>(0)?,
                "name": row.get::<_, String>(1)?,
                "path": row.get::<_, String>(2)?,
                "created_at": row.get::<_, String>(3)?,
            }))
        })?
        .collect::<Result<Vec<_>, _>>()?;

    for project in &projects {
        writeln!(writer, "{}", serde_json::to_string(project)?)?;
        report.projects += 1;
    }

    let mut session_stmt = conn.prepare(
        "SELECT id, actor, project_id, started_at, ended_at, is_open FROM sessions
         WHERE (?1 IS NULL OR project_id = ?1)
         ORDER BY started_at ASC",
    )?;
    let sessions = session_stmt
        .query_map(rusqlite::params![project_id.as_deref()], |row| {
            Ok(json!({
                "type": "session",
                "id": row.get::<_, String>(0)?,
                "actor": row.get::<_, String>(1)?,
                "project_id": row.get::<_, Option<String>>(2)?,
                "started_at": row.get::<_, String>(3)?,
                "ended_at": row.get::<_, Option<String>>(4)?,
                "is_open": row.get::<_, i32>(5)? == 1,
            }))
        })?
        .collect::<Result<Vec<_>, _>>()?;

    for session in &sessions {
        writeln!(writer, "{}", serde_json::to_string(session)?)?;
        report.sessions += 1;
    }

    let mut action_stmt = conn.prepare(
        "SELECT id FROM actions
         WHERE (?1 IS NULL OR project_id = ?1)
         ORDER BY COALESCE(timestamp_ms, 0) ASC, timestamp ASC",
    )?;
    let action_ids = action_stmt
        .query_map(rusqlite::params![project_id.as_deref()], |row| {
            row.get::<_, String>(0)
        })?
        .collect::<Result<Vec<_>, _>>()?;

    let actions = crate::storage::queries::get_actions_by_ids(
        db,
        &action_ids
            .iter()
            .map(|id| crate::core::ids::ActionId::from(id.as_str()))
            .collect::<Vec<_>>(),
    )?;

    for action in &actions {
        writeln!(
            writer,
            "{}",
            serde_json::to_string(&json!({ "type": "action", "action": action }))?
        )?;
        report.actions += 1;
    }

    writer.flush()?;
    Ok(report)
}

/// Reads a JSONL export and writes it into `db`.
pub fn cmd_import<R: Read>(db: &KnoxDb, reader: R) -> KnoxResult<TransferReport> {
    let lines = BufReader::new(reader).lines();
    let mut records: Vec<Value> = Vec::new();
    for line in lines {
        let line = line?;
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let value: Value = serde_json::from_str(trimmed)
            .map_err(|e| KnoxError::InvalidInput(format!("invalid JSONL record at import: {e}")))?;
        records.push(value);
    }

    db.with_tx(|conn| {
        // Actions can reference parents and relation targets that appear later
        // in the file; defer foreign-key checks until the whole file is in.
        conn.execute_batch("PRAGMA defer_foreign_keys = ON")?;

        let mut report = TransferReport::default();
        let mut seen_actions: HashSet<String> = HashSet::new();

        for record in &records {
            match record
                .get("type")
                .and_then(Value::as_str)
                .unwrap_or("action")
            {
                "project" => {
                    let id = required_str(record, "id")?;
                    let name = required_str(record, "name")?;
                    let path = required_str(record, "path")?;
                    let created_at = record
                        .get("created_at")
                        .and_then(Value::as_str)
                        .unwrap_or("1970-01-01T00:00:00Z");
                    let inserted = conn.execute(
                        "INSERT OR IGNORE INTO projects (id, name, path, created_at)
                         VALUES (?1, ?2, ?3, ?4)",
                        rusqlite::params![id, name, path, created_at],
                    )?;
                    if inserted > 0 {
                        report.projects += 1;
                    } else {
                        report.skipped += 1;
                    }
                }
                "session" => {
                    let id = required_str(record, "id")?;
                    let actor = required_str(record, "actor")?;
                    let started_at = required_str(record, "started_at")?;
                    let project_id = record.get("project_id").and_then(Value::as_str);
                    let ended_at = record.get("ended_at").and_then(Value::as_str);
                    let is_open = record
                        .get("is_open")
                        .and_then(Value::as_bool)
                        .unwrap_or(false);
                    let inserted = conn.execute(
                        "INSERT OR IGNORE INTO sessions
                             (id, actor, project_id, started_at, ended_at, is_open)
                         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                        rusqlite::params![
                            id,
                            actor,
                            project_id,
                            started_at,
                            ended_at,
                            is_open as i32
                        ],
                    )?;
                    if inserted > 0 {
                        report.sessions += 1;
                    } else {
                        report.skipped += 1;
                    }
                }
                _ => {
                    let action_value = record
                        .get("action")
                        .cloned()
                        .unwrap_or_else(|| record.clone());
                    let action: KnoxAction =
                        serde_json::from_value(action_value).map_err(KnoxError::Json)?;
                    let id = action.id.to_string();
                    if !seen_actions.insert(id.clone()) {
                        report.skipped += 1;
                        continue;
                    }
                    let exists: bool = conn.query_row(
                        "SELECT EXISTS(SELECT 1 FROM actions WHERE id = ?1)",
                        rusqlite::params![id],
                        |row| row.get(0),
                    )?;
                    if exists {
                        report.skipped += 1;
                        continue;
                    }
                    insert_action_conn(conn, &action)?;
                    report.actions += 1;
                }
            }
        }
        Ok(report)
    })
}

fn required_str<'a>(record: &'a Value, key: &str) -> KnoxResult<&'a str> {
    record
        .get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| KnoxError::InvalidInput(format!("import record missing '{key}'")))
}

/// Convenience wrapper for exporting to a file.
pub fn export_to_file(
    db: &KnoxDb,
    project: Option<&str>,
    path: &Path,
) -> KnoxResult<TransferReport> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)?;
        }
    }
    let mut file = fs::File::create(path)?;
    cmd_export(db, project, &mut file)
}

/// Convenience wrapper for importing from a file.
pub fn import_from_file(db: &KnoxDb, path: &Path) -> KnoxResult<TransferReport> {
    let file = fs::File::open(path)?;
    cmd_import(db, file)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::ids::ProjectId;
    use crate::core::types::Project;
    use crate::storage::nodes::insert_project;
    use chrono::Utc;

    fn seed(db: &KnoxDb) -> ProjectId {
        let project = Project {
            id: ProjectId::new(),
            name: "demo".to_string(),
            path: "/work/demo".to_string(),
            created_at: Utc::now(),
        };
        insert_project(db, &project).unwrap();
        let session = crate::storage::sessions::start_session(db, "gemini", Some("demo")).unwrap();
        crate::mcp::handlers::handle_record(
            db,
            json!({
                "actor": "gemini",
                "project": "demo",
                "session_id": session.id.as_str(),
                "entries": [
                    {"ref": "p", "kind": "plan", "summary": "Plan parser", "files": ["src/parser.rs"]},
                    {"kind": "action", "summary": "Write parser", "files": ["src/parser.rs"], "caused_by": "p"}
                ]
            }),
        )
        .unwrap();
        project.id
    }

    #[test]
    fn export_import_round_trip_preserves_actions() {
        let source = KnoxDb::open_memory().unwrap();
        seed(&source);

        let mut buffer = Vec::new();
        let exported = cmd_export(&source, Some("demo"), &mut buffer).unwrap();
        assert_eq!(exported.projects, 1);
        assert_eq!(exported.sessions, 1);
        assert_eq!(exported.actions, 2);

        let target = KnoxDb::open_memory().unwrap();
        let imported = cmd_import(&target, buffer.as_slice()).unwrap();
        assert_eq!(imported.projects, 1);
        assert_eq!(imported.sessions, 1);
        assert_eq!(imported.actions, 2);

        let source_count: i64 = source
            .conn()
            .query_row("SELECT COUNT(*) FROM actions", [], |r| r.get(0))
            .unwrap();
        let target_count: i64 = target
            .conn()
            .query_row("SELECT COUNT(*) FROM actions", [], |r| r.get(0))
            .unwrap();
        assert_eq!(source_count, target_count);

        // Importing the same file twice is a no-op.
        let again = cmd_import(&target, buffer.as_slice()).unwrap();
        assert_eq!(again.actions, 0);
        assert!(again.skipped >= 4);
    }

    #[test]
    fn import_rejects_malformed_jsonl() {
        let db = KnoxDb::open_memory().unwrap();
        assert!(matches!(
            cmd_import(&db, "not json\n".as_bytes()),
            Err(KnoxError::InvalidInput(_))
        ));
    }
}
