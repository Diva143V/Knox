//! Guardrail pre-flight check command.
//!
//! Business logic for `knox_check`: validates target files, renames `intent`
//! to `proposed_action`, and delegates to the guardrail rule matcher with a
//! token budget (default 600).

use serde_json::{json, Value};

use crate::core::budget::{DEFAULT_CHECK_BUDGET, truncate_to_budget};
use crate::core::error::{KnoxError, KnoxResult};
use crate::storage::db::KnoxDb;

/// Runs the pre-flight guardrail check for `knox_check`.
///
/// Validates that `files` is a non-empty list, normalizes `intent` to
/// `proposed_action`, renders the guardrail report as Markdown, and truncates
/// it to the caller's token budget (default 600).
pub fn cmd_check(db: &KnoxDb, mut params: Value) -> KnoxResult<Value> {
    let start_instant = std::time::Instant::now();
    let budget = params
        .as_object()
        .and_then(|o| o.get("budget"))
        .and_then(Value::as_u64)
        .unwrap_or(DEFAULT_CHECK_BUDGET as u64) as usize;

    let project = params
        .get("project")
        .and_then(Value::as_str)
        .map(String::from);

    if let Value::Object(ref mut object) = params {
        let files = object
            .get("files")
            .and_then(Value::as_array)
            .ok_or_else(|| KnoxError::InvalidInput("check requires 'files'".to_string()))?;
        if files.is_empty()
            || files
                .iter()
                .any(|file| file.as_str().is_none_or(|path| path.trim().is_empty()))
        {
            return Err(KnoxError::InvalidInput(
                "check requires at least one non-empty file path".to_string(),
            ));
        }
        if let Some(intent) = object.remove("intent") {
            object
                .entry("proposed_action".to_string())
                .or_insert(intent);
        }
        // Default format is markdown so the response can be truncated to budget.
        object
            .entry("format".to_string())
            .or_insert_with(|| Value::String("markdown".to_string()));
    } else {
        return Err(KnoxError::InvalidInput(
            "check params must be an object".to_string(),
        ));
    }

    let result = crate::mcp::handlers::check_guardrails::handle_check_guardrails(db, params)?;

    let markdown = result
        .get("markdown")
        .and_then(Value::as_str)
        .unwrap_or("");
    let (md, budget_used, truncated) = truncate_to_budget(markdown, budget);

    let reason = result.get("reason").and_then(Value::as_str);

    let latency_ms = start_instant.elapsed().as_millis() as u64;
    crate::storage::record_query_log_best_effort(
        db,
        &crate::core::types::QueryLogEntry {
            id: format!("ql_{}", uuid::Uuid::new_v4().simple()),
            ts_ms: chrono::Utc::now().timestamp_millis(),
            tool: "knox_check".to_string(),
            project_id: project,
            rows: result.get("decision_count").and_then(Value::as_u64).unwrap_or(0) as usize,
            tokens: budget_used,
            latency_ms,
            reason: reason.map(String::from),
        },
    );

    let mut response = json!({
        "pass": result.get("pass"),
        "violations_count": result.get("violations_count"),
        "violations": result.get("violations"),
        "violation_details": result.get("violation_details"),
        "decision_count": result.get("decision_count"),
        "open_doubts_count": result.get("open_doubts_count"),
        "open_doubts": result.get("open_doubts"),
        "files": result.get("files"),
        "active_constraints_count": result.get("active_constraints_count"),
        "active_constraints": result.get("active_constraints"),
        "superseded_patterns": result.get("superseded_patterns"),
        "markdown": md,
        "budget": budget,
        "budget_used": budget_used,
        "truncated": truncated,
    });
    if let Some(r) = reason {
        response["reason"] = json!(r);
    }

    Ok(response)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn adapts_intent_to_guardrail_proposal() {
        let db = KnoxDb::open_memory().unwrap();
        let result = cmd_check(
            &db,
            json!({"files": ["src/parser.rs"], "intent": "change parser"}),
        )
        .unwrap();
        assert!(result.get("violations").is_some());
        assert_eq!(result["files"][0], "src/parser.rs");
        assert!(result.get("budget_used").is_some());
    }

    #[test]
    fn truncates_to_budget() {
        let db = KnoxDb::open_memory().unwrap();
        // A tiny budget should truncate the markdown.
        let result = cmd_check(
            &db,
            json!({"files": ["src/parser.rs"], "budget": 10}),
        )
        .unwrap();
        let budget_used: usize = result["budget_used"].as_u64().unwrap() as usize;
        assert!(budget_used <= 10);
    }
}
