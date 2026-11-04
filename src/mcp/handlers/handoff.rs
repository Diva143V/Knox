//! MCP tool handler for `knox_handoff`.
//!
//! Thin adapter: parses params, delegates to `cmd_handoff`, renders response.

use serde_json::Value;

use crate::core::error::KnoxResult;
use crate::storage::db::KnoxDb;

pub fn handle_handoff(db: &KnoxDb, params: Value) -> KnoxResult<Value> {
    crate::commands::handoff::cmd_handoff(db, params)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn returns_digest_and_closes_session() {
        let db = KnoxDb::open_memory().unwrap();
        let session = crate::storage::sessions::start_session(&db, "gemini", Some("demo")).unwrap();
        let result = handle_handoff(
            &db,
            json!({"session_id": session.id.to_string(), "budget": 100}),
        )
        .unwrap();
        assert_eq!(result["status"], "closed");
        assert!(!crate::storage::sessions::get_session(&db, &session.id)
            .unwrap()
            .unwrap()
            .is_open);
    }
}
