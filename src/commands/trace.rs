//! Causal trace command.
//!
//! Business logic for `knox_trace`: resolves an anchor action by ID or file
//! path, then delegates to the lineage handler for ancestor traversal.

use serde_json::{Value, json};

use crate::core::error::{KnoxError, KnoxResult};
use crate::storage::db::KnoxDb;

/// Runs the causal trace for `knox_trace`.
///
/// Accepts either `from` (an action ID) or `file_path` as the anchor,
/// normalizes parameters for the lineage handler, and returns the causal
/// ancestry chain.
pub fn cmd_trace(db: &KnoxDb, params: Value) -> KnoxResult<Value> {
    let object = params
        .as_object()
        .ok_or_else(|| KnoxError::InvalidInput("trace params must be an object".to_string()))?;
    if let Some(from) = object.get("from").and_then(Value::as_str) {
        let mut lineage = params.clone();
        if let Value::Object(ref mut map) = lineage {
            map.insert("action_id".to_string(), Value::String(from.to_string()));
            map.remove("from");
            map.remove("to");
            if let Some(depth) = map.remove("depth") {
                map.insert("max_depth".to_string(), depth);
            }
            map.insert("format".to_string(), Value::String("json".to_string()));
        }
        return crate::mcp::handlers::get_lineage::handle_get_lineage(db, lineage);
    }
    if let Some(file_path) = object.get("file_path").and_then(Value::as_str) {
        return crate::mcp::handlers::get_lineage::handle_get_lineage(
            db,
            json!({
                "file_path": file_path,
                "max_depth": object.get("depth").and_then(Value::as_u64).unwrap_or(10),
                "format": "json"
            }),
        );
    }
    Err(KnoxError::InvalidInput(
        "trace requires 'from' or 'file_path'".to_string(),
    ))
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
}
