//! MCP tool handler for `knox_get_path`.

use serde::Deserialize;
use serde_json::{Value, json};

use crate::core::error::{KnoxError, KnoxResult};
use crate::core::ids::ActionId;
use crate::core::types::QueryFilter;
use crate::graph::loader::load_subgraph;
use crate::graph::traversal::path_between;
use crate::ingest::summary::normalize_file_path;
use crate::storage::db::KnoxDb;

/// Input parameters for `knox_get_path`.
#[derive(Debug, Deserialize, Default)]
pub struct GetPathParams {
    /// Starting action identifier.
    #[serde(alias = "from_id", alias = "source_id")]
    pub from_action_id: Option<String>,
    /// Starting file path (resolves to latest action on this file).
    pub from_file: Option<String>,
    /// Destination action identifier.
    #[serde(alias = "to_id", alias = "target_id")]
    pub to_action_id: Option<String>,
    /// Destination file path (resolves to latest action on this file).
    pub to_file: Option<String>,
    /// Traversal search depth (defaults to 10).
    pub max_depth: Option<usize>,
    /// Output representation format ("markdown" | "json", defaults to "markdown").
    pub format: Option<String>,
}

/// Handles the `knox_get_path` tool invocation.
pub fn handle_get_path(db: &KnoxDb, params: Value) -> KnoxResult<Value> {
    let input: GetPathParams = serde_json::from_value(params)
        .map_err(|e| KnoxError::InvalidInput(format!("Invalid get_path params: {e}")))?;

    let from_id = if let Some(ref id_str) = input.from_action_id {
        ActionId::from(id_str.trim())
    } else if let Some(ref fp) = input.from_file {
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
                KnoxError::InvalidInput(format!("No actions found for from_file: {fp}"))
            })?
    } else {
        return Err(KnoxError::InvalidInput(
            "Either from_action_id or from_file must be specified".to_string(),
        ));
    };

    let to_id = if let Some(ref id_str) = input.to_action_id {
        ActionId::from(id_str.trim())
    } else if let Some(ref fp) = input.to_file {
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
                KnoxError::InvalidInput(format!("No actions found for to_file: {fp}"))
            })?
    } else {
        return Err(KnoxError::InvalidInput(
            "Either to_action_id or to_file must be specified".to_string(),
        ));
    };

    let max_depth = input.max_depth.unwrap_or(10);

    let graph = load_subgraph(db, &[from_id.clone(), to_id.clone()], max_depth)?;
    let path_opt = path_between(&graph, &from_id, &to_id);

    let format_mode = input.format.as_deref().unwrap_or("markdown").to_ascii_lowercase();

    match path_opt {
        Some(path) => {
            if format_mode == "json" {
                Ok(json!({
                    "connected": true,
                    "length": path.len(),
                    "path": path
                }))
            } else {
                let mut md = format!(
                    "### Causal Path from `{}` to `{}` ({} hops)\n\n",
                    from_id,
                    to_id,
                    path.len()
                );
                for (i, act) in path.iter().enumerate() {
                    let arrow = if i == 0 { "🏁" } else { " ↳ " };
                    md.push_str(&format!(
                        "{} **{}** [`{}`] by `@{}`: {}\n",
                        arrow,
                        act.action_type.to_uppercase(),
                        act.id,
                        act.actor,
                        act.summary
                    ));
                }
                Ok(json!({
                    "connected": true,
                    "length": path.len(),
                    "path": path,
                    "markdown": md
                }))
            }
        }
        None => {
            let msg = format!("No directed causal path found between `{from_id}` and `{to_id}` within {max_depth} hops.");
            if format_mode == "json" {
                Ok(json!({
                    "connected": false,
                    "length": 0,
                    "path": []
                }))
            } else {
                Ok(json!({
                    "connected": false,
                    "length": 0,
                    "path": [],
                    "markdown": format!("### Causal Path Check\n*{}*", msg)
                }))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use crate::core::types::KnoxAction;

    #[test]
    fn test_handle_get_path_connected() {
        let db = KnoxDb::open_memory().unwrap();
        let act1 = KnoxAction {
            id: ActionId::new(),
            actor: "human".to_string(),
            action_type: "plan".to_string(),
            summary: "Root plan".to_string(),
            timestamp: Utc::now(),
            ..Default::default()
        };
        crate::storage::nodes::insert_action(&db, &act1).unwrap();

        let act2 = KnoxAction {
            id: ActionId::new(),
            actor: "gemini".to_string(),
            action_type: "write".to_string(),
            summary: "Leaf write".to_string(),
            parent_action_id: Some(act1.id.clone()),
            timestamp: Utc::now(),
            ..Default::default()
        };
        crate::storage::nodes::insert_action(&db, &act2).unwrap();

        let res = handle_get_path(
            &db,
            json!({
                "from_action_id": act1.id.as_str(),
                "to_action_id": act2.id.as_str()
            }),
        )
        .unwrap();

        assert_eq!(res.get("connected").unwrap(), true);
        assert_eq!(res.get("length").unwrap(), 2);
    }

    #[test]
    fn test_handle_get_path_with_file_paths() {
        let db = KnoxDb::open_memory().unwrap();
        let act1 = KnoxAction {
            id: ActionId::new(),
            actor: "human".to_string(),
            action_type: "plan".to_string(),
            summary: "Root plan".to_string(),
            files: vec!["docs/plan.md".to_string()],
            timestamp: Utc::now(),
            is_current: true,
            ..Default::default()
        };
        crate::storage::nodes::insert_action(&db, &act1).unwrap();

        let act2 = KnoxAction {
            id: ActionId::new(),
            actor: "gemini".to_string(),
            action_type: "write".to_string(),
            summary: "Leaf write".to_string(),
            files: vec!["src/app.rs".to_string()],
            parent_action_id: Some(act1.id.clone()),
            timestamp: Utc::now(),
            is_current: true,
            ..Default::default()
        };
        crate::storage::nodes::insert_action(&db, &act2).unwrap();

        let res = handle_get_path(
            &db,
            json!({
                "from_file": "docs/plan.md",
                "to_file": "src/app.rs"
            }),
        )
        .unwrap();

        assert_eq!(res.get("connected").unwrap(), true);
        assert_eq!(res.get("length").unwrap(), 2);
    }
}
