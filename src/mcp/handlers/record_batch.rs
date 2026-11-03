//! MCP tool handler for `knox_record_batch`.

use serde::Deserialize;
use serde_json::{Value, json};

use super::record_action::{
    ActionRelationInput, default_significance, normalize_action_type, resolve_session,
};
use crate::core::error::{KnoxError, KnoxResult};
use crate::core::ids::ActionId;
use crate::core::types::{ActionRelation, KnoxAction, RelationType};
use crate::ingest::summary::normalize_file_path_with_root;
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
    /// Optional significance level ("major" vs "minor").
    #[serde(default)]
    pub significance: Option<String>,
    /// Optional resolution status (false for open doubts, true for resolved/standard).
    #[serde(default)]
    pub is_resolved: Option<bool>,
    /// Optional resolution note.
    #[serde(default)]
    pub resolution_note: Option<String>,
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
        return Err(KnoxError::InvalidInput(
            "actor must not be empty".to_string(),
        ));
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

    let session = crate::storage::sessions::get_session(db, &session_id)?
        .ok_or_else(|| KnoxError::SessionNotFound(session_id.to_string()))?;
    let project_id = session.project_id.clone();
    let project_root = project_id
        .as_ref()
        .and_then(|pid| crate::storage::nodes::get_project(db, pid).ok().flatten())
        .map(|p| p.path);

    let actor = input.actor.trim().to_string();
    let batch_link = input.link_to_previous;
    let batch_items = input.actions;

    // One transaction for the whole batch: the tool description promises
    // atomicity, so a failure on item N must not leave items 1..N-1 committed.
    let (recorded_ids, _last_action_id) = db.with_tx(|conn| {
        let mut recorded_ids = Vec::with_capacity(batch_items.len());
        let mut last_action_id: Option<ActionId> = None;

        for item in &batch_items {
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

            let mut parent_id = item.parent_action_id.clone().map(ActionId::from);
            let should_link = item.link_to_previous.or(batch_link).unwrap_or(false);
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

            let norm_files: Vec<String> = item
                .files
                .iter()
                .map(|f| normalize_file_path_with_root(f, project_root.as_deref()))
                .collect();
            let action_id = ActionId::new();
            let now = chrono::Utc::now();

            let norm_action_type = normalize_action_type(&item.action_type);

            let is_doubt = norm_action_type == "doubt" || norm_action_type == "question";
            let is_resolved = item.is_resolved.or(Some(!is_doubt));
            let significance = item
                .significance
                .clone()
                .or_else(|| Some(default_significance(&norm_action_type).to_string()));

            let action = KnoxAction {
                id: action_id.clone(),
                actor: actor.clone(),
                action_type: norm_action_type,
                summary: item.summary.trim().to_string(),
                files: norm_files,
                tags: item.tags.clone(),
                confidence: item.confidence,
                is_current: true,
                timestamp: now,
                session_id: Some(session_id.clone()),
                project_id: project_id.clone(),
                parent_action_id: parent_id,
                supersedes: item.supersedes.clone().map(ActionId::from),
                relations,
                symbols: item.symbols.clone(),
                rationale: item.rationale.clone(),
                rejected_alternatives: item.rejected_alternatives.clone(),
                context_summary: item.context_summary.clone(),
                significance,
                is_resolved,
                resolution_note: item.resolution_note.clone(),
                commit_sha: None,
                delta_summary: None,
            };

            crate::storage::nodes::insert_action_conn(conn, &action)?;
            last_action_id = Some(action_id.clone());
            recorded_ids.push(action_id.to_string());
        }

        Ok((recorded_ids, last_action_id))
    })?;

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
