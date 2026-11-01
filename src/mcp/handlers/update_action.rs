//! MCP tool handler for `knox_update_action`.

use serde::Deserialize;
use serde_json::{Value, json};

use crate::core::error::{KnoxError, KnoxResult};
use crate::core::ids::ActionId;
use crate::core::types::{ActionRelation, RelationType, UpdateActionParams};
use crate::mcp::handlers::record_action::ActionRelationInput;
use crate::storage::db::KnoxDb;

/// Input parameters for `knox_update_action`.
#[derive(Debug, Deserialize)]
pub struct UpdateActionInput {
    /// Identifier of the action to update (required).
    pub action_id: String,
    /// New human-readable summary.
    pub summary: Option<String>,
    /// Updated action type (e.g. write, debug, read, review, plan, change, decide, doubt).
    pub action_type: Option<String>,
    /// Software or agent name.
    pub actor: Option<String>,
    /// Full replacement list of relative file paths.
    pub files: Option<Vec<String>>,
    /// Full replacement list of tags.
    pub tags: Option<Vec<String>>,
    /// Full replacement list of touched code symbols.
    pub symbols: Option<Vec<String>>,
    /// Updated architectural rationale or technical justification.
    pub rationale: Option<String>,
    /// Updated context summary.
    pub context_summary: Option<String>,
    /// Updated list of rejected alternatives.
    pub rejected_alternatives: Option<Vec<String>>,
    /// Updated significance level ("major" vs "minor").
    pub significance: Option<String>,
    /// Updated resolution status (e.g. resolving a doubt).
    pub is_resolved: Option<bool>,
    /// Updated resolution note.
    pub resolution_note: Option<String>,
    /// Updated causal parent action ID.
    pub parent_action_id: Option<String>,
    /// Set to true to explicitly remove the causal parent link.
    pub clear_parent: Option<bool>,
    /// Full replacement list of outgoing semantic relations.
    pub relations: Option<Vec<ActionRelationInput>>,
    /// Incremental relations to append without replacing existing ones.
    pub add_relations: Option<Vec<ActionRelationInput>>,
    /// List of target action IDs to remove from outgoing relations.
    pub remove_relations: Option<Vec<String>>,
}

fn convert_relations(inputs: Vec<ActionRelationInput>) -> Vec<ActionRelation> {
    inputs
        .into_iter()
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
        .collect()
}

/// Handles the `knox_update_action` tool invocation.
pub fn handle_update_action(db: &KnoxDb, params: Value) -> KnoxResult<Value> {
    let input: UpdateActionInput = serde_json::from_value(params)
        .map_err(|e| KnoxError::InvalidInput(format!("Invalid knox_update_action params: {e}")))?;

    if input.action_id.trim().is_empty() {
        return Err(KnoxError::InvalidInput(
            "action_id must not be empty".to_string(),
        ));
    }

    let parent_action_id = if input.clear_parent.unwrap_or(false) {
        Some(None)
    } else {
        input.parent_action_id.map(|p| {
            let trimmed = p.trim();
            if trimmed.is_empty() {
                None
            } else {
                Some(ActionId::from(trimmed))
            }
        })
    };

    let update_params = UpdateActionParams {
        id: ActionId::from(input.action_id.trim()),
        summary: input.summary.map(|s| s.trim().to_string()),
        action_type: input.action_type.map(|t| t.trim().to_lowercase()),
        actor: input.actor.map(|a| a.trim().to_string()),
        files: input.files,
        tags: input.tags,
        symbols: input.symbols,
        rationale: input.rationale.map(|r| r.trim().to_string()),
        context_summary: input.context_summary.map(|c| c.trim().to_string()),
        rejected_alternatives: input.rejected_alternatives,
        significance: input.significance.map(|s| s.trim().to_lowercase()),
        is_resolved: input.is_resolved,
        resolution_note: input.resolution_note.map(|rn| rn.trim().to_string()),
        parent_action_id,
        relations: input.relations.map(convert_relations),
        add_relations: input.add_relations.map(convert_relations),
        remove_relations: input.remove_relations,
    };

    let updated = crate::storage::nodes::update_action(db, &update_params)?;

    // Construct high-signal markdown digest for AI agents
    let mut md = format!("### ✏️ Updated Action: `{}`\n\n", updated.id);
    md.push_str(&format!("- **Summary**: {}\n", updated.summary));
    md.push_str(&format!(
        "- **Type**: `{}` | **Actor**: `{}`",
        updated.action_type, updated.actor
    ));
    if let Some(ref sig) = updated.significance {
        md.push_str(&format!(" | **Significance**: `{sig}`"));
    }
    md.push('\n');

    if !updated.files.is_empty() {
        md.push_str(&format!(
            "- **Files** ({}): {}\n",
            updated.files.len(),
            updated.files.join(", ")
        ));
    }
    if !updated.symbols.is_empty() {
        md.push_str(&format!(
            "- **Symbols** ({}): `{}`\n",
            updated.symbols.len(),
            updated.symbols.join("`, `")
        ));
    }
    if !updated.relations.is_empty() {
        let rel_strs: Vec<String> = updated
            .relations
            .iter()
            .map(|r| format!("`{}` -> {}", r.relation_type, r.target_id))
            .collect();
        md.push_str(&format!(
            "- **Relations** ({}): {}\n",
            updated.relations.len(),
            rel_strs.join(", ")
        ));
    }
    if let Some(resolved) = updated.is_resolved {
        md.push_str(&format!(
            "- **Resolved**: {}\n",
            if resolved { "Yes" } else { "Open Doubt" }
        ));
        if let Some(ref note) = updated.resolution_note {
            md.push_str(&format!("  *Note*: {note}\n"));
        }
    }

    Ok(json!({
        "status": "updated",
        "action": updated,
        "markdown": md
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mcp::handlers::record_action::handle_record_action;

    #[test]
    fn test_handle_update_action_flow() {
        let db = KnoxDb::open_memory().unwrap();
        let session = crate::storage::sessions::start_session(&db, "gemini", None).unwrap();

        let rec_params = json!({
            "session_id": session.id.as_str(),
            "actor": "gemini",
            "action_type": "write",
            "summary": "Initial draft of update_action",
            "files": ["src/lib.rs"]
        });
        let rec_res = handle_record_action(&db, rec_params).unwrap();
        let action_id = rec_res.get("id").unwrap().as_str().unwrap();

        let update_payload = json!({
            "action_id": action_id,
            "summary": "Refined draft of update_action with full tests",
            "files": ["src/lib.rs", "src/mcp/handlers/update_action.rs"],
            "symbols": ["handle_update_action"],
            "tags": ["mcp", "feature"],
            "rationale": "Empowers agents and GUI users to edit KG nodes in-place"
        });

        let update_res = handle_update_action(&db, update_payload).unwrap();
        assert_eq!(update_res.get("status").unwrap(), "updated");

        let action_val = update_res.get("action").unwrap();
        assert_eq!(
            action_val.get("summary").unwrap(),
            "Refined draft of update_action with full tests"
        );
        let files = action_val.get("files").unwrap().as_array().unwrap();
        assert_eq!(files.len(), 2);
    }
}
