//! SQLite database schema migrations for Knox.

use crate::core::error::KnoxResult;
use rusqlite::Connection;

/// Runs all pending database migrations in sequence.
pub fn run_migrations(conn: &Connection) -> KnoxResult<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS knox_meta (
            key   TEXT PRIMARY KEY,
            value TEXT NOT NULL
        );",
    )?;

    let current_version = get_schema_version(conn)?;

    if current_version < 1 {
        apply_v1(conn)?;
    }

    Ok(())
}

/// Retrieves the current schema version from `knox_meta`.
fn get_schema_version(conn: &Connection) -> KnoxResult<i32> {
    let mut stmt = conn.prepare("SELECT value FROM knox_meta WHERE key = 'schema_version'")?;
    let mut rows = stmt.query([])?;

    if let Some(row) = rows.next()? {
        let version_str: String = row.get(0)?;
        Ok(version_str.parse::<i32>().unwrap_or(0))
    } else {
        Ok(0)
    }
}

const V1_SCHEMA_SQL: &str = "BEGIN TRANSACTION;

CREATE TABLE IF NOT EXISTS projects (
    id         TEXT PRIMARY KEY,
    name       TEXT NOT NULL,
    path       TEXT NOT NULL UNIQUE,
    created_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS sessions (
    id         TEXT PRIMARY KEY,
    actor      TEXT NOT NULL,
    project_id TEXT,
    started_at TEXT NOT NULL,
    ended_at   TEXT,
    is_open    INTEGER NOT NULL DEFAULT 1,
    FOREIGN KEY (project_id) REFERENCES projects(id)
);

CREATE TABLE IF NOT EXISTS actions (
    id               TEXT PRIMARY KEY,
    actor            TEXT NOT NULL,
    action_type      TEXT NOT NULL,
    summary          TEXT NOT NULL,
    confidence       REAL,
    timestamp        TEXT NOT NULL,
    session_id       TEXT,
    parent_action_id TEXT,
    superseded_by    TEXT,
    is_current       INTEGER NOT NULL DEFAULT 1,
    FOREIGN KEY (session_id) REFERENCES sessions(id),
    FOREIGN KEY (parent_action_id) REFERENCES actions(id),
    FOREIGN KEY (superseded_by) REFERENCES actions(id)
);

CREATE TABLE IF NOT EXISTS action_files (
    action_id TEXT NOT NULL,
    file_path TEXT NOT NULL,
    PRIMARY KEY (action_id, file_path),
    FOREIGN KEY (action_id) REFERENCES actions(id)
);

CREATE TABLE IF NOT EXISTS action_tags (
    action_id TEXT NOT NULL,
    tag       TEXT NOT NULL,
    PRIMARY KEY (action_id, tag),
    FOREIGN KEY (action_id) REFERENCES actions(id)
);

CREATE INDEX IF NOT EXISTS idx_actions_session     ON actions(session_id);
CREATE INDEX IF NOT EXISTS idx_actions_actor        ON actions(actor);
CREATE INDEX IF NOT EXISTS idx_actions_timestamp    ON actions(timestamp);
CREATE INDEX IF NOT EXISTS idx_actions_is_current   ON actions(is_current);
CREATE INDEX IF NOT EXISTS idx_actions_parent       ON actions(parent_action_id);
CREATE INDEX IF NOT EXISTS idx_action_files_path    ON action_files(file_path);
CREATE INDEX IF NOT EXISTS idx_action_files_action  ON action_files(action_id);
CREATE INDEX IF NOT EXISTS idx_action_tags_tag      ON action_tags(tag);

CREATE VIRTUAL TABLE IF NOT EXISTS actions_fts USING fts5(
    summary,
    content='actions',
    content_rowid='rowid'
);

CREATE TRIGGER IF NOT EXISTS actions_ai AFTER INSERT ON actions BEGIN
    INSERT INTO actions_fts(rowid, summary) VALUES (new.rowid, new.summary);
END;

CREATE TRIGGER IF NOT EXISTS actions_au AFTER UPDATE ON actions BEGIN
    INSERT INTO actions_fts(actions_fts, rowid, summary) VALUES('delete', old.rowid, old.summary);
    INSERT INTO actions_fts(rowid, summary) VALUES (new.rowid, new.summary);
END;

INSERT OR REPLACE INTO knox_meta (key, value) VALUES ('schema_version', '1');
INSERT OR REPLACE INTO knox_meta (key, value) VALUES ('knox_version', '0.1.0');
INSERT OR REPLACE INTO knox_meta (key, value) VALUES ('created_at', datetime('now'));

COMMIT;";

/// Applies version 1 of the Knox database schema.
fn apply_v1(conn: &Connection) -> KnoxResult<()> {
    conn.execute_batch(V1_SCHEMA_SQL)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_migrations_apply_cleanly() {
        let conn = Connection::open_in_memory().unwrap();
        run_migrations(&conn).unwrap();
        let version = get_schema_version(&conn).unwrap();
        assert_eq!(version, 1);

        // Re-running migrations should be idempotent
        run_migrations(&conn).unwrap();
        let version2 = get_schema_version(&conn).unwrap();
        assert_eq!(version2, 1);
    }
}
