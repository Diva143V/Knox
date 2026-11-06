//! Workstream A acceptance tests: honest vocabulary, tool alignment, and claims verification.

use std::fs;
use serde_json::json;
use knox::core::types::KnoxAction;
use knox::core::ids::ActionId;
use knox::storage::db::KnoxDb;
use knox::mcp::tools::LISTED_TOOL_NAMES;
use knox::mcp::format::format_markdown_digest;
use knox::commands::init::render_knox_md;
use knox::commands::check::cmd_check;
use knox::commands::context::cmd_context;

#[test]
fn test_check_response_has_no_confidence_field() {
    let db = KnoxDb::open_memory().unwrap();

    // Record an active decision with a rejected alternative
    let dec = KnoxAction {
        id: ActionId::new(),
        actor: "security-auditor".to_string(),
        action_type: "decide".to_string(),
        summary: "Mandate AES-256-GCM for encrypted records".to_string(),
        rationale: Some("Compliance requirement".to_string()),
        rejected_alternatives: vec!["ChaCha20-Poly1305 without hardware acceleration".to_string()],
        files: vec!["src/crypto.rs".to_string()],
        timestamp: chrono::Utc::now(),
        is_current: true,
        ..Default::default()
    };
    knox::storage::nodes::insert_action(&db, &dec).unwrap();

    let check_res = cmd_check(
        &db,
        json!({
            "files": ["src/crypto.rs"],
            "intent": "Use ChaCha20-Poly1305 without hardware acceleration for quick encryption"
        }),
    )
    .unwrap();

    assert_eq!(check_res["pass"], false);
    assert_eq!(check_res["violations_count"], 1);

    // Verify top-level response has no confidence field
    assert!(check_res.get("confidence").is_none(), "Response must not have a top-level confidence field");

    // Verify violation_details has overlap and NOT confidence
    let details = check_res["violation_details"]
        .as_array()
        .expect("violation_details must be an array");
    assert!(!details.is_empty());
    for v in details {
        assert!(v.get("overlap").is_some(), "Violation missing 'overlap' field: {v:?}");
        let overlap = v["overlap"].as_f64().expect("overlap must be a float");
        assert!(overlap > 0.0 && overlap <= 1.0);
        assert!(v.get("confidence").is_none(), "Violation must NOT contain 'confidence' field: {v:?}");
    }
}

#[test]
fn test_grouped_context_heading_and_mode() {
    let db = KnoxDb::open_memory().unwrap();
    let project = knox::core::types::Project {
        id: knox::core::ids::ProjectId::new(),
        name: "test-proj".to_string(),
        path: "/workspace/test".to_string(),
        remote: None,
        created_at: chrono::Utc::now(),
    };
    let proj_id = knox::storage::nodes::insert_project(&db, &project).unwrap();

    let action = KnoxAction {
        id: ActionId::new(),
        actor: "gemini".to_string(),
        project_id: Some(proj_id),
        action_type: "decide".to_string(),
        summary: "Use rusqlite bundled feature".to_string(),
        rationale: Some("Self contained binary".to_string()),
        files: vec!["Cargo.toml".to_string()],
        timestamp: chrono::Utc::now(),
        is_current: true,
        ..Default::default()
    };
    knox::storage::nodes::insert_action(&db, &action).unwrap();

    // 1. Direct markdown formatting snapshot test
    let md = format_markdown_digest("Context", &[action], true);
    assert!(
        md.starts_with("### Context — Grouped Context\n\n"),
        "Grouped digest header was: {:?}",
        md.lines().next()
    );
    assert!(!md.contains("Synthesized Context"), "Heading must not contain 'Synthesized Context'");

    // 2. cmd_context mode verification
    let ctx_res = cmd_context(
        &db,
        json!({
            "project": "test-proj",
            "file_path": "Cargo.toml",
            "synthesis": true
        }),
    )
    .unwrap();
    assert_eq!(ctx_res["mode"], "grouped");
    let ctx_md = ctx_res["markdown"].as_str().unwrap();
    assert!(
        ctx_md.contains(" — Grouped Context\n\n"),
        "Context markdown should contain grouped heading: {ctx_md}"
    );
    assert!(!ctx_md.contains("Synthesized Context"));
}

#[test]
fn test_readme_has_no_bare_percentages_in_prose() {
    let readme = fs::read_to_string("README.md").expect("README.md must exist");

    // Must not contain the ungrounded 70% claim
    assert!(!readme.contains("70%"), "README.md must not contain the 70% claim");

    // Scan lines to ensure no bare prose percentages exist
    let mut in_code_block = false;
    for (line_num, line) in readme.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.starts_with("```") {
            in_code_block = !in_code_block;
            continue;
        }
        if in_code_block {
            continue;
        }

        // Skip image badges like rust-1.85%2B-orange.svg
        if trimmed.contains("img.shields.io") || trimmed.contains("badge/") {
            continue;
        }

        // Skip Windows environment variables like %LOCALAPPDATA%
        let scrubbed_env = line.replace("%LOCALAPPDATA%", "");

        // Find matches of \b\d+%\b
        for word in scrubbed_env.split_whitespace() {
            let clean = word.trim_matches(|c: char| !c.is_alphanumeric() && c != '%');
            if let Some(num_part) = clean.strip_suffix('%') {
                if num_part.chars().all(|c| c.is_ascii_digit()) && !num_part.is_empty() {
                    panic!(
                        "Found bare prose percentage '{clean}' on README.md line {}: {line}",
                        line_num + 1
                    );
                }
            }
        }
    }
}

#[test]
fn test_instructions_and_tool_list_agree() {
    let knox_md = render_knox_md("test-proj", "~/.knox/knox.db", "2026-11-05T00:00:00Z");

    // Check knox.md tools_registered
    for tool_name in LISTED_TOOL_NAMES {
        assert!(
            knox_md.contains(&format!("- {tool_name}")),
            "knox.md tools_registered missing {tool_name}"
        );
    }

    // Verify retired tools are never present in knox.md
    const RETIRED_TOOLS: &[&str] = &[
        "knox_start_session",
        "knox_end_session",
        "knox_record_action",
        "knox_record_decision",
        "knox_record_doubt",
        "knox_resolve_doubt",
        "knox_record_batch",
        "knox_update_action",
        "knox_get_context",
        "knox_get_history",
        "knox_get_lineage",
        "knox_get_impact",
        "knox_get_path",
        "knox_check_guardrails",
        "knox_search",
        "knox_list_stale",
        "knox_scan_staged",
        "knox_diff_inspect",
        "knox_blame_causal",
        "knox_verify_drift",
    ];

    for retired in RETIRED_TOOLS {
        assert!(
            !knox_md.contains(&format!("- {retired}\n")),
            "knox.md must not register retired tool: {retired}"
        );
    }

    // Verify SERVER_INSTRUCTIONS
    let init_res = knox::mcp::handlers::dispatch(
        "initialize",
        json!({
            "protocolVersion": "2025-06-18",
            "capabilities": {}
        }),
        &KnoxDb::open_memory().unwrap(),
    );
    let instructions = init_res.result.unwrap()["instructions"]
        .as_str()
        .expect("instructions string")
        .to_string();

    for tool_name in LISTED_TOOL_NAMES {
        assert!(
            instructions.contains(tool_name),
            "SERVER_INSTRUCTIONS missing mention of {tool_name}"
        );
    }
    for retired in RETIRED_TOOLS {
        assert!(
            !instructions.contains(retired),
            "SERVER_INSTRUCTIONS must not mention retired tool: {retired}"
        );
    }
}
