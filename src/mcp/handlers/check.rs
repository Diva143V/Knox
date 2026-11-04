//! MCP tool handler for `knox_check`.
//!
//! Thin adapter: parses params, delegates to `cmd_check`, renders response.

use serde_json::Value;

use crate::core::error::KnoxResult;
use crate::storage::db::KnoxDb;

pub fn handle_check(db: &KnoxDb, params: Value) -> KnoxResult<Value> {
    crate::commands::check::cmd_check(db, params)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn adapts_intent_to_guardrail_proposal() {
        let db = KnoxDb::open_memory().unwrap();
        let result = handle_check(
            &db,
            json!({"files": ["src/parser.rs"], "intent": "change parser"}),
        )
        .unwrap();
        assert!(result.get("violations").is_some());
        assert_eq!(result["files"][0], "src/parser.rs");
    }
}
