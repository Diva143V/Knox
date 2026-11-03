//! MCP tool handler for `knox_blame_causal`.

use std::path::PathBuf;
use serde::Deserialize;
use serde_json::{Value, json};

use crate::core::error::{KnoxError, KnoxResult};
use crate::storage::db::KnoxDb;

/// Input parameters for `knox_blame_causal`.
#[derive(Debug, Deserialize)]
pub struct BlameCausalParams {
    /// Relative file path within the project workspace.
    pub file_path: String,
    /// 1-based line number to causally trace.
    pub line_number: u32,
    /// Optional known git commit SHA.
    #[serde(default)]
    pub commit_sha: Option<String>,
}

/// Handles the `knox_blame_causal` MCP tool call.
pub fn handle_blame_causal(db: &KnoxDb, params: Value) -> KnoxResult<Value> {
    let input: BlameCausalParams = serde_json::from_value(params)
        .map_err(|e| KnoxError::InvalidInput(format!("Invalid parameters for knox_blame_causal: {e}")))?;

    let norm_path = input.file_path.replace('\\', "/").trim().to_string();
    if norm_path.is_empty() {
        return Err(KnoxError::InvalidInput("file_path must not be empty".to_string()));
    }
    if input.line_number == 0 {
        return Err(KnoxError::InvalidInput("line_number must be greater than 0".to_string()));
    }

    // 1. Resolve commit SHA if not explicitly provided
    let mut resolved_sha = input.commit_sha.filter(|s| !s.trim().is_empty());
    if resolved_sha.is_none() {
        let repo_root = resolve_repo_root(db);
        if let Ok(Some(sha)) = crate::git::diff::get_git_blame_commit(&repo_root, &norm_path, input.line_number) {
            resolved_sha = Some(sha);
        }
    }

    // 2. Query knowledge graph for causal attribution
    let causal = crate::storage::nodes::find_causal_action_by_file_and_commit(
        db,
        &norm_path,
        resolved_sha.as_deref(),
        input.line_number,
    )?;

    let (action, delta) = match causal {
        Some((a, d)) => (Some(a), d),
        None => (None, None),
    };

    let mut md = String::new();
    md.push_str(&format!("## Causal Blame: `{}:{}`\n\n", norm_path, input.line_number));

    if let Some(ref sha) = resolved_sha {
        md.push_str(&format!("- **Git Commit**: `{}`\n", sha));
    }

    if let Some(ref act) = action {
        md.push_str(&format!("- **Causal Action**: `{}`\n", act.id));
        md.push_str(&format!("- **Actor**: `{}` | **Type**: `{}`\n", act.actor, act.action_type));
        md.push_str(&format!("- **Summary**: {}\n", act.summary));
        md.push_str(&format!("- **Timestamp**: {}\n", act.timestamp.to_rfc3339()));

        if let Some(ref rationale) = act.rationale {
            md.push_str(&format!("\n> **Architectural Rationale**: {rationale}\n\n"));
        }

        if let Some(ref ctx) = act.context_summary {
            md.push_str(&format!("- **Context**: {}\n", ctx));
        }

        if let Some(ref d) = delta {
            md.push_str(&format!("\n### Matched Code Delta (`{}`)\n\n", d.change_type));
            if let Some(ref hunk) = d.diff_hunk {
                md.push_str("```diff\n");
                md.push_str(hunk);
                md.push_str("\n```\n");
            }
        }
    } else {
        md.push_str("_No causal Knox action found for this file and line._\n");
    }

    Ok(json!({
        "file_path": norm_path,
        "line_number": input.line_number,
        "commit_sha": resolved_sha,
        "action": action,
        "delta": delta,
        "markdown": md
    }))
}

fn resolve_repo_root(db: &KnoxDb) -> PathBuf {
    if let Ok(projects) = crate::storage::nodes::list_projects(db) {
        if let Some(p) = projects.into_iter().next() {
            return PathBuf::from(p.path);
        }
    }
    std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
}
