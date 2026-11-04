//! MCP tool handler for `knox_trace`.
//!
//! Thin adapter: parses params, delegates to `cmd_trace`, renders response.

use serde_json::Value;

use crate::core::error::KnoxResult;
use crate::storage::db::KnoxDb;

pub fn handle_trace(db: &KnoxDb, params: Value) -> KnoxResult<Value> {
    crate::commands::trace::cmd_trace(db, params)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn rejects_trace_without_anchor() {
        let db = KnoxDb::open_memory().unwrap();
        assert!(handle_trace(&db, json!({})).is_err());
    }
}
