//! MCP tool handler for `knox_diff_inspect`.

use serde::Deserialize;
use serde_json::{Value, json};

use crate::core::error::{KnoxError, KnoxResult};
use crate::core::ids::ActionId;
use crate::storage::db::KnoxDb;

/// Input parameters for `knox_diff_inspect`.
#[derive(Debug, Deserialize)]
pub struct DiffInspectParams {
    /// Action identifier whose deltas should be inspected.
    pub action_id: String,
    /// Optional file path filter.
    #[serde(default)]
    pub file_path: Option<String>,
}

/// Handles the `knox_diff_inspect` MCP tool call.
pub fn handle_diff_inspect(db: &KnoxDb, params: Value) -> KnoxResult<Value> {
    let input: DiffInspectParams = serde_json::from_value(params)
        .map_err(|e| KnoxError::InvalidInput(format!("Invalid parameters for knox_diff_inspect: {e}")))?;

    let action_id = ActionId::from(input.action_id.trim());
    let action = crate::storage::nodes::get_action(db, &action_id)?
        .ok_or_else(|| KnoxError::ActionNotFound(action_id.to_string()))?;

    let mut deltas = crate::storage::nodes::get_deltas_by_action(db, &action_id)?;

    if let Some(ref filter_file) = input.file_path {
        let norm_filter = filter_file.replace('\\', "/");
        deltas.retain(|d| d.file_path.replace('\\', "/") == norm_filter);
    }

    let mut md = String::new();
    md.push_str(&format!("## Code Deltas for Action `{}`\n\n", action.id));
    md.push_str(&format!("- **Summary**: {}\n", action.summary));
    md.push_str(&format!("- **Actor**: `{}` | **Type**: `{}`\n", action.actor, action.action_type));
    if let Some(ref sha) = action.commit_sha {
        md.push_str(&format!("- **Commit**: `{}`\n", sha));
    }
    if let Some(ref summary) = action.delta_summary {
        md.push_str(&format!(
            "- **Metrics**: {} file(s) touched (+{} / -{} lines)\n",
            summary.total_files, summary.lines_added, summary.lines_deleted
        ));
    }
    md.push_str("\n---\n\n");

    if deltas.is_empty() {
        md.push_str("_No code delta records captured for this action._\n");
    } else {
        for delta in &deltas {
            md.push_str(&format!(
                "### `{}` (`{}` | +{} / -{})\n\n",
                delta.file_path, delta.change_type, delta.lines_added, delta.lines_deleted
            ));

            if let Some(ref old) = delta.old_path {
                md.push_str(&format!("_Renamed from `{old}`_\n\n"));
            }

            if let Some(ref reason) = delta.deletion_reason {
                md.push_str(&format!("> **Deletion Rationale**: {reason}\n\n"));
            }

            if let Some(ref hunk) = delta.diff_hunk {
                md.push_str("```diff\n");
                md.push_str(hunk);
                md.push_str("\n```\n\n");
            }

            if let Some(ref del_code) = delta.deleted_code {
                md.push_str("**Deleted Code Snippet**:\n```\n");
                md.push_str(del_code);
                md.push_str("\n```\n\n");
            }
        }
    }

    Ok(json!({
        "action_id": action.id.to_string(),
        "summary": action.summary,
        "commit_sha": action.commit_sha,
        "deltas": deltas,
        "markdown": md
    }))
}
