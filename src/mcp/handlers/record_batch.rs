//! MCP tool handler for `knox_record_batch`.

use serde::Deserialize;
use serde_json::{Value, json};

use super::record_action::{ActionRelationInput, resolve_session};
use crate::core::error::{KnoxError, KnoxResult};
use crate::core::ids::ActionId;
use crate::core::types::{ActionRelation, KnoxAction, RelationType};
use crate::ingest::summary::normalize_file_path;
use crate::storage::db::KnoxDb;

/// A single action entry within a batch request.
#[derive(Debug, Deserialize)]
pub struct BatchActionItem {
    pub action_type: String,
    pub summary: String,
    #[serde(default)]
    pub files: Vec<String>,
    #[serde(default)]
    pub parent_action_id: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub confidence: Option<f64>,
    #[serde(default)]
    pub supersedes: Option<String>,
    #[serde(default)]
    pub relations: Vec<ActionRelationInput>,
    #[serde(default)]
    pub symbols: Vec<String>,
    #[serde(default)]
    pub rationale: Option<String>,
    #[serde(default)]
    pub rejected_alternatives: Vec<String>,
    #[serde(default)]
    pub context_summary: Option<String>,
    /// Convenience helper: automatically set parent_action_id to the previous action in this batch.
    #[serde(default)]
    pub link_to_previous: Option<bool>,
}

/// Input parameters for `knox_record_batch`.
#[derive(Debug, Deserialize)]
pub struct RecordBatchParams {
    pub actor: String,
    #[serde(default)]
    pub session_id: Option<String>,
    #[serde(default)]
    pub project: Option<String>,
    #[serde(default)]
    pub link_to_previous: Option<bool>,
    pub actions: Vec<BatchActionItem>,
}

/// Handles the `knox_record_batch` tool invocation.
pub fn handle_record_batch(db: &KnoxDb, params: Value) -> KnoxResult<Value> {
    let input: RecordBatchParams = serde_json::from_value(params)
        .map_err(|e| KnoxError::InvalidInput(format!("Invalid record_batch params: {e}")))?;

    if input.actor.trim().is_empty() {
        return Err(KnoxError::InvalidInput("actor must not be empty".to_string()));
    }
    if input.actions.is_empty() {
        return Err(KnoxError::InvalidInput(
            "actions array must not be empty".to_string(),
        ));
    }

    let (session_id, is_implicit) = resolve_session(
        db,
        input.actor.trim(),
        input.project.as_deref(),
        input.session_id.as_deref(),
    )?;

    let mut recorded_ids = Vec::with_capacity(input.actions.len());
    let mut last_action_id: Option<ActionId> = None;

    for item in input.actions {
        if item.action_type.trim().is_empty() {
            return Err(KnoxError::InvalidInput(
                "action_type must not be empty on all batch items".to_string(),
            ));
        }
        if item.summary.trim().is_empty() {
            return Err(KnoxError::InvalidInput(
                "summary must not be empty on all batch items".to_string(),
            ));
        }

        let mut parent_id = item.parent_action_id.map(ActionId::from);
        let should_link = item.link_to_previous.or(input.link_to_previous).unwrap_or(false);
        if should_link && parent_id.is_none() {
            parent_id = last_action_id.clone();
        }

        let relations: Vec<ActionRelation> = item
            .relations
            .iter()
            .map(|r| {
                let rel_type = r
                    .relation_type
                    .as_deref()
                    .unwrap_or("relates_to")
                    .parse::<RelationType>()
                    .unwrap_or(RelationType::RelatesTo);
                ActionRelation {
                    target_id: ActionId::from(r.target_id.trim()),
                    relation_type: rel_type,
                }
            })
            .collect();

        let norm_files = item.files.iter().map(|f| normalize_file_path(f)).collect();
        let action_id = ActionId::new();
        let now = chrono::Utc::now();

        let norm_action_type = match item.action_type.trim().to_lowercase().as_str() {
            "decision" => "decide".to_string(),
            other => other.to_string(),
        };

        let action = KnoxAction {
            id: action_id.clone(),
            actor: input.actor.trim().to_string(),
            action_type: norm_action_type,
            summary: item.summary.trim().to_string(),
            files: norm_files,
            tags: item.tags,
            confidence: item.confidence,
            is_current: true,
            timestamp: now,
            session_id: Some(session_id.clone()),
            parent_action_id: parent_id,
            supersedes: item.supersedes.map(ActionId::from),
            relations,
            symbols: item.symbols,
            rationale: item.rationale,
            rejected_alternatives: item.rejected_alternatives,
            context_summary: item.context_summary,
        };

        crate::storage::nodes::insert_action(db, &action)?;
        last_action_id = Some(action_id.clone());
        recorded_ids.push(action_id.to_string());
    }

    Ok(json!({
        "status": "recorded",
        "count": recorded_ids.len(),
        "recorded_count": recorded_ids.len(),
        "ids": recorded_ids.clone(),
        "action_ids": recorded_ids,
        "session_id": session_id.to_string(),
        "session_implicit": is_implicit
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_handle_record_batch_with_link_to_previous() {
        let db = KnoxDb::open_memory().unwrap();
        let params = json!({
            "actor": "gemini",
            "actions": [
                {
                    "action_type": "decide",
                    "summary": "Architecture decision 1",
                    "symbols": ["Server"]
                },
                {
                    "action_type": "write",
                    "summary": "Implementation 1",
                    "files": ["src/main.rs"],
                    "link_to_previous": true
                }
            ]
        });

        let res = handle_record_batch(&db, params).unwrap();
        assert_eq!(res.get("count").unwrap(), 2);
        let ids = res.get("ids").unwrap().as_array().unwrap();
        assert_eq!(ids.len(), 2);

        let second_id = ActionId::from(ids[1].as_str().unwrap());
        let second_act = crate::storage::nodes::get_action(&db, &second_id)
            .unwrap()
            .unwrap();
        assert_eq!(
            second_act.parent_action_id.unwrap().as_str(),
            ids[0].as_str().unwrap()
        );
    }
}
