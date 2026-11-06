//! Grounded question answering command for Knox CLI and MCP.

use serde_json::Value;

use crate::core::error::KnoxResult;
use crate::storage::db::KnoxDb;

/// Executes `knox ask`, synthesizing grounded answers across project memory.
pub fn cmd_ask(db: &KnoxDb, params: Value) -> KnoxResult<Value> {
    crate::mcp::handlers::ask::handle_ask(db, params)
}

/// Formats the result of `knox ask` for human-readable CLI display.
pub fn format_ask_response(val: &Value) -> String {
    let mut out = String::new();
    if let Some(answer) = val.get("answer").and_then(Value::as_str) {
        out.push_str(answer);
        out.push('\n');
    }
    if let Some(citations) = val.get("citations").and_then(Value::as_array) {
        if !citations.is_empty() {
            out.push_str("\nCitations:\n");
            for cit in citations {
                let id = cit.get("action_id").and_then(Value::as_str).unwrap_or("");
                let summary = cit.get("summary").and_then(Value::as_str).unwrap_or("");
                let file = cit.get("file_path").and_then(Value::as_str).unwrap_or("");
                if !file.is_empty() {
                    out.push_str(&format!("  * [{id}] {summary} ({file})\n"));
                } else {
                    out.push_str(&format!("  * [{id}] {summary}\n"));
                }
            }
        }
    }
    if let Some(model) = val.get("model").and_then(Value::as_str) {
        if !model.is_empty() {
            out.push_str(&format!("\n[Model: {model}]\n"));
        }
    }
    out
}

