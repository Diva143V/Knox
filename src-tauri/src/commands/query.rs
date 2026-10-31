//! IPC command handlers for querying and retrieving Knox actions and decisions.

use chrono::{DateTime, Utc};
use knox::core::ids::ActionId;
use knox::core::types::{KnoxAction, QueryFilter, QueryResult};
use knox::storage::db::KnoxDb;
use knox::storage::nodes::get_action as db_get_action;
use knox::storage::queries::{query_actions as db_query_actions, search_actions};
use tauri::State;

use crate::AppState;

/// Parses an optional RFC 3339 timestamp string into UTC DateTime.
fn parse_since_timestamp(since: Option<&str>) -> Option<DateTime<Utc>> {
    since.and_then(|s| {
        DateTime::parse_from_rfc3339(s.trim())
            .ok()
            .map(|dt| dt.with_timezone(&Utc))
    })
}

/// Executes full-text search against the SQLite FTS5 index.
fn search_via_fts(
    db: &KnoxDb,
    query: &str,
    limit: usize,
    current_only: bool,
) -> Result<QueryResult, String> {
    let actions = search_actions(db, query, limit).map_err(|e| e.to_string())?;
    let filtered_actions: Vec<KnoxAction> = if current_only {
        actions.into_iter().filter(|a| a.is_current).collect()
    } else {
        actions
    };
    let count = filtered_actions.len();
    Ok(QueryResult {
        results: filtered_actions,
        total_count: count,
        has_more: false,
    })
}

/// Applies action type and search substring filters in memory to a QueryResult.
fn apply_in_memory_filters(
    result: &mut QueryResult,
    action_type: Option<&str>,
    search: Option<&str>,
) {
    if let Some(act_type) = action_type {
        let trimmed_type = act_type.trim().to_lowercase();
        if !trimmed_type.is_empty() {
            result
                .results
                .retain(|a| a.action_type.to_lowercase() == trimmed_type);
        }
    }

    if let Some(q) = search {
        let trimmed = q.trim().to_lowercase();
        if !trimmed.is_empty() {
            result.results.retain(|a| {
                a.summary.to_lowercase().contains(&trimmed)
                    || a.files.iter().any(|f| f.to_lowercase().contains(&trimmed))
                    || a.tags.iter().any(|t| t.to_lowercase().contains(&trimmed))
            });
        }
    }
}

/// Queries actions with flexible filtering or full-text search.
#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub async fn query_actions(
    state: State<'_, AppState>,
    search: Option<String>,
    file_path: Option<String>,
    actor: Option<String>,
    action_type: Option<String>,
    since: Option<String>,
    current_only: Option<bool>,
    limit: Option<usize>,
) -> Result<QueryResult, String> {
    let db = state.db.lock().await;
    let max_limit = limit.unwrap_or(50).clamp(1, 500);
    let parsed_since = parse_since_timestamp(since.as_deref());
    let current_only_flag = current_only.unwrap_or(true);

    if let Some(ref q) = search {
        let trimmed = q.trim();
        let no_extra_filters = file_path.is_none()
            && actor.is_none()
            && action_type.is_none()
            && parsed_since.is_none();
        if !trimmed.is_empty() && no_extra_filters {
            return search_via_fts(&db, trimmed, max_limit, current_only_flag);
        }
    }

    let filter = QueryFilter {
        file_path: file_path.filter(|s| !s.trim().is_empty()),
        actor: actor.filter(|s| !s.trim().is_empty()),
        since: parsed_since,
        current_only: current_only_flag,
        limit: max_limit,
        ..Default::default()
    };

    let mut result = db_query_actions(&db, &filter).map_err(|e| e.to_string())?;
    apply_in_memory_filters(&mut result, action_type.as_deref(), search.as_deref());
    Ok(result)
}

/// Retrieves a single KnoxAction by its ID.
#[tauri::command]
pub async fn get_action(
    state: State<'_, AppState>,
    id: String,
) -> Result<Option<KnoxAction>, String> {
    let db = state.db.lock().await;
    db_get_action(&db, &ActionId::from(id)).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_since_timestamp_valid_and_invalid() {
        let valid = "2026-10-30T10:00:00Z";
        let parsed = parse_since_timestamp(Some(valid));
        assert!(parsed.is_some());
        assert_eq!(parsed.unwrap().to_rfc3339(), "2026-10-30T10:00:00+00:00");

        let invalid = "not-a-date";
        assert!(parse_since_timestamp(Some(invalid)).is_none());
        assert!(parse_since_timestamp(None).is_none());
    }

    #[test]
    fn test_apply_in_memory_filters() {
        let action1 = KnoxAction {
            id: ActionId::new(),
            actor: "human".to_string(),
            action_type: "decide".to_string(),
            summary: "Decided on SQLite architecture".to_string(),
            files: vec!["src/storage.rs".to_string()],
            tags: vec!["architecture".to_string()],
            confidence: None,
            timestamp: Utc::now(),
            session_id: None,
            parent_action_id: None,
            supersedes: None,
            is_current: true,
            ..Default::default()
        };
        let action2 = KnoxAction {
            id: ActionId::new(),
            actor: "gemini".to_string(),
            action_type: "write".to_string(),
            summary: "Implemented graph traversal".to_string(),
            files: vec!["src/graph.rs".to_string()],
            tags: vec!["graph".to_string()],
            confidence: None,
            timestamp: Utc::now(),
            session_id: None,
            parent_action_id: None,
            supersedes: None,
            is_current: true,
            ..Default::default()
        };

        let mut qr = QueryResult {
            results: vec![action1, action2],
            total_count: 2,
            has_more: false,
        };

        // Filter by action_type
        apply_in_memory_filters(&mut qr, Some("decide"), None);
        assert_eq!(qr.results.len(), 1);
        assert_eq!(qr.results[0].action_type, "decide");

        // Filter by search substring in summary
        apply_in_memory_filters(&mut qr, None, Some("sqlite"));
        assert_eq!(qr.results.len(), 1);

        // Filter by search substring that does not match
        apply_in_memory_filters(&mut qr, None, Some("nonexistent"));
        assert_eq!(qr.results.len(), 0);
    }
}
