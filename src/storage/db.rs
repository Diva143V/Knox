//! SQLite database connection management for Knox.

use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

use super::migrations;
use crate::core::error::KnoxResult;

/// Comprehensive telemetry and disk usage statistics for the Knox SQLite database.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StorageStats {
    pub db_size_bytes: u64,
    pub wal_size_bytes: u64,
    pub page_count: i64,
    pub page_size: i64,
    pub total_actions: usize,
    pub total_sessions: usize,
    pub total_relations: usize,
    pub schema_version: i32,
}

/// The central SQLite database handle for Knox.
/// This is the only struct in the codebase that encapsulates the underlying database connection.
pub struct KnoxDb {
    conn: Connection,
}

impl KnoxDb {
    /// Opens or creates the SQLite database at the given filesystem path.
    /// Ensures parent directories exist, sets WAL mode and foreign keys, and applies migrations.
    pub fn open(path: &Path) -> KnoxResult<Self> {
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                fs::create_dir_all(parent)?;
            }
        }

        let conn = Connection::open(path)?;
        let _: String = conn.query_row("PRAGMA journal_mode = WAL", [], |r| r.get(0))?;
        conn.pragma_update(None, "foreign_keys", "ON")?;
        conn.pragma_update(None, "synchronous", "NORMAL")?;

        let db = Self { conn };
        db.migrate()?;
        tracing::info!(path = %path.display(), "Opened Knox SQLite database");
        Ok(db)
    }

    /// Opens an in-memory SQLite database with foreign keys enabled and migrations applied.
    /// Ideal for fast, isolated tests without filesystem dependencies.
    pub fn open_memory() -> KnoxResult<Self> {
        let conn = Connection::open_in_memory()?;
        conn.pragma_update(None, "foreign_keys", "ON")?;

        let db = Self { conn };
        db.migrate()?;
        tracing::debug!("Opened in-memory Knox SQLite database");
        Ok(db)
    }

    /// Applies all pending database migrations.
    fn migrate(&self) -> KnoxResult<()> {
        migrations::run_migrations(&self.conn)
    }

    /// Runs a SQLite integrity check and returns "ok" or failure diagnostics.
    pub fn integrity_check(&self) -> KnoxResult<String> {
        let result: String = self.conn.query_row("PRAGMA integrity_check", [], |r| r.get(0))?;
        Ok(result)
    }

    /// Reclaims unused database pages and defragments storage via SQLite VACUUM.
    pub fn vacuum(&self) -> KnoxResult<()> {
        self.conn.execute("VACUUM", [])?;
        Ok(())
    }

    /// Rebuilds database indexes via SQLite REINDEX.
    pub fn reindex(&self) -> KnoxResult<()> {
        self.conn.execute("REINDEX", [])?;
        Ok(())
    }

    /// Gathers live storage telemetry, schema version, and node counts.
    pub fn get_storage_stats(&self, db_path: Option<&Path>) -> KnoxResult<StorageStats> {
        let page_count: i64 = self
            .conn
            .query_row("PRAGMA page_count", [], |r| r.get(0))
            .unwrap_or(0);
        let page_size: i64 = self
            .conn
            .query_row("PRAGMA page_size", [], |r| r.get(0))
            .unwrap_or(4096);
        let total_actions: usize = self
            .conn
            .query_row("SELECT COUNT(*) FROM actions", [], |r| r.get::<_, i64>(0))
            .map(|c| c as usize)
            .unwrap_or(0);
        let total_sessions: usize = self
            .conn
            .query_row("SELECT COUNT(*) FROM sessions", [], |r| r.get::<_, i64>(0))
            .map(|c| c as usize)
            .unwrap_or(0);
        let total_relations: usize = self
            .conn
            .query_row("SELECT COUNT(*) FROM action_relations", [], |r| {
                r.get::<_, i64>(0)
            })
            .map(|c| c as usize)
            .unwrap_or(0);

        let schema_version: i32 = self
            .conn
            .query_row(
                "SELECT value FROM knox_meta WHERE key = 'schema_version'",
                [],
                |r| r.get::<_, String>(0),
            )
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(0);

        let (db_size_bytes, wal_size_bytes) = if let Some(path) = db_path {
            let db_len = fs::metadata(path)
                .map(|m| m.len())
                .unwrap_or_else(|_| (page_count * page_size) as u64);
            let mut wal_path = path.to_path_buf();
            let orig_file_name = wal_path
                .file_name()
                .and_then(|f| f.to_str())
                .unwrap_or("knox.db");
            wal_path.set_file_name(format!("{orig_file_name}-wal"));
            let wal_len = fs::metadata(wal_path).map(|m| m.len()).unwrap_or(0);
            (db_len, wal_len)
        } else {
            ((page_count * page_size) as u64, 0)
        };

        Ok(StorageStats {
            db_size_bytes,
            wal_size_bytes,
            page_count,
            page_size,
            total_actions,
            total_sessions,
            total_relations,
            schema_version,
        })
    }

    /// Provides access to the underlying connection for internal storage modules.
    pub(crate) fn conn(&self) -> &Connection {
        &self.conn
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_open_memory_creates_and_migrates() {
        let db = KnoxDb::open_memory().unwrap();
        let mut stmt = db
            .conn()
            .prepare("SELECT key, value FROM knox_meta")
            .unwrap();
        let rows = stmt
            .query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })
            .unwrap();

        let count = rows.count();
        assert!(count >= 3);
    }

    #[test]
    fn test_maintenance_and_stats() {
        let db = KnoxDb::open_memory().unwrap();
        let check = db.integrity_check().unwrap();
        assert_eq!(check.to_lowercase(), "ok");

        db.vacuum().unwrap();
        db.reindex().unwrap();

        let stats = db.get_storage_stats(None).unwrap();
        assert_eq!(stats.total_actions, 0);
        assert_eq!(stats.total_sessions, 0);
        assert!(stats.schema_version >= 3);
    }
}
