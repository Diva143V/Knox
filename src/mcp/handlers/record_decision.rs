//! MCP tool handler for `knox_record_decision`.

use serde_json::{Value, json};

use super::record_action::handle_record_action;
use crate::core::error::KnoxResult;
use crate::storage::db::KnoxDb;

/// Handles the `knox_record_decision` tool invocation.
/// Forces `action_type` to `"decide"` regardless of caller input.
pub fn handle_record_decision(db: &KnoxDb, mut params: Value) -> KnoxResult<Value> {
    if let Value::Object(ref mut map) = params {
        map.insert("action_type".to_string(), json!("decide"));
    }
    handle_record_action(db, params)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::ids::ActionId;

    #[test]
    fn test_handle_record_decision_forces_decide() {
        let db = KnoxDb::open_memory().unwrap();
        let session = crate::storage::sessions::start_session(&db, "gemini", None).unwrap();

        let params = json!({
            "session_id": session.id.as_str(),
            "actor": "gemini",
            "action_type": "write", // Should be overridden to "decide"
            "summary": "Decided to use nom parser"
        });

        let res = handle_record_decision(&db, params).unwrap();
        let action_id_str = res.get("id").unwrap().as_str().unwrap();
        let action_id = ActionId::from(action_id_str);

        let action = crate::storage::nodes::get_action(&db, &action_id)
            .unwrap()
            .expect("must find recorded decision");
        assert_eq!(action.action_type, "decide");
        assert_eq!(action.significance.as_deref(), Some("major"));
    }

    #[test]
    fn test_handle_record_minor_decision() {
        let db = KnoxDb::open_memory().unwrap();
        let session = crate::storage::sessions::start_session(&db, "gemini", None).unwrap();

        let params = json!({
            "session_id": session.id.as_str(),
            "actor": "gemini",
            "summary": "Decided on 8px padding for buttons",
            "significance": "minor"
        });

        let res = handle_record_decision(&db, params).unwrap();
        let action_id_str = res.get("id").unwrap().as_str().unwrap();
        let action_id = ActionId::from(action_id_str);

        let action = crate::storage::nodes::get_action(&db, &action_id)
            .unwrap()
            .expect("must find recorded decision");
        assert_eq!(action.action_type, "decide");
        assert_eq!(action.significance.as_deref(), Some("minor"));
    }
}
