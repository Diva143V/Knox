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

#[test]
fn test_readme_tool_count_matches_implementation() {
    let readme = fs::read_to_string("README.md").expect("README.md must exist");
    let count = LISTED_TOOL_NAMES.len();
    assert_eq!(count, 8, "LISTED_TOOL_NAMES must contain exactly 8 tools");
    assert!(
        readme.contains(&format!("{count} Tools")),
        "README.md should mention '{count} Tools'"
    );
    assert!(
        readme.contains(&format!("TOOL_COUNT = {count}")),
        "README.md should mention 'TOOL_COUNT = {count}'"
    );
    assert!(
        !readme.contains("6 Tools"),
        "README.md must not retain stale '6 Tools' reference"
    );
    assert!(
        !readme.contains("7 Tools"),
        "README.md must not retain stale '7 Tools' reference"
    );
    assert!(
        !readme.contains("exposes 6 tools"),
        "README.md must not retain stale 'exposes 6 tools' reference"
    );
}

#[test]
fn test_readme_knowledge_graph_four_questions_and_retired_tools_absence() {
    let readme = fs::read_to_string("README.md").expect("README.md must exist");

    // 1. Assert the 4 grounded knowledge graph questions are documented
    const FOUR_QUESTIONS: &[&str] = &[
        "Why does this exist?",
        "What breaks if I change this?",
        "What contradicts what?",
        "What decisions touch this file?",
    ];
    for q in FOUR_QUESTIONS {
        assert!(
            readme.contains(q),
            "README.md must explicitly define the graph question: '{q}'"
        );
    }

    // 2. Assert retired tools are NEVER documented as tool headers in README.md
    const RETIRED_TOOL_HEADERS: &[&str] = &[
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
    for retired in RETIRED_TOOL_HEADERS {
        assert!(
            !readme.contains(&format!("### `{retired}`")) && !readme.contains(&format!("### 1. `{retired}`")),
            "README.md must not retain header documentation for retired tool: '{retired}'"
        );
    }

    // 3. Assert the 7 active tools are documented under MCP Tools Reference
    for active in LISTED_TOOL_NAMES {
        assert!(
            readme.contains(&format!("`{active}`")),
            "README.md must document active MCP tool '{active}'"
        );
    }
}

#[test]
fn test_embeddings_linear_scan_ceiling_and_hnsw_trigger_documented() {
    let embeddings_src = fs::read_to_string("src/storage/embeddings.rs")
        .expect("src/storage/embeddings.rs must exist");

    // 1. Verify memory allocation calculations are explicitly documented
    assert!(
        embeddings_src.contains("384 dimensions"),
        "embeddings.rs must document 384 dimensions"
    );
    assert!(
        embeddings_src.contains("15.36 MB"),
        "embeddings.rs must document ~15.36 MB allocation at 10k"
    );
    assert!(
        embeddings_src.contains("76.8 MB"),
        "embeddings.rs must document ~76.8 MB allocation at 50k"
    );
    assert!(
        embeddings_src.contains("768 MB"),
        "embeddings.rs must document ~768 MB allocation at 500k"
    );

    // 2. Verify project isolation scoping is documented and implemented
    assert!(
        embeddings_src.contains("project_id"),
        "embeddings.rs must document project_id scoping"
    );
    assert!(
        embeddings_src.contains("JOIN actions a ON a.id = e.action_id WHERE a.project_id = ?1"),
        "embeddings.rs must implement project-scoped JOIN query"
    );

    // 3. Verify index trigger criteria and engine recommendations
    assert!(
        embeddings_src.contains("vector_count > 50,000"),
        "embeddings.rs must document > 50,000 vector count trigger"
    );
    assert!(
        embeddings_src.contains("p95 semantic search latency > 50ms"),
        "embeddings.rs must document > 50ms p95 latency trigger"
    );
    assert!(
        embeddings_src.contains("instant-distance") || embeddings_src.contains("hnsw_rs"),
        "embeddings.rs must document pure-Rust HNSW target index"
    );
    assert!(
        embeddings_src.contains("sqlite-vss"),
        "embeddings.rs must document why sqlite-vss was rejected"
    );
    assert!(
        embeddings_src.contains("usearch"),
        "embeddings.rs must document why usearch was rejected"
    );
}

#[test]
fn test_knox_md_template_tool_census_and_numbering() {
    let knox_md = render_knox_md("test-proj", "~/.knox/knox.db", "2026-11-05T00:00:00Z");
    assert!(
        knox_md.contains("Knox exposes 8 tools, 3 resources, and 2 prompts"),
        "knox.md must state '8 tools'"
    );
    assert!(
        knox_md.contains("### 3.7 `knox_ask`"),
        "knox.md must document knox_ask under 3.7"
    );
    assert!(
        knox_md.contains("### 3.8 `knox_update`"),
        "knox.md must document knox_update under 3.8"
    );
    assert!(
        knox_md.contains("### 3.9 Native MCP Resources"),
        "knox.md must number resources as 3.9"
    );
    assert!(
        knox_md.contains("### 3.10 Native MCP Prompts"),
        "knox.md must number prompts as 3.10"
    );
    assert!(
        !knox_md.contains("### 3.20"),
        "knox.md must not have broken section numbering 3.20"
    );
}
