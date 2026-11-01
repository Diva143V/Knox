//! MCP tool handler for `knox_get_impact`.

use serde::Deserialize;
use serde_json::{Value, json};

use crate::core::error::{KnoxError, KnoxResult};
use crate::core::ids::ActionId;
use crate::core::types::QueryFilter;
use crate::graph::loader::load_subgraph;
use crate::graph::traversal::descendants;
use crate::ingest::summary::normalize_file_path;
use crate::storage::db::KnoxDb;

/// Input parameters for `knox_get_impact`.
#[derive(Debug, Deserialize, Default)]
pub struct GetImpactParams {
    /// Root action or decision to compute downstream impact for.
    pub action_id: Option<String>,
    /// Or target file path to compute downstream impact for (resolves to latest action).
    pub file_path: Option<String>,
    /// Traversal search depth (defaults to 5).
    pub max_depth: Option<usize>,
    /// Output representation format ("markdown" | "json", defaults to "markdown").
    pub format: Option<String>,
}

/// Handles the `knox_get_impact` tool invocation.
pub fn handle_get_impact(db: &KnoxDb, params: Value) -> KnoxResult<Value> {
    let input: GetImpactParams = serde_json::from_value(params)
        .map_err(|e| KnoxError::InvalidInput(format!("Invalid get_impact params: {e}")))?;

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
            .ok_or_else(|| KnoxError::InvalidInput(format!("No actions found for file: {fp}")))?
    } else {
        return Err(KnoxError::InvalidInput(
            "Either action_id or file_path must be specified".to_string(),
        ));
    };

    let max_depth = input.max_depth.unwrap_or(5);

    let graph = load_subgraph(db, std::slice::from_ref(&target_id), max_depth)?;
    let desc_actions = descendants(&graph, &target_id);

    // Extract all unique files affected downstream
    let mut affected_files: Vec<String> = desc_actions
        .iter()
        .flat_map(|a| a.files.iter().cloned())
        .collect();
    affected_files.sort();
    affected_files.dedup();

    let format_mode = input
        .format
        .as_deref()
        .unwrap_or("markdown")
        .to_ascii_lowercase();

    if format_mode == "json" {
        Ok(json!({
            "root_action_id": target_id.to_string(),
            "impacted_actions_count": desc_actions.len(),
            "affected_files": affected_files,
            "actions": desc_actions.clone(),
            "impacted": desc_actions
        }))
    } else {
        let mut md = format!(
            "### Downstream Impact of Action `{}`\n- **Impacted Actions**: {}\n- **Affected Files** ({}): `{}`\n\n",
            target_id,
            desc_actions.len(),
            affected_files.len(),
            if affected_files.is_empty() {
                "None".to_string()
            } else {
                affected_files.join("`, `")
            }
        );

        if desc_actions.is_empty() {
            md.push_str("*No downstream actions depend on this item yet.*");
        } else {
            for act in &desc_actions {
                md.push_str(&format!(
                    "- **{}** [`{}`] by `@{}`: {}\n",
                    act.action_type.to_uppercase(),
                    act.id,
                    act.actor,
                    act.summary
                ));
            }
        }

        Ok(json!({
            "root_action_id": target_id.to_string(),
            "impacted_actions_count": desc_actions.len(),
            "affected_files": affected_files,
            "actions": desc_actions.clone(),
            "impacted": desc_actions,
            "markdown": md
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::types::KnoxAction;
    use chrono::Utc;

    #[test]
    fn test_handle_get_impact() {
        let db = KnoxDb::open_memory().unwrap();
        let dec = KnoxAction {
            id: ActionId::new(),
            actor: "gemini".to_string(),
            action_type: "decide".to_string(),
            summary: "Decided on SQLite architecture".to_string(),
            timestamp: Utc::now(),
            ..Default::default()
        };
        crate::storage::nodes::insert_action(&db, &dec).unwrap();

        let write1 = KnoxAction {
            id: ActionId::new(),
            actor: "gemini".to_string(),
            action_type: "write".to_string(),
            summary: "Implemented DB migrations".to_string(),
            files: vec!["src/storage/migrations.rs".to_string()],
            parent_action_id: Some(dec.id.clone()),
            timestamp: Utc::now(),
            ..Default::default()
        };
        crate::storage::nodes::insert_action(&db, &write1).unwrap();

        let res = handle_get_impact(
            &db,
            json!({
                "action_id": dec.id.as_str()
            }),
        )
        .unwrap();

        let count = res.get("impacted_actions_count").unwrap().as_u64().unwrap();
        assert_eq!(count, 1);
        let files = res.get("affected_files").unwrap().as_array().unwrap();
        assert_eq!(files.len(), 1);

        // Test querying impact by file_path of the root action
        let root_with_file = KnoxAction {
            id: ActionId::new(),
            actor: "gemini".to_string(),
            action_type: "decide".to_string(),
            summary: "Root schema decision".to_string(),
            files: vec!["src/schema.rs".to_string()],
            timestamp: Utc::now(),
            is_current: true,
            ..Default::default()
        };
        crate::storage::nodes::insert_action(&db, &root_with_file).unwrap();

        let child = KnoxAction {
            id: ActionId::new(),
            actor: "cursor".to_string(),
            action_type: "write".to_string(),
            summary: "Implemented parser".to_string(),
            files: vec!["src/parser.rs".to_string()],
            parent_action_id: Some(root_with_file.id.clone()),
            timestamp: Utc::now(),
            is_current: true,
            ..Default::default()
        };
        crate::storage::nodes::insert_action(&db, &child).unwrap();

        let file_impact_res = handle_get_impact(
            &db,
            json!({
                "file_path": "src/schema.rs"
            }),
        )
        .unwrap();

        let file_count = file_impact_res
            .get("impacted_actions_count")
            .unwrap()
            .as_u64()
            .unwrap();
        assert_eq!(file_count, 1);
        let affected = file_impact_res
            .get("affected_files")
            .unwrap()
            .as_array()
            .unwrap();
        assert!(
            affected
                .iter()
                .any(|f| f.as_str().unwrap() == "src/parser.rs")
        );
    }
}
