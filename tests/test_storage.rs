//! Comprehensive integration tests for the Knox storage layer.

use chrono::Utc;
use knox::core::*;
use knox::storage::*;
use std::time::Duration;

#[test]
fn test_storage_insert_action_and_get_by_id() {
    let db = KnoxDb::open_memory().unwrap();
    let session = start_session(&db, "antigravity", None).unwrap();
    let action_id = ActionId::new();

    let action = KnoxAction {
        id: action_id.clone(),
        actor: "antigravity".to_string(),
        action_type: "write".to_string(),
        summary: "Implemented storage layer migrations and CRUD".to_string(),
        files: vec![
            "src/storage/db.rs".to_string(),
            "src/storage/nodes.rs".to_string(),
        ],
        tags: vec!["sqlite".to_string(), "storage".to_string()],
        confidence: Some(0.98),
        timestamp: Utc::now(),
        session_id: Some(session.id.clone()),
        parent_action_id: None,
        supersedes: None,
        is_current: true,
    };

    let inserted_id = insert_action(&db, &action).unwrap();
    assert_eq!(inserted_id, action_id);

    let fetched = get_action(&db, &action_id)
        .unwrap()
        .expect("action should be present");
    assert_eq!(fetched.id, action_id);
    assert_eq!(fetched.actor, "antigravity");
    assert_eq!(fetched.action_type, "write");
    assert_eq!(
        fetched.summary,
        "Implemented storage layer migrations and CRUD"
    );
    assert_eq!(
        fetched.files,
        vec!["src/storage/db.rs", "src/storage/nodes.rs"]
    );
    assert_eq!(fetched.tags, vec!["sqlite", "storage"]);
    assert_eq!(fetched.confidence, Some(0.98));
    assert_eq!(fetched.session_id, Some(session.id));
    assert!(fetched.is_current);
}

fn create_test_action(
    actor: &str,
    action_type: &str,
    summary: &str,
    file: &str,
    tag: &str,
    supersedes: Option<ActionId>,
) -> KnoxAction {
    KnoxAction {
        id: ActionId::new(),
        actor: actor.to_string(),
        action_type: action_type.to_string(),
        summary: summary.to_string(),
        files: vec![file.to_string()],
        tags: vec![tag.to_string()],
        confidence: None,
        timestamp: Utc::now(),
        session_id: None,
        parent_action_id: None,
        supersedes,
        is_current: true,
    }
}

#[test]
fn test_storage_query_by_tag_and_file() {
    let db = KnoxDb::open_memory().unwrap();

    let action1 = create_test_action(
        "human",
        "plan",
        "Design database architecture",
        "docs/schema.md",
        "database",
        None,
    );
    let action2 = create_test_action(
        "gemini",
        "write",
        "Build frontend dashboard view",
        "src/ui/dashboard.rs",
        "dashboard",
        None,
    );

    insert_action(&db, &action1).unwrap();
    insert_action(&db, &action2).unwrap();

    let filter_tag = QueryFilter {
        tags: vec!["database".to_string()],
        ..Default::default()
    };
    let res_tag = query_actions(&db, &filter_tag).unwrap();
    assert_eq!(res_tag.results.len(), 1);
    assert_eq!(res_tag.results[0].id, action1.id);

    let filter_file = QueryFilter {
        file_path: Some("src/ui/dashboard.rs".to_string()),
        ..Default::default()
    };
    let res_file = query_actions(&db, &filter_file).unwrap();
    assert_eq!(res_file.results.len(), 1);
    assert_eq!(res_file.results[0].id, action2.id);
}

#[test]
fn test_storage_supersede_and_current_only_filtering() {
    let db = KnoxDb::open_memory().unwrap();

    let action1 = create_test_action(
        "claude",
        "decide",
        "Approach 1: Redis",
        "cache.rs",
        "cache",
        None,
    );
    insert_action(&db, &action1).unwrap();

    let action2 = create_test_action(
        "antigravity",
        "decide",
        "Approach 2: LRU",
        "cache.rs",
        "cache",
        Some(action1.id.clone()),
    );
    insert_action(&db, &action2).unwrap();

    let current_res = query_actions(&db, &QueryFilter::default()).unwrap();
    assert_eq!(current_res.results.len(), 1);
    assert_eq!(current_res.results[0].id, action2.id);

    let filter_all = QueryFilter {
        current_only: false,
        ..Default::default()
    };
    let all_res = query_actions(&db, &filter_all).unwrap();
    assert_eq!(all_res.total_count, 2);
    assert_eq!(all_res.results.len(), 2);
}

#[test]
fn test_storage_session_lifecycle() {
    let db = KnoxDb::open_memory().unwrap();

    let session = start_session(&db, "test_agent", Some("knox")).unwrap();
    assert!(session.is_open);
    assert!(session.ended_at.is_none());

    let fetched_session = get_session(&db, &session.id).unwrap().unwrap();
    assert_eq!(fetched_session.actor, "test_agent");
    assert!(fetched_session.is_open);

    let ended = end_session(&db, &session.id).unwrap();
    assert!(!ended.is_open);
    assert!(ended.ended_at.is_some());
    assert!(ended.ended_at.unwrap() >= session.started_at);
}

#[test]
fn test_storage_timeout_stale_sessions() {
    let db = KnoxDb::open_memory().unwrap();

    let s1 = start_session(&db, "agent_1", None).unwrap();
    let s2 = start_session(&db, "agent_2", None).unwrap();

    // With 0 duration, both idle sessions should be closed
    let timed_out = timeout_stale_sessions(&db, Duration::from_secs(0)).unwrap();
    assert_eq!(timed_out.len(), 2);
    assert!(timed_out.contains(&s1.id));
    assert!(timed_out.contains(&s2.id));

    let f1 = get_session(&db, &s1.id).unwrap().unwrap();
    let f2 = get_session(&db, &s2.id).unwrap().unwrap();
    assert!(!f1.is_open);
    assert!(!f2.is_open);
}

#[test]
fn test_storage_fts_search() {
    let db = KnoxDb::open_memory().unwrap();

    let action1 = KnoxAction {
        id: ActionId::new(),
        actor: "cursor".to_string(),
        action_type: "write".to_string(),
        summary: "Implemented tokenizer and abstract syntax tree parser".to_string(),
        files: vec!["parser.rs".to_string()],
        tags: vec![],
        confidence: None,
        timestamp: Utc::now(),
        session_id: None,
        parent_action_id: None,
        supersedes: None,
        is_current: true,
    };
    let action2 = KnoxAction {
        id: ActionId::new(),
        actor: "gemini".to_string(),
        action_type: "write".to_string(),
        summary: "Added http connection pooling and keepalive".to_string(),
        files: vec!["http.rs".to_string()],
        tags: vec![],
        confidence: None,
        timestamp: Utc::now(),
        session_id: None,
        parent_action_id: None,
        supersedes: None,
        is_current: true,
    };

    insert_action(&db, &action1).unwrap();
    insert_action(&db, &action2).unwrap();

    let search_results = search_actions(&db, "tokenizer syntax tree", 10).unwrap();
    assert_eq!(search_results.len(), 1);
    assert_eq!(search_results[0].id, action1.id);

    let search_http = search_actions(&db, "connection pooling", 10).unwrap();
    assert_eq!(search_http.len(), 1);
    assert_eq!(search_http[0].id, action2.id);
}

#[test]
fn test_storage_status_report() {
    let db = KnoxDb::open_memory().unwrap();

    let s1 = start_session(&db, "gemini", Some("knox")).unwrap();

    let action = KnoxAction {
        id: ActionId::new(),
        actor: "gemini".to_string(),
        action_type: "plan".to_string(),
        summary: "Planning release".to_string(),
        files: vec![],
        tags: vec![],
        confidence: None,
        timestamp: Utc::now(),
        session_id: Some(s1.id),
        parent_action_id: None,
        supersedes: None,
        is_current: true,
    };
    insert_action(&db, &action).unwrap();

    let report = get_status_report(&db).unwrap();
    assert_eq!(report.total_actions, 1);
    assert_eq!(report.total_sessions, 1);
    assert_eq!(report.projects_tracked, 1);
    assert_eq!(report.actions_by_actor.get("gemini"), Some(&1));
    assert_eq!(report.actions_by_type.get("plan"), Some(&1));
    assert!(report.last_action_timestamp.is_some());
}

#[test]
fn test_storage_project_crud_and_idempotency() {
    let db = KnoxDb::open_memory().unwrap();

    let p1 = Project {
        id: ProjectId::new(),
        name: "Knox".to_string(),
        path: r"\\?\D:\Knox".to_string(),
        created_at: Utc::now(),
    };
    insert_project(&db, &p1).unwrap();

    // Re-inserting same path with updated name should not fail (idempotent ON CONFLICT)
    let p1_updated = Project {
        id: ProjectId::new(),
        name: "Knox Refined".to_string(),
        path: r"D:\Knox".to_string(),
        created_at: Utc::now(),
    };
    insert_project(&db, &p1_updated).unwrap();

    let fetched = get_project_by_path(&db, r"D:\Knox").unwrap().unwrap();
    assert_eq!(fetched.name, "Knox Refined");
    assert_eq!(fetched.path, r"D:\Knox");

    let fetched_verbatim = get_project_by_path(&db, r"\\?\D:\Knox").unwrap().unwrap();
    assert_eq!(fetched_verbatim.path, r"D:\Knox");

    let list = list_projects(&db).unwrap();
    assert_eq!(list.len(), 1);
}
