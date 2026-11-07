//! Query execution and result formatting command for Knox.
//! Allows searching the knowledge graph by file, actor, keyword, or project.

use crate::core::error::KnoxResult;
use crate::core::types::{QueryFilter, QueryResult};
use crate::graph::loader::KnoxGraph;
use crate::storage::db::KnoxDb;
use crate::storage::queries::query_actions;

/// Parses prefix query patterns such as project, actor, and tag.
fn parse_prefix_filter(trimmed: &str) -> Option<QueryFilter> {
    if let Some(rest) = trimmed.strip_prefix("project:") {
        Some(QueryFilter {
            project: Some(rest.trim().to_string()),
            ..Default::default()
        })
    } else if let Some(rest) = trimmed.strip_prefix("actor:") {
        Some(QueryFilter {
            actor: Some(rest.trim().to_string()),
            ..Default::default()
        })
    } else {
        trimmed.strip_prefix("tag:").map(|rest| QueryFilter {
            tags: vec![rest.trim().to_string()],
            ..Default::default()
        })
    }
}

/// Executes a query against the Knox knowledge graph.
///
/// 1. Inspects the pattern for prefixes (`project:`, `actor:`, `tag:`) or file paths.
/// 2. If the pattern is a keyword query, attempts full-text search on action summaries.
/// 3. Otherwise executes a filtered query on the database.
pub fn cmd_query(
    pattern: &str,
    db: &KnoxDb,
    _graph: Option<&KnoxGraph>,
    project: Option<String>,
    file: Option<String>,
    limit: Option<usize>,
) -> KnoxResult<QueryResult> {
    let start_instant = std::time::Instant::now();
    let trimmed = pattern.trim();
    let query_limit = limit.unwrap_or(20);

    let res = if let Some(mut filter) = parse_prefix_filter(trimmed) {
        if let Some(p) = project.clone() {
            filter.project = Some(p);
        }
        if let Some(f) = file.clone() {
            filter.file_path = Some(f);
        }
        filter.limit = query_limit;
        query_actions(db, &filter)?
    } else if is_file_pattern(trimmed) {
        let target_file = file.clone().unwrap_or_else(|| trimmed.to_string());
        let filter = QueryFilter {
            file_path: Some(target_file),
            project: project.clone(),
            limit: query_limit,
            ..Default::default()
        };
        query_actions(db, &filter)?
    } else {
        let fts_results = crate::storage::queries::search_actions_scoped(
            db,
            trimmed,
            project.as_deref(),
            file.as_deref(),
            query_limit,
        )?;
        if !fts_results.is_empty() {
            let count = fts_results.len();
            QueryResult {
                results: fts_results,
                total_count: count,
                has_more: false,
                reason: None,
                next: None,
            }
        } else {
            let filter = QueryFilter {
                file_path: Some(file.unwrap_or_else(|| trimmed.to_string())),
                project: project.clone(),
                limit: query_limit,
                ..Default::default()
            };
            query_actions(db, &filter)?
        }
    };

    let latency_ms = start_instant.elapsed().as_millis() as u64;
    crate::storage::record_query_log_best_effort(
        db,
        &crate::core::types::QueryLogEntry {
            id: format!("ql_{}", uuid::Uuid::new_v4().simple()),
            ts_ms: chrono::Utc::now().timestamp_millis(),
            tool: "knox_query".to_string(),
            project_id: project,
            rows: res.results.len(),
            tokens: 0,
            latency_ms,
            reason: res.reason.clone(),
        },
    );

    Ok(res)
}

/// Determines whether a search string resembles a file path.
fn is_file_pattern(pattern: &str) -> bool {
    pattern.contains('/')
        || pattern.contains('\\')
        || pattern.ends_with(".rs")
        || pattern.ends_with(".md")
        || pattern.ends_with(".json")
        || pattern.ends_with(".toml")
        || pattern.ends_with(".txt")
}

/// Formats a `QueryResult` into a human-readable table for terminal display.
pub fn format_query_results_table(result: &QueryResult) -> String {
    if result.results.is_empty() {
        return "No actions found matching the query.".to_string();
    }

    let mut lines = Vec::new();
    lines.push(format!(
        "{:<8} | {:<12} | {:<8} | {:<40} | {:<19}",
        "ID", "ACTOR", "TYPE", "SUMMARY", "TIMESTAMP"
    ));
    lines.push("-".repeat(95));

    for action in &result.results {
        let short_id = if action.id.as_str().len() > 8 {
            &action.id.as_str()[..8]
        } else {
            action.id.as_str()
        };

        let summary = truncate_summary(&action.summary, 40);
        let time_str = action.timestamp.format("%Y-%m-%d %H:%M:%S").to_string();

        lines.push(format!(
            "{:<8} | {:<12} | {:<8} | {:<40} | {:<19}",
            short_id, action.actor, action.action_type, summary, time_str
        ));
    }

    if result.has_more {
        lines.push(format!(
            "\n... and {} more results (use --limit to expand)",
            result.total_count - result.results.len()
        ));
    }

    lines.join("\n")
}

/// Formats a `QueryResult` into pretty-printed JSON.
pub fn format_query_results_json(result: &QueryResult) -> KnoxResult<String> {
    serde_json::to_string_pretty(result).map_err(crate::core::error::KnoxError::Json)
}

/// Truncates summary to max length safely.
fn truncate_summary(s: &str, max: usize) -> String {
    if s.chars().count() > max {
        let mut truncated: String = s.chars().take(max - 3).collect();
        truncated.push_str("...");
        truncated
    } else {
        s.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::ids::ActionId;
    use crate::core::types::KnoxAction;
    use crate::storage::nodes::insert_action;
    use chrono::Utc;

    #[test]
    fn test_cmd_query_by_file_and_fts() {
        let db = KnoxDb::open_memory().unwrap();

        let action = KnoxAction {
            id: ActionId::new(),
            actor: "antigravity".to_string(),
            action_type: "write".to_string(),
            summary: "Implemented parser grammar rules".to_string(),
            files: vec!["src/parser.rs".to_string()],
            tags: vec![],
            confidence: None,
            timestamp: Utc::now(),
            session_id: None,
            parent_action_id: None,
            supersedes: None,
            is_current: true,
            ..Default::default()
        };
        insert_action(&db, &action).unwrap();

        // Query by file path
        let res_file = cmd_query("src/parser.rs", &db, None, None, None, None).unwrap();
        assert_eq!(res_file.results.len(), 1);
        assert_eq!(res_file.results[0].id, action.id);

        // Query by keyword
        let res_kw = cmd_query("grammar", &db, None, None, None, None).unwrap();
        assert_eq!(res_kw.results.len(), 1);
        assert_eq!(res_kw.results[0].id, action.id);

        // Table formatting
        let table = format_query_results_table(&res_kw);
        assert!(table.contains("antigravity"));
        assert!(table.contains("parser"));
    }
}
