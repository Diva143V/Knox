//! Auto-Capture Integration Tests (Workstream G).
//!
//! Validates:
//! - G1: Ingesting transcript lines since the last offset on begin when a path is configured.
//!   Acceptance: A session with zero `record` calls still produces memory.
//! - G2: Extending git hooks to summarize the commit message into the record.
//!   Acceptance: Commit text appears in the record, scoped to the project.

use std::fs;
use std::path::Path;
use std::process::Command;
use tempfile::tempdir;

use serde_json::json;

use knox::commands::begin::cmd_begin;
use knox::commands::context::cmd_context;
use knox::commands::hook::{cmd_hook_install, cmd_hook_record_commit, cmd_hook_uninstall};
use knox::commands::init::cmd_init;
use knox::core::config::KnoxConfig;
use knox::storage::db::KnoxDb;
use knox::storage::queries::query_actions;

fn setup_git_repo(path: &Path) {
    let _ = Command::new("git").args(["init"]).current_dir(path).output();
    let _ = Command::new("git")
        .args(["config", "user.name", "Knox Tester"])
        .current_dir(path)
        .output();
    let _ = Command::new("git")
        .args(["config", "user.email", "tester@knox.local"])
        .current_dir(path)
        .output();
}

#[test]
fn test_g1_session_with_zero_records_produces_memory_from_transcript() {
    let temp = tempdir().unwrap();
    let project_dir = temp.path().join("my_service");
    fs::create_dir_all(&project_dir).unwrap();

    let db = KnoxDb::open_memory().unwrap();

    // Initialize project
    let config = KnoxConfig {
        kg_path: temp.path().join("global.db"),
        session_timeout: std::time::Duration::from_secs(1800),
        global_mode: true,
        transcript_path: None,
        ..Default::default()
    };
    cmd_init(&project_dir, &config).unwrap();

    // Create an Antigravity transcript file with user prompt and agent actions
    let transcript_path = project_dir.join("transcript.jsonl");
    let transcript_lines = [
        json!({
            "step_index": 0,
            "source": "USER_EXPLICIT",
            "type": "USER_INPUT",
            "content": "Implement zero-downtime database failover protocol"
        }).to_string(),
        json!({
            "step_index": 1,
            "source": "MODEL",
            "type": "PLANNER_RESPONSE",
            "thinking": "We need to design Raft consensus replication before switching storage backends.",
            "tool_calls": [{
                "name": "edit_file",
                "args": {
                    "Path": "src/failover.rs",
                    "Target": "old_fn()",
                    "Replacement": "new_fn()"
                }
            }]
        }).to_string(),
    ];
    fs::write(&transcript_path, transcript_lines.join("\n")).unwrap();

    // Call knox_begin WITH configured transcript_path
    let begin_res = cmd_begin(
        &db,
        json!({
            "workspace": project_dir.to_str().unwrap(),
            "actor": "antigravity",
            "task": "failover protocol",
            "transcript_path": transcript_path.to_str().unwrap()
        }),
    )
    .unwrap();

    let actions_captured = begin_res["scope"]["actions_captured"].as_u64().unwrap_or(0);
    assert!(
        actions_captured >= 1,
        "knox_begin must capture actions from the transcript"
    );

    // CRITICAL ACCEPTANCE CHECK: The agent executes ZERO calls to `knox_record`!
    // Verify that memory was still produced in SQLite
    let query_res = query_actions(
        &db,
        &knox::core::types::QueryFilter {
            project: Some(project_dir.to_str().unwrap().to_string()),
            ..Default::default()
        },
    )
    .unwrap();

    assert!(
        !query_res.results.is_empty(),
        "A session with zero `record` calls must still produce memory"
    );

    let failover_action = query_res.results.iter().find(|a| {
        a.summary.to_lowercase().contains("failover")
            || a.files.iter().any(|f| f.contains("failover.rs"))
    });
    assert!(
        failover_action.is_some(),
        "Captured action must reflect transcript contents"
    );

    // Verify context retrieval incorporates the captured memory
    let ctx = cmd_context(
        &db,
        json!({
            "project": project_dir.to_str().unwrap(),
            "view": "context"
        }),
    )
    .unwrap();

    let markdown = ctx["markdown"].as_str().unwrap_or_default();
    assert!(
        markdown.contains("failover") || markdown.contains("failover.rs"),
        "knox_context must return context captured automatically from transcript"
    );
}

#[test]
fn test_g1_transcript_incremental_offset_tracking() {
    let temp = tempdir().unwrap();
    let project_dir = temp.path().join("incremental_proj");
    fs::create_dir_all(&project_dir).unwrap();
    let db = KnoxDb::open_memory().unwrap();

    let transcript_path = project_dir.join("chat.jsonl");

    // Phase 1: 2 lines
    let lines_phase1 = [
        json!({
            "step_index": 0,
            "source": "USER_EXPLICIT",
            "type": "USER_INPUT",
            "content": "Add telemetry exporter"
        }).to_string(),
        json!({
            "step_index": 1,
            "source": "MODEL",
            "type": "PLANNER_RESPONSE",
            "tool_calls": [{
                "name": "write_file",
                "args": {
                    "Path": "src/telemetry.rs",
                    "Content": "pub fn export() {}"
                }
            }]
        }).to_string(),
    ];
    fs::write(&transcript_path, lines_phase1.join("\n") + "\n").unwrap();

    // Turn 1 begin
    let turn1 = cmd_begin(
        &db,
        json!({
            "workspace": project_dir.to_str().unwrap(),
            "actor": "antigravity",
            "transcript_path": transcript_path.to_str().unwrap()
        }),
    )
    .unwrap();
    let captured_turn1 = turn1["scope"]["actions_captured"].as_u64().unwrap();
    assert!(captured_turn1 >= 1);

    // Turn 2 begin without file changes -> 0 new actions
    let turn2 = cmd_begin(
        &db,
        json!({
            "workspace": project_dir.to_str().unwrap(),
            "actor": "antigravity",
            "transcript_path": transcript_path.to_str().unwrap()
        }),
    )
    .unwrap();
    let captured_turn2 = turn2["scope"]["actions_captured"].as_u64().unwrap();
    assert_eq!(
        captured_turn2, 0,
        "No actions should be re-ingested without transcript updates"
    );

    // Phase 2: Append 2 new lines
    let lines_phase2 = [
        json!({
            "step_index": 2,
            "source": "USER_EXPLICIT",
            "type": "USER_INPUT",
            "content": "Add Prometheus metrics endpoint"
        }).to_string(),
        json!({
            "step_index": 3,
            "source": "MODEL",
            "type": "PLANNER_RESPONSE",
            "tool_calls": [{
                "name": "write_file",
                "args": {
                    "Path": "src/metrics.rs",
                    "Content": "pub fn metrics() {}"
                }
            }]
        }).to_string(),
    ];
    let mut file = fs::OpenOptions::new()
        .append(true)
        .open(&transcript_path)
        .unwrap();
    use std::io::Write;
    file.write_all((lines_phase2.join("\n") + "\n").as_bytes())
        .unwrap();

    // Turn 3 begin: only the newly appended lines are captured
    let turn3 = cmd_begin(
        &db,
        json!({
            "workspace": project_dir.to_str().unwrap(),
            "actor": "antigravity",
            "transcript_path": transcript_path.to_str().unwrap()
        }),
    )
    .unwrap();
    let captured_turn3 = turn3["scope"]["actions_captured"].as_u64().unwrap();
    assert!(
        captured_turn3 >= 1,
        "Appended transcript lines must be incrementally captured"
    );

    // Total distinct actions in database
    let all_actions = query_actions(
        &db,
        &knox::core::types::QueryFilter {
            project: Some(project_dir.to_str().unwrap().to_string()),
            ..Default::default()
        },
    )
    .unwrap();

    assert!(
        all_actions.results.len() >= 2,
        "Total actions should equal sum of incremental phases without duplicate spam"
    );
}

#[test]
fn test_g2_hook_records_commit_message_scoped() {
    let temp = tempdir().unwrap();
    let repo_dir = temp.path().join("repo");
    fs::create_dir_all(&repo_dir).unwrap();
    setup_git_repo(&repo_dir);

    let db = KnoxDb::open_memory().unwrap();

    // Initialize knox in the repo
    let config = KnoxConfig {
        kg_path: temp.path().join("global.db"),
        session_timeout: std::time::Duration::from_secs(1800),
        global_mode: true,
        transcript_path: None,
        ..Default::default()
    };
    cmd_init(&repo_dir, &config).unwrap();

    // Create and commit a file in git
    let code_file = repo_dir.join("payment.rs");
    fs::write(&code_file, "pub fn process_payment() -> bool { true }").unwrap();

    let _ = Command::new("git")
        .args(["add", "payment.rs"])
        .current_dir(&repo_dir)
        .output();

    let commit_subject = "feat(billing): implement idempotent Stripe webhooks";
    let commit_body = "Ensures retried payment notifications do not double-charge customers.";
    let full_commit_msg = format!("{commit_subject}\n\n{commit_body}\n\n# Ignored git comment");

    let _ = Command::new("git")
        .args(["commit", "-m", &full_commit_msg])
        .current_dir(&repo_dir)
        .output();

    // Trigger the hook recording
    let action_id = cmd_hook_record_commit(&repo_dir, &db, None, None)
        .unwrap()
        .expect("must record a commit action");

    let action = knox::storage::nodes::get_action(&db, &action_id)
        .unwrap()
        .expect("recorded action must exist");

    // Acceptance: Commit text appears in the record, scoped
    assert_eq!(
        action.summary, commit_subject,
        "Commit subject must appear as the action summary"
    );
    assert!(
        action.rationale.as_ref().unwrap().contains(commit_body),
        "Commit body must appear in the action rationale"
    );
    assert!(
        !action.rationale.as_ref().unwrap().contains("# Ignored git comment"),
        "Git comments must be stripped from the record"
    );
    assert!(
        action.commit_sha.is_some(),
        "Action must record the git commit SHA"
    );
    assert!(
        action.files.iter().any(|f| f.contains("payment.rs")),
        "Committed files must be attached to the action"
    );
    assert_eq!(
        action.actor, "Knox Tester",
        "Author from git log must be captured as actor"
    );
    assert!(
        action.project_id.is_some(),
        "Record must be scoped to the project"
    );
}

#[test]
fn test_g2_hook_install_and_uninstall_lifecycle() {
    let temp = tempdir().unwrap();
    let repo_dir = temp.path().join("hook_lifecycle");
    fs::create_dir_all(&repo_dir).unwrap();
    setup_git_repo(&repo_dir);

    let hooks_dir = repo_dir.join(".git").join("hooks");

    // 1. Install hooks
    cmd_hook_install(&repo_dir).unwrap();
    assert!(hooks_dir.join("pre-commit").exists());
    assert!(hooks_dir.join("post-commit").exists());

    let pre_content = fs::read_to_string(hooks_dir.join("pre-commit")).unwrap();
    assert!(pre_content.contains("knox scan"));

    let post_content = fs::read_to_string(hooks_dir.join("post-commit")).unwrap();
    assert!(post_content.contains("knox hook record-commit"));

    // 2. Uninstall hooks
    cmd_hook_uninstall(&repo_dir).unwrap();
    assert!(!hooks_dir.join("pre-commit").exists());
    assert!(!hooks_dir.join("post-commit").exists());
}

#[test]
fn test_g2_hook_enriches_recent_staged_action_without_duplication() {
    let temp = tempdir().unwrap();
    let repo_dir = temp.path().join("enrich_repo");
    fs::create_dir_all(&repo_dir).unwrap();
    setup_git_repo(&repo_dir);

    let db = KnoxDb::open_memory().unwrap();

    let config = KnoxConfig {
        kg_path: temp.path().join("global.db"),
        session_timeout: std::time::Duration::from_secs(1800),
        global_mode: true,
        transcript_path: None,
        ..Default::default()
    };
    cmd_init(&repo_dir, &config).unwrap();

    // 1. Create and stage a file
    let file_path = repo_dir.join("worker.rs");
    fs::write(&file_path, "pub fn work() {}").unwrap();
    let _ = Command::new("git")
        .args(["add", "worker.rs"])
        .current_dir(&repo_dir)
        .output();

    // 2. Pre-commit triggers knox scan
    let scan_report = knox::commands::scan::cmd_scan(&repo_dir, &db).unwrap();
    assert_eq!(scan_report.human_edits_detected, 1);

    // Verify staged action exists
    let staged_actions = query_actions(
        &db,
        &knox::core::types::QueryFilter {
            project: Some(repo_dir.to_str().unwrap().to_string()),
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(staged_actions.results.len(), 1);
    let original_id = staged_actions.results[0].id.clone();
    assert!(staged_actions.results[0].tags.contains(&"staged".to_string()));

    // 3. User commits with git commit
    let _ = Command::new("git")
        .args(["commit", "-m", "refactor(worker): optimize worker event loop\n\nReduces CPU polling overhead."])
        .current_dir(&repo_dir)
        .output();

    // 4. Post-commit triggers knox hook record-commit
    let enriched_id = cmd_hook_record_commit(&repo_dir, &db, None, None)
        .unwrap()
        .expect("must record/enrich action");

    // Must enrich the SAME action ID rather than inserting a duplicate!
    assert_eq!(
        enriched_id, original_id,
        "Commit hook must enrich the recent staged action rather than creating a duplicate"
    );

    let updated_actions = query_actions(
        &db,
        &knox::core::types::QueryFilter {
            project: Some(repo_dir.to_str().unwrap().to_string()),
            ..Default::default()
        },
    )
    .unwrap();

    assert_eq!(
        updated_actions.results.len(), 1,
        "No duplicate action rows should exist after pre-commit scan and post-commit hook"
    );

    let action = &updated_actions.results[0];
    assert_eq!(action.summary, "refactor(worker): optimize worker event loop");
    assert!(action.rationale.as_ref().unwrap().contains("Reduces CPU polling overhead."));
    assert!(action.tags.contains(&"commit".to_string()));
    assert!(action.commit_sha.is_some());
}
