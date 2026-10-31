//! MCP tool handler for `knox_search`.

use chrono::{DateTime, Utc};
use serde::Deserialize;
use serde_json::Value;

use crate::core::error::{KnoxError, KnoxResult};
use crate::core::types::{KnoxAction, QueryResult};
use crate::storage::db::KnoxDb;

/// Input parameters for `knox_search`.
#[derive(Debug, Deserialize)]
pub struct SearchParams {
    /// Keyword search query across action summaries.
    pub query: String,
    /// Optional project scope filter.
    pub project: Option<String>,
    /// Optional actor name filter.
    pub actor: Option<String>,
    /// Optional tags filter.
    #[serde(default)]
    pub tags: Vec<String>,
    /// Optional ISO 8601 timestamp string filter.
    pub since: Option<String>,
    /// Optional maximum results to return (defaults to 20).
    pub limit: Option<usize>,
}

/// Handles the `knox_search` tool invocation.
pub fn handle_search(db: &KnoxDb, params: Value) -> KnoxResult<Value> {
    let input: SearchParams = serde_json::from_value(params)
        .map_err(|e| KnoxError::InvalidInput(format!("Invalid search params: {e}")))?;

    let query_str = input.query.trim();
    if query_str.is_empty() {
        return Err(KnoxError::InvalidInput(
            "query must not be empty".to_string(),
        ));
    }

    let since_dt = if let Some(ref s) = input.since {
        Some(
            DateTime::parse_from_rfc3339(s)
                .map(|dt| dt.with_timezone(&Utc))
                .map_err(|e| KnoxError::InvalidInput(format!("Invalid 'since' format: {e}")))?,
        )
    } else {
        None
    };

    let limit = input.limit.unwrap_or(20);
    let mut actions = crate::storage::queries::search_actions(db, query_str, limit.max(100))?;

    filter_search_actions(&mut actions, db, &input, since_dt);

    let total_count = actions.len();
    if actions.len() > limit {
        actions.truncate(limit);
    }

    let res = QueryResult {
        has_more: total_count > limit,
        total_count,
        results: actions,
    };

    serde_json::to_value(&res).map_err(KnoxError::Json)
}

/// Applies auxiliary filters on full-text search candidate actions.
fn filter_search_actions(
    actions: &mut Vec<KnoxAction>,
    db: &KnoxDb,
    input: &SearchParams,
    since_dt: Option<DateTime<Utc>>,
) {
    if let Some(ref actor) = input.actor {
        actions.retain(|a| a.actor.as_str() == actor.as_str());
    }

    if !input.tags.is_empty() {
        actions.retain(|a| input.tags.iter().all(|t| a.tags.contains(t)));
    }

    if let Some(since) = since_dt {
        actions.retain(|a| a.timestamp >= since);
    }

    if let Some(ref project) = input.project {
        actions.retain(|a| {
            if let Some(ref s_id) = a.session_id {
                crate::storage::sessions::get_session(db, s_id)
                    .ok()
                    .flatten()
                    .and_then(|s| s.project)
                    .is_some_and(|p| p.as_str() == project.as_str())
            } else {
                false
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::ids::ActionId;
    use serde_json::json;

    #[test]
    fn test_handle_search_success_and_filtering() {
        let db = KnoxDb::open_memory().unwrap();
        let session = crate::storage::sessions::start_session(&db, "gemini", None).unwrap();

        let action = KnoxAction {
            id: ActionId::new(),
            actor: "gemini".to_string(),
            action_type: "write".to_string(),
            summary: "Implemented nom-based parser for markdown".to_string(),
            files: vec!["src/parser.rs".to_string()],
            tags: vec!["parser".to_string()],
            confidence: None,
            is_current: true,
            timestamp: Utc::now(),
            session_id: Some(session.id.clone()),
            parent_action_id: None,
            supersedes: None,
            ..Default::default()
        };
        crate::storage::nodes::insert_action(&db, &action).unwrap();

        let params = json!({
            "query": "parser markdown"
        });

        let res = handle_search(&db, params).unwrap();
        let count = res.get("total_count").unwrap().as_u64().unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn test_handle_search_empty_query() {
        let db = KnoxDb::open_memory().unwrap();
        let params = json!({
            "query": "   "
        });

        let err = handle_search(&db, params).unwrap_err();
        assert!(matches!(err, KnoxError::InvalidInput(_)));
    }
}
