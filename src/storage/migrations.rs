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

    let mut current_version = get_schema_version(conn)?;

    if current_version < 1 {
        apply_v1(conn)?;
        current_version = 1;
    }

    if current_version < 2 {
        apply_v2(conn)?;
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

const V2_SCHEMA_SQL: &str = "BEGIN TRANSACTION;

-- Extended columns on actions
ALTER TABLE actions ADD COLUMN rationale TEXT;
ALTER TABLE actions ADD COLUMN context_summary TEXT;
ALTER TABLE actions ADD COLUMN rejected_alternatives TEXT;

-- Normalized action_relations table
CREATE TABLE IF NOT EXISTS action_relations (
    from_action_id TEXT NOT NULL,
    to_action_id   TEXT NOT NULL,
    relation_type  TEXT NOT NULL,
    PRIMARY KEY (from_action_id, to_action_id, relation_type),
    FOREIGN KEY (from_action_id) REFERENCES actions(id) ON DELETE CASCADE,
    FOREIGN KEY (to_action_id)   REFERENCES actions(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_action_relations_from ON action_relations(from_action_id);
CREATE INDEX IF NOT EXISTS idx_action_relations_to   ON action_relations(to_action_id);
CREATE INDEX IF NOT EXISTS idx_action_relations_type ON action_relations(relation_type);

-- Normalized action_symbols table
CREATE TABLE IF NOT EXISTS action_symbols (
    action_id TEXT NOT NULL,
    symbol    TEXT NOT NULL,
    PRIMARY KEY (action_id, symbol),
    FOREIGN KEY (action_id) REFERENCES actions(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_action_symbols_action ON action_symbols(action_id);
CREATE INDEX IF NOT EXISTS idx_action_symbols_symbol ON action_symbols(symbol);

-- Rebuild FTS5 virtual table indexing summary, rationale, and context_summary
DROP TRIGGER IF EXISTS actions_ai;
DROP TRIGGER IF EXISTS actions_au;
DROP TABLE IF EXISTS actions_fts;

CREATE VIRTUAL TABLE IF NOT EXISTS actions_fts USING fts5(
    summary,
    rationale,
    context_summary,
    content='actions',
    content_rowid='rowid'
);

INSERT INTO actions_fts(rowid, summary, rationale, context_summary)
SELECT rowid, summary, coalesce(rationale, ''), coalesce(context_summary, '') FROM actions;

CREATE TRIGGER IF NOT EXISTS actions_ai AFTER INSERT ON actions BEGIN
    INSERT INTO actions_fts(rowid, summary, rationale, context_summary)
    VALUES (new.rowid, new.summary, coalesce(new.rationale, ''), coalesce(new.context_summary, ''));
END;

CREATE TRIGGER IF NOT EXISTS actions_au AFTER UPDATE ON actions BEGIN
    INSERT INTO actions_fts(actions_fts, rowid, summary, rationale, context_summary)
    VALUES ('delete', old.rowid, old.summary, coalesce(old.rationale, ''), coalesce(old.context_summary, ''));
    INSERT INTO actions_fts(rowid, summary, rationale, context_summary)
    VALUES (new.rowid, new.summary, coalesce(new.rationale, ''), coalesce(new.context_summary, ''));
END;

CREATE TRIGGER IF NOT EXISTS actions_ad AFTER DELETE ON actions BEGIN
    INSERT INTO actions_fts(actions_fts, rowid, summary, rationale, context_summary)
    VALUES ('delete', old.rowid, old.summary, coalesce(old.rationale, ''), coalesce(old.context_summary, ''));
END;

UPDATE knox_meta SET value = '2' WHERE key = 'schema_version';

COMMIT;";

/// Applies version 2 of the Knox database schema.
fn apply_v2(conn: &Connection) -> KnoxResult<()> {
    conn.execute_batch(V2_SCHEMA_SQL)?;
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
        assert_eq!(version, 2);

        // Re-running migrations should be idempotent
        run_migrations(&conn).unwrap();
        let version2 = get_schema_version(&conn).unwrap();
        assert_eq!(version2, 2);
    }
}
