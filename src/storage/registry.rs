//! Cross-project registry stored at `~/.knox/registry.db`.
//!
//! The registry is a lightweight index: it never holds memory content, only the
//! root path, display name, optional git remote, and when the project was last
//! opened. Project-local databases stay authoritative; this file just makes
//! them discoverable.

use chrono::Utc;
use rusqlite::{Connection, OptionalExtension, params};

use crate::core::config::home_dir;
use crate::core::error::KnoxResult;

/// One row in the cross-project registry.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RegistryEntry {
    pub root: String,
    pub name: String,
    pub remote: Option<String>,
    pub last_seen: String,
}

/// Default registry path (`~/.knox/registry.db`), overridable with
/// `KNOX_REGISTRY_PATH` for tests and CI.
pub fn registry_path() -> std::path::PathBuf {
    if let Ok(path) = std::env::var("KNOX_REGISTRY_PATH") {
        if !path.trim().is_empty() {
            return std::path::PathBuf::from(path.trim());
        }
    }
    home_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join(".knox")
        .join("registry.db")
}

fn open_registry(path: &std::path::Path) -> KnoxResult<Connection> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)?;
        }
    }
    let conn = Connection::open(path)?;
    conn.busy_timeout(std::time::Duration::from_secs(5))?;
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS projects (
            root TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            remote TEXT,
            last_seen TEXT NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_registry_name ON projects(name);",
    )?;
    Ok(conn)
}

/// Registers or refreshes a project in the default registry.
pub fn register_project(root: &str, name: &str, remote: Option<&str>) -> KnoxResult<()> {
    register_project_at(&registry_path(), root, name, remote)
}

/// Registers or refreshes a project at an explicit registry path.
pub fn register_project_at(
    registry: &std::path::Path,
    root: &str,
    name: &str,
    remote: Option<&str>,
) -> KnoxResult<()> {
    let conn = open_registry(registry)?;
    conn.execute(
        "INSERT INTO projects (root, name, remote, last_seen)
         VALUES (?1, ?2, ?3, ?4)
         ON CONFLICT(root) DO UPDATE SET
             name = excluded.name,
             remote = COALESCE(excluded.remote, projects.remote),
             last_seen = excluded.last_seen",
        params![root, name, remote, Utc::now().to_rfc3339()],
    )?;
    Ok(())
}

/// Lists every registered project, newest activity first.
pub fn list_registered_projects() -> KnoxResult<Vec<RegistryEntry>> {
    list_registered_projects_at(&registry_path())
}

/// Lists registered projects from an explicit registry path.
pub fn list_registered_projects_at(registry: &std::path::Path) -> KnoxResult<Vec<RegistryEntry>> {
    let conn = open_registry(registry)?;
    let mut stmt =
        conn.prepare("SELECT root, name, remote, last_seen FROM projects ORDER BY last_seen DESC")?;
    let entries = stmt
        .query_map([], |row| {
            Ok(RegistryEntry {
                root: row.get(0)?,
                name: row.get(1)?,
                remote: row.get(2)?,
                last_seen: row.get(3)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(entries)
}

/// Looks up a single registered project by its canonical root path.
pub fn find_registered_project(root: &str) -> KnoxResult<Option<RegistryEntry>> {
    find_registered_project_at(&registry_path(), root)
}

/// Looks up a single registered project at an explicit registry path.
pub fn find_registered_project_at(
    registry: &std::path::Path,
    root: &str,
) -> KnoxResult<Option<RegistryEntry>> {
    let conn = open_registry(registry)?;
    let entry = conn
        .query_row(
            "SELECT root, name, remote, last_seen FROM projects WHERE root = ?1",
            params![root],
            |row| {
                Ok(RegistryEntry {
                    root: row.get(0)?,
                    name: row.get(1)?,
                    remote: row.get(2)?,
                    last_seen: row.get(3)?,
                })
            },
        )
        .optional()?;
    Ok(entry)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registers_and_updates_projects() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("registry.db");

        register_project_at(&path, "/work/a", "a", Some("git@example.com:a.git")).unwrap();
        register_project_at(&path, "/work/b", "b", None).unwrap();
        register_project_at(&path, "/work/a", "a-renamed", None).unwrap();

        let entries = list_registered_projects_at(&path).unwrap();
        assert_eq!(entries.len(), 2);
        let a = find_registered_project_at(&path, "/work/a")
            .unwrap()
            .unwrap();
        assert_eq!(a.name, "a-renamed");
        assert_eq!(a.remote.as_deref(), Some("git@example.com:a.git"));
        assert!(
            find_registered_project_at(&path, "/work/missing")
                .unwrap()
                .is_none()
        );
    }
}
