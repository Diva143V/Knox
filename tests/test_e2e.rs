//! End-to-end integration tests for Knox.
//! Validates the complete user and agent workflow:
//! init -> ingest -> query -> status -> MCP interaction -> scan.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;
use tempfile::tempdir;

use knox::commands::*;
use knox::core::config::KnoxConfig;
use knox::mcp::handlers::dispatch;
use knox::storage::db::KnoxDb;
use serde_json::json;

fn fixture_transcript_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("fixtures")
        .join("transcript_antigravity.jsonl")
}

fn init_test_git_repo(path: &Path) {
    let _ = Command::new("git")
        .args(["init"])
        .current_dir(path)
        .output();
    let _ = Command::new("git")
        .args(["config", "user.name", "KnoxTester"])
        .current_dir(path)
        .output();
    let _ = Command::new("git")
        .args(["config", "user.email", "knox@test.com"])
        .current_dir(path)
        .output();
    fs::write(path.join("README.md"), "# E2E Test").unwrap();
    let _ = Command::new("git")
        .args(["add", "README.md"])
        .current_dir(path)
        .output();
    let _ = Command::new("git")
        .args(["commit", "-m", "initial"])
        .current_dir(path)
        .output();
}

#[test]
fn test_e2e_init_ingest_query_status_walkthrough() {
    let dir = tempdir().unwrap();
    let project_dir = dir.path().join("my-project");
    fs::create_dir_all(&project_dir).unwrap();

    let db_path = dir.path().join("knox_e2e.db");
    let config = KnoxConfig {
        kg_path: db_path.clone(),
        session_timeout: Duration::from_secs(1800),
        global_mode: true,
    };

    // 1. Initialize project
    cmd_init(&project_dir, &config).unwrap();
    let knox_md = project_dir.join("knox.md");
    assert!(knox_md.exists());
    let md_text = fs::read_to_string(&knox_md).unwrap();
    assert!(md_text.contains("<!-- knox:config"));
    assert!(md_text.contains("MANDATORY PROTOCOL"));

    // 2. Open DB and ingest transcript
    let db = KnoxDb::open(&db_path).unwrap();
    let fixture = fixture_transcript_path();
    let ingest_report = cmd_ingest(&fixture, &db).unwrap();
    assert_eq!(ingest_report.actions_imported, 6);
    assert_eq!(ingest_report.sessions_created, 1);

    // 3. Query knowledge graph
    let query_res = cmd_query("parser", &db, None).unwrap();
    assert!(!query_res.results.is_empty());
    assert_eq!(query_res.results[0].files, vec!["src/parser.rs"]);

    // 4. Status inspection
    let status_report = cmd_status(&db).unwrap();
    assert_eq!(status_report.total_actions, 6);
    assert_eq!(status_report.total_sessions, 1);
    assert_eq!(status_report.projects_tracked, 1);
}

fn record_e2e_mcp_action(db: &KnoxDb) {
    let start_res = dispatch("knox_start_session", json!({"actor": "e2e-agent"}), db);
    let session_id = start_res.result.unwrap()["session_id"]
        .as_str()
        .unwrap()
        .to_string();

    let rec_res = dispatch(
        "knox_record_action",
        json!({
            "session_id": session_id,
            "actor": "e2e-agent",
            "action_type": "write",
            "summary": "Added AST visitor pattern",
            "files": ["src/ast.rs"]
        }),
        db,
    );
    assert_eq!(rec_res.result.unwrap()["status"], "recorded");
}

fn simulate_human_edit_and_scan(path: &std::path::Path, db: &KnoxDb) {
    fs::write(path.join("notes.txt"), "Human developer notes").unwrap();
    let _ = Command::new("git")
        .args(["add", "notes.txt"])
        .current_dir(path)
        .output();

    let scan_report = cmd_scan(path, db).unwrap();
    assert_eq!(scan_report.human_edits_detected, 1);
    assert_eq!(scan_report.actions_recorded, 1);
}

#[test]
fn test_e2e_mcp_context_and_human_scan_flow() {
    let dir = tempdir().unwrap();
    init_test_git_repo(dir.path());

    let db_path = dir.path().join("knox_mcp_scan.db");
    let db = KnoxDb::open(&db_path).unwrap();

    cmd_ingest(&fixture_transcript_path(), &db).unwrap();

    let ctx_res = dispatch(
        "knox_get_context",
        json!({"file_path": "src/parser.rs"}),
        &db,
    );
    assert!(
        !ctx_res.result.unwrap()["results"]
            .as_array()
            .unwrap()
            .is_empty()
    );

    record_e2e_mcp_action(&db);
    simulate_human_edit_and_scan(dir.path(), &db);

    let status = cmd_status(&db).unwrap();
    assert_eq!(status.total_actions, 8);
}
