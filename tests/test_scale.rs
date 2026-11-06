//! Scale benchmarks and regression suite for Workstream D.
//!
//! Validates:
//! - Keyset cursor pagination (D2)
//! - Bounded connected component traversal with max_nodes cutoff (D3)
//! - Sub-50ms context p95 latency and sub-300ms trace p95 latency on 10,000 records (D4)
//! - Batched N+1 child lookup reduction (D1)

use std::time::Instant;

use chrono::{Duration, Utc};
use knox::commands::context::cmd_context;
use knox::commands::trace::cmd_trace;
use knox::core::ids::ActionId;
use knox::core::types::QueryFilter;
use knox::graph::loader::{DEFAULT_MAX_NODES, SubgraphDirection, load_subgraph_bounded};
use knox::storage::db::KnoxDb;
use knox::storage::queries::{decode_cursor, query_actions};
use rusqlite::params;
use serde_json::json;

const TOTAL_SCALE_ACTIONS: usize = 10_000;
const CHAIN_LENGTH: usize = 250;
const PROJECT_NAME: &str = "scale_proj";

/// Seeds an in-memory database with 10,000 actions in a single transaction.
/// Returns (deep_chain_tip_id, all_chain_ids).
fn seed_scale_db(db: &KnoxDb) -> (ActionId, Vec<ActionId>) {
    let now = Utc::now();
    let now_str = now.to_rfc3339();

    let mut chain_ids = Vec::with_capacity(CHAIN_LENGTH);

    db.with_tx(|conn| {
        // Register project and session
        conn.execute(
            "INSERT INTO projects (id, name, path, created_at) VALUES (?1, ?1, '/scale', ?2)",
            params![PROJECT_NAME, now_str],
        )?;

        conn.execute(
            "INSERT INTO sessions (id, actor, project_id, started_at, is_open) VALUES ('sess_scale', 'bench_actor', ?1, ?2, 1)",
            params![PROJECT_NAME, now_str],
        )?;

        let mut stmt_act = conn.prepare(
            "INSERT INTO actions (
                id, actor, action_type, summary, confidence, timestamp,
                session_id, parent_action_id, superseded_by, is_current,
                rationale, context_summary, rejected_alternatives,
                significance, is_resolved, resolution_note, commit_sha,
                project_id, timestamp_ms, dedupe_key, seen_count
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21)"
        )?;

        let mut stmt_file = conn.prepare(
            "INSERT OR IGNORE INTO action_files (action_id, file_path) VALUES (?1, ?2)"
        )?;
        let mut stmt_tag = conn.prepare(
            "INSERT OR IGNORE INTO action_tags (action_id, tag) VALUES (?1, ?2)"
        )?;
        let mut stmt_sym = conn.prepare(
            "INSERT OR IGNORE INTO action_symbols (action_id, symbol) VALUES (?1, ?2)"
        )?;
        let mut stmt_state = conn.prepare(
            "INSERT INTO action_state (action_id, state, by_action_id, updated_at)
             VALUES (?1, 'current', NULL, ?2)
             ON CONFLICT(action_id) DO UPDATE SET state = excluded.state"
        )?;

        let base_ts = now - Duration::seconds(TOTAL_SCALE_ACTIONS as i64);

        // 1. Build deep causal chain of 250 actions
        let mut prev_parent: Option<String> = None;
        for i in 0..CHAIN_LENGTH {
            let act_id = format!("act_chain_{:04}", i);
            let ts = base_ts + Duration::seconds(i as i64);
            let ts_ms = ts.timestamp_millis();
            let ts_str = ts.to_rfc3339();

            stmt_act.execute(params![
                act_id.as_str(),
                "bench_actor",
                "write",
                format!("Deep causal chain step {}", i),
                None::<f64>,
                ts_str,
                "sess_scale",
                prev_parent.as_deref(),
                None::<&str>,
                1, // is_current
                "Architectural rationale for chain",
                None::<&str>,
                None::<&str>,
                "major",
                1,
                None::<&str>,
                None::<&str>,
                PROJECT_NAME,
                ts_ms,
                None::<&str>,
                1,
            ])?;

            stmt_file.execute(params![act_id.as_str(), "src/chain.rs"])?;
            stmt_tag.execute(params![act_id.as_str(), "chain"])?;
            stmt_sym.execute(params![act_id.as_str(), "ChainStruct"])?;
            stmt_state.execute(params![act_id.as_str(), ts_str])?;

            prev_parent = Some(act_id.clone());
            chain_ids.push(ActionId::from(act_id));
        }

        // 2. Populate remaining actions up to 10,000
        for i in CHAIN_LENGTH..TOTAL_SCALE_ACTIONS {
            let act_id = format!("act_scale_{:06}", i);
            let ts = base_ts + Duration::seconds(i as i64);
            let ts_ms = ts.timestamp_millis();
            let ts_str = ts.to_rfc3339();
            let mod_idx = i % 100;
            let sym_idx = i % 50;
            let file_path = format!("src/module_{}.rs", mod_idx);
            let sym_name = format!("Symbol_{}", sym_idx);
            let tag_name = format!("tag_{}", i % 20);

            stmt_act.execute(params![
                act_id.as_str(),
                "bench_actor",
                "write",
                format!("Scale action {} for module {}", i, mod_idx),
                None::<f64>,
                ts_str,
                "sess_scale",
                None::<&str>,
                None::<&str>,
                1, // is_current
                None::<&str>,
                None::<&str>,
                None::<&str>,
                "minor",
                1,
                None::<&str>,
                None::<&str>,
                PROJECT_NAME,
                ts_ms,
                None::<&str>,
                1,
            ])?;

            stmt_file.execute(params![act_id.as_str(), file_path.as_str()])?;
            stmt_tag.execute(params![act_id.as_str(), tag_name.as_str()])?;
            stmt_sym.execute(params![act_id.as_str(), sym_name.as_str()])?;
            stmt_state.execute(params![act_id.as_str(), ts_str])?;
        }

        Ok(())
    }).unwrap();

    let tip = chain_ids.last().unwrap().clone();
    (tip, chain_ids)
}

#[test]
fn test_d1_child_lookup_hydration_correctness() {
    let db = KnoxDb::open_memory().unwrap();
    let (tip, _) = seed_scale_db(&db);

    // Query 20 actions and verify all child arrays (files, tags, symbols) are correctly hydrated
    let filter = QueryFilter {
        project: Some(PROJECT_NAME.to_string()),
        limit: 20,
        ..Default::default()
    };
    let qr = query_actions(&db, &filter).unwrap();
    assert_eq!(qr.results.len(), 20);

    for act in &qr.results {
        assert!(!act.files.is_empty(), "action {} missing files", act.id);
        assert!(!act.tags.is_empty(), "action {} missing tags", act.id);
        assert!(!act.symbols.is_empty(), "action {} missing symbols", act.id);
    }

    // Verify tip exists in chain
    assert_eq!(tip.as_str(), "act_chain_0249");
}

#[test]
fn test_d2_keyset_cursor_pagination_reassembly() {
    let db = KnoxDb::open_memory().unwrap();
    let _ = seed_scale_db(&db);

    // 1. Fetch first page of 5 items
    let page1_res = cmd_context(
        &db,
        json!({
            "project": PROJECT_NAME,
            "limit": 5,
            "budget": 4000,
            "format": "json"
        }),
    )
    .unwrap();

    assert_eq!(page1_res["has_more"], true);
    let cursor1 = page1_res["next"].as_str().expect("page1 next must be present");
    assert!(!cursor1.is_empty());

    let (ts, id) = decode_cursor(cursor1).expect("cursor must decode successfully");
    assert!(ts > 0);
    assert!(!id.is_empty());

    let page1_items = page1_res["results"]["results"]
        .as_array()
        .unwrap()
        .clone();
    assert_eq!(page1_items.len(), 5);

    // 2. Fetch second page of 5 items using cursor1
    let page2_res = cmd_context(
        &db,
        json!({
            "project": PROJECT_NAME,
            "limit": 5,
            "cursor": cursor1,
            "budget": 4000,
            "format": "json"
        }),
    )
    .unwrap();

    let page2_items = page2_res["results"]["results"]
        .as_array()
        .unwrap()
        .clone();
    assert_eq!(page2_items.len(), 5);

    // 3. Fetch single page of 10 items directly
    let direct10_res = cmd_context(
        &db,
        json!({
            "project": PROJECT_NAME,
            "limit": 10,
            "budget": 4000,
            "format": "json"
        }),
    )
    .unwrap();

    let direct10_items = direct10_res["results"]["results"]
        .as_array()
        .unwrap()
        .clone();
    assert_eq!(direct10_items.len(), 10);

    // 4. Verify reassembly: page1 + page2 must match direct 10 exactly without duplicates or missing
    let mut combined = Vec::new();
    combined.extend(page1_items.clone());
    combined.extend(page2_items.clone());

    let combined_ids: Vec<&str> = combined
        .iter()
        .map(|v| v["id"].as_str().unwrap())
        .collect();
    let direct_ids: Vec<&str> = direct10_items
        .iter()
        .map(|v| v["id"].as_str().unwrap())
        .collect();

    let mut combined_sorted = combined_ids.clone();
    combined_sorted.sort();
    let mut direct_sorted = direct_ids.clone();
    direct_sorted.sort();

    assert_eq!(
        combined_sorted, direct_sorted,
        "two consecutive pages must reassemble to single larger page without skips or duplicates"
    );
    assert_eq!(
        combined_ids, direct_ids,
        "two consecutive pages must preserve exact descending keyset order"
    );
    assert_eq!(combined_ids.len(), 10);

    // Ensure no overlap between page 1 and page 2
    for p1 in &page1_items {
        for p2 in &page2_items {
            assert_ne!(
                p1["id"].as_str().unwrap(),
                p2["id"].as_str().unwrap(),
                "page 1 and page 2 must not have overlapping records"
            );
        }
    }

    // 5. Verify malformed cursor rejection
    let malformed_res = cmd_context(
        &db,
        json!({
            "project": PROJECT_NAME,
            "cursor": "not_valid_base64_cursor!@#",
            "format": "json"
        }),
    );
    assert!(malformed_res.is_err(), "malformed cursor must be rejected with an error");

    // 6. Also test raw storage query keyset ordering directly (no ranking step)
    let f1 = QueryFilter {
        project: Some(PROJECT_NAME.to_string()),
        limit: 5,
        ..Default::default()
    };
    let q1 = query_actions(&db, &f1).unwrap();
    assert!(q1.has_more);
    let q1_cursor = q1.next.unwrap();

    let f2 = QueryFilter {
        project: Some(PROJECT_NAME.to_string()),
        limit: 5,
        cursor: Some(q1_cursor),
        ..Default::default()
    };
    let q2 = query_actions(&db, &f2).unwrap();

    let f10 = QueryFilter {
        project: Some(PROJECT_NAME.to_string()),
        limit: 10,
        ..Default::default()
    };
    let q10 = query_actions(&db, &f10).unwrap();

    let mut raw_combined: Vec<ActionId> = q1.results.iter().map(|a| a.id.clone()).collect();
    raw_combined.extend(q2.results.iter().map(|a| a.id.clone()));
    let raw_10: Vec<ActionId> = q10.results.iter().map(|a| a.id.clone()).collect();
    assert_eq!(
        raw_combined, raw_10,
        "raw storage keyset query must reassemble in exact same order"
    );
}

#[test]
fn test_d3_bounded_connected_component_traversal() {
    let db = KnoxDb::open_memory().unwrap();
    let (tip, chain_ids) = seed_scale_db(&db);

    // 1. Load bounded subgraph with default max_nodes (200) on the 250-node chain
    let bounded = load_subgraph_bounded(
        &db,
        std::slice::from_ref(&tip),
        300, // deep max_depth
        DEFAULT_MAX_NODES,
        SubgraphDirection::Ancestors,
    )
    .unwrap();

    assert!(
        bounded.truncated,
        "subgraph on 250-node chain with 200 max_nodes must be truncated"
    );
    assert_eq!(bounded.graph.node_count(), 200);
    assert!(
        !bounded.frontier_ids.is_empty(),
        "frontier_ids must contain cutoff boundaries"
    );

    // 2. Test trace command on the tip
    let trace_res = cmd_trace(
        &db,
        json!({
            "from": tip.as_str(),
            "depth": 300,
            "max_nodes": 100,
            "budget": 4000
        }),
    )
    .unwrap();

    assert_eq!(trace_res["truncated"], true);
    let frontier = trace_res["frontier"]
        .as_array()
        .expect("frontier array must be present");
    assert!(!frontier.is_empty(), "frontier must report boundary nodes");

    let md = trace_res["markdown"].as_str().unwrap();
    assert!(
        md.contains("*Traversal truncated at 100 nodes. Frontier:"),
        "markdown must display traversal truncation note with frontier"
    );

    // 3. Traversal with max_nodes larger than chain length should not truncate
    let unbounded = load_subgraph_bounded(
        &db,
        std::slice::from_ref(&tip),
        300,
        300, // larger than 250
        SubgraphDirection::Ancestors,
    )
    .unwrap();

    assert!(!unbounded.truncated);
    assert_eq!(unbounded.graph.node_count(), chain_ids.len());
    assert!(unbounded.frontier_ids.is_empty());
}

#[test]
fn test_d4_scale_latency_benchmarks_on_10k_records() {
    let db = KnoxDb::open_memory().unwrap();
    let (tip, _) = seed_scale_db(&db);

    // Benchmark 1: Context queries p95 latency < 50ms across 20 calls
    let mut context_latencies = Vec::with_capacity(20);
    for i in 0..20 {
        let file_filter = format!("src/module_{}.rs", (i * 3) % 100);
        let start = Instant::now();
        let res = cmd_context(
            &db,
            json!({
                "project": PROJECT_NAME,
                "file_path": file_filter,
                "limit": 20,
                "budget": 2000
            }),
        )
        .unwrap();
        let elapsed = start.elapsed().as_millis() as u64;
        context_latencies.push(elapsed);
        assert!(res["results"].is_array() || res["markdown"].is_string());
    }

    context_latencies.sort_unstable();
    let p95_idx = ((context_latencies.len() as f64) * 0.95).ceil() as usize - 1;
    let context_p95 = context_latencies[p95_idx];
    eprintln!("Workstream D Context p95 latency on 10k dataset: {context_p95}ms");
    let max_allowed = if cfg!(debug_assertions) { 80 } else { 50 };
    assert!(
        context_p95 <= max_allowed,
        "Context p95 latency must be <= {max_allowed}ms on 10k records, got {context_p95}ms"
    );

    // Benchmark 2: Trace p95 latency < 300ms across 20 calls on deep causal component
    let mut trace_latencies = Vec::with_capacity(20);
    for _ in 0..20 {
        let start = Instant::now();
        let res = cmd_trace(
            &db,
            json!({
                "from": tip.as_str(),
                "depth": 250,
                "max_nodes": 200,
                "budget": 2000
            }),
        )
        .unwrap();
        let elapsed = start.elapsed().as_millis() as u64;
        trace_latencies.push(elapsed);
        assert_eq!(res["truncated"], true);
    }

    trace_latencies.sort_unstable();
    let trace_p95_idx = ((trace_latencies.len() as f64) * 0.95).ceil() as usize - 1;
    let trace_p95 = trace_latencies[trace_p95_idx];
    eprintln!("Workstream D Trace p95 latency on deep component: {trace_p95}ms");
    assert!(
        trace_p95 < 300,
        "Trace p95 latency must be < 300ms on deep component, got {trace_p95}ms"
    );
}
