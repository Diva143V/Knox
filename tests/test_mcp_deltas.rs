//! Integration tests for Causal Code Delta capture, secret scrubbing, and new MCP tools:
//! `knox_diff_inspect`, `knox_blame_causal`, `knox_verify_drift`.

use serde_json::json;

use knox::core::types::DeltaSummary;
use knox::mcp::handlers::dispatch;
use knox::storage::db::KnoxDb;

#[test]
fn test_record_action_with_deltas_and_secret_scrubbing() {
    let db = KnoxDb::open_memory().expect("open memory db");

    let secret_hunk = "@@ -1,5 +1,5 @@\n-let old = 1;\n+let api_key = \"ghp_123456789012345678901234567890123456\";";
    let secret_deleted = "let token = \"Bearer secret_token_xyz_1234567890123456\";";

    let res = dispatch(
        "knox_record_action",
        json!({
            "actor": "antigravity",
            "action_type": "write",
            "summary": "Added authenticated client",
            "files": ["src/client.rs"],
            "commit_sha": "a1b2c3d4e5f67890",
            "deletion_reason": "Replaced legacy hardcoded auth",
            "auto_git_diff": false,
            "deltas": [{
                "file_path": "src/client.rs",
                "change_type": "modified",
                "lines_added": 1,
                "lines_deleted": 1,
                "diff_hunk": secret_hunk,
                "deleted_code": secret_deleted,
            }]
        }),
        &db,
    );

    assert!(res.error.is_none(), "record action should succeed: {:?}", res.error);
    let result = res.result.unwrap();
    assert_eq!(result["status"], "recorded");
    assert_eq!(result["commit_sha"], "a1b2c3d4e5f67890");

    let summary: DeltaSummary = serde_json::from_value(result["delta_summary"].clone())
        .expect("delta summary should deserialize");
    assert_eq!(summary.total_files, 1);
    assert_eq!(summary.lines_added, 1);
    assert_eq!(summary.lines_deleted, 1);

    let action_id = result["id"].as_str().unwrap();

    // Verify through knox_diff_inspect that secrets are redacted
    let inspect_res = dispatch(
        "knox_diff_inspect",
        json!({
            "action_id": action_id
        }),
        &db,
    );

    assert!(inspect_res.error.is_none());
    let inspect_data = inspect_res.result.unwrap();
    let deltas = inspect_data["deltas"].as_array().expect("deltas array");
    assert_eq!(deltas.len(), 1);

    let delta = &deltas[0];
    assert_eq!(delta["file_path"], "src/client.rs");
    assert_eq!(delta["change_type"], "modified");
    assert_eq!(delta["deletion_reason"], "Replaced legacy hardcoded auth");

    let stored_hunk = delta["diff_hunk"].as_str().unwrap();
    assert!(!stored_hunk.contains("ghp_123456789012345678901234567890123456"));
    assert!(stored_hunk.contains("[REDACTED_SECRET]"));

    let stored_deleted = delta["deleted_code"].as_str().unwrap();
    assert!(!stored_deleted.contains("secret_token_xyz_1234567890123456"));
    assert!(stored_deleted.contains("[REDACTED_SECRET]"));
}

#[test]
fn test_delta_bounds_truncation() {
    let db = KnoxDb::open_memory().expect("open memory db");

    // Generate hunk with 300 lines (exceeds MAX_DIFF_LINES = 250)
    let mut big_hunk = String::from("@@ -1,1 +1,300 @@\n");
    for i in 0..300 {
        big_hunk.push_str(&format!("+line {i} added with padding data to fill lines\n"));
    }

    let res = dispatch(
        "knox_record_action",
        json!({
            "actor": "cursor",
            "action_type": "write",
            "summary": "Massive generated file",
            "files": ["src/generated.rs"],
            "auto_git_diff": false,
            "deltas": [{
                "file_path": "src/generated.rs",
                "change_type": "added",
                "lines_added": 300,
                "lines_deleted": 0,
                "diff_hunk": big_hunk
            }]
        }),
        &db,
    );

    assert!(res.error.is_none());
    let action_id = res.result.unwrap()["id"].as_str().unwrap().to_string();

    let inspect_res = dispatch("knox_diff_inspect", json!({"action_id": action_id}), &db);
    let inspect_data = inspect_res.result.unwrap();
    let hunk = inspect_data["deltas"][0]["diff_hunk"].as_str().unwrap();
    assert!(hunk.contains("[diff truncated: exceeded line limit]"));
}

#[test]
fn test_blame_causal_by_commit_and_line_overlap() {
    let db = KnoxDb::open_memory().expect("open memory db");

    // 1. Record an architectural decision and write action
    let act1 = dispatch(
        "knox_record_action",
        json!({
            "actor": "claude",
            "action_type": "write",
            "summary": "Implemented binary serialization router",
            "files": ["src/router.rs"],
            "commit_sha": "c0ffee1234567890abcdef",
            "rationale": "Switched from JSON to bincode for latency reduction",
            "context_summary": "Optimizing IPC channel throughput",
            "auto_git_diff": false,
            "deltas": [{
                "file_path": "src/router.rs",
                "change_type": "modified",
                "lines_added": 15,
                "lines_deleted": 3,
                "diff_hunk": "@@ -20,3 +20,15 @@\n+pub fn route_binary() {}\n"
            }]
        }),
        &db,
    );
    assert!(act1.error.is_none());
    let action_id = act1.result.unwrap()["id"].as_str().unwrap().to_string();

    // 2. Query blame causal by exact commit SHA
    let blame1 = dispatch(
        "knox_blame_causal",
        json!({
            "file_path": "src/router.rs",
            "line_number": 25,
            "commit_sha": "c0ffee1234567890"
        }),
        &db,
    );
    assert!(blame1.error.is_none());
    let blame1_data = blame1.result.unwrap();
    assert_eq!(blame1_data["action"]["id"], action_id);
    assert_eq!(blame1_data["action"]["actor"], "claude");
    assert_eq!(
        blame1_data["action"]["rationale"],
        "Switched from JSON to bincode for latency reduction"
    );
    assert!(!blame1_data["delta"].is_null());
    assert_eq!(blame1_data["delta"]["file_path"], "src/router.rs");

    // 3. Query blame causal by line number overlap (without commit SHA)
    let blame2 = dispatch(
        "knox_blame_causal",
        json!({
            "file_path": "src/router.rs",
            "line_number": 28
        }),
        &db,
    );
    assert!(blame2.error.is_none());
    let blame2_data = blame2.result.unwrap();
    assert_eq!(blame2_data["action"]["id"], action_id);
    assert!(!blame2_data["delta"].is_null());

    // 4. Query blame causal fallback to file match
    let blame3 = dispatch(
        "knox_blame_causal",
        json!({
            "file_path": "src/router.rs",
            "line_number": 9999
        }),
        &db,
    );
    assert!(blame3.error.is_none());
    let blame3_data = blame3.result.unwrap();
    assert_eq!(blame3_data["action"]["id"], action_id);
}

#[test]
fn test_verify_drift_clean_workspace() {
    let db = KnoxDb::open_memory().expect("open memory db");

    let res = dispatch("knox_verify_drift", json!({}), &db);
    assert!(res.error.is_none());
    let data = res.result.unwrap();
    assert!(data.get("is_clean").is_some());
    assert!(data.get("drift_summary").is_some());
}

#[test]
fn test_diff_inspect_filter_by_file() {
    let db = KnoxDb::open_memory().expect("open memory db");

    let act = dispatch(
        "knox_record_action",
        json!({
            "actor": "antigravity",
            "action_type": "write",
            "summary": "Multi-file patch",
            "files": ["src/a.rs", "src/b.rs"],
            "auto_git_diff": false,
            "deltas": [
                {
                    "file_path": "src/a.rs",
                    "change_type": "modified",
                    "lines_added": 5,
                    "lines_deleted": 0,
                    "diff_hunk": "@@ -1,1 +1,6 @@\n+a"
                },
                {
                    "file_path": "src/b.rs",
                    "change_type": "added",
                    "lines_added": 10,
                    "lines_deleted": 0,
                    "diff_hunk": "@@ -0,0 +1,10 @@\n+b"
                }
            ]
        }),
        &db,
    );

    let action_id = act.result.unwrap()["id"].as_str().unwrap().to_string();

    // Inspect all deltas
    let all = dispatch("knox_diff_inspect", json!({"action_id": action_id}), &db);
    assert_eq!(all.result.unwrap()["deltas"].as_array().unwrap().len(), 2);

    // Inspect filtered by file
    let filtered = dispatch(
        "knox_diff_inspect",
        json!({
            "action_id": action_id,
            "file_path": "src/b.rs"
        }),
        &db,
    );
    let filtered_deltas = filtered.result.unwrap()["deltas"].as_array().unwrap().clone();
    assert_eq!(filtered_deltas.len(), 1);
    assert_eq!(filtered_deltas[0]["file_path"], "src/b.rs");
}
