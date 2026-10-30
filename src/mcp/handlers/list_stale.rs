//! MCP tool handler for `knox_list_stale`.

use serde::Deserialize;
use serde_json::Value;

use crate::core::error::{KnoxError, KnoxResult};
use crate::core::types::QueryFilter;
use crate::ingest::summary::normalize_file_path;
use crate::storage::db::KnoxDb;

/// Input parameters for `knox_list_stale`.
#[derive(Debug, Deserialize, Default)]
pub struct ListStaleParams {
    /// Optional project scope filter.
    pub project: Option<String>,
    /// Optional affected file path filter.
    pub file_path: Option<String>,
    /// Maximum number of stale items to return (defaults to 50).
    pub limit: Option<usize>,
}

/// Handles the `knox_list_stale` tool invocation.
/// Returns exclusively superseded (`is_current: false`) actions and decisions.
pub fn handle_list_stale(db: &KnoxDb, params: Value) -> KnoxResult<Value> {
    let input: ListStaleParams = serde_json::from_value(params)
        .map_err(|e| KnoxError::InvalidInput(format!("Invalid list_stale params: {e}")))?;

    let limit = input.limit.unwrap_or(50);

    let filter = QueryFilter {
        file_path: input.file_path.map(|fp| normalize_file_path(&fp)),
        project: input.project,
        session_id: None,
        actor: None,
        since: None,
        current_only: false,
        tags: Vec::new(),
        limit: limit.max(100),
    };

    let mut query_res = crate::storage::queries::query_actions(db, &filter)?;
    query_res.results.retain(|a| !a.is_current);

    let total_count = query_res.results.len();
    if query_res.results.len() > limit {
        query_res.results.truncate(limit);
    }
    query_res.total_count = total_count;
    query_res.has_more = total_count > limit;

    serde_json::to_value(&query_res).map_err(KnoxError::Json)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::ids::ActionId;
    use crate::core::types::KnoxAction;
    use chrono::Utc;
    use serde_json::json;

    #[test]
    fn test_handle_list_stale_returns_only_superseded() {
        let db = KnoxDb::open_memory().unwrap();
        let session = crate::storage::sessions::start_session(&db, "gemini", None).unwrap();

        let old_act = KnoxAction {
            id: ActionId::new(),
            actor: "gemini".to_string(),
            action_type: "write".to_string(),
            summary: "Old stale action".to_string(),
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
            summary: "Current action".to_string(),
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

        let res = handle_list_stale(&db, json!({})).unwrap();
        let actions = res.get("results").unwrap().as_array().unwrap();
        assert_eq!(actions.len(), 1);
        assert_eq!(actions[0].get("summary").unwrap(), "Old stale action");
    }
}
