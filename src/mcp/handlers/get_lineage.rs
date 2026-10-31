//! MCP tool handler for `knox_get_lineage`.

use serde::Deserialize;
use serde_json::{Value, json};

use crate::core::error::{KnoxError, KnoxResult};
use crate::core::ids::ActionId;
use crate::core::types::QueryFilter;
use crate::graph::loader::load_subgraph;
use crate::graph::traversal::causal_chain;
use crate::ingest::summary::normalize_file_path;
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
    /// Output representation format ("markdown" | "json", defaults to "markdown").
    pub format: Option<String>,
}

/// Handles the `knox_get_lineage` tool invocation.
pub fn handle_get_lineage(db: &KnoxDb, params: Value) -> KnoxResult<Value> {
    let input: GetLineageParams = serde_json::from_value(params)
        .map_err(|e| KnoxError::InvalidInput(format!("Invalid get_lineage params: {e}")))?;

    let target_id = if let Some(ref id_str) = input.action_id {
        ActionId::from(id_str.trim())
    } else if let Some(ref fp) = input.file_path {
        let norm = normalize_file_path(fp);
        let filter = QueryFilter {
            file_path: Some(norm),
            current_only: true,
            limit: 1,
            ..Default::default()
        };
        let query_res = crate::storage::queries::query_actions(db, &filter)?;
        query_res
            .results
            .first()
            .map(|a| a.id.clone())
            .ok_or_else(|| {
                KnoxError::InvalidInput(format!("No actions found for file: {fp}"))
            })?
    } else {
        return Err(KnoxError::InvalidInput(
            "Either action_id or file_path must be specified".to_string(),
        ));
    };

    let max_depth = input.max_depth.unwrap_or(10);
    let graph = load_subgraph(db, std::slice::from_ref(&target_id), max_depth)?;
    let chain = causal_chain(&graph, &target_id);

    let format_mode = input.format.as_deref().unwrap_or("markdown").to_ascii_lowercase();
    let label = input
        .file_path
        .as_deref()
        .unwrap_or_else(|| target_id.as_str());

    if format_mode == "json" {
        Ok(json!({
            "target_id": target_id.to_string(),
            "depth": chain.len(),
            "chain": chain.clone(),
            "lineage": chain
        }))
    } else {
        let md = crate::mcp::format::format_lineage_markdown(label, &chain);
        Ok(json!({
            "target_id": target_id.to_string(),
            "depth": chain.len(),
            "chain": chain.clone(),
            "lineage": chain,
            "markdown": md
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use crate::core::types::KnoxAction;

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
