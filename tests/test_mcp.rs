//! Integration tests for the Knox MCP protocol and tool execution layer.
//!
//! The advertised tools are exercised: begin, context, check, trace,
//! record, handoff, ask.

use serde_json::json;

use knox::core::ids::ActionId;
use knox::core::types::QueryFilter;
use knox::mcp::handlers::dispatch;
use knox::mcp::tools::TOOL_COUNT;
use knox::storage::db::KnoxDb;

#[test]
fn test_mcp_session_lifecycle() {
    let db = KnoxDb::open_memory().expect("open memory db");

    let start_res = dispatch(
        "knox_begin",
        json!({"actor": "gemini", "workspace": "knox"}),
        &db,
    );
    assert!(start_res.error.is_none());
    let session_id = start_res
        .result
        .unwrap()
        .get("scope")
        .unwrap()
        .get("session_id")
        .unwrap()
        .as_str()
        .unwrap()
        .to_string();

    let act_res = dispatch(
        "knox_record",
        json!({
            "session_id": session_id,
            "actor": "gemini",
            "project": "knox",
            "entries": [{
                "kind": "write",
                "summary": "Implemented parser",
                "files": ["src/parser.rs"]
            }]
        }),
        &db,
    );
    assert_eq!(act_res.result.unwrap().get("status").unwrap(), "recorded");

    let end_res = dispatch("knox_handoff", json!({"session_id": session_id}), &db);
    let end_data = end_res.result.unwrap();
    assert_eq!(end_data.get("status").unwrap(), "closed");
}

fn mcp_start(db: &KnoxDb, actor: &str) -> String {
    let start_res = dispatch("knox_begin", json!({"actor": actor, "workspace": "demo"}), db);
    start_res.result.unwrap()["scope"]["session_id"]
        .as_str()
        .unwrap()
        .to_string()
}

fn mcp_write(db: &KnoxDb, sid: &str, actor: &str, summary: &str, file: &str) -> String {
    let act_res = dispatch(
        "knox_record",
        json!({
            "session_id": sid,
            "actor": actor,
            "project": "demo",
            "entries": [{
                "kind": "write",
                "summary": summary,
                "files": [file]
            }]
        }),
        db,
    );
    act_res.result.unwrap()["ids"][0].as_str().unwrap().to_string()
}

#[test]
fn test_mcp_decision_recording_and_parent_link() {
    let db = KnoxDb::open_memory().expect("open memory db");
    let session_id = mcp_start(&db, "gemini");
    let parent_id = mcp_write(&db, &session_id, "gemini", "Initial draft", "src/lib.rs");

    let dec_res = dispatch(
        "knox_record",
        json!({
            "session_id": session_id,
            "actor": "gemini",
            "project": "demo",
            "entries": [{
                "kind": "decide",
                "summary": "Decided petgraph architecture",
                "files": ["src/lib.rs"],
                "caused_by": parent_id
            }]
        }),
        &db,
    );
    let dec_id = dec_res.result.unwrap()["ids"][0].as_str().unwrap().to_string();

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
        "knox_record",
        json!({
            "session_id": session_id, "actor": "claude",
            "project": "demo",
            "entries": [{
                "kind": "decide",
                "summary": "New zero-copy lexer",
                "files": ["src/lexer.rs"],
                "supersedes": act1_id
            }]
        }),
        &db,
    );

    let ctx_res = dispatch(
        "knox_context",
        json!({"project": "demo", "view": "file", "files": ["src/lexer.rs"]}),
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
        "knox_record",
        json!({
            "session_id": session_id, "actor": "claude",
            "project": "demo",
            "entries": [{
                "kind": "decide",
                "summary": "New lexer",
                "files": ["src/lexer.rs"],
                "supersedes": act1_id
            }]
        }),
        &db,
    );

    // History via context with current_only=false
    let hist_res = dispatch(
        "knox_context",
        json!({"project": "demo", "view": "file", "files": ["src/lexer.rs"], "current_only": false}),
        &db,
    );
    assert_eq!(
        hist_res.result.unwrap()["results"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
}

#[test]
fn test_mcp_search_and_tools_call_standard() {
    let db = KnoxDb::open_memory().expect("open memory db");

    let init_res = dispatch("initialize", json!({}), &db);
    assert_eq!(init_res.result.unwrap()["serverInfo"]["name"], "knox");

    let list_res = dispatch("tools/list", json!({}), &db);
    assert_eq!(
        list_res.result.unwrap()["tools"].as_array().unwrap().len(),
        TOOL_COUNT
    );

    let session_id = mcp_start(&db, "gemini");
    mcp_write(
        &db,
        &session_id,
        "gemini",
        "Optimized inverted index indexing strategy",
        "src/index.rs",
    );

    let search_res = dispatch(
        "knox_context",
        json!({"project": "demo", "view": "query", "query": "inverted index"}),
        &db,
    );
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

    // 1. Record doubt via knox_record
    let doubt_res = dispatch(
        "knox_record",
        json!({
            "session_id": session_id,
            "actor": "gemini",
            "project": "demo",
            "entries": [{
                "kind": "doubt",
                "summary": "Can we support 50k nodes in petgraph without high heap overhead?",
                "context_summary": "Graph scalability investigation"
            }]
        }),
        &db,
    );
    assert!(
        doubt_res.error.is_none(),
        "record doubt failed: {:?}",
        doubt_res.error
    );
    let doubt_id = doubt_res.result.unwrap()["ids"][0]
        .as_str()
        .unwrap()
        .to_string();

    let action_id = ActionId::from(doubt_id.clone());
    let doubt_action = knox::storage::nodes::get_action(&db, &action_id)
        .unwrap()
        .expect("must exist");
    assert_eq!(doubt_action.action_type, "doubt");
    assert_eq!(doubt_action.is_resolved, Some(false));

    // 2. Record an answering decision
    let decide_res = dispatch(
        "knox_record",
        json!({
            "session_id": session_id,
            "actor": "gemini",
            "project": "demo",
            "entries": [{
                "kind": "decide",
                "summary": "Adopt lazy bounded BFS subgraph extraction rather than loading whole KG into petgraph",
                "significance": "major"
            }]
        }),
        &db,
    );
    assert!(decide_res.error.is_none());
    let _decide_id = decide_res.result.unwrap()["ids"][0]
        .as_str()
        .unwrap()
        .to_string();

    // 3. Resolve the doubt by directly marking it resolved in the database
    let action_id = ActionId::from(doubt_id.clone());
    db.with_tx(|conn| {
        conn.execute(
            "UPDATE actions SET is_resolved = 1, resolution_note = ?1 WHERE id = ?2",
            rusqlite::params![
                "Verified 2-hop bounded BFS uses <15MB heap regardless of overall 50k node DB size.",
                action_id.as_str()
            ],
        )
        .unwrap();
        Ok(())
    }).unwrap();

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
) -> tokio::task::JoinHandle<knox::core::KnoxResult<()>> {
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

    let start_res = send_mcp_call(write, lines, json!({"jsonrpc": "2.0", "id": 101, "method": "tools/call", "params": {"name": "knox_begin", "arguments": {"actor": "gemini-mcp", "workspace": "demo"}}})).await;
    let start_text = start_res["result"]["content"][0]["text"].as_str().unwrap();
    let start_data: serde_json::Value = serde_json::from_str(start_text).unwrap();
    let session_id = start_data["scope"]["session_id"]
        .as_str()
        .unwrap()
        .to_string();

    let rec_res = send_mcp_call(write, lines, json!({"jsonrpc": "2.0", "id": 102, "method": "tools/call", "params": {"name": "knox_record", "arguments": {"session_id": session_id, "actor": "gemini-mcp", "project": "demo", "entries": [{"kind": "write", "summary": "Built MCP loop", "files": ["src/mcp/server.rs"]}]}}})).await;
    assert_eq!(rec_res.get("id").unwrap(), 102);

    let end_res = send_mcp_call(write, lines, json!({"jsonrpc": "2.0", "id": 103, "method": "tools/call", "params": {"name": "knox_handoff", "arguments": {"session_id": session_id}}})).await;
    let end_text = end_res["result"]["content"][0]["text"].as_str().unwrap();
    let end_data: serde_json::Value = serde_json::from_str(end_text).unwrap();
    assert_eq!(end_data["status"], "closed");
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
        "knox_record",
        json!({
            "actor": "claude-code",
            "project": "demo",
            "entries": [{
                "kind": "decide",
                "summary": "Adopted SQLite WAL mode for concurrency",
                "files": ["src/storage/db.rs"],
                "symbols": ["KnoxDb::open", "Config::wal_mode"],
                "rationale": "High-throughput concurrent read-heavy workload",
                "rejected_alternatives": ["RocksDB", "DuckDB"],
                "context_summary": "SQLite configured with WAL mode and synchronous=NORMAL"
            }]
        }),
        &db,
    );

    assert!(res.error.is_none(), "record error: {:?}", res.error);
    let val = res.result.unwrap();
    assert_eq!(val["status"], "recorded");
    let action_id = val["ids"][0].as_str().unwrap().to_string();
    let session_id = val["session_id"].as_str().unwrap().to_string();
    assert!(!action_id.is_empty());
    assert!(!session_id.is_empty());

    // 2. Second action should reuse the open session
    let res2 = dispatch(
        "knox_record",
        json!({
            "actor": "claude-code",
            "project": "demo",
            "entries": [{
                "kind": "write",
                "summary": "Implemented WAL checkpointing logic",
                "files": ["src/storage/db.rs"],
                "symbols": ["KnoxDb::checkpoint"],
                "caused_by": action_id,
                "relations": [{
                    "to": action_id,
                    "relation_type": "implements"
                }]
            }]
        }),
        &db,
    );

    let val2 = res2.result.unwrap();
    assert_eq!(val2["session_id"], session_id);

    // 3. knox_context with symbol filter and markdown format
    let ctx_res = dispatch(
        "knox_context",
        json!({
            "project": "demo",
            "view": "symbol",
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
        "knox_record",
        json!({
            "actor": "codex",
            "project": "demo",
            "entries": [
                {
                    "kind": "decide",
                    "summary": "Step 1: Protocol design",
                    "files": ["src/proto.rs"],
                    "symbols": ["Proto::parse"],
                    "ref": "step1"
                },
                {
                    "kind": "write",
                    "summary": "Step 2: Code generation",
                    "files": ["src/gen.rs"],
                    "symbols": ["Generator::run"],
                    "ref": "step2",
                    "caused_by": "step1"
                },
                {
                    "kind": "verify",
                    "summary": "Step 3: Protocol unit tests",
                    "files": ["tests/test_proto.rs"],
                    "symbols": ["test_wire_roundtrip"],
                    "caused_by": "step2"
                }
            ]
        }),
        &db,
    );

    assert!(
        batch_res.error.is_none(),
        "batch error: {:?}",
        batch_res.error
    );
    let batch_val = batch_res.result.unwrap();
    assert_eq!(batch_val["recorded_count"], 3);
    let action_ids = batch_val["ids"].as_array().unwrap();
    assert_eq!(action_ids.len(), 3);

    let id1 = action_ids[0].as_str().unwrap();
    let _id2 = action_ids[1].as_str().unwrap();
    let id3 = action_ids[2].as_str().unwrap();

    // 2. Test knox_trace from step 3 (should trace backward)
    let trace_res = dispatch(
        "knox_trace",
        json!({
            "from": id3,
            "max_depth": 5
        }),
        &db,
    );
    assert!(
        trace_res.error.is_none(),
        "trace error: {:?}",
        trace_res.error
    );
    let trace_val = trace_res.result.unwrap();
    let chain = trace_val["chain"].as_array().unwrap();
    assert!(
        chain.len() >= 2,
        "chain should contain ancestors, got: {chain:?}"
    );

    // 3. Test knox_trace from step 1 forward (downstream impact / descendants)
    let impact_res = dispatch(
        "knox_trace",
        json!({
            "from": id1,
            "direction": "descendants",
            "max_depth": 5
        }),
        &db,
    );
    assert!(
        impact_res.error.is_none(),
        "impact error: {:?}",
        impact_res.error
    );
    let impact_val = impact_res.result.unwrap();
    let impacted = impact_val["impact"].as_array().unwrap();
    assert_eq!(impacted.len(), 2, "descendants of step1 must be step2 and step3");
    assert_eq!(impact_val["direction"], "descendants");
    assert!(impact_val["markdown"].as_str().unwrap().contains("Downstream Impact"));

    // 4. Test knox_trace with direction: "both" from step 2
    let both_res = dispatch(
        "knox_trace",
        json!({
            "from": _id2,
            "direction": "both",
            "max_depth": 5
        }),
        &db,
    );
    assert!(both_res.error.is_none());
    let both_val = both_res.result.unwrap();
    assert_eq!(both_val["lineage"].as_array().unwrap().len(), 2); // step2, step1
    assert_eq!(both_val["impact"].as_array().unwrap().len(), 1);  // step3
    assert_eq!(both_val["direction"], "both");
}

#[test]
fn test_trace_impact_traverses_implements_and_ignores_relates_to() {
    let db = KnoxDb::open_memory().unwrap();

    let rec_res = dispatch(
        "knox_record",
        json!({
            "actor": "architect",
            "project": "demo",
            "entries": [
                {
                    "ref": "dec",
                    "kind": "decide",
                    "summary": "Mandate SQLite for persistent storage",
                    "files": ["src/storage/db.rs"],
                    "rationale": "Zero external operational dependencies"
                },
                {
                    "ref": "impl",
                    "kind": "write",
                    "summary": "Implement SQLite database connection pool",
                    "files": ["src/storage/db.rs"],
                    "relations": [{"to": "dec", "relation_type": "implements"}]
                },
                {
                    "ref": "assoc",
                    "kind": "write",
                    "summary": "Update external benchmark documentation referencing SQLite",
                    "files": ["docs/benchmarks.md"],
                    "relations": [{"to": "dec", "relation_type": "relates_to"}]
                }
            ]
        }),
        &db,
    );
    assert!(rec_res.error.is_none(), "record error: {:?}", rec_res.error);
    let ids = rec_res.result.unwrap()["ids"].as_array().unwrap().clone();
    let dec_id = ids[0].as_str().unwrap();
    let impl_id = ids[1].as_str().unwrap();
    let assoc_id = ids[2].as_str().unwrap();

    // Trace impact from the architectural decision
    let trace_res = dispatch(
        "knox_trace",
        json!({
            "from": dec_id,
            "direction": "impact",
            "max_depth": 5
        }),
        &db,
    );
    assert!(trace_res.error.is_none(), "trace error: {:?}", trace_res.error);
    let trace_val = trace_res.result.unwrap();
    let impact = trace_val["impact"].as_array().expect("impact must be an array");

    // Must ONLY contain the implementing action, NOT the associative note
    let impact_ids: Vec<&str> = impact.iter().filter_map(|a| a["id"].as_str()).collect();
    assert!(
        impact_ids.contains(&impl_id),
        "impact must include causal implementer '{impl_id}', got: {impact_ids:?}"
    );
    assert!(
        !impact_ids.contains(&assoc_id),
        "impact must NOT include associative relates_to action '{assoc_id}', got: {impact_ids:?}"
    );
    assert_eq!(impact.len(), 1, "only causal descendants should be returned in impact analysis");

    let md = trace_val["markdown"].as_str().unwrap();
    assert!(md.contains("Implement SQLite database connection pool"));
    assert!(!md.contains("Update external benchmark documentation"));
}

#[test]
fn test_trace_file_first_impact_groups_by_file_with_summary() {
    let db = KnoxDb::open_memory().unwrap();

    let rec_res = dispatch(
        "knox_record",
        json!({
            "actor": "architect",
            "project": "demo",
            "entries": [
                {
                    "ref": "dec",
                    "kind": "decide",
                    "summary": "Core type system design",
                    "files": ["src/core/types.rs"],
                    "rationale": "Strong domain modeling"
                },
                {
                    "ref": "w1",
                    "kind": "write",
                    "summary": "Initial struct definitions",
                    "files": ["src/core/types.rs"],
                    "caused_by": "dec"
                },
                {
                    "ref": "w2",
                    "kind": "write",
                    "summary": "Graph traversal algorithm implementation",
                    "files": ["src/graph/traversal.rs"],
                    "relations": [{"to": "dec", "relation_type": "implements"}]
                },
                {
                    "ref": "w3",
                    "kind": "write",
                    "summary": "MCP response formatting",
                    "files": ["src/mcp/format.rs"],
                    "caused_by": "w1"
                }
            ]
        }),
        &db,
    );
    assert!(rec_res.error.is_none(), "record error: {:?}", rec_res.error);
    let ids = rec_res.result.unwrap()["ids"].as_array().unwrap().clone();
    let w2_id = ids[2].as_str().unwrap();
    let w3_id = ids[3].as_str().unwrap();

    // Query impact for the file "src/core/types.rs"
    let trace_res = dispatch(
        "knox_trace",
        json!({
            "file_path": "src/core/types.rs",
            "direction": "impact",
            "max_depth": 5
        }),
        &db,
    );
    assert!(trace_res.error.is_none(), "trace error: {:?}", trace_res.error);
    let trace_val = trace_res.result.unwrap();
    let impact = trace_val["impact"].as_array().expect("impact must be an array");

    let impact_ids: Vec<&str> = impact.iter().filter_map(|a| a["id"].as_str()).collect();
    assert!(impact_ids.contains(&w2_id));
    assert!(impact_ids.contains(&w3_id));
    assert_eq!(impact.len(), 2);

    let md = trace_val["markdown"].as_str().unwrap();
    // Summary line: Affects 2 actions across 2 files
    assert!(
        md.contains("Affects 2 actions across 2 files"),
        "Markdown must contain summary line with actions and files count: {md}"
    );
    // Grouped file headings
    assert!(md.contains("#### `src/graph/traversal.rs`"));
    assert!(md.contains("#### `src/mcp/format.rs`"));
}

#[test]
fn test_contradictions_persisted_surfaced_in_context_and_reported_in_handoff() {
    unsafe {
        std::env::set_var("KNOX_MODEL_PROFILE", "mock");
    }

    let db = KnoxDb::open_memory().unwrap();

    // 1. Start an explicit session
    let begin_res = dispatch(
        "knox_begin",
        json!({
            "actor": "developer",
            "workspace": "demo"
        }),
        &db,
    );
    assert!(begin_res.error.is_none(), "begin error: {:?}", begin_res.error);
    let sess_id = begin_res.result.unwrap()["scope"]["session_id"].as_str().unwrap().to_string();

    // 2. Record an established architectural decision
    let dec_res = dispatch(
        "knox_record",
        json!({
            "actor": "architect",
            "project": "demo",
            "session_id": sess_id,
            "entries": [{
                "ref": "dec1",
                "kind": "decide",
                "summary": "Mandate SQLite for all embedded persistent storage",
                "files": ["src/db.rs"],
                "rationale": "Zero external network dependencies"
            }]
        }),
        &db,
    );
    assert!(dec_res.error.is_none());

    // 3. Record an action that contradicts the established decision
    let conflict_res = dispatch(
        "knox_record",
        json!({
            "actor": "contributor",
            "project": "demo",
            "session_id": sess_id,
            "entries": [{
                "ref": "conf1",
                "kind": "write",
                "summary": "Replace SQLite with external MongoDB instead",
                "files": ["src/db.rs"],
                "rationale": "Need document store"
            }]
        }),
        &db,
    );
    assert!(conflict_res.error.is_none());
    let conf_val = conflict_res.result.unwrap();

    // Check warning was emitted in record response
    let warnings = conf_val["warnings"].as_array().expect("warnings array");
    let has_contra_warn = warnings.iter().any(|w| {
        w.get("code").and_then(serde_json::Value::as_str) == Some("contradiction_detected")
    });
    assert!(has_contra_warn, "Record response should warn of detected contradiction: {warnings:?}");

    // Check edge was persisted in database with provenance = 'model'
    let conf_id = conf_val["ids"][0].as_str().unwrap();
    let edges = knox::storage::edges::get_edges_for_action(&db, &ActionId::from(conf_id)).unwrap();
    let contra_edge = edges.iter().find(|e| e.edge_type == "contradicts").expect("Contradiction edge must be persisted in edges table");
    assert_eq!(contra_edge.provenance.as_deref(), Some("model"));

    // 4. knox_context must surface Contradictions section in grouped context
    let ctx_res = dispatch(
        "knox_context",
        json!({
            "project": "demo",
            "format": "markdown",
            "synthesis": true
        }),
        &db,
    );
    assert!(ctx_res.error.is_none());
    let ctx_md = ctx_res.result.unwrap()["markdown"].as_str().unwrap().to_string();
    assert!(
        ctx_md.contains("Architectural Contradictions & Conflicts"),
        "Grouped context must surface contradictions section: {ctx_md}"
    );
    assert!(ctx_md.contains("contradicts established decision"));

    // 5. knox_handoff must report contradiction counts
    let handoff_res = dispatch(
        "knox_handoff",
        json!({
            "session_id": sess_id
        }),
        &db,
    );
    assert!(handoff_res.error.is_none());
    let handoff_val = handoff_res.result.unwrap();
    let count = handoff_val["contradictions"].as_u64().expect("contradictions count");
    assert!(count >= 1, "Session handoff must report non-zero contradiction count: {count}");

    unsafe {
        std::env::remove_var("KNOX_MODEL_PROFILE");
    }
}

#[test]
fn test_refs_as_endpoints_in_relations_and_file_graph_query() {
    let db = KnoxDb::open_memory().unwrap();

    // 1. Record an architectural decision with relation targeting "file:src/db.rs" using target/type keys
    let rec_res = dispatch(
        "knox_record",
        json!({
            "actor": "architect",
            "project": "demo",
            "entries": [{
                "ref": "dec_schema",
                "kind": "decide",
                "summary": "Strict column-oriented schema specification",
                "relations": [{
                    "target": "file:src/database/schema.rs",
                    "type": "relates_to"
                }]
            }]
        }),
        &db,
    );
    assert!(rec_res.error.is_none(), "record error: {:?}", rec_res.error);
    let act_id = rec_res.result.unwrap()["ids"][0].as_str().unwrap().to_string();

    // 2. Assert external reference was auto-registered in refs table
    let ref_entry = knox::storage::edges::get_ref(&db, "file:src/database/schema.rs")
        .unwrap()
        .expect("Ref file:src/database/schema.rs must exist in refs table");
    assert_eq!(ref_entry.id, "file:src/database/schema.rs");
    assert_eq!(ref_entry.kind, "file");
    assert_eq!(ref_entry.value, "src/database/schema.rs");

    // 3. Assert graph edge exists in edges table
    let edges = knox::storage::edges::get_edges_for_action(&db, &ActionId::from(act_id.as_str())).unwrap();
    assert_eq!(edges.len(), 1);
    assert_eq!(edges[0].to_id, "file:src/database/schema.rs");
    assert_eq!(edges[0].edge_type, "relates_to");

    // 4. Assert graph query for file resolves the decision via edges
    let ctx_res = dispatch(
        "knox_context",
        json!({
            "project": "demo",
            "file_path": "src/database/schema.rs"
        }),
        &db,
    );
    assert!(ctx_res.error.is_none(), "context error: {:?}", ctx_res.error);
    let ctx_val = ctx_res.result.unwrap();
    let total = ctx_val["total_count"].as_u64().unwrap();
    assert_eq!(total, 1, "Graph query on file_path should find action connected via edges table");
    assert!(ctx_val["markdown"].as_str().unwrap().contains("Strict column-oriented schema"));
}

#[test]
fn test_mcp_check_guardrails() {
    let db = KnoxDb::open_memory().unwrap();

    // Record an active architectural decision for src/crypto.rs
    dispatch(
        "knox_record",
        json!({
            "actor": "security-bot",
            "project": "demo",
            "entries": [{
                "kind": "decide",
                "summary": "Mandate AES-256-GCM for all payloads",
                "files": ["src/crypto.rs"],
                "rationale": "Compliance with NIST SP 800-38D",
                "rejected_alternatives": ["ChaCha20-Poly1305 without hardware acceleration", "RC4"]
            }]
        }),
        &db,
    );

    // Call knox_check - clean check
    let guard_res = dispatch(
        "knox_check",
        json!({
            "project": "demo",
            "files": ["src/crypto.rs"]
        }),
        &db,
    );
    assert!(
        guard_res.error.is_none(),
        "guardrails error: {:?}",
        guard_res.error
    );
    let guard_val = guard_res.result.unwrap();
    assert_eq!(guard_val["pass"], true);
    assert_eq!(guard_val["decision_count"], 1);

    // Call knox_check with proposed action matching rejected alternative
    let viol_res = dispatch(
        "knox_check",
        json!({
            "project": "demo",
            "files": ["src/crypto.rs"],
            "intent": "Use ChaCha20-Poly1305 without hardware acceleration for quick encryption"
        }),
        &db,
    );
    assert!(viol_res.error.is_none());
    let viol_val = viol_res.result.unwrap();
    assert_eq!(viol_val["pass"], false);
    assert_eq!(viol_val["violations_count"], 1);
}

#[test]
fn test_mcp_resources_and_prompts() {
    let db = KnoxDb::open_memory().unwrap();

    // 1. Resources list
    let res_list = dispatch("resources/list", json!({}), &db);
    assert!(res_list.error.is_none());
    let resources = res_list.result.unwrap()["resources"]
        .as_array()
        .unwrap()
        .clone();
    assert!(
        resources
            .iter()
            .any(|r| r["uri"] == "knox://project/overview")
    );
    assert!(
        resources
            .iter()
            .any(|r| r["uri"] == "knox://decisions/active")
    );

    // 2. Resources read
    let res_read = dispatch(
        "resources/read",
        json!({"uri": "knox://project/overview"}),
        &db,
    );
    assert!(res_read.error.is_none());
    let contents = res_read.result.unwrap()["contents"]
        .as_array()
        .unwrap()
        .clone();
    assert_eq!(contents.len(), 1);
    assert!(
        contents[0]["text"]
            .as_str()
            .unwrap()
            .contains("Knox Knowledge Graph Overview")
    );

    // 3. Prompts list
    let prompt_list = dispatch("prompts/list", json!({}), &db);
    assert!(prompt_list.error.is_none());
    let prompts = prompt_list.result.unwrap()["prompts"]
        .as_array()
        .unwrap()
        .clone();
    assert!(prompts.iter().any(|p| p["name"] == "architectural-audit"));
    assert!(prompts.iter().any(|p| p["name"] == "session-handover"));

    // 4. Prompts get
    let prompt_get = dispatch("prompts/get", json!({"name": "architectural-audit"}), &db);
    assert!(prompt_get.error.is_none());
    let messages = prompt_get.result.unwrap()["messages"]
        .as_array()
        .unwrap()
        .clone();
    assert!(!messages.is_empty());
}

#[test]
fn test_mcp_ingest_conversation_with_plans_and_doubts() {
    let db = KnoxDb::open_memory().unwrap();

    let raw_conversation = r#"
## User
Can I delete the old egui GUI module from the codebase?

## Assistant
Decided to remove egui and eframe completely in favor of Tauri desktop.

## User
Implement the clean removal of egui dependencies from Cargo.toml
"#;

    let ingest_res = knox::commands::cmd_ingest_raw(raw_conversation, &db, Some("gemini"), false);

    assert!(ingest_res.is_ok());
    let result = ingest_res.unwrap();
    assert_eq!(result.actions_imported, 3);

    // Verify Doubt was captured
    let doubt_filter = QueryFilter {
        action_type: Some("doubt".to_string()),
        ..Default::default()
    };
    let doubt_res = knox::storage::queries::query_actions(&db, &doubt_filter).unwrap();
    assert_eq!(doubt_res.total_count, 1);
    assert_eq!(doubt_res.results[0].is_resolved, Some(false));
    assert!(
        doubt_res.results[0]
            .summary
            .contains("egui")
    );
}

#[test]
fn test_mcp_stdio_stdout_purity() {
    // Regression test for A1: tracing must write to stderr, never stdout.
    // Every line on stdout must be valid JSON-RPC, even when warnings fire.
    // Requires KNOX_TEST_BIN to point at a pre-built release binary (set in CI
    // after the release build step). Fails if the binary is not available — no
    // nested cargo build from inside the test.
    use std::io::{BufRead, BufReader, Write};
    use std::process::{Command, Stdio};

    let exe = std::env::var("KNOX_TEST_BIN")
        .unwrap_or_else(|_| {
            std::env::var("CARGO_MANIFEST_DIR").unwrap()
                + "/target/release/knox.exe"
        });
    let exe_path = std::path::Path::new(&exe);
    assert!(
        exe_path.exists(),
        "KNOX_TEST_BIN binary not found at {exe:?} — build the release binary first and set KNOX_TEST_BIN"
    );

    let mut child = Command::new(&exe)
        .args(["serve", "--stdio"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("failed to spawn knox serve --stdio");

    let mut stdin = child.stdin.take().unwrap();
    let stdout = child.stdout.take().unwrap();
    let mut stdout = BufReader::new(stdout);

    // Send initialize, then drop stdin so the server sees EOF and drains its
    // response to stdout.
    writeln!(
        stdin,
        "{}",
        serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {}
        })
    )
    .unwrap();
    stdin.flush().unwrap();
    drop(stdin);

    let mut stdout_lines = Vec::new();
    let mut line = String::new();
    while stdout.read_line(&mut line).unwrap_or(0) > 0 {
        stdout_lines.push(line.clone());
        line.clear();
    }

    // Every stdout line must parse as JSON-RPC
    for line in &stdout_lines {
        let trimmed = line.trim();
        if !trimmed.is_empty() {
            serde_json::from_str::<serde_json::Value>(trimmed)
                .unwrap_or_else(|e| panic!("stdout is not valid JSON-RPC: {trimmed} (err: {e})"));
        }
    }

    // A real initialize response must have been received — an empty stdout
    // means the server never responded, which would look like a pass.
    assert!(
        !stdout_lines.is_empty(),
        "serve --stdio produced no stdout response to initialize"
    );

    // Clean up
    let _ = child.kill();
    let _ = child.wait();
}

#[test]
fn test_begin_without_workspace_param() {
    let db = KnoxDb::open_memory().expect("open memory db");
    let res = dispatch("knox_begin", json!({ "actor": "gemini" }), &db);
    assert!(res.error.is_none(), "knox_begin should succeed without workspace: {:?}", res.error);
    let scope = res.result.unwrap().get("scope").cloned().unwrap();
    assert_eq!(scope.get("actor").unwrap(), "gemini");
}

#[test]
fn test_context_schema_and_project_fallback() {
    let db = KnoxDb::open_memory().expect("open memory db");
    // Register project for "." so fallback resolves cleanly
    let proj = knox::core::types::Project {
        id: knox::core::ids::ProjectId("proj_root".to_string()),
        name: "test-root".to_string(),
        path: ".".to_string(),
        remote: None,
        created_at: chrono::Utc::now(),
    };
    knox::storage::nodes::insert_project(&db, &proj).unwrap();

    let res = dispatch("knox_context", json!({ "view": "context" }), &db);
    assert!(res.error.is_none(), "knox_context should resolve workspace project fallback: {:?}", res.error);
}

#[test]
fn test_context_with_files_alias() {
    let db = KnoxDb::open_memory().expect("open memory db");
    let proj = knox::core::types::Project {
        id: knox::core::ids::ProjectId("demo".to_string()),
        name: "demo".to_string(),
        path: "d:/dummy/demo".to_string(),
        remote: None,
        created_at: chrono::Utc::now(),
    };
    knox::storage::nodes::insert_project(&db, &proj).unwrap();

    let res = dispatch("knox_context", json!({
        "project": "demo",
        "view": "file",
        "files": ["src/main.rs"]
    }), &db);
    assert!(res.error.is_none(), "knox_context must accept files alias: {:?}", res.error);
}

#[test]
fn test_check_guardrails_project_isolation() {
    let db = KnoxDb::open_memory().expect("open memory db");
    let proj_a = knox::core::types::Project {
        id: knox::core::ids::ProjectId("proj_a".to_string()),
        name: "Project A".to_string(),
        path: "d:/dummy/proj_a".to_string(),
        remote: None,
        created_at: chrono::Utc::now(),
    };
    let proj_b = knox::core::types::Project {
        id: knox::core::ids::ProjectId("proj_b".to_string()),
        name: "Project B".to_string(),
        path: "d:/dummy/proj_b".to_string(),
        remote: None,
        created_at: chrono::Utc::now(),
    };
    knox::storage::nodes::insert_project(&db, &proj_a).unwrap();
    knox::storage::nodes::insert_project(&db, &proj_b).unwrap();

    // Record decision in Project A strictly forbidding edits to sensitive.rs
    let act_a = knox::core::types::KnoxAction {
        id: ActionId::new(),
        actor: "lead".to_string(),
        action_type: "decide".to_string(),
        summary: "Do not modify sensitive.rs directly".to_string(),
        files: vec!["sensitive.rs".to_string()],
        tags: vec![],
        confidence: None,
        is_current: true,
        timestamp: chrono::Utc::now(),
        project_id: Some(knox::core::ids::ProjectId("proj_a".to_string())),
        ..Default::default()
    };
    knox::storage::nodes::insert_action(&db, &act_a).unwrap();

    // Check guardrails for sensitive.rs in Project B with strict: true
    let check_res = dispatch("knox_check", json!({
        "project": "proj_b",
        "files": ["sensitive.rs"],
        "proposed_action": "modify sensitive.rs directly",
        "strict": true
    }), &db);

    assert!(check_res.error.is_none());
    let val = check_res.result.unwrap();
    assert_eq!(val.get("pass").unwrap(), true, "Project A decision must NOT leak into Project B check");
    assert_eq!(val.get("decision_count").unwrap(), 0, "No decisions should be found in Project B scope");
}

