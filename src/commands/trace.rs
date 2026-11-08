//! Causal trace command.
//!
//! Business logic for `knox_trace`: resolves an anchor action by ID or file
//! path, then delegates to the lineage handler for ancestor traversal with a
//! token budget (default 800).

use serde_json::{Value, json};

use crate::core::budget::{DEFAULT_TRACE_BUDGET, estimate_tokens, truncate_to_budget};
use crate::core::error::{KnoxError, KnoxResult};
use crate::storage::db::KnoxDb;

/// Runs the causal trace for `knox_trace`.
///
/// Accepts either `from` (an action ID) or `file_path` as the anchor,
/// normalizes parameters for the lineage handler, renders the causal chain
/// as Markdown, and truncates it to the caller's token budget (default 800).
pub fn cmd_trace(db: &KnoxDb, params: Value) -> KnoxResult<Value> {
    let start_instant = std::time::Instant::now();
    let object = params
        .as_object()
        .ok_or_else(|| KnoxError::InvalidInput("trace params must be an object".to_string()))?;

    let budget = object
        .get("budget")
        .and_then(Value::as_u64)
        .unwrap_or(DEFAULT_TRACE_BUDGET as u64) as usize;

    let project = object
        .get("project")
        .and_then(Value::as_str)
        .map(String::from);

    // If both 'from' and 'to' are provided, trace the directed causal path between them.
    if let (Some(from_str), Some(to_str)) = (
        object.get("from").and_then(Value::as_str),
        object.get("to").and_then(Value::as_str),
    ) {
        let from_id = crate::core::ids::ActionId::from(from_str.trim());
        let to_id = crate::core::ids::ActionId::from(to_str.trim());

        let from_act = crate::storage::nodes::get_action(db, &from_id)?;
        let to_act = crate::storage::nodes::get_action(db, &to_id)?;

        if from_act.is_none() || to_act.is_none() {
            let reason = if from_act.is_none() {
                "from_not_found"
            } else {
                "to_not_found"
            };
            let result = json!({
                "target_id": from_str,
                "to_id": to_str,
                "direction": "path",
                "depth": 0,
                "chain": [],
                "markdown": format!("No causal path found between `{from_str}` and `{to_str}` ({reason})."),
                "reason": reason,
                "truncated": false,
            });
            return render_trace_budget(db, result, budget, project, start_instant);
        }

        let max_nodes = object
            .get("max_nodes")
            .and_then(Value::as_u64)
            .unwrap_or(crate::graph::loader::DEFAULT_MAX_NODES as u64) as usize;
        let max_depth = object
            .get("depth")
            .and_then(Value::as_u64)
            .unwrap_or(10) as usize;

        let bounded = crate::graph::loader::load_subgraph_bounded(
            db,
            &[from_id.clone(), to_id.clone()],
            max_depth,
            max_nodes,
            crate::graph::loader::SubgraphDirection::Both,
        )?;

        let (path_opt, forward) = if let Some(p) =
            crate::graph::traversal::path_between(&bounded.graph, &from_id, &to_id)
        {
            (Some(p), true)
        } else if let Some(p) =
            crate::graph::traversal::path_between(&bounded.graph, &to_id, &from_id)
        {
            (Some(p), false)
        } else {
            (None, true)
        };

        if let Some(path_nodes) = path_opt {
            let chain_json: Vec<Value> = path_nodes
                .iter()
                .map(|a| {
                    json!({
                        "id": a.id.as_str(),
                        "action_type": a.action_type,
                        "summary": a.summary,
                        "actor": a.actor,
                        "files": a.files,
                        "timestamp": a.timestamp.to_rfc3339(),
                    })
                })
                .collect();

            let header = if forward {
                format!(
                    "### Causal Path from `{from_str}` to `{to_str}` ({} hops)\n",
                    path_nodes.len().saturating_sub(1)
                )
            } else {
                format!(
                    "### Causal Path from `{to_str}` to `{from_str}` ({} hops)\n",
                    path_nodes.len().saturating_sub(1)
                )
            };

            let mut md_lines = vec![header];
            for (idx, node) in path_nodes.iter().enumerate() {
                let file_ref = if let Some(first_file) = node.files.first() {
                    format!(" | `{first_file}`")
                } else {
                    String::new()
                };
                md_lines.push(format!(
                    "{}. `[{}]` **{}** - {}{}",
                    idx + 1,
                    node.id.as_str(),
                    node.action_type,
                    node.summary,
                    file_ref
                ));
            }
            let markdown = md_lines.join("\n");

            let result = json!({
                "target_id": from_str,
                "to_id": to_str,
                "direction": "path",
                "depth": path_nodes.len().saturating_sub(1),
                "chain": chain_json,
                "markdown": markdown,
                "truncated": bounded.truncated,
                "frontier": bounded.frontier_ids.into_iter().map(|id| id.0).collect::<Vec<String>>(),
            });
            return render_trace_budget(db, result, budget, project, start_instant);
        } else {
            let result = json!({
                "target_id": from_str,
                "to_id": to_str,
                "direction": "path",
                "depth": 0,
                "chain": [],
                "markdown": format!("No causal path found between `{from_str}` and `{to_str}`."),
                "reason": "no_causal_path",
                "truncated": bounded.truncated,
                "frontier": bounded.frontier_ids.into_iter().map(|id| id.0).collect::<Vec<String>>(),
            });
            return render_trace_budget(db, result, budget, project, start_instant);
        }
    }

    if let Some(from) = object.get("from").and_then(Value::as_str) {
        let mut lineage = params.clone();
        if let Value::Object(ref mut map) = lineage {
            map.insert("action_id".to_string(), Value::String(from.to_string()));
            map.remove("from");
            map.remove("to");
            if let Some(depth) = map.remove("depth") {
                map.insert("max_depth".to_string(), depth);
            }
            if let Some(mode) = map.remove("mode") {
                map.entry("direction".to_string()).or_insert(mode);
            }
            // Default format is markdown so the response can be truncated to budget.
            map.entry("format".to_string())
                .or_insert_with(|| Value::String("markdown".to_string()));
        }
        let result = crate::mcp::handlers::get_lineage::handle_get_lineage(db, lineage)?;
        return render_trace_budget(db, result, budget, project, start_instant);
    }
    if let Some(file_path) = object.get("file_path").and_then(Value::as_str) {
        let max_nodes = object.get("max_nodes").and_then(Value::as_u64);
        let direction = object
            .get("direction")
            .or_else(|| object.get("mode"))
            .and_then(Value::as_str)
            .unwrap_or("ancestors");
        let mut lineage_params = json!({
            "file_path": file_path,
            "max_depth": object.get("depth").and_then(Value::as_u64).unwrap_or(10),
            "direction": direction,
            // Default format is markdown so the response can be truncated to budget.
            "format": object.get("format").and_then(Value::as_str).unwrap_or("markdown")
        });
        if let Some(mn) = max_nodes {
            lineage_params["max_nodes"] = json!(mn);
        }
        let result = crate::mcp::handlers::get_lineage::handle_get_lineage(db, lineage_params)?;
        return render_trace_budget(db, result, budget, project, start_instant);
    }
    Err(KnoxError::InvalidInput(
        "trace requires 'from' or 'file_path'".to_string(),
    ))
}

/// Renders a lineage handler result as budgeted Markdown and records telemetry.
fn render_trace_budget(
    db: &KnoxDb,
    result: Value,
    budget: usize,
    project: Option<String>,
    start_instant: std::time::Instant,
) -> KnoxResult<Value> {
    let markdown = result
        .get("markdown")
        .and_then(Value::as_str)
        .unwrap_or("");
    let (md, budget_used, truncated) = truncate_to_budget(markdown, budget);
    let reason = result.get("reason").and_then(Value::as_str);

    let latency_ms = start_instant.elapsed().as_millis() as u64;
    crate::storage::record_query_log_best_effort(
        db,
        &crate::core::types::QueryLogEntry {
            id: format!("ql_{}", uuid::Uuid::new_v4().simple()),
            ts_ms: chrono::Utc::now().timestamp_millis(),
            tool: "knox_trace".to_string(),
            project_id: project,
            rows: result.get("depth").and_then(Value::as_u64).unwrap_or(0) as usize,
            tokens: budget_used,
            latency_ms,
            reason: reason.map(String::from),
        },
    );

    let budget_truncated = truncated;
    let component_truncated = result.get("truncated").and_then(Value::as_bool).unwrap_or(false);
    let is_truncated = budget_truncated || component_truncated;

    let mut response = json!({
        "target_id": result.get("target_id"),
        "direction": result.get("direction").unwrap_or(&json!("ancestors")),
        "depth": result.get("depth"),
        "chain": result.get("chain"),
        "lineage": result.get("lineage"),
        "impact": result.get("impact"),
        "markdown": md,
        "budget": budget,
        "budget_used": budget_used,
        "truncated": is_truncated,
        "estimate_tokens": estimate_tokens(markdown),
    });
    if let Some(to_id) = result.get("to_id") {
        response["to_id"] = to_id.clone();
    }
    if let Some(frontier) = result.get("frontier") {
        response["frontier"] = frontier.clone();
    }
    if let Some(r) = reason {
        response["reason"] = json!(r);
    }

    Ok(response)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn rejects_trace_without_anchor() {
        let db = KnoxDb::open_memory().unwrap();
        assert!(cmd_trace(&db, json!({})).is_err());
    }

    #[test]
    fn truncates_to_budget() {
        let db = KnoxDb::open_memory().unwrap();
        // Create an action to trace.
        let act = crate::core::types::KnoxAction {
            id: crate::core::ids::ActionId::new(),
            actor: "gemini".to_string(),
            action_type: "write".to_string(),
            summary: "Trace target action".to_string(),
            files: vec!["src/target.rs".to_string()],
            timestamp: chrono::Utc::now(),
            ..Default::default()
        };
        crate::storage::nodes::insert_action(&db, &act).unwrap();

        // A tiny budget should truncate the markdown.
        let result = cmd_trace(
            &db,
            json!({"from": act.id.as_str(), "budget": 10}),
        )
        .unwrap();
        let budget_used: usize = result["budget_used"].as_u64().unwrap() as usize;
        assert!(budget_used <= 10);
    }
}
