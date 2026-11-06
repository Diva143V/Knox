//! Operational database migration, validation, restore, and split commands.

use rusqlite::{Connection, DatabaseName, params};
use serde::Serialize;
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
        assert_eq!(before.schema_version, 10);
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
