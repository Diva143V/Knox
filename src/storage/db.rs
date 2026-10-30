//! SQLite database connection management for Knox.

use rusqlite::Connection;
use std::fs;
use std::path::Path;

use super::migrations;
use crate::core::error::KnoxResult;

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
}
