//! Guardrail pre-flight check command.
//!
//! Business logic for `knox_check`: validates target files, renames `intent`
//! to `proposed_action`, and delegates to the guardrail rule matcher.

use serde_json::Value;

use crate::core::error::{KnoxError, KnoxResult};
use crate::storage::db::KnoxDb;

/// Runs the pre-flight guardrail check for `knox_check`.
///
/// Validates that `files` is a non-empty list, normalizes `intent` to
/// `proposed_action`, forces `format` to `json`, then delegates to the
/// guardrail rule matcher.
pub fn cmd_check(db: &KnoxDb, mut params: Value) -> KnoxResult<Value> {
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
        object
            .entry("format".to_string())
            .or_insert_with(|| Value::String("json".to_string()));
    } else {
        return Err(KnoxError::InvalidInput(
            "check params must be an object".to_string(),
        ));
    }
    crate::mcp::handlers::check_guardrails::handle_check_guardrails(db, params)
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
    }
}
