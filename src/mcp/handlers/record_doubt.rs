//! MCP tool handler for `knox_record_doubt`.

use serde_json::{Value, json};

use super::record_action::handle_record_action;
use crate::core::error::KnoxResult;
use crate::storage::db::KnoxDb;

/// Handles the `knox_record_doubt` tool invocation.
/// Forces `action_type` to `"doubt"` and `is_resolved` to `false`.
pub fn handle_record_doubt(db: &KnoxDb, mut params: Value) -> KnoxResult<Value> {
    if let Value::Object(ref mut map) = params {
        map.insert("action_type".to_string(), json!("doubt"));
        map.insert("is_resolved".to_string(), json!(false));
        if !map.contains_key("significance") {
            map.insert("significance".to_string(), json!("minor"));
        }
    }
    handle_record_action(db, params)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::ids::ActionId;

    #[test]
    fn test_handle_record_doubt_creates_unresolved_doubt() {
        let db = KnoxDb::open_memory().unwrap();
        let session = crate::storage::sessions::start_session(&db, "gemini", None).unwrap();

        let params = json!({
            "session_id": session.id.as_str(),
            "actor": "gemini",
            "summary": "Will SQLite handle multiple agent concurrent writes without timeout?",
            "context_summary": "Evaluating connection pooling strategy"
        });

        let res = handle_record_doubt(&db, params).unwrap();
        let action_id_str = res.get("id").unwrap().as_str().unwrap();
        let action_id = ActionId::from(action_id_str);

        let action = crate::storage::nodes::get_action(&db, &action_id)
            .unwrap()
            .expect("must find recorded doubt");
        assert_eq!(action.action_type, "doubt");
        assert_eq!(action.is_resolved, Some(false));
        assert_eq!(action.resolution_note, None);
    }
}

