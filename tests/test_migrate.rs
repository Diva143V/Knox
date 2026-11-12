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
