//! Operational database migration, validation, restore, and split commands.

use rusqlite::{Connection, DatabaseName, params};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::core::error::{KnoxError, KnoxResult};
use crate::storage::db::KnoxDb;

#[derive(Clone, Debug, Serialize)]
pub struct MigrationReport {
    pub path: String,
    pub schema_version: i32,
    pub integrity: String,
    pub projects: i64,
    pub actions: i64,
    pub orphan_actions: i64,
    pub orphan_relations: i64,
}

pub fn migration_check(path: &Path) -> KnoxResult<MigrationReport> {
    let conn = Connection::open(path)?;
    let schema_version = conn
        .query_row(
            "SELECT CAST(value AS INTEGER) FROM knox_meta WHERE key = 'schema_version'",
            [],
            |row| row.get(0),
        )
        .unwrap_or(0);
    let integrity: String = conn.query_row("PRAGMA integrity_check", [], |row| row.get(0))?;
    let projects = count(&conn, "SELECT COUNT(*) FROM projects")?;
    let actions = count(&conn, "SELECT COUNT(*) FROM actions")?;
    let orphan_actions = if has_column(&conn, "actions", "project_id")? {
        count(
            &conn,
            "SELECT COUNT(*) FROM actions a
             LEFT JOIN projects p ON p.id = a.project_id
             WHERE a.project_id IS NOT NULL AND p.id IS NULL",
        )?
    } else {
        0
    };
    let orphan_relations = if has_table(&conn, "action_relations")? {
        count(
            &conn,
            "SELECT COUNT(*) FROM action_relations r
             LEFT JOIN actions a ON a.id = r.from_action_id
             LEFT JOIN actions b ON b.id = r.to_action_id
             WHERE a.id IS NULL OR b.id IS NULL",
        )?
    } else {
        0
    };
    Ok(MigrationReport {
        path: path.display().to_string(),
        schema_version,
        integrity,
        projects,
        actions,
        orphan_actions,
        orphan_relations,
    })
}

pub fn migration_apply(path: &Path) -> KnoxResult<(MigrationReport, PathBuf)> {
    let backup = backup_path(path)?;
    snapshot_database(path, &backup)?;
    let result = (|| {
        let _db = KnoxDb::open(path)?;
        let report = migration_check(path)?;
        validate_report(&report, "migration")?;
        Ok(report)
    })();
    if result.is_err() {
        restore_snapshot(&backup, path)?;
    }
    result.map(|report| (report, backup))
}

pub fn migration_restore(path: &Path) -> KnoxResult<PathBuf> {
    let backup = newest_backup(path)?;
    restore_snapshot(&backup, path)?;
    let report = migration_check(path)?;
    validate_report(&report, "restored database")?;
    Ok(backup)
}

pub fn migration_split(path: &Path, output: &Path) -> KnoxResult<Vec<PathBuf>> {
    let source = Connection::open(path)?;
    let projects = {
        let mut stmt = source.prepare("SELECT id FROM projects ORDER BY id")?;
        stmt.query_map([], |row| row.get::<_, String>(0))?
            .collect::<Result<Vec<_>, _>>()?
    };
    if projects.is_empty() {
        return Err(KnoxError::InvalidInput(
            "cannot split a database with no projects".to_string(),
        ));
    }
    fs::create_dir_all(output)?;
    let mut created = Vec::new();
    for project_id in projects {
        let target = output.join(&project_id).join("knox.db");
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent)?;
        }
        if target.exists() {
            fs::remove_file(&target)?;
        }
        source.execute("VACUUM INTO ?1", [target.to_string_lossy().as_ref()])?;
        let db = KnoxDb::open(&target)?;
        db.with_tx(|tx| {
            tx.execute(
                "UPDATE actions
                 SET parent_action_id = NULL
                 WHERE project_id = ?1
                   AND parent_action_id IS NOT NULL
                   AND parent_action_id NOT IN (
                       SELECT id FROM actions WHERE project_id = ?1
                   )",
                params![project_id],
            )?;
            tx.execute(
                "UPDATE actions
                 SET superseded_by = NULL
                 WHERE project_id = ?1
                   AND superseded_by IS NOT NULL
                   AND superseded_by NOT IN (
                       SELECT id FROM actions WHERE project_id = ?1
                   )",
                params![project_id],
            )?;
            tx.execute(
                "UPDATE actions
                 SET session_id = NULL
                 WHERE project_id = ?1
                   AND session_id IS NOT NULL
                   AND session_id NOT IN (
                       SELECT id FROM sessions WHERE project_id = ?1
                   )",
                params![project_id],
            )?;
            tx.execute(
                "UPDATE action_state
                 SET by_action_id = NULL
                 WHERE by_action_id IS NOT NULL
                   AND by_action_id NOT IN (
                       SELECT id FROM actions WHERE project_id = ?1
                   )",
                params![project_id],
            )?;
            tx.execute(
                "DELETE FROM action_relations
                 WHERE from_action_id NOT IN (
                     SELECT id FROM actions WHERE project_id = ?1
                 )
                    OR to_action_id NOT IN (
                     SELECT id FROM actions WHERE project_id = ?1
                 )",
                params![project_id],
            )?;
            tx.execute(
                "DELETE FROM action_symbols
                 WHERE action_id NOT IN (
                     SELECT id FROM actions WHERE project_id = ?1
                 )",
                params![project_id],
            )?;
            tx.execute(
                "DELETE FROM action_files
                 WHERE action_id NOT IN (
                     SELECT id FROM actions WHERE project_id = ?1
                 )",
                params![project_id],
            )?;
            tx.execute(
                "DELETE FROM action_tags
                 WHERE action_id NOT IN (
                     SELECT id FROM actions WHERE project_id = ?1
                 )",
                params![project_id],
            )?;
            tx.execute(
                "DELETE FROM actions WHERE project_id IS NOT ?1",
                params![project_id],
            )?;
            tx.execute(
                "DELETE FROM sessions WHERE project_id IS NOT ?1",
                params![project_id],
            )?;
            tx.execute(
                "DELETE FROM projects WHERE id IS NOT ?1",
                params![project_id],
            )?;
            Ok(())
        })?;
        drop(db);
        let report = migration_check(&target)?;
        if report.integrity != "ok" || report.orphan_actions > 0 || report.orphan_relations > 0 {
            return Err(KnoxError::InvalidInput(format!(
                "split validation failed for {}",
                target.display()
            )));
        }
        created.push(target);
    }
    Ok(created)
}

fn count(conn: &Connection, sql: &str) -> KnoxResult<i64> {
    Ok(conn.query_row(sql, [], |row| row.get(0))?)
}

fn has_table(conn: &Connection, table: &str) -> KnoxResult<bool> {
    Ok(conn.query_row(
        "SELECT EXISTS(
            SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = ?1
        )",
        [table],
        |row| row.get(0),
    )?)
}

fn has_column(conn: &Connection, table: &str, column: &str) -> KnoxResult<bool> {
    let mut stmt = conn.prepare(&format!("PRAGMA table_info({table})"))?;
    Ok(stmt
        .query_map([], |row| row.get::<_, String>(1))?
        .collect::<Result<Vec<_>, _>>()?
        .iter()
        .any(|name| name == column))
}

fn validate_report(report: &MigrationReport, label: &str) -> KnoxResult<()> {
    if report.integrity != "ok" || report.orphan_actions > 0 || report.orphan_relations > 0 {
        return Err(KnoxError::InvalidInput(format!(
            "{label} validation failed: integrity={}, orphan_actions={}, orphan_relations={}",
            report.integrity, report.orphan_actions, report.orphan_relations
        )));
    }
    Ok(())
}

fn snapshot_database(source: &Path, destination: &Path) -> KnoxResult<()> {
    if destination.exists() {
        remove_sqlite_sidecars(destination)?;
        fs::remove_file(destination)?;
    }
    let source_conn = Connection::open(source)?;
    source_conn.backup(DatabaseName::Main, destination, None)?;
    Ok(())
}

fn restore_snapshot(source: &Path, destination: &Path) -> KnoxResult<()> {
    let temporary = destination.with_extension("restore.tmp");
    snapshot_database(source, &temporary)?;
    remove_sqlite_sidecars(destination)?;
    if destination.exists() {
        fs::remove_file(destination)?;
    }
    fs::rename(&temporary, destination)?;
    Ok(())
}

fn remove_sqlite_sidecars(path: &Path) -> KnoxResult<()> {
    for suffix in ["-wal", "-shm"] {
        let sidecar = PathBuf::from(format!("{}{}", path.display(), suffix));
        if sidecar.exists() {
            fs::remove_file(sidecar)?;
        }
    }
    Ok(())
}

fn backup_path(path: &Path) -> KnoxResult<PathBuf> {
    if !path.exists() {
        return Err(KnoxError::InvalidInput(format!(
            "database does not exist: {}",
            path.display()
        )));
    }
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| KnoxError::InvalidInput(error.to_string()))?
        .as_secs();
    Ok(path.with_file_name(format!(
        "{}.bak-v5-{}",
        path.file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("knox.db"),
        timestamp
    )))
}

fn newest_backup(path: &Path) -> KnoxResult<PathBuf> {
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    let prefix = format!(
        "{}.bak-v",
        path.file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("knox.db")
    );
    let mut backups = fs::read_dir(parent)?
        .filter_map(Result::ok)
        .filter(|entry| entry.file_name().to_string_lossy().starts_with(&prefix))
        .collect::<Vec<_>>();
    backups.sort_by_key(|entry| entry.metadata().and_then(|meta| meta.modified()).ok());
    backups.pop().map(|entry| entry.path()).ok_or_else(|| {
        KnoxError::InvalidInput(format!(
            "no migration backup found beside {}",
            path.display()
        ))
    })
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct ConsolidationReport {
    pub source_path: String,
    pub target_path: String,
    pub projects_moved: usize,
    pub sessions_moved: usize,
    pub actions_moved: usize,
    pub actions_skipped: usize,
    pub edges_moved: usize,
}

/// Consolidates records from a source project-local SQLite database into the target global database.
pub fn migration_consolidate(target_db: &KnoxDb, source_path: &Path) -> KnoxResult<ConsolidationReport> {
    if !source_path.exists() {
        return Err(KnoxError::InvalidInput(format!(
            "Source database does not exist: {}",
            source_path.display()
        )));
    }
    let source_canonical = fs::canonicalize(source_path).unwrap_or_else(|_| source_path.to_path_buf());
    if let Some(target_path) = target_db.path() {
        let target_canonical = fs::canonicalize(target_path).unwrap_or_else(|_| target_path.to_path_buf());
        if source_canonical == target_canonical {
            return Err(KnoxError::InvalidInput(
                "Source database and target database cannot be the same file".to_string(),
            ));
        }
    }

    let src_conn = Connection::open_with_flags(source_path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)?;
    let schema_v: i32 = src_conn
        .query_row(
            "SELECT CAST(value AS INTEGER) FROM knox_meta WHERE key = 'schema_version'",
            [],
            |r| r.get(0),
        )
        .unwrap_or(0);
    if schema_v == 0 {
        return Err(KnoxError::InvalidInput(format!(
            "Invalid source database at {}: missing knox_meta schema_version",
            source_path.display()
        )));
    }

    let mut report = ConsolidationReport {
        source_path: source_path.display().to_string(),
        target_path: target_db
            .path()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|| "in-memory".to_string()),
        ..Default::default()
    };

    target_db.with_tx(|tx| {
        // 1. Projects mapping
        let mut project_map: std::collections::HashMap<String, String> = std::collections::HashMap::new();
        let has_remote = has_column(&src_conn, "projects", "remote")?;
        let proj_sql = if has_remote {
            "SELECT id, name, path, remote, created_at FROM projects"
        } else {
            "SELECT id, name, path, NULL, created_at FROM projects"
        };
        let mut project_stmt = src_conn.prepare(proj_sql)?;
        let project_rows = project_stmt.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, Option<String>>(3)?,
                row.get::<_, String>(4)?,
            ))
        })?;

        for p in project_rows {
            let (src_id, name, path, remote, created_at) = p?;
            let existing_id: Option<String> = tx
                .query_row(
                    "SELECT id FROM projects WHERE path = ?1 OR (remote IS NOT NULL AND remote = ?2) OR id = ?3 LIMIT 1",
                    params![path, remote, src_id],
                    |r| r.get(0),
                )
                .ok();

            if let Some(target_proj_id) = existing_id {
                project_map.insert(src_id, target_proj_id);
            } else {
                tx.execute(
                    "INSERT INTO projects (id, name, path, remote, created_at) VALUES (?1, ?2, ?3, ?4, ?5)",
                    params![src_id, name, path, remote, created_at],
                )?;
                project_map.insert(src_id.clone(), src_id);
                report.projects_moved += 1;
            }
        }

        let edges_count_before: usize = tx.query_row("SELECT COUNT(*) FROM edges", [], |r| r.get(0)).unwrap_or(0);

        // 2. Sessions mapping
        let mut session_stmt = src_conn.prepare(
            "SELECT id, actor, project_id, started_at, ended_at, is_open FROM sessions"
        )?;
        let session_rows = session_stmt.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, Option<String>>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, Option<String>>(4)?,
                row.get::<_, i64>(5)?,
            ))
        })?;

        for s in session_rows {
            let (id, actor, proj_id, started_at, ended_at, is_open) = s?;
            let remapped_proj_id = proj_id.and_then(|p| project_map.get(&p).cloned().or(Some(p)));
            let changes = tx.execute(
                "INSERT OR IGNORE INTO sessions (id, actor, project_id, started_at, ended_at, is_open)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![id, actor, remapped_proj_id, started_at, ended_at, is_open],
            )?;
            if changes > 0 {
                report.sessions_moved += 1;
            }
        }

        // 3. Actions migration with deduplication
        let mut action_map: std::collections::HashMap<String, String> = std::collections::HashMap::new();
        let has_rationale = has_column(&src_conn, "actions", "rationale")?;
        let has_context_summary = has_column(&src_conn, "actions", "context_summary")?;
        let has_rejected_alternatives = has_column(&src_conn, "actions", "rejected_alternatives")?;
        let has_significance = has_column(&src_conn, "actions", "significance")?;
        let has_is_resolved = has_column(&src_conn, "actions", "is_resolved")?;
        let has_resolution_note = has_column(&src_conn, "actions", "resolution_note")?;
        let has_commit_sha = has_column(&src_conn, "actions", "commit_sha")?;
        let has_project_id = has_column(&src_conn, "actions", "project_id")?;
        let has_timestamp_ms = has_column(&src_conn, "actions", "timestamp_ms")?;
        let has_dedupe_key = has_column(&src_conn, "actions", "dedupe_key")?;
        let has_seen_count = has_column(&src_conn, "actions", "seen_count")?;

        let action_sql = format!(
            "SELECT id, actor, action_type, summary, confidence, timestamp,
                    session_id, parent_action_id, superseded_by, is_current,
                    {}, {}, {},
                    {}, {}, {}, {},
                    {}, {}, {}, {}
             FROM actions ORDER BY timestamp ASC",
            if has_rationale { "rationale" } else { "NULL" },
            if has_context_summary { "context_summary" } else { "NULL" },
            if has_rejected_alternatives { "rejected_alternatives" } else { "NULL" },
            if has_significance { "significance" } else { "'major'" },
            if has_is_resolved { "is_resolved" } else { "1" },
            if has_resolution_note { "resolution_note" } else { "NULL" },
            if has_commit_sha { "commit_sha" } else { "NULL" },
            if has_project_id { "project_id" } else { "NULL" },
            if has_timestamp_ms { "timestamp_ms" } else { "NULL" },
            if has_dedupe_key { "dedupe_key" } else { "NULL" },
            if has_seen_count { "seen_count" } else { "1" },
        );
        let mut action_stmt = src_conn.prepare(&action_sql)?;
        let action_rows = action_stmt.query_map([], |row| {
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
                row.get::<_, i64>(9)?,
                row.get::<_, Option<String>>(10)?,
                row.get::<_, Option<String>>(11)?,
                row.get::<_, Option<String>>(12)?,
                row.get::<_, Option<String>>(13)?,
                row.get::<_, Option<i64>>(14)?,
                row.get::<_, Option<String>>(15)?,
                row.get::<_, Option<String>>(16)?,
                row.get::<_, Option<String>>(17)?,
                row.get::<_, Option<i64>>(18)?,
                row.get::<_, Option<String>>(19)?,
                row.get::<_, Option<i64>>(20)?,
            ))
        })?;

        for a in action_rows {
            let (
                id, actor, action_type, summary, confidence, timestamp,
                session_id, parent_id, superseded_by, is_current,
                rationale, context_summary, rejected_alternatives,
                significance, is_resolved, resolution_note, commit_sha,
                project_id, timestamp_ms, dedupe_key, seen_count
            ) = a?;
            
            let existing_id: Option<String> = if let Some(ref key) = dedupe_key {
                tx.query_row(
                    "SELECT id FROM actions WHERE dedupe_key = ?1 LIMIT 1",
                    params![key],
                    |r| r.get(0),
                ).ok()
            } else {
                tx.query_row(
                    "SELECT id FROM actions WHERE timestamp = ?1 AND summary = ?2 AND actor = ?3 LIMIT 1",
                    params![timestamp, summary, actor],
                    |r| r.get(0),
                ).ok()
            };

            if let Some(target_act_id) = existing_id {
                action_map.insert(id, target_act_id);
                report.actions_skipped += 1;
            } else {
                let remapped_proj_id = project_id.and_then(|p| project_map.get(&p).cloned().or(Some(p)));
                let remapped_parent = parent_id.and_then(|p| action_map.get(&p).cloned().or(Some(p)));
                let remapped_superseded = superseded_by.and_then(|s| action_map.get(&s).cloned().or(Some(s)));

                tx.execute(
                    "INSERT INTO actions (
                        id, actor, action_type, summary, confidence, timestamp,
                        session_id, parent_action_id, superseded_by, is_current,
                        rationale, context_summary, rejected_alternatives,
                        significance, is_resolved, resolution_note, commit_sha,
                        project_id, timestamp_ms, dedupe_key, seen_count
                    ) VALUES (
                        ?1, ?2, ?3, ?4, ?5, ?6,
                        ?7, ?8, ?9, ?10,
                        ?11, ?12, ?13,
                        ?14, ?15, ?16, ?17,
                        ?18, ?19, ?20, ?21
                    )",
                    params![
                        id, actor, action_type, summary, confidence, timestamp,
                        session_id, remapped_parent, remapped_superseded, is_current,
                        rationale, context_summary, rejected_alternatives,
                        significance.as_deref().unwrap_or("major"), is_resolved.unwrap_or(1),
                        resolution_note, commit_sha,
                        remapped_proj_id, timestamp_ms, dedupe_key, seen_count.unwrap_or(1)
                    ],
                )?;
                action_map.insert(id.clone(), id);
                report.actions_moved += 1;
            }
        }

        // 4. Child tables
        if has_table(&src_conn, "action_files")? {
            let mut stmt = src_conn.prepare("SELECT action_id, file_path FROM action_files")?;
            let rows = stmt.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))?;
            for r in rows {
                let (act_id, file_path) = r?;
                if let Some(target_act_id) = action_map.get(&act_id) {
                    tx.execute(
                        "INSERT OR IGNORE INTO action_files (action_id, file_path) VALUES (?1, ?2)",
                        params![target_act_id, file_path],
                    )?;
                }
            }
        }

        if has_table(&src_conn, "action_tags")? {
            let mut stmt = src_conn.prepare("SELECT action_id, tag FROM action_tags")?;
            let rows = stmt.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))?;
            for r in rows {
                let (act_id, tag) = r?;
                if let Some(target_act_id) = action_map.get(&act_id) {
                    tx.execute(
                        "INSERT OR IGNORE INTO action_tags (action_id, tag) VALUES (?1, ?2)",
                        params![target_act_id, tag],
                    )?;
                }
            }
        }

        if has_table(&src_conn, "action_symbols")? {
            let mut stmt = src_conn.prepare("SELECT action_id, symbol FROM action_symbols")?;
            let rows = stmt.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))?;
            for r in rows {
                let (act_id, symbol) = r?;
                if let Some(target_act_id) = action_map.get(&act_id) {
                    tx.execute(
                        "INSERT OR IGNORE INTO action_symbols (action_id, symbol) VALUES (?1, ?2)",
                        params![target_act_id, symbol],
                    )?;
                }
            }
        }

        if has_table(&src_conn, "action_deltas")? {
            let mut stmt = src_conn.prepare(
                "SELECT id, action_id, file_path, change_type, old_path, lines_added, lines_deleted, diff_hunk, deleted_code, deletion_reason, created_at FROM action_deltas"
            )?;
            let rows = stmt.query_map([], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, String>(2)?,
                    r.get::<_, String>(3)?,
                    r.get::<_, Option<String>>(4)?,
                    r.get::<_, i64>(5)?,
                    r.get::<_, i64>(6)?,
                    r.get::<_, Option<String>>(7)?,
                    r.get::<_, Option<String>>(8)?,
                    r.get::<_, Option<String>>(9)?,
                    r.get::<_, String>(10)?,
                ))
            })?;
            for r in rows {
                let (id, act_id, file_path, change_type, old_path, added, deleted, diff_hunk, deleted_code, deletion_reason, created_at) = r?;
                if let Some(target_act_id) = action_map.get(&act_id) {
                    tx.execute(
                        "INSERT OR IGNORE INTO action_deltas (
                            id, action_id, file_path, change_type, old_path, lines_added, lines_deleted, diff_hunk, deleted_code, deletion_reason, created_at
                        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
                        params![id, target_act_id, file_path, change_type, old_path, added, deleted, diff_hunk, deleted_code, deletion_reason, created_at],
                    )?;
                }
            }
        }

        // 5. Polymorphic refs
        if has_table(&src_conn, "refs")? {
            let mut stmt = src_conn.prepare("SELECT id, kind, value, project_id, created_at FROM refs")?;
            let rows = stmt.query_map([], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, String>(2)?,
                    r.get::<_, Option<String>>(3)?,
                    r.get::<_, String>(4)?,
                ))
            })?;
            for r in rows {
                let (id, kind, value, project_id, created_at) = r?;
                let remapped_proj_id = project_id.and_then(|p| project_map.get(&p).cloned().or(Some(p)));
                tx.execute(
                    "INSERT OR IGNORE INTO refs (id, kind, value, project_id, created_at) VALUES (?1, ?2, ?3, ?4, ?5)",
                    params![id, kind, value, remapped_proj_id, created_at],
                )?;
            }
        }

        // 6. Edges
        if has_table(&src_conn, "edges")? {
            let has_to_id = has_column(&src_conn, "edges", "to_id")?;
            let has_label = has_column(&src_conn, "edges", "label")?;
            let has_provenance = has_column(&src_conn, "edges", "provenance")?;
            let has_note = has_column(&src_conn, "edges", "note")?;

            let edge_sql = format!(
                "SELECT id, from_action_id, {}, edge_type, created_at, {}, {}, {} FROM edges",
                if has_to_id { "to_id" } else { "to_action_id" },
                if has_label { "label" } else { "NULL" },
                if has_provenance { "provenance" } else { "NULL" },
                if has_note { "note" } else { "NULL" },
            );
            let mut stmt = src_conn.prepare(&edge_sql)?;
            let rows = stmt.query_map([], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, String>(2)?,
                    r.get::<_, String>(3)?,
                    r.get::<_, String>(4)?,
                    r.get::<_, Option<String>>(5)?,
                    r.get::<_, Option<String>>(6)?,
                    r.get::<_, Option<String>>(7)?,
                ))
            })?;
            for r in rows {
                let (edge_id, from_id, to_id, edge_type, created_at, label, provenance, note) = r?;
                if let Some(target_from) = action_map.get(&from_id) {
                    let target_to = action_map.get(&to_id).cloned().unwrap_or(to_id);

                    // Normalize legacy edge types (e.g. 'follows' -> 'caused_by')
                    let normalized_edge_type = match edge_type.as_str() {
                        "follows" => "caused_by",
                        "caused_by" | "contradicts" | "implements" | "depends_on" | "verifies"
                        | "relates_to" | "reverts" | "supersedes" | "resolves" => edge_type.as_str(),
                        _ => "relates_to",
                    };

                    // Guard against orphan to_id violating edges_bi_check_to_id trigger
                    let to_exists: bool = tx.query_row(
                        "SELECT EXISTS(SELECT 1 FROM actions WHERE id = ?1 UNION ALL SELECT 1 FROM refs WHERE id = ?1)",
                        params![target_to],
                        |r| r.get(0),
                    ).unwrap_or(false);

                    if to_exists {
                        tx.execute(
                            "INSERT OR IGNORE INTO edges (id, from_action_id, to_id, edge_type, created_at, label, provenance, note)
                             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                            params![edge_id, target_from, target_to, normalized_edge_type, created_at, label, provenance, note],
                        )?;
                    }
                }
            }
        }

        // 7. action_relations
        if has_table(&src_conn, "action_relations")? {
            let mut stmt = src_conn.prepare("SELECT from_action_id, to_action_id, relation_type FROM action_relations")?;
            let rows = stmt.query_map([], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, String>(2)?,
                ))
            })?;
            for r in rows {
                let (from_id, to_id, rel_type) = r?;
                if let (Some(target_from), Some(target_to)) = (action_map.get(&from_id), action_map.get(&to_id)) {
                    let _ = tx.execute(
                        "INSERT OR IGNORE INTO action_relations (from_action_id, to_action_id, relation_type)
                         VALUES (?1, ?2, ?3)",
                        params![target_from, target_to, rel_type],
                    );
                }
            }
        }

        let edges_count_after: usize = tx.query_row("SELECT COUNT(*) FROM edges", [], |r| r.get(0)).unwrap_or(0);
        report.edges_moved = edges_count_after.saturating_sub(edges_count_before);

        // Rebuild FTS
        tx.execute("INSERT INTO actions_fts(actions_fts) VALUES('rebuild')", [])?;

        Ok(())
    })?;

    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::ids::ProjectId;
    use crate::core::types::{KnoxAction, Project};
    use crate::storage::{insert_action, insert_project};
    use chrono::Utc;
    use tempfile::tempdir;

    #[test]
    fn check_apply_and_restore_are_validated() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("knox.db");
        let db = KnoxDb::open(&path).unwrap();
        drop(db);

        let before = migration_check(&path).unwrap();
        assert_eq!(before.schema_version, 11);
        let (after, backup) = migration_apply(&path).unwrap();
        assert_eq!(after.integrity, "ok");
        assert!(backup.exists());
        assert_eq!(migration_restore(&path).unwrap(), backup);
        assert_eq!(migration_check(&path).unwrap().integrity, "ok");
    }

    #[test]
    fn split_creates_one_valid_database_per_project() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("knox.db");
        let db = KnoxDb::open(&path).unwrap();
        for name in ["alpha", "beta"] {
            insert_project(
                &db,
                &Project {
                    id: ProjectId::new(),
                    name: name.to_string(),
                    path: dir.path().join(name).display().to_string(),
                    remote: None,
                    created_at: Utc::now(),
                },
            )
            .unwrap();
        }
        drop(db);

        let output = dir.path().join("split");
        let created = migration_split(&path, &output).unwrap();
        assert_eq!(created.len(), 2);
        for target in created {
            assert_eq!(migration_check(&target).unwrap().integrity, "ok");
        }
    }

    #[test]
    fn split_detaches_cross_project_action_references() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("knox.db");
        let db = KnoxDb::open(&path).unwrap();
        let project_a = ProjectId::new();
        let project_b = ProjectId::new();
        for (id, name) in [(project_a.clone(), "alpha"), (project_b.clone(), "beta")] {
            insert_project(
                &db,
                &Project {
                    id,
                    name: name.to_string(),
                    path: dir.path().join(name).display().to_string(),
                    remote: None,
                    created_at: Utc::now(),
                },
            )
            .unwrap();
        }
        let parent = KnoxAction {
            project_id: Some(project_b.clone()),
            summary: "external parent".to_string(),
            ..Default::default()
        };
        let child = KnoxAction {
            project_id: Some(project_a.clone()),
            parent_action_id: Some(parent.id.clone()),
            summary: "local child".to_string(),
            ..Default::default()
        };
        insert_action(&db, &parent).unwrap();
        insert_action(&db, &child).unwrap();
        drop(db);

        let created = migration_split(&path, &dir.path().join("split")).unwrap();
        assert_eq!(created.len(), 2);
        for target in created {
            assert_eq!(migration_check(&target).unwrap().orphan_actions, 0);
            assert_eq!(migration_check(&target).unwrap().orphan_relations, 0);
        }
    }

    #[test]
    fn split_detaches_cross_project_lifecycle_references() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("knox.db");
        let db = KnoxDb::open(&path).unwrap();
        let project_a = ProjectId::new();
        let project_b = ProjectId::new();
        for (id, name) in [(project_a.clone(), "alpha"), (project_b.clone(), "beta")] {
            insert_project(
                &db,
                &Project {
                    id,
                    name: name.to_string(),
                    path: dir.path().join(name).display().to_string(),
                    remote: None,
                    created_at: Utc::now(),
                },
            )
            .unwrap();
        }
        let older = KnoxAction {
            project_id: Some(project_b),
            summary: "external older decision".to_string(),
            ..Default::default()
        };
        let newer = KnoxAction {
            project_id: Some(project_a.clone()),
            supersedes: Some(older.id.clone()),
            summary: "local replacement".to_string(),
            ..Default::default()
        };
        insert_action(&db, &older).unwrap();
        insert_action(&db, &newer).unwrap();
        drop(db);

        let created = migration_split(&path, &dir.path().join("split")).unwrap();
        assert_eq!(created.len(), 2);
        for target in created {
            assert_eq!(migration_check(&target).unwrap().integrity, "ok");
        }
    }
}
