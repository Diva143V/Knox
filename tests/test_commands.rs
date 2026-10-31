//! Integration tests for Knox CLI command business logic.

use std::fs;
use std::path::PathBuf;
use tempfile::tempdir;

use knox::commands::*;
use knox::core::config::KnoxConfig;
use knox::storage::db::KnoxDb;

#[test]
fn test_command_init_flow_and_idempotency() {
    let dir = tempdir().unwrap();
    let project_dir = dir.path().join("demo-project");
    fs::create_dir_all(&project_dir).unwrap();

    let db_path = dir.path().join("knox_global.db");
    let config = KnoxConfig {
        kg_path: db_path,
        session_timeout: std::time::Duration::from_secs(1800),
        global_mode: true,
    };

    // 1. First init creates knox.md
    cmd_init(&project_dir, &config).unwrap();
    let knox_md = project_dir.join("knox.md");
    assert!(knox_md.exists());

    let content_first = fs::read_to_string(&knox_md).unwrap();
    assert!(content_first.contains("<!-- knox:config"));
    assert!(content_first.contains("project_id:"));
    assert!(content_first.contains("MANDATORY PROTOCOL"));

    // 2. Second init is idempotent and preserves project_id
    cmd_init(&project_dir, &config).unwrap();
    let content_second = fs::read_to_string(&knox_md).unwrap();
    assert_eq!(content_first, content_second);
}

#[test]
fn test_command_ingest_and_status_and_query_flow() {
    let db = KnoxDb::open_memory().unwrap();
    let fixture_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("fixtures")
        .join("transcript_antigravity.jsonl");

    // 1. Ingest fixture
    let ingest_report = cmd_ingest(&fixture_path, &db).unwrap();
    assert_eq!(ingest_report.actions_imported, 6);
    assert_eq!(ingest_report.files_touched, 2);
    assert_eq!(ingest_report.sessions_created, 1);

    // 2. Status check
    let status_report = cmd_status(&db).unwrap();
    assert_eq!(status_report.total_actions, 6);
    assert_eq!(status_report.total_sessions, 1);
    assert_eq!(*status_report.actions_by_actor.get("human").unwrap(), 2);
    assert_eq!(
        *status_report.actions_by_actor.get("antigravity").unwrap(),
        4
    );
    assert_eq!(*status_report.actions_by_type.get("plan").unwrap(), 2);
    assert_eq!(*status_report.actions_by_type.get("write").unwrap(), 2);
    assert_eq!(*status_report.actions_by_type.get("debug").unwrap(), 2);

    let status_text = format_status_report(&status_report);
    assert!(status_text.contains("Total Actions:     6"));

    // 3. Query by file
    let query_file = cmd_query("src/parser.rs", &db, None).unwrap();
    assert_eq!(query_file.results.len(), 1);
    assert_eq!(query_file.results[0].files, vec!["src/parser.rs"]);

    // 4. Query by keyword
    let query_kw = cmd_query("parser", &db, None).unwrap();
    assert!(!query_kw.results.is_empty());

    // 5. Query by actor prefix
    let query_actor = cmd_query("actor:human", &db, None).unwrap();
    assert_eq!(query_actor.results.len(), 2);

    // 6. Query formatting
    let table = format_query_results_table(&query_file);
    assert!(table.contains("src/parser.rs"));

    let json_output = format_query_results_json(&query_file).unwrap();
    assert!(json_output.contains("\"total_count\": 1"));
}

#[test]
fn test_command_hook_management() {
    let dir = tempdir().unwrap();
    let git_dir = dir.path().join(".git");
    fs::create_dir_all(&git_dir).unwrap();

    cmd_hook_install(dir.path()).unwrap();
    let hook_path = git_dir.join("hooks").join("pre-commit");
    assert!(hook_path.exists());

    cmd_hook_uninstall(dir.path()).unwrap();
    assert!(!hook_path.exists());
}

fn setup_temp_git_repo(path: &std::path::Path) {
    use std::process::Command;
    let _ = Command::new("git")
        .args(["init"])
        .current_dir(path)
        .output();
    let _ = Command::new("git")
        .args(["config", "user.name", "Test"])
        .current_dir(path)
        .output();
    let _ = Command::new("git")
        .args(["config", "user.email", "test@example.com"])
        .current_dir(path)
        .output();
    fs::write(path.join("init.txt"), "hello").unwrap();
    let _ = Command::new("git")
        .args(["add", "init.txt"])
        .current_dir(path)
        .output();
    let _ = Command::new("git")
        .args(["commit", "-m", "init"])
        .current_dir(path)
        .output();
}

fn verify_human_edit_detected(path: &std::path::Path, db: &KnoxDb) {
    fs::write(path.join("human_edit.txt"), "modifications").unwrap();
    let _ = std::process::Command::new("git")
        .args(["add", "human_edit.txt"])
        .current_dir(path)
        .output();

    let report1 = cmd_scan(path, db).unwrap();
    assert_eq!(report1.human_edits_detected, 1);
    assert_eq!(report1.actions_recorded, 1);

    let query_human = cmd_query("human_edit.txt", db, None).unwrap();
    assert_eq!(query_human.results[0].actor, "human");
    assert_eq!(query_human.results[0].action_type, "human_write");

    let _ = std::process::Command::new("git")
        .args(["commit", "-m", "commit human edit"])
        .current_dir(path)
        .output();
}

fn verify_recent_ai_edit_skipped(path: &std::path::Path, db: &KnoxDb) {
    use chrono::Utc;
    use knox::core::ids::ActionId;
    use knox::core::types::KnoxAction;

    let ai_action = KnoxAction {
        id: ActionId::new(),
        actor: "gemini".to_string(),
        action_type: "write".to_string(),
        summary: "AI generated code".to_string(),
        files: vec!["ai_file.txt".to_string()],
        tags: vec![],
        confidence: None,
        is_current: true,
        timestamp: Utc::now(),
        session_id: None,
        parent_action_id: None,
        supersedes: None,
        ..Default::default()
    };
    knox::storage::nodes::insert_action(db, &ai_action).unwrap();

    fs::write(path.join("ai_file.txt"), "ai code").unwrap();
    let _ = std::process::Command::new("git")
        .args(["add", "ai_file.txt"])
        .current_dir(path)
        .output();

    let report2 = cmd_scan(path, db).unwrap();
    assert_eq!(report2.human_edits_detected, 0);
    assert_eq!(report2.actions_recorded, 0);
}

#[test]
fn test_command_scan_records_human_edit_and_skips_recent_ai() {
    let dir = tempdir().unwrap();
    setup_temp_git_repo(dir.path());
    let db = KnoxDb::open_memory().unwrap();

    verify_human_edit_detected(dir.path(), &db);
    verify_recent_ai_edit_skipped(dir.path(), &db);
}
