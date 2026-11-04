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
    let object = params
        .as_object()
        .ok_or_else(|| KnoxError::InvalidInput("trace params must be an object".to_string()))?;

    let budget = object
        .get("budget")
        .and_then(Value::as_u64)
        .unwrap_or(DEFAULT_TRACE_BUDGET as u64) as usize;

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
        return render_trace_budget(result, budget);
    }
    if let Some(file_path) = object.get("file_path").and_then(Value::as_str) {
        let result = crate::mcp::handlers::get_lineage::handle_get_lineage(
            db,
            json!({
                "file_path": file_path,
                "max_depth": object.get("depth").and_then(Value::as_u64).unwrap_or(10),
                // Default format is markdown so the response can be truncated to budget.
                "format": "markdown"
            }),
        )?;
        return render_trace_budget(result, budget);
    }
    Err(KnoxError::InvalidInput(
        "trace requires 'from' or 'file_path'".to_string(),
    ))
}

/// Renders a lineage handler result as budgeted Markdown.
fn render_trace_budget(result: Value, budget: usize) -> KnoxResult<Value> {
    let markdown = result
        .get("markdown")
        .and_then(Value::as_str)
        .unwrap_or("");
    let (md, budget_used, truncated) = truncate_to_budget(markdown, budget);

    Ok(json!({
        "target_id": result.get("target_id"),
        "depth": result.get("depth"),
        "chain": result.get("chain"),
        "lineage": result.get("lineage"),
        "markdown": md,
        "budget": budget,
        "budget_used": budget_used,
        "truncated": truncated,
        "estimate_tokens": estimate_tokens(markdown),
    }))
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
