//! MCP tool handler for `knox_get_lineage`.

use serde::Deserialize;
use serde_json::{Value, json};

use crate::core::error::{KnoxError, KnoxResult};
use crate::core::ids::ActionId;
use crate::core::types::QueryFilter;
use crate::graph::loader::{DEFAULT_MAX_NODES, SubgraphDirection, load_subgraph_bounded};
use crate::graph::traversal::causal_chain;
use crate::ingest::summary::normalize_query_file_path;
use crate::storage::db::KnoxDb;

/// Input parameters for `knox_get_lineage`.
#[derive(Debug, Deserialize, Default)]
pub struct GetLineageParams {
    /// Target action identifier to trace backwards.
    pub action_id: Option<String>,
    /// Or target file path to trace the latest action for.
    pub file_path: Option<String>,
    /// Traversal search depth (defaults to 10).
    pub max_depth: Option<usize>,
    /// Optional maximum number of component nodes to explore (defaults to 200).
    pub max_nodes: Option<usize>,
    /// Output representation format ("markdown" | "json", defaults to "markdown").
    pub format: Option<String>,
}

/// Handles the `knox_get_lineage` tool invocation.
pub fn handle_get_lineage(db: &KnoxDb, params: Value) -> KnoxResult<Value> {
    let input: GetLineageParams = serde_json::from_value(params)
        .map_err(|e| KnoxError::InvalidInput(format!("Invalid get_lineage params: {e}")))?;

    let (target_id, mut empty_reason) = if let Some(ref id_str) = input.action_id {
        let trimmed_id = id_str.trim();
        let target = ActionId::from(trimmed_id);
        let exists = crate::storage::nodes::get_action(db, &target)?.is_some();
        if !exists {
            let reason = crate::storage::telemetry::determine_empty_reason(db, None, None);
            (Some(target), Some(reason.to_string()))
        } else {
            (Some(target), None)
        }
    } else if let Some(ref fp) = input.file_path {
        let is_out_of_scope = fp.starts_with("..") || fp.contains("/../") || fp.contains("\\..\\");
        if is_out_of_scope {
            (None, Some("out_of_scope".to_string()))
        } else {
            let norm = normalize_query_file_path(fp);
            let filter = QueryFilter {
                file_path: Some(norm),
                current_only: true,
                limit: 1,
                ..Default::default()
            };
            let query_res = crate::storage::queries::query_actions(db, &filter)?;
            if let Some(first) = query_res.results.first() {
                (Some(first.id.clone()), None)
            } else {
                let reason = crate::storage::telemetry::determine_empty_reason(db, None, Some(fp));
                (None, Some(reason.to_string()))
            }
        }
    } else {
        return Err(KnoxError::InvalidInput(
            "Either action_id or file_path must be specified".to_string(),
        ));
    };

    let max_depth = input.max_depth.unwrap_or(10);
    let max_nodes = input.max_nodes.unwrap_or(DEFAULT_MAX_NODES);
    let mut truncated = false;
    let mut frontier: Vec<String> = Vec::new();

    let chain = if let Some(ref tid) = target_id {
        if empty_reason.is_none() {
            let bounded = load_subgraph_bounded(
                db,
                std::slice::from_ref(tid),
                max_depth,
                max_nodes,
                SubgraphDirection::Ancestors,
            )?;
            truncated = bounded.truncated;
            frontier = bounded.frontier_ids.into_iter().map(|id| id.0).collect();
            let c = causal_chain(&bounded.graph, tid);
            if c.is_empty() {
                empty_reason = Some(
                    crate::storage::telemetry::determine_empty_reason(
                        db,
                        None,
                        input.file_path.as_deref(),
                    )
                    .to_string(),
                );
            }
            c
        } else {
            Vec::new()
        }
    } else {
        Vec::new()
    };

    let format_mode = input
        .format
        .as_deref()
        .unwrap_or("markdown")
        .to_ascii_lowercase();
    let label = input
        .file_path
        .as_deref()
        .or_else(|| target_id.as_ref().map(|id| id.as_str()))
        .unwrap_or("-");

    let target_id_str = target_id
        .as_ref()
        .map(|id| id.to_string())
        .unwrap_or_default();

    if format_mode == "json" {
        let mut val = json!({
            "target_id": target_id_str,
            "depth": chain.len(),
            "chain": chain.clone(),
            "lineage": chain,
            "truncated": truncated,
            "frontier": frontier,
        });
        if let Some(ref r) = empty_reason {
            val["reason"] = json!(r);
        }
        Ok(val)
    } else {
        let mut md = crate::mcp::format::format_lineage_markdown_with_reason(
            label,
            &chain,
            empty_reason.as_deref(),
        );
        if truncated {
            let note = format!(
                "*Traversal truncated at {max_nodes} nodes. Frontier: {}*\n\n",
                frontier.join(", ")
            );
            if let Some(pos) = md.find("\n\n") {
                md.insert_str(pos + 2, &note);
            } else {
                md.push_str(&note);
            }
        }
        let mut val = json!({
            "target_id": target_id_str,
            "depth": chain.len(),
            "chain": chain.clone(),
            "lineage": chain,
            "markdown": md,
            "truncated": truncated,
            "frontier": frontier,
        });
        if let Some(ref r) = empty_reason {
            val["reason"] = json!(r);
        }
        Ok(val)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::types::KnoxAction;
    use chrono::Utc;

    #[test]
    fn test_handle_get_lineage() {
        let db = KnoxDb::open_memory().unwrap();
        let plan_act = KnoxAction {
            id: ActionId::new(),
            actor: "human".to_string(),
            action_type: "plan".to_string(),
            summary: "Implement feature X".to_string(),
            timestamp: Utc::now(),
            ..Default::default()
        };
        crate::storage::nodes::insert_action(&db, &plan_act).unwrap();

        let write_act = KnoxAction {
            id: ActionId::new(),
            actor: "gemini".to_string(),
            action_type: "write".to_string(),
            summary: "Wrote code for feature X".to_string(),
            parent_action_id: Some(plan_act.id.clone()),
            files: vec!["src/x.rs".to_string()],
            timestamp: Utc::now(),
            ..Default::default()
        };
        crate::storage::nodes::insert_action(&db, &write_act).unwrap();

        let res = handle_get_lineage(
            &db,
            json!({
                "action_id": write_act.id.as_str()
            }),
        )
        .unwrap();

        let depth = res.get("depth").unwrap().as_u64().unwrap();
        assert_eq!(depth, 2);
        assert!(res.get("markdown").is_some());
    }
}
