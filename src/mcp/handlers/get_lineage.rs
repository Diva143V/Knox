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
    /// Target action identifier to trace backwards or forwards.
    pub action_id: Option<String>,
    /// Or target file path to trace the latest action for.
    pub file_path: Option<String>,
    /// Traversal search depth (defaults to 10).
    pub max_depth: Option<usize>,
    /// Optional maximum number of component nodes to explore (defaults to 200).
    pub max_nodes: Option<usize>,
    /// Output representation format ("markdown" | "json", defaults to "markdown").
    pub format: Option<String>,
    /// Traversal direction ("ancestors" | "descendants" | "impact" | "both", defaults to "ancestors").
    pub direction: Option<String>,
}

/// Handles the `knox_get_lineage` tool invocation.
pub fn handle_get_lineage(db: &KnoxDb, params: Value) -> KnoxResult<Value> {
    let input: GetLineageParams = serde_json::from_value(params)
        .map_err(|e| KnoxError::InvalidInput(format!("Invalid get_lineage params: {e}")))?;

    let (target_ids, primary_target_id, mut empty_reason) = if let Some(ref id_str) = input.action_id {
        let trimmed_id = id_str.trim();
        let target = ActionId::from(trimmed_id);
        let exists = crate::storage::nodes::get_action(db, &target)?.is_some();
        if !exists {
            let reason = crate::storage::telemetry::determine_empty_reason(db, None, None);
            (vec![target.clone()], Some(target), Some(reason.to_string()))
        } else {
            (vec![target.clone()], Some(target), None)
        }
    } else if let Some(ref fp) = input.file_path {
        let is_out_of_scope = fp.starts_with("..") || fp.contains("/../") || fp.contains("\\..\\");
        if is_out_of_scope {
            (Vec::new(), None, Some("out_of_scope".to_string()))
        } else {
            let norm = normalize_query_file_path(fp);
            let filter = QueryFilter {
                file_path: Some(norm),
                current_only: true,
                limit: 50,
                ..Default::default()
            };
            let query_res = crate::storage::queries::query_actions(db, &filter)?;
            if query_res.results.is_empty() {
                let reason = crate::storage::telemetry::determine_empty_reason(db, None, Some(fp));
                (Vec::new(), None, Some(reason.to_string()))
            } else {
                let first_id = query_res.results.first().map(|a| a.id.clone());
                let ids: Vec<ActionId> = query_res.results.into_iter().map(|a| a.id).collect();
                (ids, first_id, None)
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

    let direction_str = input
        .direction
        .as_deref()
        .unwrap_or("ancestors")
        .to_ascii_lowercase();

    let (subgraph_dir, is_impact, is_both) = match direction_str.as_str() {
        "descendants" | "impact" | "downstream" => (SubgraphDirection::Descendants, true, false),
        "both" => (SubgraphDirection::Both, false, true),
        _ => (SubgraphDirection::Ancestors, false, false),
    };

    let (chain, impact_list) = if !target_ids.is_empty() && empty_reason.is_none() {
        let bounded = load_subgraph_bounded(
            db,
            &target_ids,
            max_depth,
            max_nodes,
            subgraph_dir,
        )?;
        truncated = bounded.truncated;
        frontier = bounded.frontier_ids.into_iter().map(|id| id.0).collect();

        if is_impact {
            let mut seen = std::collections::HashSet::new();
            for tid in &target_ids {
                seen.insert(tid.clone());
            }
            let mut desc_all = Vec::new();
            for tid in &target_ids {
                for act in crate::graph::traversal::descendants(&bounded.graph, tid) {
                    if seen.insert(act.id.clone()) {
                        desc_all.push(act);
                    }
                }
            }
            if desc_all.is_empty() {
                empty_reason = Some("no_downstream_dependents".to_string());
            }
            (Vec::new(), desc_all)
        } else if is_both {
            let c = if let Some(ref tid) = primary_target_id {
                causal_chain(&bounded.graph, tid)
            } else {
                Vec::new()
            };
            let mut seen = std::collections::HashSet::new();
            for tid in &target_ids {
                seen.insert(tid.clone());
            }
            let mut desc_all = Vec::new();
            for tid in &target_ids {
                for act in crate::graph::traversal::descendants(&bounded.graph, tid) {
                    if seen.insert(act.id.clone()) {
                        desc_all.push(act);
                    }
                }
            }
            (c, desc_all)
        } else {
            let c = if let Some(ref tid) = primary_target_id {
                causal_chain(&bounded.graph, tid)
            } else {
                Vec::new()
            };
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
            (c, Vec::new())
        }
    } else {
        (Vec::new(), Vec::new())
    };

    let format_mode = input
        .format
        .as_deref()
        .unwrap_or("markdown")
        .to_ascii_lowercase();
    let label = input
        .file_path
        .as_deref()
        .or_else(|| primary_target_id.as_ref().map(|id| id.as_str()))
        .unwrap_or("-");

    let target_id_str = primary_target_id
        .as_ref()
        .map(|id| id.to_string())
        .unwrap_or_default();

    let total_depth = if is_impact {
        impact_list.len()
    } else if is_both {
        chain.len() + impact_list.len()
    } else {
        chain.len()
    };

    let mut raw_md = if is_impact {
        crate::mcp::format::format_impact_markdown_with_reason(
            label,
            &impact_list,
            empty_reason.as_deref(),
        )
    } else if is_both {
        let anc_md = crate::mcp::format::format_lineage_markdown_with_reason(
            label,
            &chain,
            empty_reason.as_deref(),
        );
        let imp_md = crate::mcp::format::format_impact_markdown_with_reason(
            label,
            &impact_list,
            None,
        );
        format!("{anc_md}\n\n{imp_md}")
    } else {
        crate::mcp::format::format_lineage_markdown_with_reason(
            label,
            &chain,
            empty_reason.as_deref(),
        )
    };

    if truncated {
        let note = format!(
            "*Traversal truncated at {max_nodes} nodes. Frontier: {}*\n\n",
            frontier.join(", ")
        );
        if let Some(pos) = raw_md.find("\n\n") {
            raw_md.insert_str(pos + 2, &note);
        } else {
            raw_md.push_str(&note);
        }
    }

    let mut val = json!({
        "target_id": target_id_str,
        "direction": direction_str,
        "depth": total_depth,
        "chain": if is_impact { impact_list.clone() } else { chain.clone() },
        "lineage": chain,
        "impact": impact_list,
        "markdown": raw_md,
        "truncated": truncated,
        "frontier": frontier,
    });
    if let Some(ref r) = empty_reason {
        val["reason"] = json!(r);
    }
    if format_mode == "json" {
        val.as_object_mut().map(|m| m.remove("markdown"));
    }
    Ok(val)
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
