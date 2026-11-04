//! Context query command.
//!
//! Business logic for `knox_context`: builds a query filter, executes the
//! search, ranks results, packs within a token budget, and renders the
//! response in the requested format.

use chrono::{DateTime, Utc};
use serde::Deserialize;
use serde_json::Value;

use crate::core::budget::{
    DEFAULT_CONTEXT_BUDGET, MAX_CONTEXT_BUDGET, estimate_tokens, truncate_to_budget,
};
use crate::core::ids::SessionId;
use crate::core::error::KnoxResult;
use crate::core::types::QueryFilter;
use crate::ingest::summary::normalize_query_file_path;
use crate::mcp::format::format_compact_digest;
use crate::mcp::format::format_markdown_digest;
use crate::retrieval::pack::pack_actions;
use crate::retrieval::rank::rank_actions;
use crate::storage::db::KnoxDb;
use crate::storage::queries::query_actions;

/// Input parameters for `knox_context`.
#[derive(Debug, Deserialize, Default)]
pub struct ContextParams {
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

/// Runs the context query for `knox_context`.
///
/// Builds a `QueryFilter` from params, executes the search, ranks results,
/// packs within the token budget, and renders in the requested format.
pub fn cmd_context(db: &KnoxDb, params: Value) -> KnoxResult<Value> {
    let input: ContextParams = serde_json::from_value(params)
        .map_err(|e| crate::core::error::KnoxError::InvalidInput(format!(
            "Invalid context params: {e}"
        )))?;

    let since_dt = if let Some(ref s) = input.since {
        Some(
            DateTime::parse_from_rfc3339(s)
                .map(|dt| dt.with_timezone(&Utc))
                .map_err(|e| {
                    crate::core::error::KnoxError::InvalidInput(format!(
                        "Invalid 'since' timestamp format: {e}"
                    ))
                })?,
        )
    } else {
        None
    };

    let ranking_file = input.file_path.clone();
    let ranking_symbols = input.symbols.clone();
    let ranking_query = input.query.clone();
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

    let mut result = query_actions(db, &filter)?;
    result.results = rank_actions(
        result.results,
        ranking_file.as_deref(),
        &ranking_symbols,
        ranking_query.as_deref(),
    );
    let budget = input.budget.unwrap_or(DEFAULT_CONTEXT_BUDGET);
    if budget == 0 || budget > MAX_CONTEXT_BUDGET {
        return Err(crate::core::error::KnoxError::InvalidInput(format!(
            "'budget' must be between 1 and {MAX_CONTEXT_BUDGET}"
        )));
    }

    let format_mode = input
        .format
        .as_deref()
        .unwrap_or("markdown")
        .to_ascii_lowercase();
    let (packed_results, packed_truncated) = pack_actions(&result.results, budget);

    let title = if let Some(ref fp) = input.file_path {
        format!("Context for `{fp}`")
    } else if let Some(ref p) = input.project {
        format!("Project Context `{p}`")
    } else {
        "Active Knox Context".to_string()
    };
    let compact = format_compact_digest(&packed_results);

    if format_mode == "json" {
        let structured = serde_json::to_value(&result)?;
        let structured_tokens = estimate_tokens(&serde_json::to_string(&structured)?);
        if structured_tokens <= budget {
            return Ok(serde_json::json!({
            "results": structured,
            "total_count": result.total_count,
            "has_more": result.has_more,
            "budget": budget,
            "budget_used": structured_tokens,
            "truncated": false
            }));
        }
    }

    let rendered = if format_mode == "compact" || format_mode == "json" {
        compact
    } else {
        format_markdown_digest(
            &title,
            &packed_results,
            input.synthesis.unwrap_or(false),
        )
    };
    let (md, budget_used, truncated) = truncate_to_budget(&rendered, budget);
    let truncated = truncated || packed_truncated;
    Ok(serde_json::json!({
        "results": if truncated { serde_json::Value::Null } else { serde_json::to_value(&result.results)? },
        "total_count": result.total_count,
        "has_more": result.has_more || packed_truncated || truncated,
        "markdown": md,
        "budget": budget,
        "budget_used": budget_used,
        "truncated": truncated
    }))
}

/// Compact context surface with default format and budget.
///
/// Sets `format` to `"compact"` and `budget` to `DEFAULT_CONTEXT_BUDGET`
/// when not already present, then delegates to `cmd_context`.
pub fn cmd_context_compact(db: &KnoxDb, mut params: Value) -> KnoxResult<Value> {
    if let Value::Object(ref mut object) = params {
        object
            .entry("format")
            .or_insert_with(|| Value::String("compact".to_string()));
        object
            .entry("budget")
            .or_insert_with(|| Value::from(DEFAULT_CONTEXT_BUDGET));
    }
    cmd_context(db, params)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::ids::ActionId;
    use crate::core::types::KnoxAction;
    use serde_json::json;

    #[test]
    fn context_defaults_to_current_only() {
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

        let res = cmd_context(&db, json!({})).unwrap();
        let total = res.get("total_count").unwrap().as_u64().unwrap();
        assert_eq!(total, 1);
    }

    #[test]
    fn context_returns_budget_metadata_and_compact_output() {
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

        let result = cmd_context_compact(&db, json!({})).unwrap();
        assert_eq!(result["budget"], DEFAULT_CONTEXT_BUDGET);
        assert!(!result["markdown"].as_str().unwrap().contains("###"));
        assert_eq!(result["truncated"], false);
    }

    #[test]
    fn context_rejects_invalid_budget() {
        let db = KnoxDb::open_memory().unwrap();
        let error = cmd_context(&db, json!({ "budget": 0 })).unwrap_err();
        assert!(error.to_string().contains("between 1 and"));
    }

    #[test]
    fn json_context_falls_back_to_bounded_output() {
        let db = KnoxDb::open_memory().unwrap();
        for index in 0..10 {
            let action = KnoxAction {
                id: ActionId::new(),
                actor: "gemini".to_string(),
                action_type: "write".to_string(),
                summary: format!("A long context summary with details {index}"),
                files: vec!["src/parser.rs".to_string()],
                timestamp: Utc::now(),
                ..Default::default()
            };
            crate::storage::nodes::insert_action(&db, &action).unwrap();
        }

        let result = cmd_context(&db, json!({ "format": "json", "budget": 20 })).unwrap();
        assert_eq!(result["truncated"], true);
        assert!(result["results"].is_null());
        assert!(result["budget_used"].as_u64().unwrap() <= 20);
    }
}
