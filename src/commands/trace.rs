//! Causal trace command.
//!
//! Business logic for `knox_trace`: resolves an anchor action by ID or file
//! path, then delegates to the lineage handler for ancestor traversal with a
//! token budget (default 800).

use serde_json::{Value, json};

use crate::core::budget::{DEFAULT_TRACE_BUDGET, estimate_tokens, truncate_to_budget};
use crate::core::error::{KnoxError, KnoxResult};
use crate::storage::db::KnoxDb;

/// Runs the causal trace for `knox_trace`.
///
/// Accepts either `from` (an action ID) or `file_path` as the anchor,
/// normalizes parameters for the lineage handler, renders the causal chain
/// as Markdown, and truncates it to the caller's token budget (default 800).
pub fn cmd_trace(db: &KnoxDb, params: Value) -> KnoxResult<Value> {
    let start_instant = std::time::Instant::now();
    let object = params
        .as_object()
        .ok_or_else(|| KnoxError::InvalidInput("trace params must be an object".to_string()))?;

    let budget = object
        .get("budget")
        .and_then(Value::as_u64)
        .unwrap_or(DEFAULT_TRACE_BUDGET as u64) as usize;

    let project = object
        .get("project")
        .and_then(Value::as_str)
        .map(String::from);

    if let Some(from) = object.get("from").and_then(Value::as_str) {
        let mut lineage = params.clone();
        if let Value::Object(ref mut map) = lineage {
            map.insert("action_id".to_string(), Value::String(from.to_string()));
            map.remove("from");
            map.remove("to");
            if let Some(depth) = map.remove("depth") {
                map.insert("max_depth".to_string(), depth);
            }
            // Default format is markdown so the response can be truncated to budget.
            map.entry("format".to_string())
                .or_insert_with(|| Value::String("markdown".to_string()));
        }
        let result = crate::mcp::handlers::get_lineage::handle_get_lineage(db, lineage)?;
        return render_trace_budget(db, result, budget, project, start_instant);
    }
    if let Some(file_path) = object.get("file_path").and_then(Value::as_str) {
        let max_nodes = object.get("max_nodes").and_then(Value::as_u64);
        let mut lineage_params = json!({
            "file_path": file_path,
            "max_depth": object.get("depth").and_then(Value::as_u64).unwrap_or(10),
            // Default format is markdown so the response can be truncated to budget.
            "format": object.get("format").and_then(Value::as_str).unwrap_or("markdown")
        });
        if let Some(mn) = max_nodes {
            lineage_params["max_nodes"] = json!(mn);
        }
        let result = crate::mcp::handlers::get_lineage::handle_get_lineage(db, lineage_params)?;
        return render_trace_budget(db, result, budget, project, start_instant);
    }
    Err(KnoxError::InvalidInput(
        "trace requires 'from' or 'file_path'".to_string(),
    ))
}

/// Renders a lineage handler result as budgeted Markdown and records telemetry.
fn render_trace_budget(
    db: &KnoxDb,
    result: Value,
    budget: usize,
    project: Option<String>,
    start_instant: std::time::Instant,
) -> KnoxResult<Value> {
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
            tool: "knox_trace".to_string(),
            project_id: project,
            rows: result.get("depth").and_then(Value::as_u64).unwrap_or(0) as usize,
            tokens: budget_used,
            latency_ms,
            reason: reason.map(String::from),
        },
    );

    let budget_truncated = truncated;
    let component_truncated = result.get("truncated").and_then(Value::as_bool).unwrap_or(false);
    let is_truncated = budget_truncated || component_truncated;

    let mut response = json!({
        "target_id": result.get("target_id"),
        "depth": result.get("depth"),
        "chain": result.get("chain"),
        "lineage": result.get("lineage"),
        "markdown": md,
        "budget": budget,
        "budget_used": budget_used,
        "truncated": is_truncated,
        "estimate_tokens": estimate_tokens(markdown),
    });
    if let Some(frontier) = result.get("frontier") {
        response["frontier"] = frontier.clone();
    }
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
    fn rejects_trace_without_anchor() {
        let db = KnoxDb::open_memory().unwrap();
        assert!(cmd_trace(&db, json!({})).is_err());
    }

    #[test]
    fn truncates_to_budget() {
        let db = KnoxDb::open_memory().unwrap();
        // Create an action to trace.
        let act = crate::core::types::KnoxAction {
            id: crate::core::ids::ActionId::new(),
            actor: "gemini".to_string(),
            action_type: "write".to_string(),
            summary: "Trace target action".to_string(),
            files: vec!["src/target.rs".to_string()],
            timestamp: chrono::Utc::now(),
            ..Default::default()
        };
        crate::storage::nodes::insert_action(&db, &act).unwrap();

        // A tiny budget should truncate the markdown.
        let result = cmd_trace(
            &db,
            json!({"from": act.id.as_str(), "budget": 10}),
        )
        .unwrap();
        let budget_used: usize = result["budget_used"].as_u64().unwrap() as usize;
        assert!(budget_used <= 10);
    }
}
