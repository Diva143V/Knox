//! MCP tool handler for `knox_verify_drift`.

use std::path::PathBuf;
use serde::Deserialize;
use serde_json::{Value, json};

use crate::core::error::{KnoxError, KnoxResult};
use crate::storage::db::KnoxDb;

/// Input parameters for `knox_verify_drift`.
#[derive(Debug, Deserialize, Default)]
pub struct VerifyDriftParams {
    /// Optional project directory root.
    #[serde(default)]
    pub project_path: Option<String>,
}

/// Handles the `knox_verify_drift` MCP tool call.
pub fn handle_verify_drift(db: &KnoxDb, params: Value) -> KnoxResult<Value> {
    let input: VerifyDriftParams = serde_json::from_value(params)
        .map_err(|e| KnoxError::InvalidInput(format!("Invalid parameters for knox_verify_drift: {e}")))?;

    let repo_root = if let Some(ref path_str) = input.project_path {
        PathBuf::from(path_str.trim())
    } else if let Ok(projects) = crate::storage::nodes::list_projects(db) {
        if let Some(p) = projects.into_iter().next() {
            PathBuf::from(p.path)
        } else {
            std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
        }
    } else {
        std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
    };

    let (untracked_files, modified_files) = crate::git::diff::verify_repository_drift(&repo_root)?;
    let total_drift_files = untracked_files.len() + modified_files.len();
    let is_clean = total_drift_files == 0;

    let drift_summary = if is_clean {
        "Working tree is clean; no untracked or modified drift detected.".to_string()
    } else {
        format!(
            "{} uncommitted modified file(s), {} untracked file(s)",
            modified_files.len(),
            untracked_files.len()
        )
    };

    let mut md = String::new();
    md.push_str("## Workspace Drift Verification\n\n");
    md.push_str(&format!("- **Repository**: `{}`\n", repo_root.display()));
    md.push_str(&format!("- **Status**: {}\n", if is_clean { "✅ Clean" } else { "⚠️ Drift Detected" }));
    md.push_str(&format!("- **Summary**: {}\n\n", drift_summary));

    if !modified_files.is_empty() {
        md.push_str("### Uncommitted Modifications\n");
        for file in &modified_files {
            md.push_str(&format!("- `{file}`\n"));
        }
        md.push('\n');
    }

    if !untracked_files.is_empty() {
        md.push_str("### Untracked Files\n");
        for file in &untracked_files {
            md.push_str(&format!("- `{file}`\n"));
        }
        md.push('\n');
    }

    Ok(json!({
        "is_clean": is_clean,
        "untracked_files": untracked_files,
        "modified_files": modified_files,
        "total_drift_files": total_drift_files,
        "drift_summary": drift_summary,
        "markdown": md
    }))
}
