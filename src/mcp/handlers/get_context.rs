//! MCP tool handler for `knox_get_context`.

use chrono::{DateTime, Utc};
use serde::Deserialize;
use serde_json::Value;

use crate::core::error::{KnoxError, KnoxResult};
use crate::core::budget::{DEFAULT_CONTEXT_BUDGET, MAX_CONTEXT_BUDGET, truncate_to_budget};
use crate::core::ids::SessionId;
use crate::core::types::QueryFilter;
use crate::ingest::summary::normalize_query_file_path;
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
    /// Optional filter by action type (e.g. "decide", "write").
    #[serde(default)]
    pub action_type: Option<String>,
    /// Optional filter by touched code symbols.
    #[serde(default)]
    pub symbols: Vec<String>,
    /// Optional keyword search query across summaries and rationales.
    #[serde(default)]
    pub query: Option<String>,
    /// Output representation format ("markdown" | "json" | "compact", defaults to "markdown").
    #[serde(default)]
    pub format: Option<String>,
    /// Synthesize context into categorised decisions, symbols, and constraints.
    #[serde(default)]
    pub synthesis: Option<bool>,
    /// Optional maximum response size in estimated tokens.
    pub budget: Option<usize>,
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
        file_path: input.file_path.as_deref().map(normalize_query_file_path),
        file_path_exact: false,
        project: input.project.clone(),
        session_id: input.session_id.map(SessionId::from),
        actor: input.actor,
        since: since_dt,
        current_only: input.current_only.unwrap_or(true),
        tags: input.tags,
        limit: input.limit.unwrap_or(20),
        action_type: input.action_type,
        symbols: input.symbols,
        query: input.query,
    };

    let result = crate::storage::queries::query_actions(db, &filter)?;
    let budget = input.budget.unwrap_or(DEFAULT_CONTEXT_BUDGET);
    if budget == 0 || budget > MAX_CONTEXT_BUDGET {
        return Err(KnoxError::InvalidInput(format!(
            "'budget' must be between 1 and {MAX_CONTEXT_BUDGET}"
        )));
    }

    let format_mode = input
        .format
        .as_deref()
        .unwrap_or("markdown")
        .to_ascii_lowercase();

    if format_mode == "json" {
        serde_json::to_value(&result).map_err(KnoxError::Json)
    } else {
        let title = if let Some(ref fp) = input.file_path {
            format!("Context for `{fp}`")
        } else if let Some(ref p) = input.project {
            format!("Project Context `{p}`")
        } else {
            "Active Knox Context".to_string()
        };
        let rendered = if format_mode == "compact" {
            crate::mcp::format::format_compact_digest(&result.results)
        } else {
            crate::mcp::format::format_markdown_digest(
            &title,
            &result.results,
            input.synthesis.unwrap_or(false),
            )
        };
        let (md, budget_used, truncated) = truncate_to_budget(&rendered, budget);
        Ok(serde_json::json!({
            "results": if truncated { serde_json::Value::Null } else { serde_json::to_value(&result.results)? },
            "total_count": result.total_count,
            "has_more": result.has_more,
            "markdown": md,
            "budget": budget,
            "budget_used": budget_used,
            "truncated": truncated
        }))
    }
}

/// Handles the new compact context surface while keeping legacy parameters
/// and filters compatible with `knox_get_context`.
pub fn handle_context(db: &KnoxDb, mut params: Value) -> KnoxResult<Value> {
    if let Value::Object(ref mut object) = params {
        object
            .entry("format")
            .or_insert_with(|| Value::String("compact".to_string()));
        object
            .entry("budget")
            .or_insert_with(|| Value::from(DEFAULT_CONTEXT_BUDGET));
    }
    handle_get_context(db, params)
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
            timestamp: Utc::now(),
            session_id: Some(session.id.clone()),
            parent_action_id: None,
            supersedes: None,
            ..Default::default()
        };
        crate::storage::nodes::insert_action(&db, &new_act).unwrap();

        let res = handle_get_context(&db, json!({})).unwrap();
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
            timestamp: Utc::now(),
            ..Default::default()
        };
        crate::storage::nodes::insert_action(&db, &action).unwrap();

        let result = handle_context(&db, json!({})).unwrap();
        assert_eq!(result["budget"], DEFAULT_CONTEXT_BUDGET);
        assert!(!result["markdown"].as_str().unwrap().contains("###"));
        assert_eq!(result["truncated"], false);
    }
}
