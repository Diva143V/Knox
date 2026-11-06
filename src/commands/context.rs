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
use crate::mcp::format::{
    format_compact_digest_with_reason, format_markdown_digest_with_reason,
};
use crate::models::router::ModelRouter;
use crate::retrieval::pack::pack_actions;
use crate::retrieval::rank::rank_actions_hybrid;
use crate::storage::db::KnoxDb;
use crate::storage::queries::query_actions;

/// Input parameters for `knox_context`.
#[derive(Debug, Deserialize, Default)]
pub struct ContextParams {
    /// Optional file path filter.
    pub file_path: Option<String>,
    /// Project scope filter (required — cross-project reads are rejected).
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
    /// Optional pagination cursor from a previous page's `next` field.
    #[serde(default)]
    pub cursor: Option<String>,
}

/// Runs the context query for `knox_context`.
///
/// Builds a `QueryFilter` from params, executes the search, ranks results,
/// packs within the token budget, and renders in the requested format.
pub fn cmd_context(db: &KnoxDb, params: Value) -> KnoxResult<Value> {
    let start_instant = std::time::Instant::now();
    let input: ContextParams = serde_json::from_value(params)
        .map_err(|e| crate::core::error::KnoxError::InvalidInput(format!(
            "Invalid context params: {e}"
        )))?;

    let project = input.project.as_deref();
    if project.is_none() {
        return Err(crate::core::error::KnoxError::NoScope(
            "project is required — cross-project reads are rejected".to_string(),
        ));
    }

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
        cursor: input.cursor,
    };

    let mut result = query_actions(db, &filter)?;
    let config = crate::core::config::KnoxConfig::from_env();
    let router = ModelRouter::from_config(&config);
    result.results = rank_actions_hybrid(
        result.results,
        ranking_file.as_deref(),
        &ranking_symbols,
        ranking_query.as_deref(),
        Some(&router),
        Some(db),
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
    } else if input.project.is_none() {
        format!("Project Context `{}`", input.project.as_deref().unwrap_or("project"))
    } else {
        "Active Knox Context".to_string()
    };
    let compact = format_compact_digest_with_reason(&packed_results, result.reason.as_deref());

    if format_mode == "json" {
        let structured = serde_json::to_value(&result)?;
        let structured_tokens = estimate_tokens(&serde_json::to_string(&structured)?);
        if structured_tokens <= budget {
            let latency_ms = start_instant.elapsed().as_millis() as u64;
            let _ = crate::storage::record_query_log(
                db,
                &crate::core::types::QueryLogEntry {
                    id: format!("ql_{}", uuid::Uuid::new_v4().simple()),
                    ts_ms: Utc::now().timestamp_millis(),
                    tool: "knox_context".to_string(),
                    project_id: input.project.clone(),
                    rows: result.results.len(),
                    tokens: structured_tokens,
                    latency_ms,
                    reason: result.reason.clone(),
                },
            );
            let mut val = serde_json::json!({
                "results": structured,
                "total_count": result.total_count,
                "has_more": result.has_more,
                "next": result.next,
                "budget": budget,
                "budget_used": structured_tokens,
                "truncated": false
            });
            if let Some(ref r) = result.reason {
                val["reason"] = serde_json::Value::String(r.clone());
            }
            return Ok(val);
        }
    }

    let is_grouped = input.synthesis.unwrap_or(false) || format_mode == "grouped";
    let rendered = if format_mode == "compact" || format_mode == "json" {
        compact
    } else {
        format_markdown_digest_with_reason(
            &title,
            &packed_results,
            is_grouped,
            result.reason.as_deref(),
        )
    };
    let (md, budget_used, truncated) = truncate_to_budget(&rendered, budget);
    let truncated = truncated || packed_truncated;
    let next_cursor = if packed_truncated && !packed_results.is_empty() {
        packed_results
            .last()
            .map(|a| crate::storage::queries::encode_cursor(a.timestamp.timestamp_millis(), a.id.as_str()))
    } else {
        result.next.clone()
    };
    let mut response = serde_json::json!({
        "results": if truncated { serde_json::Value::Null } else { serde_json::to_value(&result.results)? },
        "total_count": result.total_count,
        "has_more": result.has_more || packed_truncated || truncated,
        "next": next_cursor,
        "markdown": md,
        "budget": budget,
        "budget_used": budget_used,
        "truncated": truncated
    });
    if let Some(ref r) = result.reason {
        response["reason"] = serde_json::Value::String(r.clone());
    }
    if is_grouped {
        response["mode"] = serde_json::Value::String("grouped".to_string());
    }

    let latency_ms = start_instant.elapsed().as_millis() as u64;
    let _ = crate::storage::record_query_log(
        db,
        &crate::core::types::QueryLogEntry {
            id: format!("ql_{}", uuid::Uuid::new_v4().simple()),
            ts_ms: Utc::now().timestamp_millis(),
            tool: "knox_context".to_string(),
            project_id: input.project.clone(),
            rows: packed_results.len(),
            tokens: budget_used,
            latency_ms,
            reason: result.reason.clone(),
        },
    );

    Ok(response)
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
            timestamp: Utc::now(),
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
            timestamp: Utc::now(),
            session_id: Some(session.id.clone()),
            parent_action_id: None,
            supersedes: None,
            project_id: Some(project_id),
            ..Default::default()
        };
        crate::storage::nodes::insert_action(&db, &new_act).unwrap();

        let res = cmd_context(&db, json!({ "project": "demo" })).unwrap();
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
            project_id: Some(crate::core::ids::ProjectId("demo".to_string())),
            ..Default::default()
        };
        crate::storage::nodes::insert_action(&db, &action).unwrap();

        let result = cmd_context_compact(&db, json!({ "project": "demo" })).unwrap();
        assert_eq!(result["budget"], DEFAULT_CONTEXT_BUDGET);
        assert!(!result["markdown"].as_str().unwrap().contains("###"));
        assert_eq!(result["truncated"], false);
    }

    #[test]
    fn context_rejects_invalid_budget() {
        let db = KnoxDb::open_memory().unwrap();
        let error = cmd_context(&db, json!({ "project": "demo", "budget": 0 })).unwrap_err();
        assert!(error.to_string().contains("between 1 and"));
    }

    #[test]
    fn json_context_falls_back_to_bounded_output() {
        let db = KnoxDb::open_memory().unwrap();
        crate::storage::nodes::insert_project(&db, &crate::core::types::Project {
            id: crate::core::ids::ProjectId("demo".to_string()),
            name: "demo".to_string(),
            path: "demo".to_string(),
            remote: None,
            created_at: Utc::now(),
        }).unwrap();
        for index in 0..10 {
            let action = KnoxAction {
                id: ActionId::new(),
                actor: "gemini".to_string(),
                action_type: "write".to_string(),
                summary: format!("A long context summary with details {index}"),
                files: vec!["src/parser.rs".to_string()],
                timestamp: Utc::now(),
                project_id: Some(crate::core::ids::ProjectId("demo".to_string())),
                ..Default::default()
            };
            crate::storage::nodes::insert_action(&db, &action).unwrap();
        }

        let result = cmd_context(&db, json!({ "project": "demo", "format": "json", "budget": 20 })).unwrap();
        assert_eq!(result["truncated"], true);
        assert!(result["results"].is_null());
        assert!(result["budget_used"].as_u64().unwrap() <= 20);
    }
}
