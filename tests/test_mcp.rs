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
        9
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
