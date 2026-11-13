//! Integration tests for database migration and consolidation engine.

use chrono::Utc;
use knox::commands::migrate::migration_consolidate;
use knox::core::*;
use knox::storage::*;

#[test]
fn test_consolidate_idempotency() {
    let temp_src = tempfile::tempdir().unwrap();
    let src_path = temp_src.path().join("legacy.db");
    let src_db = KnoxDb::open(&src_path).unwrap();

    let project = Project {
        id: ProjectId::new(),
        name: "Legacy Project".to_string(),
        path: "/path/to/legacy".to_string(),
        remote: None,
        created_at: Utc::now(),
    };
    insert_project(&src_db, &project).unwrap();

    let session = start_session(&src_db, "legacy_agent", Some("/path/to/legacy")).unwrap();

    let a1 = KnoxAction {
        id: ActionId::new(),
        actor: "legacy_agent".to_string(),
        action_type: "write".to_string(),
        summary: "Legacy action 1".to_string(),
        session_id: Some(session.id.clone()),
        dedupe_key: Some("dedupe-1".to_string()),
        ..Default::default()
    };
    insert_action(&src_db, &a1).unwrap();

    let a2 = KnoxAction {
        id: ActionId::new(),
        actor: "legacy_agent".to_string(),
        action_type: "plan".to_string(),
        summary: "Legacy action 2".to_string(),
        session_id: Some(session.id),
        parent_action_id: Some(a1.id.clone()),
        dedupe_key: Some("dedupe-2".to_string()),
        ..Default::default()
    };
    insert_action(&src_db, &a2).unwrap();

    // Create target global database
    let temp_target = tempfile::tempdir().unwrap();
    let target_path = temp_target.path().join("global.db");
    let target_db = KnoxDb::open(&target_path).unwrap();

    // First consolidation pass
    let r1 = migration_consolidate(&target_db, &src_path).unwrap();
    assert_eq!(r1.projects_moved, 1);
    assert_eq!(r1.sessions_moved, 1);
    assert_eq!(r1.actions_moved, 2);
    assert_eq!(r1.actions_skipped, 0);
    assert_eq!(r1.edges_moved, 1);

    // Second consolidation pass must be idempotent
    let r2 = migration_consolidate(&target_db, &src_path).unwrap();
    assert_eq!(r2.actions_moved, 0);
    assert_eq!(r2.actions_skipped, 2);
    assert_eq!(r2.edges_moved, 0);
}

#[test]
fn test_consolidate_foreign_keys() {
    let temp_src = tempfile::tempdir().unwrap();
    let src_path = temp_src.path().join("legacy.db");
    let src_db = KnoxDb::open(&src_path).unwrap();

    let project = Project {
        id: ProjectId::new(),
        name: "FK Project".to_string(),
        path: "/path/to/fk_project".to_string(),
        remote: None,
        created_at: Utc::now(),
    };
    insert_project(&src_db, &project).unwrap();

    let session = start_session(&src_db, "fk_agent", Some("/path/to/fk_project")).unwrap();

    let a1 = KnoxAction {
        id: ActionId::new(),
        actor: "fk_agent".to_string(),
        action_type: "write".to_string(),
        summary: "FK action 1".to_string(),
        session_id: Some(session.id.clone()),
        project_id: Some(project.id.clone()),
        dedupe_key: Some("fk-1".to_string()),
        ..Default::default()
    };
    insert_action(&src_db, &a1).unwrap();

    let temp_target = tempfile::tempdir().unwrap();
    let target_path = temp_target.path().join("global.db");
    let target_db = KnoxDb::open(&target_path).unwrap();

    let r = migration_consolidate(&target_db, &src_path).unwrap();
    assert_eq!(r.actions_moved, 1);

    // Verify imported action in target
    let target_action = get_action(&target_db, &a1.id).unwrap().expect("action present in target");
    assert_eq!(target_action.summary, "FK action 1");

    // Target database projects list contains imported project
    let projs = list_projects(&target_db).unwrap();
    assert!(projs.iter().any(|p| p.path == "/path/to/fk_project"));
}

#[test]
fn test_consolidate_legacy_v8_schema() {
    let temp_src = tempfile::tempdir().unwrap();
    let src_path = temp_src.path().join("legacy_v8.db");
    
    // Create raw schema v8 database manually
    {
        let conn = knox::rusqlite::Connection::open(&src_path).unwrap();
        conn.execute_batch(
            "CREATE TABLE knox_meta (key TEXT PRIMARY KEY, value TEXT NOT NULL);
             INSERT INTO knox_meta (key, value) VALUES ('schema_version', '8'), ('knox_version', '0.1.0');
             
             CREATE TABLE projects (
                 id TEXT PRIMARY KEY,
                 name TEXT NOT NULL,
                 path TEXT NOT NULL UNIQUE,
                 created_at TEXT NOT NULL
             );
             INSERT INTO projects (id, name, path, created_at)
             VALUES ('proj-v8', 'Legacy V8 Proj', '/path/to/legacy_v8', '2026-11-05T00:00:00Z');
             
             CREATE TABLE sessions (
                 id TEXT PRIMARY KEY,
                 actor TEXT NOT NULL,
                 project_id TEXT,
                 started_at TEXT NOT NULL,
                 ended_at TEXT,
                 is_open INTEGER NOT NULL DEFAULT 1
             );
             INSERT INTO sessions (id, actor, project_id, started_at, ended_at, is_open)
             VALUES ('sess-v8', 'agent-v8', 'proj-v8', '2026-11-05T00:00:00Z', NULL, 1);
             
             CREATE TABLE actions (
                 id TEXT PRIMARY KEY,
                 actor TEXT NOT NULL,
                 action_type TEXT NOT NULL,
                 summary TEXT NOT NULL,
                 confidence REAL,
                 timestamp TEXT NOT NULL,
                 session_id TEXT,
                 parent_action_id TEXT,
                 superseded_by TEXT,
                 is_current INTEGER NOT NULL DEFAULT 1,
                 rationale TEXT,
                 context_summary TEXT,
                 rejected_alternatives TEXT,
                 significance TEXT DEFAULT 'major',
                 is_resolved INTEGER DEFAULT 1,
                 resolution_note TEXT,
                 commit_sha TEXT,
                 project_id TEXT,
                 timestamp_ms INTEGER,
                 dedupe_key TEXT,
                 seen_count INTEGER NOT NULL DEFAULT 1
             );
             INSERT INTO actions (id, actor, action_type, summary, timestamp, session_id, project_id)
             VALUES ('act-1', 'agent-v8', 'decision', 'Legacy action without v9 fields', '2026-11-05T00:01:00Z', 'sess-v8', 'proj-v8'),
                    ('act-2', 'agent-v8', 'plan', 'Legacy child action', '2026-11-05T00:02:00Z', 'sess-v8', 'proj-v8');
                    
             CREATE TABLE edges (
                 id TEXT PRIMARY KEY,
                 from_action_id TEXT NOT NULL,
                 to_action_id TEXT NOT NULL,
                 edge_type TEXT NOT NULL,
                 created_at TEXT NOT NULL
             );
             INSERT INTO edges (id, from_action_id, to_action_id, edge_type, created_at)
             VALUES ('edge-1', 'act-2', 'act-1', 'follows', '2026-11-05T00:02:00Z');"
        ).unwrap();
    }

    let temp_target = tempfile::tempdir().unwrap();
    let target_path = temp_target.path().join("global.db");
    let target_db = KnoxDb::open(&target_path).unwrap();

    let report = migration_consolidate(&target_db, &src_path).expect("v8 migration must succeed");
    assert_eq!(report.projects_moved, 1);
    assert_eq!(report.sessions_moved, 1);
    assert_eq!(report.actions_moved, 2);
    assert_eq!(report.edges_moved, 1);

    // Verify edge was mapped from 'follows' to 'caused_by'
    let edge_type: String = target_db.with_conn(|conn| {
        conn.query_row(
            "SELECT edge_type FROM edges WHERE from_action_id = 'act-2' AND to_id = 'act-1'",
            [],
            |r| r.get(0),
        ).map_err(KnoxError::Database)
    }).unwrap();
    assert_eq!(edge_type, "caused_by");
}
