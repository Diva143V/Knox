//! Workstream C acceptance tests: trust surface, empty reason feedback, telemetry privacy, and doctor diagnostics.

use chrono::{Duration as ChronoDuration, Utc};
use serde_json::json;

use knox::commands::begin::cmd_begin;
use knox::commands::check::cmd_check;
use knox::commands::context::cmd_context;
use knox::commands::doctor::{cmd_doctor, format_doctor_report};
use knox::commands::handoff::cmd_handoff;
use knox::commands::query::cmd_query;
use knox::commands::trace::cmd_trace;
use knox::core::ids::{ActionId, ProjectId};
use knox::core::types::KnoxAction;
use knox::storage::db::KnoxDb;
use knox::storage::nodes::insert_action;
use knox::storage::sessions::start_session;

#[test]
fn test_c1_empty_reasons_on_read_tools() {
    let db = KnoxDb::open_memory().unwrap();

    // Register a valid project "registered_proj" with zero actions
    db.with_tx(|conn| {
        conn.execute(
            "INSERT INTO projects (id, name, path, created_at) VALUES (?1, ?1, ?1, ?2)",
            rusqlite::params!["registered_proj", Utc::now().to_rfc3339()],
        )?;
        Ok(())
    })
    .unwrap();

    // 1. knox_context empty reasons
    // 1a: out_of_scope (unregistered project)
    let ctx_out = cmd_context(
        &db,
        json!({
            "project": "unregistered_proj_xyz",
            "format": "json"
        }),
    )
    .unwrap();
    assert_eq!(ctx_out.get("reason").and_then(|r| r.as_str()), Some("out_of_scope"));

    // 1b: no_records (registered project with 0 actions)
    let ctx_no_rec = cmd_context(
        &db,
        json!({
            "project": "registered_proj",
            "format": "json"
        }),
    )
    .unwrap();
    assert_eq!(ctx_no_rec.get("reason").and_then(|r| r.as_str()), Some("no_records"));

    // 1c: out_of_scope via path traversal
    let ctx_trav = cmd_context(
        &db,
        json!({
            "project": "demo",
            "file_path": "../outside/secret.rs",
            "format": "json"
        }),
    )
    .unwrap();
    assert_eq!(ctx_trav.get("reason").and_then(|r| r.as_str()), Some("out_of_scope"));

    // Now insert an action for "registered_proj"
    let act = KnoxAction {
        id: ActionId::new(),
        actor: "developer".to_string(),
        action_type: "write".to_string(),
        summary: "Setup core database types".to_string(),
        files: vec!["src/core.rs".to_string()],
        tags: vec!["core".to_string()],
        project_id: Some(ProjectId::from("registered_proj")),
        timestamp: Utc::now(),
        is_current: true,
        ..Default::default()
    };
    insert_action(&db, &act).unwrap();

    // 1d: no_match (actions exist, but filter doesn't match)
    let ctx_no_match = cmd_context(
        &db,
        json!({
            "project": "registered_proj",
            "file_path": "src/missing.rs",
            "format": "json"
        }),
    )
    .unwrap();
    assert_eq!(ctx_no_match.get("reason").and_then(|r| r.as_str()), Some("no_match"));

    // 2. knox_check empty reasons
    // 2a: out_of_scope via upward path
    let check_out = cmd_check(
        &db,
        json!({
            "files": ["../../sensitive.rs"],
            "project": "registered_proj"
        }),
    )
    .unwrap();
    assert_eq!(check_out.get("reason").and_then(|r| r.as_str()), Some("out_of_scope"));

    // 2b: no_records on project with 0 actions
    let check_no_rec = cmd_check(
        &db,
        json!({
            "files": ["src/main.rs"],
            "project": "demo"
        }),
    )
    .unwrap();
    assert_eq!(check_no_rec.get("reason").and_then(|r| r.as_str()), Some("no_records"));

    // 2c: no_match on project with actions but no matching decisions
    let check_no_match = cmd_check(
        &db,
        json!({
            "files": ["src/core.rs"],
            "project": "registered_proj"
        }),
    )
    .unwrap();
    assert_eq!(check_no_match.get("reason").and_then(|r| r.as_str()), Some("no_match"));

    // 3. knox_trace empty reasons
    // 3a: out_of_scope via path
    let trace_out = cmd_trace(&db, json!({ "file_path": "../outside.rs" })).unwrap();
    assert_eq!(trace_out.get("reason").and_then(|r| r.as_str()), Some("out_of_scope"));

    // 3b: no_match on nonexistent file (since db has actions)
    let trace_no_match = cmd_trace(&db, json!({ "file_path": "src/unknown.rs" })).unwrap();
    assert_eq!(trace_no_match.get("reason").and_then(|r| r.as_str()), Some("no_match"));

    // 4. knox_begin empty reasons
    // 4a: out_of_scope
    let begin_out = cmd_begin(
        &db,
        json!({
            "actor": "claude",
            "workspace": "../unregistered_corp"
        }),
    )
    .unwrap();
    assert_eq!(begin_out.get("reason").and_then(|r| r.as_str()), Some("out_of_scope"));

    // 4b: no_records
    let begin_no_rec = cmd_begin(
        &db,
        json!({
            "actor": "claude",
            "workspace": "demo"
        }),
    )
    .unwrap();
    assert_eq!(begin_no_rec.get("reason").and_then(|r| r.as_str()), Some("no_records"));

    // 4c: no_match
    let begin_no_match = cmd_begin(
        &db,
        json!({
            "actor": "claude",
            "workspace": "registered_proj",
            "task": "completely unrelated search term 98765"
        }),
    )
    .unwrap();
    assert_eq!(begin_no_match.get("reason").and_then(|r| r.as_str()), Some("no_match"));

    // 5. knox_query empty reasons
    // 5a: out_of_scope
    let q_out = cmd_query("project:unregistered_xyz", &db, None).unwrap();
    assert_eq!(q_out.reason.as_deref(), Some("out_of_scope"));

    // 5b: no_records
    let q_no_rec = cmd_query("project:demo", &db, None).unwrap();
    assert_eq!(q_no_rec.reason.as_deref(), Some("no_records"));

    // 5c: no_match
    let q_no_match = cmd_query("tag:nonexistent_tag_xyz", &db, None).unwrap();
    assert_eq!(q_no_match.reason.as_deref(), Some("no_match"));

    // 6. knox_handoff empty reasons
    let sess = start_session(&db, "gemini", Some("demo")).unwrap();
    let handoff_res = cmd_handoff(&db, json!({ "session_id": sess.id.to_string() })).unwrap();
    assert_eq!(handoff_res.get("reason").and_then(|r| r.as_str()), Some("no_records"));
}

#[test]
fn test_c2_telemetry_query_log_privacy_and_recording() {
    let db = KnoxDb::open_memory().unwrap();

    // Verify database schema of query_log table: strictly no text/search/path columns
    let columns: Vec<String> = db
        .with_tx(|conn| {
            let mut stmt = conn.prepare("PRAGMA table_info(query_log)")?;
            let names = stmt
                .query_map([], |row| row.get::<_, String>(1))?
                .collect::<Result<Vec<_>, _>>()?;
            Ok(names)
        })
        .unwrap();

    let expected_columns = vec![
        "id".to_string(),
        "ts_ms".to_string(),
        "tool".to_string(),
        "project_id".to_string(),
        "rows".to_string(),
        "tokens".to_string(),
        "latency_ms".to_string(),
        "reason".to_string(),
    ];
    assert_eq!(columns, expected_columns);

    // Verify privacy: assert NO leaked column names exist
    let prohibited_columns = ["query", "pattern", "search", "keyword", "file_path", "path", "text", "prompt"];
    for col in &columns {
        for forbidden in &prohibited_columns {
            assert_ne!(
                col.to_lowercase(),
                *forbidden,
                "Query log schema violates privacy by containing column '{col}'"
            );
        }
    }

    // Execute read operations to populate query_log
    let _ = cmd_context(
        &db,
        json!({
            "project": "demo",
            "query": "confidential user password query",
            "file_path": "confidential/keys.pem"
        }),
    );
    let _ = cmd_check(
        &db,
        json!({
            "files": ["src/main.rs"],
            "project": "demo"
        }),
    );
    let _ = cmd_trace(&db, json!({ "file_path": "src/main.rs" }));

    // Inspect the recorded query_log rows
    type QueryLogRow = (String, String, Option<String>, i64, Option<String>);
    let log_rows: Vec<QueryLogRow> = db
        .with_tx(|conn| {
            let mut stmt = conn.prepare(
                "SELECT id, tool, project_id, rows, reason FROM query_log ORDER BY ts_ms ASC",
            )?;
            let rows = stmt
                .query_map([], |r| {
                    Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?))
                })?
                .collect::<Result<Vec<_>, _>>()?;
            Ok(rows)
        })
        .unwrap();

    assert!(
        log_rows.len() >= 3,
        "Expected at least 3 query log entries, found {}",
        log_rows.len()
    );

    let tools: Vec<&str> = log_rows.iter().map(|r| r.1.as_str()).collect();
    assert!(tools.contains(&"knox_context"));
    assert!(tools.contains(&"knox_check"));
    assert!(tools.contains(&"knox_trace"));
}

#[test]
fn test_c3_doctor_diagnostics_and_report() {
    let db = KnoxDb::open_memory().unwrap();

    // 0. Ensure project "demo" exists
    db.with_tx(|conn| {
        conn.execute(
            "INSERT OR IGNORE INTO projects (id, name, path, created_at) VALUES ('demo', 'demo', 'demo', ?1)",
            rusqlite::params![Utc::now().to_rfc3339()],
        )?;
        Ok(())
    })
    .unwrap();

    // 1. Populate query_log with some metrics
    let ql_entry1 = knox::core::types::QueryLogEntry {
        id: "ql_test_1".to_string(),
        ts_ms: Utc::now().timestamp_millis(),
        tool: "knox_context".to_string(),
        project_id: Some("demo".to_string()),
        rows: 5,
        tokens: 300,
        latency_ms: 10,
        reason: None,
    };
    let ql_entry2 = knox::core::types::QueryLogEntry {
        id: "ql_test_2".to_string(),
        ts_ms: Utc::now().timestamp_millis(),
        tool: "knox_context".to_string(),
        project_id: Some("demo".to_string()),
        rows: 0,
        tokens: 40,
        latency_ms: 5,
        reason: Some("no_match".to_string()),
    };
    let ql_entry3 = knox::core::types::QueryLogEntry {
        id: "ql_test_3".to_string(),
        ts_ms: Utc::now().timestamp_millis(),
        tool: "knox_context".to_string(),
        project_id: Some("demo".to_string()),
        rows: 0,
        tokens: 30,
        latency_ms: 4,
        reason: Some("no_records".to_string()),
    };
    knox::storage::record_query_log(&db, &ql_entry1).unwrap();
    knox::storage::record_query_log(&db, &ql_entry2).unwrap();
    knox::storage::record_query_log(&db, &ql_entry3).unwrap();

    // 2. Insert a rejected edge
    db.with_tx(|conn| {
        conn.execute(
            "INSERT INTO edge_rejections (id, ts, project_id, from_id, to_id, attempted_type, reason)
             VALUES ('er_1', ?1, 'demo', 'act_1', 'act_2', 'unsupported_edge', 'Invalid edge type')",
            rusqlite::params![Utc::now().to_rfc3339()],
        )?;
        Ok(())
    })
    .unwrap();

    // 3. Insert a zombie session (> 24 hours inactive)
    let stale_time = (Utc::now() - ChronoDuration::hours(36)).to_rfc3339();
    db.with_tx(|conn| {
        conn.execute(
            "INSERT INTO sessions (id, actor, project_id, started_at, is_open)
             VALUES ('sess_zombie', 'abandoned_agent', 'demo', ?1, 1)",
            rusqlite::params![stale_time],
        )?;
        Ok(())
    })
    .unwrap();

    // Run doctor command
    let report = cmd_doctor(&db, Some("demo")).unwrap();

    assert_eq!(report.integrity_check, "ok");
    assert_eq!(report.schema_version, 11);
    assert_eq!(report.zombie_sessions, 1);
    assert_eq!(report.empty_breakdown.no_match, 1);
    assert_eq!(report.empty_breakdown.no_records, 1);
    assert_eq!(report.empty_breakdown.out_of_scope, 0);
    assert!((report.empty_rate - (2.0 / 3.0)).abs() < 0.01);

    // Verify top rejected edge types
    assert!(!report.top_rejected_edge_types.is_empty());
    assert_eq!(report.top_rejected_edge_types[0].0, "unsupported_edge");
    assert_eq!(report.top_rejected_edge_types[0].1, 1);

    // Verify page stats
    assert!(report.page_count > 0);
    assert!(report.page_size > 0);
    assert!(report.db_size_bytes > 0);

    // Verify formatted text output
    let text = format_doctor_report(&report);
    assert!(text.contains("=== Knox Health & Diagnostics Report ==="));
    assert!(text.contains("SQLite Integrity Check : ok"));
    assert!(text.contains("Schema Version         : v11"));
    assert!(text.contains("Zombie Sessions (>24h) : 1"));
    assert!(text.contains("unsupported_edge"));
    assert!(text.contains("Query Empty Rate       : 66.67%"));
}

#[test]
fn test_c5_project_name_vs_uuid_resolution() {
    let db = KnoxDb::open_memory().unwrap();

    let project_uuid = "proj_uuid_8888";
    let project_name = "payment_service";
    let project_path = "/work/payment_service";

    db.with_tx(|conn| {
        conn.execute(
            "INSERT INTO projects (id, name, path, created_at) VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params![project_uuid, project_name, project_path, Utc::now().to_rfc3339()],
        )?;
        Ok(())
    })
    .unwrap();

    // 1. Initially 0 actions: querying by project name should give "no_records"
    let empty_ctx = cmd_context(
        &db,
        json!({
            "project": project_name,
            "format": "json"
        }),
    )
    .unwrap();
    assert_eq!(empty_ctx.get("reason").and_then(|r| r.as_str()), Some("no_records"));

    // 2. Insert action referencing project UUID
    let act = KnoxAction {
        id: ActionId::new(),
        actor: "copilot".to_string(),
        action_type: "decide".to_string(),
        summary: "Use Stripe API v2024 for payments".to_string(),
        files: vec!["src/stripe.rs".to_string()],
        project_id: Some(ProjectId::from(project_uuid)),
        timestamp: Utc::now(),
        is_current: true,
        ..Default::default()
    };
    insert_action(&db, &act).unwrap();

    // 3. Querying by project name for nonexistent file must return "no_match", NOT "no_records"
    let no_match_ctx = cmd_context(
        &db,
        json!({
            "project": project_name,
            "file_path": "src/paypal.rs",
            "format": "json"
        }),
    )
    .unwrap();
    assert_eq!(no_match_ctx.get("reason").and_then(|r| r.as_str()), Some("no_match"));

    // 4. Querying by project path for nonexistent file must also return "no_match"
    let path_no_match = cmd_context(
        &db,
        json!({
            "project": project_path,
            "file_path": "src/paypal.rs",
            "format": "json"
        }),
    )
    .unwrap();
    assert_eq!(path_no_match.get("reason").and_then(|r| r.as_str()), Some("no_match"));

    // 5. Querying by unregistered project must return "out_of_scope"
    let oos_ctx = cmd_context(
        &db,
        json!({
            "project": "unregistered_other_app",
            "format": "json"
        }),
    )
    .unwrap();
    assert_eq!(oos_ctx.get("reason").and_then(|r| r.as_str()), Some("out_of_scope"));

    // 6. Check doctor report when queried by project name
    let doc_report = cmd_doctor(&db, Some(project_name)).unwrap();
    assert!(doc_report.bound_models.contains(&"copilot".to_string()));
}


