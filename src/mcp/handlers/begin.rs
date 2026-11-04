//! MCP tool handler for `knox_begin`.
//!
//! Thin adapter: parses params, delegates to `cmd_begin`, renders response.

use serde_json::Value;

use crate::core::error::KnoxResult;
use crate::storage::db::KnoxDb;

pub fn handle_begin(db: &KnoxDb, params: Value) -> KnoxResult<Value> {
    crate::commands::begin::cmd_begin(db, params)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn resumes_same_open_actor_and_project_session() {
        let db = KnoxDb::open_memory().unwrap();
        let first = handle_begin(
            &db,
            json!({"actor": "gemini", "workspace": "knox", "task": "inspect storage"}),
        )
        .unwrap();
        let second = handle_begin(
            &db,
            json!({"actor": "gemini", "workspace": "knox", "task": "inspect storage"}),
        )
        .unwrap();
        assert_eq!(first["scope"]["session_id"], second["scope"]["session_id"]);
        assert_eq!(second["scope"]["resumed"], true);
    }

    #[test]
    fn isolates_open_sessions_by_actor() {
        let db = KnoxDb::open_memory().unwrap();
        let first = handle_begin(&db, json!({"actor": "gemini"})).unwrap();
        let second = handle_begin(&db, json!({"actor": "claude"})).unwrap();
        assert_ne!(first["scope"]["session_id"], second["scope"]["session_id"]);
    }

    #[test]
    fn new_session_links_to_the_previous_one() {
        let db = KnoxDb::open_memory().unwrap();
        let first = handle_begin(&db, json!({"actor": "gemini"})).unwrap();
        let first_id = first["scope"]["session_id"].as_str().unwrap().to_string();
        crate::commands::handoff::cmd_handoff(
            &db,
            json!({"session_id": first_id}),
        )
        .unwrap();

        let second = handle_begin(&db, json!({"actor": "gemini"})).unwrap();
        assert_eq!(second["scope"]["resumed"], false);
        assert_eq!(
            second["scope"]["parent_session_id"].as_str().unwrap(),
            first_id
        );
    }
}
