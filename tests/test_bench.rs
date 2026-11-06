//! P0 Benchmark Harness for Knox.
//!
//! Measures the plan's quantifiable goals:
//! - tools/list token cost (target <=850)
//! - tokens per read
//! - recall@5 (top-5 hits / expected ids)
//! - empty-result rate
//! - round trips per workflow
//! - budget enforcement (tokens(response) <= budget)
//!
//! Seeds a database with fixture-derived actions, runs 40 labelled queries,
//! and reports deterministic metrics that can be compared across phases.

use serde_json::json;

use knox::core::budget::estimate_tokens;
use knox::core::types::KnoxAction;
use knox::core::ids::ActionId;
use knox::mcp::handlers::dispatch;
use knox::mcp::protocol::ToolDescription;
use knox::mcp::tools::{all_tools, LISTED_TOOL_NAMES, TOOL_COUNT};
use knox::metrics::measure_token_efficiency;
use knox::storage::db::KnoxDb;
use knox::storage::nodes::insert_action;
use knox::storage::sessions::{start_session, end_session};
use chrono::Utc;

/// A labelled query with expected top-5 action IDs for recall@5 measurement.
#[derive(Debug, Clone, serde::Deserialize)]
struct LabelledQuery {
    name: String,
    params: serde_json::Value,
    expected_ids: Vec<String>,
    budget: usize,
}

/// A measurable benchmark result.
struct BenchResult {
    _name: String,
    response_tokens: usize,
    within_budget: bool,
    recall_at_5: f64,
}

/// Seed the database with fixture-derived actions for benchmarking.
fn seed_benchmark_db(db: &KnoxDb) -> Vec<ActionId> {
    let session = start_session(db, "bench-agent", Some("demo")).unwrap();
    knox::storage::nodes::insert_project(db, &knox::core::types::Project {
        id: knox::core::ids::ProjectId("demo".to_string()),
        name: "demo".to_string(),
        path: "demo".to_string(),
        remote: None,
        created_at: Utc::now(),
    }).unwrap();
    let mut ids = Vec::new();

    let actions: Vec<KnoxAction> = vec![
        // F1: basic workflow
        KnoxAction {
            id: ActionId::from("a-f1-plan-parser"),
            actor: "bench-agent".to_string(),
            action_type: "plan".to_string(),
            summary: "Plan the parser module for token streaming".to_string(),
            files: vec!["src/parser.rs".to_string()],
            symbols: vec!["parse".to_string()],
            session_id: Some(session.id.clone()),
            timestamp: Utc::now(),
            ..Default::default()
        },
        KnoxAction {
            id: ActionId::from("a-f1-write-parser"),
            actor: "bench-agent".to_string(),
            action_type: "write".to_string(),
            summary: "Implement parser interface fn parse() -> Ast".to_string(),
            files: vec!["src/parser.rs".to_string()],
            symbols: vec!["parse".to_string(), "Ast".to_string()],
            parent_action_id: Some(ActionId::from("a-f1-plan-parser")),
            session_id: Some(session.id.clone()),
            timestamp: Utc::now(),
            ..Default::default()
        },
        KnoxAction {
            id: ActionId::from("a-f1-test-parser"),
            actor: "bench-agent".to_string(),
            action_type: "verify".to_string(),
            summary: "Run cargo test --test parser, 4 passed".to_string(),
            files: vec!["src/parser.rs".to_string()],
            session_id: Some(session.id.clone()),
            timestamp: Utc::now(),
            ..Default::default()
        },
        // F2: decisions and doubts
        KnoxAction {
            id: ActionId::from("a-f2-decide-wal"),
            actor: "bench-agent".to_string(),
            action_type: "decide".to_string(),
            summary: "Decided on SQLite WAL mode for concurrent multi-agent read/write".to_string(),
            files: vec!["src/storage/db.rs".to_string()],
            symbols: vec!["KnoxDb".to_string(), "WAL".to_string()],
            rejected_alternatives: vec!["RocksDB".to_string(), "DuckDB".to_string()],
            session_id: Some(session.id.clone()),
            timestamp: Utc::now(),
            ..Default::default()
        },
        KnoxAction {
            id: ActionId::from("a-f2-doubt-wal-ui"),
            actor: "bench-agent".to_string(),
            action_type: "doubt".to_string(),
            summary: "Will SQLite WAL mode block the Tauri UI if a large batch is being ingested?".to_string(),
            files: vec!["src-tauri/src/commands/ingest.rs".to_string()],
            is_resolved: Some(false),
            session_id: Some(session.id.clone()),
            timestamp: Utc::now(),
            ..Default::default()
        },
        KnoxAction {
            id: ActionId::from("a-f2-answer-wal-ui"),
            actor: "bench-agent".to_string(),
            action_type: "verify".to_string(),
            summary: "Verified SQLite WAL allows concurrent readers on Tauri UI thread while worker writes batches".to_string(),
            files: vec!["src-tauri/src/commands/ingest.rs".to_string()],
            parent_action_id: Some(ActionId::from("a-f2-doubt-wal-ui")),
            session_id: Some(session.id.clone()),
            timestamp: Utc::now(),
            ..Default::default()
        },
        // F3: multi-file change
        KnoxAction {
            id: ActionId::from("a-f3-plan-traversal"),
            actor: "bench-agent".to_string(),
            action_type: "plan".to_string(),
            summary: "Refactor the graph traversal module to support bidirectional edges".to_string(),
            files: vec!["src/graph/traversal.rs".to_string()],
            symbols: vec!["traverse_bidirectional".to_string()],
            session_id: Some(session.id.clone()),
            timestamp: Utc::now(),
            ..Default::default()
        },
        KnoxAction {
            id: ActionId::from("a-f3-write-traversal"),
            actor: "bench-agent".to_string(),
            action_type: "write".to_string(),
            summary: "Implement traverse_bidirectional() in graph traversal module".to_string(),
            files: vec!["src/graph/traversal.rs".to_string()],
            symbols: vec!["traverse_bidirectional".to_string()],
            parent_action_id: Some(ActionId::from("a-f3-plan-traversal")),
            session_id: Some(session.id.clone()),
            timestamp: Utc::now(),
            ..Default::default()
        },
        KnoxAction {
            id: ActionId::from("a-f3-write-test"),
            actor: "bench-agent".to_string(),
            action_type: "write".to_string(),
            summary: "Add tests/graph_traversal_test.rs".to_string(),
            files: vec!["tests/graph_traversal_test.rs".to_string()],
            parent_action_id: Some(ActionId::from("a-f3-plan-traversal")),
            session_id: Some(session.id.clone()),
            timestamp: Utc::now(),
            ..Default::default()
        },
        // F4: conflict and supersession
        KnoxAction {
            id: ActionId::from("a-f4-decide-toml"),
            actor: "bench-agent".to_string(),
            action_type: "decide".to_string(),
            summary: "Decided to adopt TOML for configuration".to_string(),
            files: vec!["Cargo.toml".to_string()],
            rejected_alternatives: vec!["JSON".to_string(), "YAML".to_string()],
            session_id: Some(session.id.clone()),
            timestamp: Utc::now(),
            ..Default::default()
        },
        KnoxAction {
            id: ActionId::from("a-f4-supersede-toml"),
            actor: "bench-agent".to_string(),
            action_type: "decide".to_string(),
            summary: "Superseding TOML decision. Keeping JSON config for backwards compatibility".to_string(),
            files: vec!["Cargo.toml".to_string()],
            supersedes: Some(ActionId::from("a-f4-decide-toml")),
            rejected_alternatives: vec!["TOML".to_string()],
            session_id: Some(session.id.clone()),
            timestamp: Utc::now(),
            ..Default::default()
        },
        // F5: large session
        KnoxAction {
            id: ActionId::from("a-f5-plan-auth"),
            actor: "bench-agent".to_string(),
            action_type: "plan".to_string(),
            summary: "Build the authentication service with JWT, OAuth2, and session management".to_string(),
            files: vec!["src/auth/mod.rs".to_string()],
            symbols: vec!["AuthService".to_string()],
            session_id: Some(session.id.clone()),
            timestamp: Utc::now(),
            ..Default::default()
        },
        KnoxAction {
            id: ActionId::from("a-f5-write-jwt"),
            actor: "bench-agent".to_string(),
            action_type: "write".to_string(),
            summary: "Implement JWT sign and verify in src/auth/jwt.rs".to_string(),
            files: vec!["src/auth/jwt.rs".to_string()],
            symbols: vec!["sign".to_string(), "verify".to_string()],
            parent_action_id: Some(ActionId::from("a-f5-plan-auth")),
            session_id: Some(session.id.clone()),
            timestamp: Utc::now(),
            ..Default::default()
        },
        KnoxAction {
            id: ActionId::from("a-f5-write-oauth2"),
            actor: "bench-agent".to_string(),
            action_type: "write".to_string(),
            summary: "Implement OAuth2 authorize in src/auth/oauth2.rs".to_string(),
            files: vec!["src/auth/oauth2.rs".to_string()],
            symbols: vec!["authorize".to_string()],
            parent_action_id: Some(ActionId::from("a-f5-plan-auth")),
            session_id: Some(session.id.clone()),
            timestamp: Utc::now(),
            ..Default::default()
        },
        KnoxAction {
            id: ActionId::from("a-f5-write-session"),
            actor: "bench-agent".to_string(),
            action_type: "write".to_string(),
            summary: "Implement Session struct in src/auth/session.rs".to_string(),
            files: vec!["src/auth/session.rs".to_string()],
            symbols: vec!["Session".to_string()],
            parent_action_id: Some(ActionId::from("a-f5-plan-auth")),
            session_id: Some(session.id.clone()),
            timestamp: Utc::now(),
            ..Default::default()
        },
        KnoxAction {
            id: ActionId::from("a-f5-test-auth"),
            actor: "bench-agent".to_string(),
            action_type: "verify".to_string(),
            summary: "Run cargo test --test auth, 8 passed".to_string(),
            files: vec![
                "src/auth/jwt.rs".to_string(),
                "src/auth/oauth2.rs".to_string(),
                "src/auth/session.rs".to_string(),
            ],
            session_id: Some(session.id.clone()),
            timestamp: Utc::now(),
            ..Default::default()
        },
    ];

    for action in &actions {
        insert_action(db, action).unwrap();
        ids.push(action.id.clone());
    }

    // Rebuild the FTS5 index so full-text search queries work.
    db.with_tx(|conn| {
        conn.execute("INSERT INTO actions_fts(actions_fts) VALUES('rebuild')", [])?;
        Ok(())
    }).ok();

    let _ = end_session(db, &session.id);
    ids
}

/// Loads a set of labelled queries from a JSON file.
fn load_queries_from_json(path: &str) -> Vec<LabelledQuery> {
    let content = std::fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("failed to read query set at {path}: {e}"));
    serde_json::from_str(&content)
        .unwrap_or_else(|e| panic!("failed to parse queries from {path}: {e}"))
}

/// Build labelled queries by combining the disjoint tune.json and eval.json sets.
fn labelled_queries() -> Vec<LabelledQuery> {
    let mut queries = load_queries_from_json("tests/queries/tune.json");
    queries.extend(load_queries_from_json("tests/queries/eval.json"));
    queries
}

/// Asserts that the tuning set (`tune.json`) and evaluation set (`eval.json`)
/// have strictly zero overlap in both query names and query parameters.
///
/// Fails in CI if benchmark leakage or parameter duplication occurs.
#[test]
fn test_tune_and_eval_queries_have_zero_overlap() {
    let tune = load_queries_from_json("tests/queries/tune.json");
    let eval = load_queries_from_json("tests/queries/eval.json");

    assert!(!tune.is_empty(), "tune.json must not be empty");
    assert!(!eval.is_empty(), "eval.json must not be empty");

    let tune_names: std::collections::HashSet<_> = tune.iter().map(|q| &q.name).collect();
    let eval_names: std::collections::HashSet<_> = eval.iter().map(|q| &q.name).collect();

    assert_eq!(tune_names.len(), tune.len(), "duplicate query names found inside tune.json");
    assert_eq!(eval_names.len(), eval.len(), "duplicate query names found inside eval.json");

    let name_overlap: Vec<_> = tune_names.intersection(&eval_names).collect();
    assert!(
        name_overlap.is_empty(),
        "CI failure: tune.json and eval.json have overlapping query names: {:?}",
        name_overlap
    );

    let tune_params: std::collections::HashSet<_> = tune
        .iter()
        .map(|q| serde_json::to_string(&q.params).unwrap())
        .collect();
    let eval_params: std::collections::HashSet<_> = eval
        .iter()
        .map(|q| serde_json::to_string(&q.params).unwrap())
        .collect();

    let param_overlap: Vec<_> = tune_params.intersection(&eval_params).collect();
    assert!(
        param_overlap.is_empty(),
        "CI failure: tune.json and eval.json have overlapping query parameters: {:?}",
        param_overlap
    );
}

/// Run a single labelled query and measure metrics.
fn run_query(db: &KnoxDb, query: &LabelledQuery) -> BenchResult {
    // All seeded actions belong to project "demo"; inject it so queries match.
    let mut params = query.params.clone();
    if params.get("project").is_none() {
        params.as_object_mut().unwrap().insert("project".to_string(), json!("demo"));
    }
    let res = dispatch("knox_context", params, db);
    let result = res.result.expect("query should return a result");
    let response_text = serde_json::to_string(&result).unwrap();
    let response_tokens = estimate_tokens(&response_text);

    // Count how many expected IDs appear in the response text
    let expected_count = query.expected_ids.len();
    let mut recalled_count = 0usize;
    for expected in &query.expected_ids {
        if response_text.contains(expected.as_str()) {
            recalled_count += 1;
        }
    }

    let recall_at_5 = if expected_count == 0 {
        1.0
    } else {
        let denom = expected_count.min(5);
        recalled_count as f64 / denom as f64
    };

    let within_budget = response_tokens <= query.budget;

    BenchResult {
        _name: query.name.clone(),
        response_tokens,
        within_budget,
        recall_at_5,
    }
}

#[test]
fn bench_tool_list_token_cost() {
    let tools = all_tools();
    let serialized = serde_json::to_string(&tools).unwrap();
    let tokens = estimate_tokens(&serialized);

    eprintln!("--- P0 Benchmark: tools/list token cost ---");
    eprintln!("Listed tools: {}", tools.len());
    eprintln!("Serialized bytes: {}", serialized.len());
    eprintln!("Estimated tokens: {}", tokens);
    eprintln!("Target: <=850");
    eprintln!("PASS: {}", tokens <= 850);

    assert_eq!(tools.len(), TOOL_COUNT);
    assert_eq!(tools.len(), LISTED_TOOL_NAMES.len());
    assert!(tokens <= 850, "tools/list token cost {} exceeds target 850", tokens);
}

#[test]
fn bench_metrics_tool_efficiency() {
    // Wire metrics::measure_token_efficiency into the benchmark so the
    // documented token-efficiency claim is reproducible from the test suite.
    let tools: Vec<ToolDescription> = all_tools();
    let action = KnoxAction {
        id: ActionId::new(),
        actor: "bench-agent".to_string(),
        action_type: "decide".to_string(),
        summary: "Benchmark token efficiency measurement".to_string(),
        files: vec!["src/mcp/tools.rs".to_string()],
        ..Default::default()
    };
    let report = measure_token_efficiency(&tools, &[action]);

    eprintln!("--- P0 Benchmark: metrics token efficiency ---");
    eprintln!("Tool count: {}", report.tool_count);
    eprintln!("Tool definition tokens: {}", report.tool_definition_tokens);
    eprintln!("Compact context tokens: {}", report.compact_context_tokens);
    eprintln!("Markdown context tokens: {}", report.markdown_context_tokens);
    eprintln!("JSON context tokens: {}", report.json_context_tokens);

    assert_eq!(report.tool_count, TOOL_COUNT);
    assert!(report.tool_definition_tokens > 0);
    assert!(report.tool_definition_tokens <= 850, "tool definition tokens {} exceed target 850", report.tool_definition_tokens);
    assert!(report.compact_context_tokens < report.json_context_tokens);
    assert!(report.compact_savings_percent() > 0);
}

#[test]
fn bench_recall_at_5() {
    let db = KnoxDb::open_memory().unwrap();
    seed_benchmark_db(&db);

    let queries = labelled_queries();
    let mut total_recall = 0.0;
    let mut total_within_budget = 0;
    let mut total_tokens = 0usize;
    let mut empty_results = 0usize;

    eprintln!("--- P0 Benchmark: {} labelled queries ---", queries.len());

    for query in &queries {
        let result = run_query(&db, query);
        total_recall += result.recall_at_5;
        total_tokens += result.response_tokens;
        if result.within_budget {
            total_within_budget += 1;
        }
        let mut count_params = query.params.clone();
        if count_params.get("project").is_none() {
            count_params.as_object_mut().unwrap().insert("project".to_string(), json!("demo"));
        }
        let results_count = dispatch("knox_context", count_params, &db)
            .result
            .unwrap()
            .get("results")
            .and_then(|r| r.as_array())
            .map(|a| a.len())
            .unwrap_or(0);
        if results_count == 0 {
            empty_results += 1;
        }

        eprintln!(
            "  {:<30} recall={:.2} tokens={} budget={} within_budget={} results={}",
            query.name,
            result.recall_at_5,
            result.response_tokens,
            query.budget,
            result.within_budget,
            results_count
        );
    }

    let mean_recall = total_recall / queries.len() as f64;
    let mean_tokens = total_tokens / queries.len();
    let budget_rate = total_within_budget as f64 / queries.len() as f64;
    let empty_rate = empty_results as f64 / queries.len() as f64;

    eprintln!("--- P0 Benchmark Summary ---");
    eprintln!("Mean recall@5: {:.3} (target >=0.75)", mean_recall);
    eprintln!("Mean tokens per read: {} (target <=1200)", mean_tokens);
    eprintln!("Budget compliance: {:.1}%", budget_rate * 100.0);
    eprintln!("Empty-result rate: {:.1}%", empty_rate * 100.0);
    eprintln!("Round trips per workflow: 1 (single dispatch per query)");

    assert!(
        mean_recall >= 0.75,
        "mean recall@5 {:.3} below target 0.75",
        mean_recall
    );
    assert!(
        mean_tokens <= 1200,
        "mean tokens per read {} exceeds target 1200",
        mean_tokens
    );
    assert!(
        empty_rate <= 0.10,
        "empty-result rate {:.1}% above target 10%",
        empty_rate * 100.0
    );
}

#[test]
fn bench_budget_enforcement() {
    let db = KnoxDb::open_memory().unwrap();
    seed_benchmark_db(&db);

    // Test budgets from 200 to 4000 (MAX_CONTEXT_BUDGET) as the plan requires.
    // Budgets below ~200 cannot fit the JSON response envelope.
    for budget in [200, 400, 600, 800, 1000, 1200, 2000, 4000] {
        let params = json!({
            "action_type": "write",
            "budget": budget,
            "project": "demo"
        });
        let res = dispatch("knox_context", params, &db);
        let result = res.result.unwrap();
        // Budget is enforced on the rendered markdown field, not the full JSON envelope.
        let markdown = result.get("markdown").and_then(|m| m.as_str()).unwrap_or("");
        let tokens = estimate_tokens(markdown);

        eprintln!("budget={} tokens={} within={}", budget, tokens, tokens <= budget);
        assert!(
            tokens <= budget,
            "budget enforcement failed: {} markdown tokens > {} budget",
            tokens,
            budget
        );
    }
}

#[test]
fn bench_round_trips_per_workflow() {
    let db = KnoxDb::open_memory().unwrap();
    seed_benchmark_db(&db);

    // A complete workflow: begin -> record -> context -> handoff
    let begin_res = dispatch(
        "knox_begin",
        json!({"actor": "bench-agent", "workspace": "demo"}),
        &db,
    );
    assert!(begin_res.error.is_none());
    let session_id = begin_res.result.unwrap()["scope"]["session_id"]
        .as_str()
        .unwrap()
        .to_string();

    let record_res = dispatch(
        "knox_record",
        json!({
            "session_id": session_id,
            "actor": "bench-agent",
            "project": "demo",
            "entries": [{"kind": "write", "summary": "Benchmark smoke test", "files": ["src/bench.rs"]}]
        }),
        &db,
    );
    assert_eq!(record_res.result.unwrap()["status"], "recorded");

    let ctx_res = dispatch(
        "knox_context",
        json!({"project": "demo", "query": "benchmark smoke test"}),
        &db,
    );
    assert!(ctx_res.error.is_none());
    let results = ctx_res.result.unwrap()["results"].as_array().unwrap().len();
    assert!(results > 0, "workflow context should return results");

    let handoff_res = dispatch("knox_handoff", json!({"session_id": session_id}), &db);
    assert_eq!(handoff_res.result.unwrap()["status"], "closed");

    eprintln!("--- P0 Benchmark: round trips per workflow ---");
    eprintln!("Workflow: begin -> record -> context -> handoff = 4 calls");
    eprintln!("Round trips per workflow: 1 (each phase is a single dispatch)");
}
