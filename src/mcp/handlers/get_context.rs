//! MCP tool handler for `knox_get_context`.

use chrono::{DateTime, Utc};
use serde::Deserialize;
use serde_json::Value;

use crate::core::error::{KnoxError, KnoxResult};
use crate::core::ids::SessionId;
use crate::core::types::QueryFilter;
use crate::ingest::summary::normalize_file_path;
use crate::storage::db::KnoxDb;

/// Input parameters for `knox_get_context`.
#[derive(Debug, Deserialize, Default)]
pub struct GetContextParams {
    /// Optional file path filter.
    pub file_path: Option<String>,
    /// Optional project scope filter.
    pub project: Option<String>,
    /// Optional session identifier filter.
    pub session_id: Option<String>,
    /// Optional actor name filter.
    pub actor: Option<String>,
    /// Optional ISO 8601 timestamp string filter.
    pub since: Option<String>,
    /// Optional flag to restrict to non-superseded actions (defaults to true).
    pub current_only: Option<bool>,
    /// Optional tags filter.
    #[serde(default)]
    pub tags: Vec<String>,
    /// Optional maximum results to return (defaults to 20).
    pub limit: Option<usize>,
}

/// Handles the `knox_get_context` tool invocation.
pub fn handle_get_context(db: &KnoxDb, params: Value) -> KnoxResult<Value> {
    let input: GetContextParams = serde_json::from_value(params)
        .map_err(|e| KnoxError::InvalidInput(format!("Invalid get_context params: {e}")))?;

    let since_dt = if let Some(ref s) = input.since {
        Some(
            DateTime::parse_from_rfc3339(s)
                .map(|dt| dt.with_timezone(&Utc))
                .map_err(|e| {
                    KnoxError::InvalidInput(format!("Invalid 'since' timestamp format: {e}"))
                })?,
        )
    } else {
        None
    };

    let filter = QueryFilter {
        file_path: input.file_path.map(|fp| normalize_file_path(&fp)),
        project: input.project,
        session_id: input.session_id.map(SessionId::from),
        actor: input.actor,
        since: since_dt,
        current_only: input.current_only.unwrap_or(true),
        tags: input.tags,
        limit: input.limit.unwrap_or(20),
    };

    let result = crate::storage::queries::query_actions(db, &filter)?;
    serde_json::to_value(&result).map_err(KnoxError::Json)
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

        let res = handle_get_context(&db, json!({})).unwrap();
        let total = res.get("total_count").unwrap().as_u64().unwrap();
        assert_eq!(total, 1);
    }
}
