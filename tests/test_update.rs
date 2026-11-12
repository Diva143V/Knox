//! Integration tests for `knox_update` tool, `cmd_update`, and `cmd_begin` discovery fallback.

use std::fs;
use serde_json::json;

use knox::core::error::KnoxError;
use knox::storage::db::KnoxDb;
use knox::storage::nodes::insert_project;
use knox::core::types::Project;
use knox::core::ids::ProjectId;
use knox::commands::update::cmd_update;
use knox::commands::begin::cmd_begin;

fn sample_transcript_content() -> &'static str {
    r#"{"type":"USER_INPUT","content":"Please add authentication"}
{"type":"PLANNER_RESPONSE","thinking":"I should use JWT tokens for auth","content":"Plan: use JWT"}
{"type":"TOOL_CALL","tool_name":"write_file","parameters":{"path":"src/auth.rs"}}
"#
}

fn create_test_project(temp: &tempfile::TempDir) -> Project {
    Project {
        id: ProjectId::from("demo"),
        name: "demo".to_string(),
        path: temp.path().display().to_string(),
        remote: None,
        created_at: chrono::Utc::now(),
    }
}

#[test]
fn test_update_explicit_path_success_and_idempotency() {
    let db = KnoxDb::open_memory().unwrap();
    let temp = tempfile::tempdir().unwrap();

    let proj = create_test_project(&temp);
    insert_project(&db, &proj).unwrap();

    let transcript_path = temp.path().join("session.jsonl");
    fs::write(&transcript_path, sample_transcript_content()).unwrap();

    // 1. Initial ingestion
    let report1 = cmd_update(&db, Some(&transcript_path), Some("demo"), false).unwrap();
    assert!(report1.actions_imported > 0, "Should import actions from transcript");
    assert_eq!(report1.source, transcript_path.display().to_string());
    assert!(report1.offset > 0, "Line offset should be recorded");
    assert!(report1.markdown.contains("Actions Imported"));

    // 2. Repeat ingestion (idempotent: zero new actions)
    let report2 = cmd_update(&db, Some(&transcript_path), Some("demo"), false).unwrap();
    assert_eq!(report2.actions_imported, 0, "Second call must import 0 new actions");
    assert_eq!(report2.offset, report1.offset, "Offset should remain unchanged");
}

#[test]
fn test_update_missing_file_returns_empty_report() {
    let db = KnoxDb::open_memory().unwrap();
    let temp = tempfile::tempdir().unwrap();

    let proj = create_test_project(&temp);
    insert_project(&db, &proj).unwrap();

    let non_existent = temp.path().join("does_not_exist.jsonl");
    let report = cmd_update(&db, Some(&non_existent), Some("demo"), false).unwrap();
    assert_eq!(report.actions_imported, 0);
    assert!(report.markdown.contains("does not exist"));
}

#[test]
fn test_update_discovery_fallback_single() {
    let db = KnoxDb::open_memory().unwrap();
    let temp = tempfile::tempdir().unwrap();

    let proj = create_test_project(&temp);
    insert_project(&db, &proj).unwrap();

    let transcript_path = temp.path().join("claude_transcript.jsonl");
    fs::write(&transcript_path, sample_transcript_content()).unwrap();

    // Run without explicit path: should auto-discover the single transcript
    let report = cmd_update(&db, None, Some("demo"), true).unwrap();
    assert!(report.actions_imported > 0);
    assert_eq!(report.source, transcript_path.display().to_string());
}

#[test]
fn test_update_discovery_ambiguous_refusal() {
    let db = KnoxDb::open_memory().unwrap();
    let temp = tempfile::tempdir().unwrap();

    let proj = create_test_project(&temp);
    insert_project(&db, &proj).unwrap();

    // Create two competing transcript files
    fs::write(temp.path().join("transcript_a.jsonl"), sample_transcript_content()).unwrap();
    fs::write(temp.path().join("transcript_b.jsonl"), sample_transcript_content()).unwrap();

    let res = cmd_update(&db, None, Some("demo"), true);
    match res {
        Err(KnoxError::InvalidInput(msg)) => {
            assert!(msg.contains("Ambiguous transcripts found"), "Error was: {msg}");
        }
        other => panic!("Expected InvalidInput ambiguous refusal, got {other:?}"),
    }
}

#[test]
fn test_update_mcp_handler_execution() {
    let db = KnoxDb::open_memory().unwrap();
    let temp = tempfile::tempdir().unwrap();

    let proj = create_test_project(&temp);
    insert_project(&db, &proj).unwrap();

    let transcript_path = temp.path().join("session.jsonl");
    fs::write(&transcript_path, sample_transcript_content()).unwrap();

    let params = json!({
        "transcript_path": transcript_path.display().to_string(),
        "project": "demo"
    });

    let res = knox::mcp::handlers::execute_tool("knox_update", params, &db).unwrap();
    assert!(res.get("actions_imported").is_some());
    assert!(res.get("offset").is_some());
    assert!(res.get("markdown").is_some());
}

#[test]
fn test_begin_discovery_fallback_soft_fail() {
    let db = KnoxDb::open_memory().unwrap();
    let temp = tempfile::tempdir().unwrap();

    let proj = create_test_project(&temp);
    insert_project(&db, &proj).unwrap();

    // Case 1: Exactly 1 transcript -> auto-captured during knox_begin
    let transcript = temp.path().join("transcript.jsonl");
    fs::write(&transcript, sample_transcript_content()).unwrap();

    let begin_res = cmd_begin(&db, json!({
        "actor": "gemini",
        "workspace": "demo"
    })).unwrap();

    assert!(begin_res.get("scope").is_some());

    // Case 2: Ambiguous transcripts -> knox_begin fails soft (succeeds without error)
    fs::write(temp.path().join("transcript_extra.jsonl"), sample_transcript_content()).unwrap();
    let begin_res2 = cmd_begin(&db, json!({
        "actor": "claude",
        "workspace": "demo"
    })).unwrap();

    assert!(begin_res2.get("scope").is_some(), "knox_begin must not fail when discovery is ambiguous");
}
