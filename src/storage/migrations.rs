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
        current_version = 2;
    }

    if current_version < 3 {
        apply_v3(conn)?;
        current_version = 3;
    }

    if current_version < 4 {
        apply_v4(conn)?;
        current_version = 4;
    }

    if current_version < 5 {
        apply_v5(conn)?;
        current_version = 5;
    }

    if current_version < 6 {
        apply_v6(conn)?;
        current_version = 6;
    }

    if current_version < 7 {
        apply_v7(conn)?;
    }

    if current_version < 8 {
        apply_v8(conn)?;
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

const V3_SCHEMA_SQL: &str = "BEGIN TRANSACTION;

-- Extended cognitive attributes on actions table
ALTER TABLE actions ADD COLUMN significance TEXT DEFAULT 'major';
ALTER TABLE actions ADD COLUMN is_resolved INTEGER DEFAULT 1;
ALTER TABLE actions ADD COLUMN resolution_note TEXT;

UPDATE knox_meta SET value = '3' WHERE key = 'schema_version';

COMMIT;";

/// Applies version 3 of the Knox database schema.
fn apply_v3(conn: &Connection) -> KnoxResult<()> {
    conn.execute_batch(V3_SCHEMA_SQL)?;
    Ok(())
}

const V4_SCHEMA_SQL: &str = "BEGIN TRANSACTION;

-- Add optional commit SHA tracking to actions
ALTER TABLE actions ADD COLUMN commit_sha TEXT;

-- Table storing granular patch hunks and deleted code blocks
CREATE TABLE IF NOT EXISTS action_deltas (
    id                TEXT PRIMARY KEY,
    action_id         TEXT NOT NULL,
    file_path         TEXT NOT NULL,
    change_type       TEXT NOT NULL CHECK(change_type IN ('added', 'modified', 'deleted', 'renamed')),
    old_path          TEXT,
    lines_added       INTEGER NOT NULL DEFAULT 0,
    lines_deleted     INTEGER NOT NULL DEFAULT 0,
    diff_hunk         TEXT,
    deleted_code      TEXT,
    deletion_reason   TEXT,
    created_at        TEXT NOT NULL,
    FOREIGN KEY (action_id) REFERENCES actions(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_actions_commit_sha     ON actions(commit_sha);
CREATE INDEX IF NOT EXISTS idx_action_deltas_action   ON action_deltas(action_id);
CREATE INDEX IF NOT EXISTS idx_action_deltas_file     ON action_deltas(file_path);
CREATE INDEX IF NOT EXISTS idx_action_deltas_type     ON action_deltas(change_type);

UPDATE knox_meta SET value = '4' WHERE key = 'schema_version';

COMMIT;";

/// Applies version 4 of the Knox database schema.
fn apply_v4(conn: &Connection) -> KnoxResult<()> {
    conn.execute_batch(V4_SCHEMA_SQL)?;
    Ok(())
}

const V5_SCHEMA_SQL: &str = "BEGIN TRANSACTION;

-- Project scope on actions. Project filtering used to go exclusively through
-- sessions, which made session-less writes (git hook, `knox scan`, transcript
-- ingestion) invisible to every project-filtered query.
ALTER TABLE actions ADD COLUMN project_id TEXT;
CREATE INDEX IF NOT EXISTS idx_actions_project ON actions(project_id);

-- Fixed-precision epoch milliseconds for ordering. RFC 3339 text with variable
-- fractional digits does not sort correctly as a string.
ALTER TABLE actions ADD COLUMN timestamp_ms INTEGER;
CREATE INDEX IF NOT EXISTS idx_actions_timestamp_ms ON actions(timestamp_ms);

UPDATE actions
   SET project_id = (SELECT s.project_id FROM sessions s WHERE s.id = actions.session_id)
 WHERE project_id IS NULL AND session_id IS NOT NULL;

UPDATE actions
   SET timestamp_ms = CAST(strftime('%s', timestamp) AS INTEGER) * 1000
 WHERE timestamp_ms IS NULL;

UPDATE knox_meta SET value = '5' WHERE key = 'schema_version';

COMMIT;";

/// Applies version 5 of the Knox database schema.
fn apply_v5(conn: &Connection) -> KnoxResult<()> {
    conn.execute_batch(V5_SCHEMA_SQL)?;
    Ok(())
}

const V6_SCHEMA_SQL: &str = "BEGIN TRANSACTION;

-- Derived lifecycle state is explicit and can be updated transactionally with
-- supersession/resolution relationships without changing the v5 action model.
CREATE TABLE IF NOT EXISTS action_state (
    action_id TEXT PRIMARY KEY REFERENCES actions(id) ON DELETE CASCADE,
    state TEXT NOT NULL CHECK (state IN ('current', 'superseded', 'resolved', 'open')),
    by_action_id TEXT REFERENCES actions(id),
    updated_at TEXT NOT NULL
);

INSERT OR IGNORE INTO action_state (action_id, state, updated_at)
SELECT id,
       CASE
           WHEN action_type IN ('doubt', 'question') AND coalesce(is_resolved, 0) = 0 THEN 'open'
           WHEN is_current = 0 THEN 'superseded'
           WHEN action_type IN ('doubt', 'question') AND coalesce(is_resolved, 0) = 1 THEN 'resolved'
           ELSE 'current'
       END,
       timestamp
FROM actions;

CREATE TABLE IF NOT EXISTS guardrail_rules (
    id TEXT PRIMARY KEY,
    action_id TEXT NOT NULL REFERENCES actions(id) ON DELETE CASCADE,
    project_id TEXT REFERENCES projects(id),
    pattern_kind TEXT NOT NULL CHECK (pattern_kind IN ('path', 'symbol', 'phrase')),
    pattern TEXT NOT NULL,
    severity TEXT NOT NULL CHECK (severity IN ('strict', 'advisory')),
    active INTEGER NOT NULL DEFAULT 1 CHECK (active IN (0, 1))
);

CREATE INDEX IF NOT EXISTS idx_guardrail_rules_project
    ON guardrail_rules(project_id, active);
CREATE INDEX IF NOT EXISTS idx_guardrail_rules_pattern
    ON guardrail_rules(pattern_kind, pattern);

CREATE TABLE IF NOT EXISTS ingest_log (
    source_hash TEXT NOT NULL,
    source_offset INTEGER NOT NULL,
    outcome TEXT NOT NULL,
    rule_id TEXT,
    reason TEXT,
    action_id TEXT REFERENCES actions(id),
    PRIMARY KEY (source_hash, source_offset)
);

UPDATE knox_meta SET value = '6' WHERE key = 'schema_version';

COMMIT;";

/// Applies the additive v6 storage foundation.
fn apply_v6(conn: &Connection) -> KnoxResult<()> {
    conn.execute_batch(V6_SCHEMA_SQL)?;
    Ok(())
}

const V7_SCHEMA_SQL: &str = "BEGIN TRANSACTION;

-- Dedupe: identical or near-identical writes inside the merge window increment
-- seen_count instead of adding a second row (D5).
ALTER TABLE actions ADD COLUMN dedupe_key TEXT;
ALTER TABLE actions ADD COLUMN seen_count INTEGER NOT NULL DEFAULT 1;
CREATE INDEX IF NOT EXISTS idx_actions_dedupe
    ON actions(dedupe_key, timestamp_ms);

-- Idempotency: one key per agent turn. The stored response is replayed when the
-- same key and payload hash are seen again; a different payload is a conflict.
CREATE TABLE IF NOT EXISTS idempotency_keys (
    key TEXT PRIMARY KEY,
    payload_hash TEXT NOT NULL,
    response_json TEXT NOT NULL,
    created_at TEXT NOT NULL
);

-- Session lineage: a resumed session can point at the session it continues.
ALTER TABLE sessions ADD COLUMN parent_session_id TEXT REFERENCES sessions(id);

UPDATE knox_meta SET value = '7' WHERE key = 'schema_version';

COMMIT;";

/// Applies the additive v7 write-path foundation (dedupe + idempotency).
fn apply_v7(conn: &Connection) -> KnoxResult<()> {
    conn.execute_batch(V7_SCHEMA_SQL)?;
    Ok(())
}

const V8_SCHEMA_SQL: &str = "BEGIN TRANSACTION;

-- Edge table: unified typed causal graph edges.
-- Replaces the need to query parent_action_id, superseded_by, and
-- action_relations separately. One query: \"everything connected to this\".
CREATE TABLE IF NOT EXISTS edges (
    id TEXT PRIMARY KEY,
    from_action_id TEXT NOT NULL,
    to_action_id TEXT NOT NULL,
    edge_type TEXT NOT NULL
        CHECK (edge_type IN ('follows', 'contradicts', 'implements', 'depends_on',
                             'verifies', 'relates_to', 'reverts', 'supersedes')),
    created_at TEXT NOT NULL,
    FOREIGN KEY (from_action_id) REFERENCES actions(id) ON DELETE CASCADE,
    FOREIGN KEY (to_action_id)   REFERENCES actions(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_edges_from ON edges(from_action_id);
CREATE INDEX IF NOT EXISTS idx_edges_to   ON edges(to_action_id);
CREATE INDEX IF NOT EXISTS idx_edges_type ON edges(edge_type);

-- Keep edges in sync with actions.parent_action_id (follows edges).
CREATE TRIGGER IF NOT EXISTS edges_ai_follows AFTER INSERT ON actions BEGIN
    INSERT INTO edges(id, from_action_id, to_action_id, edge_type, created_at)
    SELECT new.id, new.id, new.parent_action_id, 'follows', new.timestamp
    WHERE new.parent_action_id IS NOT NULL;
END;

-- Keep edges in sync with actions.superseded_by (supersedes edges).
-- superseded_by is set by mark_superseded_conn, not in the initial INSERT.
CREATE TRIGGER IF NOT EXISTS edges_au_supersedes AFTER UPDATE OF superseded_by ON actions BEGIN
    INSERT INTO edges(id, from_action_id, to_action_id, edge_type, created_at)
    SELECT new.id, old.id, new.superseded_by, 'supersedes', new.timestamp
    WHERE new.superseded_by IS NOT NULL AND old.superseded_by IS NULL;
END;

-- Migrate existing action_relations into edges.
INSERT OR IGNORE INTO edges(id, from_action_id, to_action_id, edge_type, created_at)
SELECT
    from_action_id || ':' || to_action_id || ':' || relation_type,
    from_action_id,
    to_action_id,
    relation_type,
    (SELECT timestamp FROM actions WHERE id = from_action_id LIMIT 1)
FROM action_relations;

UPDATE knox_meta SET value = '8' WHERE key = 'schema_version';

COMMIT;";

/// Applies the additive v8 edge-table foundation (typed causal edges).
fn apply_v8(conn: &Connection) -> KnoxResult<()> {
    conn.execute_batch(V8_SCHEMA_SQL)?;
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
        assert_eq!(version, 8);

        // Re-running migrations should be idempotent
        run_migrations(&conn).unwrap();
        let version2 = get_schema_version(&conn).unwrap();
        assert_eq!(version2, 8);
    }
}
