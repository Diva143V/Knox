//! Comprehensive integration tests for transcript ingestion in Knox.

use std::path::PathBuf;

use knox::core::types::{KnoxAction, QueryFilter};
use knox::ingest::*;
use knox::storage::*;

fn verify_group1(actions: &[KnoxAction]) {
    let plan1 = &actions[0];
    assert_eq!(plan1.action_type, "plan");
    assert_eq!(
        plan1.summary,
        "Refactor parser module to support streaming tokens"
    );
    let plan1_id = plan1.id.clone();

    let write1 = &actions[1];
    assert_eq!(write1.action_type, "write");
    assert_eq!(write1.parent_action_id, Some(plan1_id.clone()));
    assert!(write1.summary.contains("src/parser.rs (+2 -1)"));

    let debug1 = &actions[2];
    assert_eq!(debug1.action_type, "debug");
    assert_eq!(debug1.parent_action_id, Some(plan1_id));
}

fn verify_group2(actions: &[KnoxAction]) {
    let plan2 = &actions[3];
    assert_eq!(plan2.action_type, "plan");
    let plan2_id = plan2.id.clone();

    let write2 = &actions[4];
    assert_eq!(write2.action_type, "write");
    assert_eq!(write2.parent_action_id, Some(plan2_id.clone()));
    assert!(write2.summary.contains("docs/lexer.md (new)"));

    let debug2 = &actions[5];
    assert_eq!(debug2.action_type, "debug");
    assert_eq!(debug2.parent_action_id, Some(plan2_id));
}

#[test]
fn test_ingest_fixture_parsing_and_structure() {
    let fixture_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("fixtures")
        .join("transcript_antigravity.jsonl");

    let actions = ingest_file(&fixture_path).expect("fixture should be ingested cleanly");
    assert_eq!(
        actions.len(),
        6,
        "Expected 6 actions from fixture transcript"
    );

    verify_group1(&actions);
    verify_group2(&actions);
}

#[test]
fn test_ingest_pure_reasoning_yields_decide_action() {
    let raw = br#"
{"step_index":0,"source":"USER_EXPLICIT","type":"USER_INPUT","content":"Should we use SQLite or RocksDB?"}
{"step_index":1,"source":"MODEL","type":"PLANNER_RESPONSE","thinking":"Thinking about DB choice","content":"SQLite is selected for zero-config embedding and bundled distribution."}
"#;

    let mapper = AntigravityMapper;
    assert!(mapper.can_parse(raw));

    let actions = mapper.map(raw).expect("mapping should succeed");
    assert_eq!(actions.len(), 2);

    assert_eq!(actions[0].actor, "human");
    assert_eq!(actions[0].action_type, "doubt");
    assert_eq!(actions[0].is_resolved, Some(false));
    assert_eq!(actions[0].summary, "Should we use SQLite or RocksDB?");

    assert_eq!(actions[1].actor, "antigravity");
    assert_eq!(actions[1].action_type, "decide");
    assert_eq!(
        actions[1].summary,
        "SQLite is selected for zero-config embedding and bundled distribution."
    );
    assert_eq!(actions[1].parent_action_id, Some(actions[0].id.clone()));
}

#[test]
fn test_ingest_can_parse_validation() {
    let mapper = AntigravityMapper;

    // Valid Antigravity transcript line
    assert!(mapper.can_parse(br#"{"step_index":1,"type":"USER_INPUT"}"#));

    // Invalid JSON
    assert!(!mapper.can_parse(b"not json at all"));

    // JSON missing required Antigravity keys
    assert!(!mapper.can_parse(br#"{"id":"123","message":"hello"}"#));

    // Empty input
    assert!(!mapper.can_parse(b""));
}

#[test]
fn test_ingest_actions_persist_to_storage() {
    let fixture_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("fixtures")
        .join("transcript_antigravity.jsonl");

    let actions = ingest_file(&fixture_path).expect("fixture should be ingested");

    let db = KnoxDb::open_memory().expect("in-memory db open");

    for action in &actions {
        let inserted = insert_action(&db, action).expect("insert action");
        assert_eq!(inserted, action.id);
    }

    // Query for parser file modifications
    let filter = QueryFilter {
        file_path: Some("src/parser.rs".to_string()),
        project: None,
        session_id: None,
        actor: None,
        since: None,
        current_only: true,
        tags: vec![],
        limit: 10,
        ..Default::default()
    };

    let result = query_actions(&db, &filter).expect("query actions");
    assert_eq!(result.results.len(), 1);
    assert_eq!(result.results[0].files, vec!["src/parser.rs"]);
    assert_eq!(result.results[0].action_type, "write");

    // Full-text search for clippy
    let search_filter = QueryFilter {
        file_path: None,
        project: None,
        session_id: None,
        actor: None,
        since: None,
        current_only: true,
        tags: vec![],
        limit: 10,
        ..Default::default()
    };
    let all = query_actions(&db, &search_filter).expect("query all");
    assert_eq!(all.total_count, 6);
}
