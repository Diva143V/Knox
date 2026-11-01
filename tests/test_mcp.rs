//! Integration tests for the Knox MCP protocol and tool execution layer.

use serde_json::json;

use knox::core::KnoxResult;
use knox::core::ids::ActionId;
use knox::mcp::handlers::dispatch;
use knox::storage::db::KnoxDb;

#[test]
fn test_mcp_session_lifecycle() {
    let db = KnoxDb::open_memory().expect("open memory db");

    let start_res = dispatch(
        "knox_start_session",
        json!({"actor": "gemini", "project": "knox"}),
        &db,
    );
    assert!(start_res.error.is_none());
    let session_id = start_res
        .result
        .unwrap()
        .get("session_id")
        .unwrap()
        .as_str()
        .unwrap()
        .to_string();

    let act_res = dispatch(
        "knox_record_action",
        json!({
            "session_id": session_id,
            "actor": "gemini",
            "action_type": "write",
            "summary": "Implemented parser",
            "files": ["src/parser.rs"]
        }),
        &db,
    );
    assert_eq!(act_res.result.unwrap().get("status").unwrap(), "recorded");

    let end_res = dispatch("knox_end_session", json!({"session_id": session_id}), &db);
    let end_data = end_res.result.unwrap();
    assert_eq!(end_data.get("status").unwrap(), "closed");
    assert_eq!(end_data.get("actions_recorded").unwrap(), 1);
}

fn mcp_start(db: &KnoxDb, actor: &str) -> String {
    let start_res = dispatch("knox_start_session", json!({"actor": actor}), db);
    start_res.result.unwrap()["session_id"]
        .as_str()
        .unwrap()
        .to_string()
}

fn mcp_write(db: &KnoxDb, sid: &str, actor: &str, summary: &str, file: &str) -> String {
    let act_res = dispatch(
        "knox_record_action",
        json!({
            "session_id": sid, "actor": actor, "action_type": "write",
            "summary": summary, "files": [file]
        }),
        db,
    );
    act_res.result.unwrap()["id"].as_str().unwrap().to_string()
}

#[test]
fn test_mcp_decision_recording_and_parent_link() {
    let db = KnoxDb::open_memory().expect("open memory db");
    let session_id = mcp_start(&db, "gemini");
    let parent_id = mcp_write(&db, &session_id, "gemini", "Initial draft", "src/lib.rs");

    let dec_res = dispatch(
        "knox_record_decision",
        json!({
            "session_id": session_id,
            "actor": "gemini",
            "summary": "Decided petgraph architecture",
            "parent_action_id": parent_id
        }),
        &db,
    );
    let dec_id = dec_res.result.unwrap()["id"].as_str().unwrap().to_string();

    let dec_node = knox::storage::nodes::get_action(&db, &ActionId::from(dec_id))
        .unwrap()
        .unwrap();
    assert_eq!(dec_node.action_type, "decide");
}

#[test]
fn test_mcp_context_query_filters_superseded() {
    let db = KnoxDb::open_memory().expect("open memory db");
    let session_id = mcp_start(&db, "claude");
    let act1_id = mcp_write(&db, &session_id, "claude", "Old lexer", "src/lexer.rs");

    dispatch(
        "knox_record_decision",
        json!({
            "session_id": session_id, "actor": "claude",
            "summary": "New zero-copy lexer", "files": ["src/lexer.rs"], "supersedes": act1_id
        }),
        &db,
    );

    let ctx_res = dispatch(
        "knox_get_context",
        json!({"file_path": "src/lexer.rs"}),
        &db,
    );
    let ctx_results = ctx_res.result.unwrap()["results"]
        .as_array()
        .unwrap()
        .clone();
    assert_eq!(ctx_results.len(), 1);
    assert_eq!(ctx_results[0]["summary"], "New zero-copy lexer");
}

#[test]
fn test_mcp_history_and_stale_listing() {
    let db = KnoxDb::open_memory().expect("open memory db");
    let session_id = mcp_start(&db, "claude");
    let act1_id = mcp_write(&db, &session_id, "claude", "Old lexer", "src/lexer.rs");

    dispatch(
        "knox_record_decision",
        json!({
            "session_id": session_id, "actor": "claude",
            "summary": "New lexer", "files": ["src/lexer.rs"], "supersedes": act1_id
        }),
        &db,
    );

    let hist_res = dispatch(
        "knox_get_history",
        json!({"file_path": "src/lexer.rs"}),
        &db,
    );
    assert_eq!(
        hist_res.result.unwrap()["results"]
            .as_array()
            .unwrap()
            .len(),
        2
    );

    let stale_res = dispatch("knox_list_stale", json!({"file_path": "src/lexer.rs"}), &db);
    let stale = stale_res.result.unwrap()["results"]
        .as_array()
        .unwrap()
        .clone();
    assert_eq!(stale.len(), 1);
    assert_eq!(stale[0]["summary"], "Old lexer");
}

#[test]
fn test_mcp_search_and_tools_call_standard() {
    let db = KnoxDb::open_memory().expect("open memory db");

    let init_res = dispatch("initialize", json!({}), &db);
    assert_eq!(init_res.result.unwrap()["serverInfo"]["name"], "knox");

    let list_res = dispatch("tools/list", json!({}), &db);
    assert_eq!(
        list_res.result.unwrap()["tools"].as_array().unwrap().len(),
        16
    );

    let session_id = mcp_start(&db, "gemini");
    mcp_write(
        &db,
        &session_id,
        "gemini",
        "Optimized inverted index indexing strategy",
        "src/index.rs",
    );

    let search_res = dispatch("knox_search", json!({ "query": "inverted index" }), &db);
    let search_results = search_res.result.unwrap()["results"]
        .as_array()
        .unwrap()
        .clone();
    assert_eq!(search_results.len(), 1);
    assert_eq!(
        search_results[0]["summary"],
        "Optimized inverted index indexing strategy"
    );
}

#[test]
fn test_mcp_doubt_lifecycle_and_resolution() {
    let db = KnoxDb::open_memory().expect("open memory db");
    let session_id = mcp_start(&db, "gemini");

    // 1. Record doubt via dispatch
    let doubt_res = dispatch(
        "knox_record_doubt",
        json!({
            "session_id": session_id,
            "actor": "gemini",
            "summary": "Can we support 50k nodes in petgraph without high heap overhead?",
            "context_summary": "Graph scalability investigation"
        }),
        &db,
    );
    assert!(doubt_res.error.is_none(), "record_doubt failed: {:?}", doubt_res.error);
    let doubt_id = doubt_res.result.unwrap()["id"].as_str().unwrap().to_string();

    let action_id = ActionId::from(doubt_id.clone());
    let doubt_action = knox::storage::nodes::get_action(&db, &action_id)
        .unwrap()
        .expect("must exist");
    assert_eq!(doubt_action.action_type, "doubt");
    assert_eq!(doubt_action.is_resolved, Some(false));
    assert_eq!(doubt_action.significance.as_deref(), Some("minor"));

    // 2. Record an answering decision
    let decide_res = dispatch(
        "knox_record_decision",
        json!({
            "session_id": session_id,
            "actor": "gemini",
            "summary": "Adopt lazy bounded BFS subgraph extraction rather than loading whole KG into petgraph",
            "significance": "major"
        }),
        &db,
    );
    assert!(decide_res.error.is_none());
    let decide_id = decide_res.result.unwrap()["id"].as_str().unwrap().to_string();

    // 3. Resolve the doubt linking to the decision
    let resolve_res = dispatch(
        "knox_resolve_doubt",
        json!({
            "doubt_id": doubt_id,
            "resolution_note": "Verified 2-hop bounded BFS uses <15MB heap regardless of overall 50k node DB size.",
            "resolved_by_action_id": decide_id
        }),
        &db,
    );
    assert!(resolve_res.error.is_none(), "resolve_doubt failed: {:?}", resolve_res.error);

    let updated_doubt = knox::storage::nodes::get_action(&db, &action_id)
        .unwrap()
        .expect("must exist");
    assert_eq!(updated_doubt.is_resolved, Some(true));
    assert!(updated_doubt.resolution_note.is_some());
}

async fn send_mcp_call(
    write: &mut tokio::io::DuplexStream,
    lines: &mut tokio::io::Lines<tokio::io::BufReader<tokio::io::DuplexStream>>,
    req: serde_json::Value,
) -> serde_json::Value {
    use tokio::io::AsyncWriteExt;
    write
        .write_all(format!("{req}\n").as_bytes())
        .await
        .unwrap();
    write.flush().await.unwrap();
    let line = lines.next_line().await.unwrap().unwrap();
    serde_json::from_str(&line).unwrap()
}

fn spawn_mock_server(
    db: KnoxDb,
    read: tokio::io::DuplexStream,
    write: tokio::io::DuplexStream,
) -> tokio::task::JoinHandle<KnoxResult<()>> {
    use std::time::Duration;
    tokio::spawn(async move {
        let reader = tokio::io::BufReader::new(read);
        knox::mcp::run_mcp_server_loop(
            db,
            reader,
            write,
            Duration::from_millis(50),
            Duration::from_secs(1800),
        )
        .await
    })
}

async fn run_mock_client_exchange(
    write: &mut tokio::io::DuplexStream,
    lines: &mut tokio::io::Lines<tokio::io::BufReader<tokio::io::DuplexStream>>,
) {
    let init_res = send_mcp_call(
        write,
        lines,
        json!({"jsonrpc": "2.0", "id": 100, "method": "initialize", "params": {}}),
    )
    .await;
    assert_eq!(init_res.get("id").unwrap(), 100);

    let start_res = send_mcp_call(write, lines, json!({"jsonrpc": "2.0", "id": 101, "method": "tools/call", "params": {"name": "knox_start_session", "arguments": {"actor": "gemini-mcp"}}})).await;
    let session_id = start_res["result"]["structuredContent"]["session_id"]
        .as_str()
        .unwrap()
        .to_string();

    let rec_res = send_mcp_call(write, lines, json!({"jsonrpc": "2.0", "id": 102, "method": "tools/call", "params": {"name": "knox_record_action", "arguments": {"session_id": session_id, "actor": "gemini-mcp", "action_type": "write", "summary": "Built MCP loop", "files": ["src/mcp/server.rs"]}}})).await;
    assert_eq!(rec_res.get("id").unwrap(), 102);

    let end_res = send_mcp_call(write, lines, json!({"jsonrpc": "2.0", "id": 103, "method": "tools/call", "params": {"name": "knox_end_session", "arguments": {"session_id": session_id}}})).await;
    assert_eq!(end_res["result"]["structuredContent"]["status"], "closed");
}

#[tokio::test]
async fn test_mcp_server_loop_full_session_flow() {
    use tokio::io::{AsyncBufReadExt, duplex};

    let db = KnoxDb::open_memory().unwrap();
    let (client_read, server_write) = duplex(8192);
    let (server_read, mut client_write) = duplex(8192);
    let server_task = spawn_mock_server(db, server_read, server_write);
    let mut client_lines = tokio::io::BufReader::new(client_read).lines();

    run_mock_client_exchange(&mut client_write, &mut client_lines).await;

    drop(client_write);
    assert!(server_task.await.unwrap().is_ok());
}

#[test]
fn test_mcp_implicit_session_and_v2_fields() {
    let db = KnoxDb::open_memory().unwrap();

    // 1. Record without session_id (implicit auto-spawn)
    let res = dispatch(
        "knox_record_action",
        json!({
            "actor": "claude-code",
            "action_type": "decision",
            "summary": "Adopted SQLite WAL mode for concurrency",
            "files": ["src/storage/db.rs"],
            "symbols": ["KnoxDb::open", "Config::wal_mode"],
            "rationale": "High-throughput concurrent read-heavy workload",
            "rejected_alternatives": ["RocksDB", "DuckDB"],
            "context_summary": "SQLite configured with WAL mode and synchronous=NORMAL"
        }),
        &db,
    );

    assert!(res.error.is_none(), "record error: {:?}", res.error);
    let val = res.result.unwrap();
    assert_eq!(val["status"], "recorded");
    let action_id = val["action_id"].as_str().unwrap().to_string();
    let session_id = val["session_id"].as_str().unwrap().to_string();
    assert!(!action_id.is_empty());
    assert!(!session_id.is_empty());

    // 2. Second action should reuse the open session
    let res2 = dispatch(
        "knox_record_action",
        json!({
            "actor": "claude-code",
            "action_type": "write",
            "summary": "Implemented WAL checkpointing logic",
            "files": ["src/storage/db.rs"],
            "symbols": ["KnoxDb::checkpoint"],
            "link_to_latest": true,
            "relations": [
                {
                    "target_id": action_id,
                    "relation_type": "implements"
                }
            ]
        }),
        &db,
    );

    let val2 = res2.result.unwrap();
    assert_eq!(val2["session_id"], session_id);

    // 3. knox_get_context with symbol filter and markdown format
    let ctx_res = dispatch(
        "knox_get_context",
        json!({
            "symbols": ["KnoxDb::checkpoint"],
            "format": "markdown"
        }),
        &db,
    );
    let ctx_val = ctx_res.result.unwrap();
    let text = ctx_val["markdown"].as_str().unwrap();
    assert!(text.contains("Implemented WAL checkpointing logic"));
    assert!(text.contains("KnoxDb::checkpoint"));
}

#[test]
fn test_mcp_record_batch_and_graph_tools() {
    let db = KnoxDb::open_memory().unwrap();

    // 1. Record batch with link_to_previous chain
    let batch_res = dispatch(
        "knox_record_batch",
        json!({
            "actor": "codex",
            "link_to_previous": true,
            "actions": [
                {
                    "action_type": "decision",
                    "summary": "Step 1: Protocol design",
                    "files": ["src/proto.rs"],
                    "symbols": ["Proto::parse"]
                },
                {
                    "action_type": "write",
                    "summary": "Step 2: Code generation",
                    "files": ["src/gen.rs"],
                    "symbols": ["Generator::run"]
                },
                {
                    "action_type": "verify",
                    "summary": "Step 3: Protocol unit tests",
                    "files": ["tests/test_proto.rs"],
                    "symbols": ["test_wire_roundtrip"]
                }
            ]
        }),
        &db,
    );

    assert!(batch_res.error.is_none(), "batch error: {:?}", batch_res.error);
    let batch_val = batch_res.result.unwrap();
    assert_eq!(batch_val["recorded_count"], 3);
    let action_ids = batch_val["action_ids"].as_array().unwrap();
    assert_eq!(action_ids.len(), 3);

    let id1 = action_ids[0].as_str().unwrap();
    let _id2 = action_ids[1].as_str().unwrap();
    let id3 = action_ids[2].as_str().unwrap();

    // 2. Test knox_get_lineage from step 3 (should trace backward to step 2 and step 1)
    let lineage_res = dispatch(
        "knox_get_lineage",
        json!({
            "action_id": id3,
            "max_depth": 5,
            "format": "compact"
        }),
        &db,
    );
    assert!(lineage_res.error.is_none(), "lineage error: {:?}", lineage_res.error);
    let lineage_val = lineage_res.result.unwrap();
    let chain = lineage_val["lineage"].as_array().unwrap();
    assert!(chain.len() >= 2, "chain should contain ancestors, got: {chain:?}");

    // 3. Test knox_get_impact from step 1 (should find descendants step 2 and 3)
    let impact_res = dispatch(
        "knox_get_impact",
        json!({
            "action_id": id1,
            "max_depth": 5
        }),
        &db,
    );
    assert!(impact_res.error.is_none(), "impact error: {:?}", impact_res.error);
    let impact_val = impact_res.result.unwrap();
    let impacted = impact_val["impacted"].as_array().unwrap();
    assert!(!impacted.is_empty(), "impact should not be empty");

    // 4. Test knox_get_path between step 1 and step 3
    let path_res = dispatch(
        "knox_get_path",
        json!({
            "from_id": id1,
            "to_id": id3
        }),
        &db,
    );
    assert!(path_res.error.is_none(), "path error: {:?}", path_res.error);
    let path_val = path_res.result.unwrap();
    assert_eq!(path_val["connected"], true);
    assert_eq!(path_val["length"], 3);

    // 5. Test knox_get_impact by file_path
    let file_impact = dispatch(
        "knox_get_impact",
        json!({
            "file_path": "src/proto.rs"
        }),
        &db,
    );
    assert!(file_impact.error.is_none(), "file_impact error: {:?}", file_impact.error);
    assert_eq!(file_impact.result.unwrap()["impacted_actions_count"], 2);

    // 6. Test knox_get_path by from_file and to_file
    let file_path_res = dispatch(
        "knox_get_path",
        json!({
            "from_file": "src/proto.rs",
            "to_file": "tests/test_proto.rs"
        }),
        &db,
    );
    assert!(file_path_res.error.is_none(), "file_path_res error: {:?}", file_path_res.error);
    assert_eq!(file_path_res.result.unwrap()["connected"], true);
}

#[test]
fn test_mcp_check_guardrails() {
    let db = KnoxDb::open_memory().unwrap();

    // Record an active architectural decision for src/crypto.rs
    dispatch(
        "knox_record_action",
        json!({
            "actor": "security-bot",
            "action_type": "decision",
            "summary": "Mandate AES-256-GCM for all payloads",
            "files": ["src/crypto.rs"],
            "rationale": "Compliance with NIST SP 800-38D",
            "rejected_alternatives": ["ChaCha20-Poly1305 without hardware acceleration", "RC4"]
        }),
        &db,
    );

    // Call knox_check_guardrails - clean check
    let guard_res = dispatch(
        "knox_check_guardrails",
        json!({
            "files": ["src/crypto.rs"]
        }),
        &db,
    );
    assert!(guard_res.error.is_none(), "guardrails error: {:?}", guard_res.error);
    let guard_val = guard_res.result.unwrap();
    assert_eq!(guard_val["pass"], true);
    assert_eq!(guard_val["decision_count"], 1);
    let md = guard_val["markdown"].as_str().unwrap();
    assert!(md.contains("Mandate AES-256-GCM"));

    // Call knox_check_guardrails with proposed action matching rejected alternative
    let viol_res = dispatch(
        "knox_check_guardrails",
        json!({
            "files": ["src/crypto.rs"],
            "proposed_action": "Use ChaCha20-Poly1305 without hardware acceleration for quick encryption"
        }),
        &db,
    );
    assert!(viol_res.error.is_none());
    let viol_val = viol_res.result.unwrap();
    assert_eq!(viol_val["pass"], false);
    assert_eq!(viol_val["violations_count"], 1);
    assert!(viol_val["markdown"].as_str().unwrap().contains("Architectural Violations Detected"));
}

#[test]
fn test_mcp_resources_and_prompts() {
    let db = KnoxDb::open_memory().unwrap();

    // 1. Resources list
    let res_list = dispatch("resources/list", json!({}), &db);
    assert!(res_list.error.is_none());
    let resources = res_list.result.unwrap()["resources"].as_array().unwrap().clone();
    assert!(resources.iter().any(|r| r["uri"] == "knox://project/overview"));
    assert!(resources.iter().any(|r| r["uri"] == "knox://decisions/active"));

    // 2. Resources read
    let res_read = dispatch(
        "resources/read",
        json!({"uri": "knox://project/overview"}),
        &db,
    );
    assert!(res_read.error.is_none());
    let contents = res_read.result.unwrap()["contents"].as_array().unwrap().clone();
    assert_eq!(contents.len(), 1);
    assert!(contents[0]["text"].as_str().unwrap().contains("Knox Knowledge Graph Overview"));

    // 3. Prompts list
    let prompt_list = dispatch("prompts/list", json!({}), &db);
    assert!(prompt_list.error.is_none());
    let prompts = prompt_list.result.unwrap()["prompts"].as_array().unwrap().clone();
    assert!(prompts.iter().any(|p| p["name"] == "architectural-audit"));
    assert!(prompts.iter().any(|p| p["name"] == "session-handover"));

    // 4. Prompts get
    let prompt_get = dispatch(
        "prompts/get",
        json!({"name": "architectural-audit"}),
        &db,
    );
    assert!(prompt_get.error.is_none());
    let messages = prompt_get.result.unwrap()["messages"].as_array().unwrap().clone();
    assert!(!messages.is_empty());
}
