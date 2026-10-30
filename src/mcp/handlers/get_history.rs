//! MCP tool handler for `knox_get_history`.

use serde_json::{Value, json};

use super::get_context::handle_get_context;
use crate::core::error::KnoxResult;
use crate::storage::db::KnoxDb;

/// Handles the `knox_get_history` tool invocation.
/// Returns full append-only history with `current_only` defaulting to `false`.
pub fn handle_get_history(db: &KnoxDb, mut params: Value) -> KnoxResult<Value> {
    if let Value::Object(ref mut map) = params {
        if !map.contains_key("current_only") {
            map.insert("current_only".to_string(), json!(false));
        }
    }
    handle_get_context(db, params)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::ids::ActionId;
    use crate::core::types::KnoxAction;
    use chrono::Utc;

    #[test]
    fn test_handle_get_history_includes_superseded() {
        let db = KnoxDb::open_memory().unwrap();
        let session = crate::storage::sessions::start_session(&db, "gemini", None).unwrap();

        let old_act = KnoxAction {
            id: ActionId::new(),
            actor: "gemini".to_string(),
            action_type: "write".to_string(),
            summary: "Old summary".to_string(),
            files: vec!["src/lib.rs".to_string()],
            tags: vec![],
            confidence: None,
            is_current: false,
            timestamp: Utc::now(),
            session_id: Some(session.id.clone()),
            parent_action_id: None,
            supersedes: None,
        };
        crate::storage::nodes::insert_action(&db, &old_act).unwrap();

        let new_act = KnoxAction {
            id: ActionId::new(),
            actor: "gemini".to_string(),
            action_type: "write".to_string(),
            summary: "New summary".to_string(),
            files: vec!["src/lib.rs".to_string()],
            tags: vec![],
            confidence: None,
            is_current: true,
            timestamp: Utc::now(),
            session_id: Some(session.id.clone()),
            parent_action_id: None,
            supersedes: None,
        };
        crate::storage::nodes::insert_action(&db, &new_act).unwrap();

        let res = handle_get_history(&db, json!({})).unwrap();
        let total = res.get("total_count").unwrap().as_u64().unwrap();
        assert_eq!(total, 2);
    }
}
