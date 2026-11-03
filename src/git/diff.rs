//! Working tree diff capture and streaming parser for Knox code deltas.

use std::fs;
use std::path::Path;
use std::process::Command;
use chrono::Utc;

use crate::core::error::{KnoxError, KnoxResult};
use crate::core::guardrails::{is_file_excluded, sanitize_delta};
use crate::core::ids::ActionId;
use crate::core::types::{
    ActionDelta, DeltaChangeType, MAX_DELTA_FILE_BYTES, MAX_DIFF_LINES,
};

/// Captures working tree modifications (both diffs against HEAD/cached and untracked files)
/// for the specified files (or all touched files if empty) and transforms them into
/// sanitized, bounded `ActionDelta` structures.
pub fn capture_working_tree_diff(
    action_id: &ActionId,
    repo_root: &Path,
    files: &[String],
) -> KnoxResult<Vec<ActionDelta>> {
    let is_git = Command::new("git")
        .args(["rev-parse", "--is-inside-work-tree"])
        .current_dir(repo_root)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);

    if !is_git {
        return Ok(Vec::new());
    }

    let mut deltas = Vec::new();

    // 1. Check if HEAD exists
    let has_head = Command::new("git")
        .args(["rev-parse", "--verify", "HEAD"])
        .current_dir(repo_root)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);

    // Build diff command
    let mut cmd = Command::new("git");
    cmd.current_dir(repo_root);
    cmd.args(["diff", "-U3", "--no-color", "--no-ext-diff"]);

    if has_head {
        cmd.arg("HEAD");
    } else {
        cmd.arg("--cached");
    }

    if !files.is_empty() {
        cmd.arg("--");
        for file in files {
            cmd.arg(file);
        }
    }

    if let Ok(output) = cmd.output() {
        if output.status.success() {
            let diff_text = String::from_utf8_lossy(&output.stdout);
            let parsed = parse_unified_diff(action_id, &diff_text);
            deltas.extend(parsed);
        }
    }

    // 2. Check untracked files via status --porcelain
    let mut status_cmd = Command::new("git");
    status_cmd.current_dir(repo_root);
    status_cmd.args(["status", "--porcelain", "-uall"]);

    if !files.is_empty() {
        status_cmd.arg("--");
        for file in files {
            status_cmd.arg(file);
        }
    }

    if let Ok(status_output) = status_cmd.output() {
        if status_output.status.success() {
            let stdout = String::from_utf8_lossy(&status_output.stdout);
            for line in stdout.lines() {
                if line.starts_with("?? ") {
                    let untracked_path = line[3..].trim().trim_matches('"');
                    if !deltas.iter().any(|d| d.file_path == untracked_path) {
                        if let Some(delta) = capture_untracked_file(action_id, repo_root, untracked_path) {
                            deltas.push(delta);
                        }
                    }
                }
            }
        }
    }

    // Filter files and apply sanitization
    let mut filtered_deltas = Vec::new();
    for mut delta in deltas {
        if is_file_excluded(&delta.file_path) {
            continue;
        }
        sanitize_delta(&mut delta);
        filtered_deltas.push(delta);
    }

    Ok(filtered_deltas)
}

/// Captures a single untracked file as an `ActionDelta` with `DeltaChangeType::Added`.
pub fn capture_untracked_file(
    action_id: &ActionId,
    repo_root: &Path,
    rel_path: &str,
) -> Option<ActionDelta> {
    if is_file_excluded(rel_path) {
        return None;
    }
    let full_path = repo_root.join(rel_path);
    let metadata = fs::metadata(&full_path).ok()?;
    if !metadata.is_file() || metadata.len() > MAX_DELTA_FILE_BYTES as u64 {
        return None;
    }

    let content = fs::read_to_string(&full_path).ok()?;
    let lines: Vec<&str> = content.lines().collect();
    let total_lines = lines.len();

    let bounded_lines = lines.iter().take(MAX_DIFF_LINES);
    let mut hunk = format!("@@ -0,0 +1,{} @@\n", total_lines);
    for line in bounded_lines {
        hunk.push('+');
        hunk.push_str(line);
        hunk.push('\n');
    }

    Some(ActionDelta {
        id: uuid::Uuid::new_v4().to_string(),
        action_id: action_id.clone(),
        file_path: rel_path.to_string(),
        change_type: DeltaChangeType::Added,
        old_path: None,
        lines_added: total_lines as u32,
        lines_deleted: 0,
        diff_hunk: Some(hunk.trim_end().to_string()),
        deleted_code: None,
        deletion_reason: None,
        created_at: Utc::now(),
    })
}

/// Parses multi-file unified git diff output into individual `ActionDelta` instances.
pub fn parse_unified_diff(action_id: &ActionId, diff_text: &str) -> Vec<ActionDelta> {
    let mut deltas = Vec::new();
    let mut current_file: Option<String> = None;
    let mut current_old_path: Option<String> = None;
    let mut current_change_type = DeltaChangeType::Modified;
    let mut current_hunk_lines: Vec<String> = Vec::new();
    let mut current_deleted_code: Vec<String> = Vec::new();
    let mut lines_added: u32 = 0;
    let mut lines_deleted: u32 = 0;
    let mut in_hunk = false;

    let flush_delta = |deltas: &mut Vec<ActionDelta>,
                       file: Option<String>,
                       old_p: Option<String>,
                       ct: DeltaChangeType,
                       added: u32,
                       deleted: u32,
                       hunk_lines: &Vec<String>,
                       del_code: &Vec<String>| {
        if let Some(file_path) = file {
            deltas.push(ActionDelta {
                id: uuid::Uuid::new_v4().to_string(),
                action_id: action_id.clone(),
                file_path,
                change_type: ct,
                old_path: old_p,
                lines_added: added,
                lines_deleted: deleted,
                diff_hunk: if hunk_lines.is_empty() {
                    None
                } else {
                    Some(hunk_lines.join("\n"))
                },
                deleted_code: if del_code.is_empty() {
                    None
                } else {
                    Some(del_code.join("\n"))
                },
                deletion_reason: None,
                created_at: Utc::now(),
            });
        }
    };

    for line in diff_text.lines() {
        if line.starts_with("diff --git ") {
            // Flush previous file
            flush_delta(
                &mut deltas,
                current_file.take(),
                current_old_path.take(),
                current_change_type,
                lines_added,
                lines_deleted,
                &current_hunk_lines,
                &current_deleted_code,
            );

            // Reset state
            current_hunk_lines.clear();
            current_deleted_code.clear();
            lines_added = 0;
            lines_deleted = 0;
            current_change_type = DeltaChangeType::Modified;
            in_hunk = false;

            // Extract file paths from "diff --git a/<path> b/<path>"
            let parts: Vec<&str> = line["diff --git ".len()..].split_whitespace().collect();
            if parts.len() >= 2 {
                let a_path = parts[0].strip_prefix("a/").unwrap_or(parts[0]);
                let b_path = parts[1].strip_prefix("b/").unwrap_or(parts[1]);
                current_file = Some(b_path.to_string());
                if a_path != b_path {
                    current_old_path = Some(a_path.to_string());
                }
            }
        } else if line.starts_with("new file mode ") {
            current_change_type = DeltaChangeType::Added;
        } else if line.starts_with("deleted file mode ") {
            current_change_type = DeltaChangeType::Deleted;
        } else if line.starts_with("rename from ") {
            let from = line["rename from ".len()..].trim();
            current_old_path = Some(from.to_string());
            current_change_type = DeltaChangeType::Renamed;
        } else if line.starts_with("rename to ") {
            let to = line["rename to ".len()..].trim();
            current_file = Some(to.to_string());
            current_change_type = DeltaChangeType::Renamed;
        } else if line.starts_with("--- ") {
            if line.contains("/dev/null") {
                current_change_type = DeltaChangeType::Added;
            }
        } else if line.starts_with("+++ ") {
            if line.contains("/dev/null") {
                current_change_type = DeltaChangeType::Deleted;
            } else if let Some(path) = line["+++ ".len()..].trim().strip_prefix("b/") {
                current_file = Some(path.to_string());
            }
        } else if line.starts_with("@@") {
            in_hunk = true;
            current_hunk_lines.push(line.to_string());
        } else if in_hunk {
            current_hunk_lines.push(line.to_string());
            if line.starts_with('+') && !line.starts_with("+++") {
                lines_added += 1;
            } else if line.starts_with('-') && !line.starts_with("---") {
                lines_deleted += 1;
                current_deleted_code.push(line[1..].to_string());
            }
        }
    }

    // Flush last file
    flush_delta(
        &mut deltas,
        current_file,
        current_old_path,
        current_change_type,
        lines_added,
        lines_deleted,
        &current_hunk_lines,
        &current_deleted_code,
    );

    deltas
}

/// Queries Git blame for a specific file and line number to determine the responsible commit SHA.
pub fn get_git_blame_commit(
    repo_root: &Path,
    file_path: &str,
    line_number: u32,
) -> KnoxResult<Option<String>> {
    let line_arg = format!("{line_number},{line_number}");
    let output = Command::new("git")
        .args(["blame", "-L", &line_arg, "--porcelain", "--", file_path])
        .current_dir(repo_root)
        .output()
        .map_err(|e| KnoxError::Git(format!("Failed to execute git blame: {e}")))?;

    if !output.status.success() {
        return Ok(None);
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    if let Some(first_line) = stdout.lines().next() {
        if let Some(sha) = first_line.split_whitespace().next() {
            if sha.chars().all(|c| c == '0') {
                return Ok(None);
            }
            return Ok(Some(sha.to_string()));
        }
    }

    Ok(None)
}

/// Checks the working tree status for drift against HEAD.
pub fn verify_repository_drift(
    repo_root: &Path,
) -> KnoxResult<(Vec<String>, Vec<String>)> {
    let output = Command::new("git")
        .args(["status", "--porcelain", "-uall"])
        .current_dir(repo_root)
        .output()
        .map_err(|e| KnoxError::Git(format!("Failed to execute git status: {e}")))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(KnoxError::Git(format!("Git error: {stderr}")));
    }

    let mut untracked = Vec::new();
    let mut modified = Vec::new();

    let stdout = String::from_utf8_lossy(&output.stdout);
    for line in stdout.lines() {
        if line.len() < 3 {
            continue;
        }
        let status = &line[..2];
        let file = line[3..].trim().trim_matches('"').to_string();

        if is_file_excluded(&file) {
            continue;
        }

        if status == "??" {
            untracked.push(file);
        } else {
            modified.push(file);
        }
    }

    Ok((untracked, modified))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_unified_diff_modified() {
        let action_id = ActionId::new();
        let diff = r#"diff --git a/src/lib.rs b/src/lib.rs
index 1234567..89abcdef 100644
--- a/src/lib.rs
+++ b/src/lib.rs
@@ -10,3 +10,4 @@
 pub mod core;
-pub mod old;
+pub mod new;
+pub mod git;
"#;
        let deltas = parse_unified_diff(&action_id, diff);
        assert_eq!(deltas.len(), 1);
        let d = &deltas[0];
        assert_eq!(d.file_path, "src/lib.rs");
        assert_eq!(d.change_type, DeltaChangeType::Modified);
        assert_eq!(d.lines_added, 2);
        assert_eq!(d.lines_deleted, 1);
        assert_eq!(d.deleted_code.as_deref(), Some("pub mod old;"));
        assert!(d.diff_hunk.is_some());
    }

    #[test]
    fn test_parse_unified_diff_rename_and_added() {
        let action_id = ActionId::new();
        let diff = r#"diff --git a/old.txt b/new.txt
similarity index 100%
rename from old.txt
rename to new.txt
diff --git a/created.rs b/created.rs
new file mode 100644
--- /dev/null
+++ b/created.rs
@@ -0,0 +1,2 @@
+fn hello() {}
+fn world() {}
"#;
        let deltas = parse_unified_diff(&action_id, diff);
        assert_eq!(deltas.len(), 2);
        assert_eq!(deltas[0].file_path, "new.txt");
        assert_eq!(deltas[0].old_path.as_deref(), Some("old.txt"));
        assert_eq!(deltas[0].change_type, DeltaChangeType::Renamed);

        assert_eq!(deltas[1].file_path, "created.rs");
        assert_eq!(deltas[1].change_type, DeltaChangeType::Added);
        assert_eq!(deltas[1].lines_added, 2);
        assert_eq!(deltas[1].lines_deleted, 0);
    }
}
