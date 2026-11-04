//! MCP tool handler for `knox_context`.
//!
//! Thin adapter: parses params, delegates to `cmd_context` / `cmd_context_compact`, renders response.

use serde_json::Value;

use crate::core::error::KnoxResult;
use crate::storage::db::KnoxDb;

pub fn handle_context(db: &KnoxDb, params: Value) -> KnoxResult<Value> {
    crate::commands::context::cmd_context_compact(db, params)
}

pub fn handle_get_context(db: &KnoxDb, params: Value) -> KnoxResult<Value> {
    crate::commands::context::cmd_context(db, params)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::ids::ActionId;
    use crate::core::types::KnoxAction;
    use serde_json::json;

    #[test]
    fn test_handle_get_context_defaults_to_current_only() {
        let db = KnoxDb::open_memory().unwrap();
        let session = crate::storage::sessions::start_session(&db, "gemini", Some("demo")).unwrap();
        let project_id = crate::core::ids::ProjectId("demo".to_string());

        let old_act = KnoxAction {
            id: ActionId::new(),
            actor: "gemini".to_string(),
            action_type: "write".to_string(),
            summary: "Old summary".to_string(),
            files: vec!["src/lib.rs".to_string()],
            tags: vec![],
            confidence: None,
            is_current: false,
            timestamp: chrono::Utc::now(),
            session_id: Some(session.id.clone()),
            parent_action_id: None,
            supersedes: None,
            project_id: Some(project_id.clone()),
            ..Default::default()
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
            timestamp: chrono::Utc::now(),
            session_id: Some(session.id.clone()),
            parent_action_id: None,
            supersedes: None,
            project_id: Some(project_id),
            ..Default::default()
        };
        crate::storage::nodes::insert_action(&db, &new_act).unwrap();

        let res = handle_get_context(&db, json!({ "project": "demo" })).unwrap();
        let total = res.get("total_count").unwrap().as_u64().unwrap();
        assert_eq!(total, 1);
    }

    #[test]
    fn test_handle_context_returns_budget_metadata_and_compact_output() {
        let db = KnoxDb::open_memory().unwrap();
        let action = KnoxAction {
            id: ActionId::new(),
            actor: "gemini".to_string(),
            action_type: "write".to_string(),
            summary: "Update parser".to_string(),
            files: vec!["src/parser.rs".to_string()],
            timestamp: chrono::Utc::now(),
            project_id: Some(crate::core::ids::ProjectId("demo".to_string())),
            ..Default::default()
        };
        crate::storage::nodes::insert_action(&db, &action).unwrap();

        let result = handle_context(&db, json!({ "project": "demo" })).unwrap();
        assert_eq!(result["budget"], crate::core::budget::DEFAULT_CONTEXT_BUDGET);
        assert!(!result["markdown"].as_str().unwrap().contains("###"));
        assert_eq!(result["truncated"], false);
    }

    #[test]
    fn test_context_rejects_invalid_budget() {
        let db = KnoxDb::open_memory().unwrap();
        let error = handle_context(&db, json!({ "project": "demo", "budget": 0 })).unwrap_err();
        assert!(error.to_string().contains("between 1 and"));
    }

    #[test]
    fn test_json_context_falls_back_to_bounded_output() {
        let db = KnoxDb::open_memory().unwrap();
        crate::storage::nodes::insert_project(&db, &crate::core::types::Project {
            id: crate::core::ids::ProjectId("demo".to_string()),
            name: "demo".to_string(),
            path: "demo".to_string(),
            created_at: chrono::Utc::now(),
        }).unwrap();
        for index in 0..10 {
            let action = KnoxAction {
                id: ActionId::new(),
                actor: "gemini".to_string(),
                action_type: "write".to_string(),
                summary: format!("A long context summary with details {index}"),
                files: vec!["src/parser.rs".to_string()],
                timestamp: chrono::Utc::now(),
                project_id: Some(crate::core::ids::ProjectId("demo".to_string())),
                ..Default::default()
            };
            crate::storage::nodes::insert_action(&db, &action).unwrap();
        }

        let result = handle_context(&db, json!({ "project": "demo", "format": "json", "budget": 20 })).unwrap();
        assert_eq!(result["truncated"], true);
        assert!(result["results"].is_null());
        assert!(result["budget_used"].as_u64().unwrap() <= 20);
    }
}
