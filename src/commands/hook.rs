//! Git pre-commit hook installer and uninstaller for Knox.
//! Automatically invokes `knox scan` when developers commit staged changes.
//!
//! The installer never destroys an existing hook: it appends a clearly marked
//! block, keeps a backup of the original, and the uninstaller restores that
//! backup. The generated block uses the absolute path of the running binary and
//! always fails open so a missing or broken Knox never blocks a commit.

use std::fs;
use std::path::{Path, PathBuf};

use crate::core::error::{KnoxError, KnoxResult};

const BLOCK_BEGIN: &str = "# >>> knox managed block >>>";
const BLOCK_END: &str = "# <<< knox managed block <<<";
const BACKUP_SUFFIX: &str = ".knox-backup";

/// Returns the shell-quoted absolute path to the currently running executable.
fn knox_binary_path() -> String {
    let path = std::env::current_exe().unwrap_or_else(|_| PathBuf::from("knox"));
    let mut rendered = path.to_string_lossy().replace('\\', "/");
    // `current_exe()` may return an extended-length Windows path.
    if let Some(stripped) = rendered.strip_prefix("//?/") {
        rendered = stripped.to_string();
    }
    rendered.replace('"', "\\\"")
}

/// Builds the managed hook block, using an absolute binary path with a PATH
/// fallback and `|| true` so the hook always fails open.
fn pre_commit_block() -> String {
    format!(
        "{BLOCK_BEGIN}
# Knox Git Hook - automatically scans staged changes and records human edits
KNOX_BIN=\"${{KNOX_BIN:-{binary}}}\"
if [ -x \"$KNOX_BIN\" ]; then
    \"$KNOX_BIN\" scan --project . || true
elif command -v knox >/dev/null 2>&1; then
    knox scan --project . || true
else
    echo \"knox: hook skipped (binary not found)\" >&2
fi
{BLOCK_END}
",
        binary = knox_binary_path()
    )
}

/// Builds the managed post-commit hook block, using an absolute binary path with a PATH
/// fallback and `|| true` so the hook always fails open.
fn post_commit_block() -> String {
    format!(
        "{BLOCK_BEGIN}
# Knox Git Hook - automatically records commit message into knowledge graph
KNOX_BIN=\"${{KNOX_BIN:-{binary}}}\"
if [ -x \"$KNOX_BIN\" ]; then
    \"$KNOX_BIN\" hook record-commit --project . || true
elif command -v knox >/dev/null 2>&1; then
    knox hook record-commit --project . || true
else
    echo \"knox: hook skipped (binary not found)\" >&2
fi
{BLOCK_END}
",
        binary = knox_binary_path()
    )
}

/// Installs or updates a managed hook block in a specific hook file.
fn install_hook_file(hook_path: &Path, block: &str) -> KnoxResult<()> {
    if hook_path.exists() {
        let existing = fs::read_to_string(hook_path)?;
        let updated = if existing.contains(BLOCK_BEGIN) {
            format!("{}{}", strip_managed_block(&existing), block)
        } else {
            let backup = backup_path(hook_path);
            if !backup.exists() {
                fs::copy(hook_path, &backup)?;
            }
            format!("{}\n{}", existing.trim_end(), block)
        };
        fs::write(hook_path, updated)?;
    } else {
        fs::write(hook_path, format!("#!/bin/sh\n{block}"))?;
    }
    make_executable(hook_path)?;
    Ok(())
}

/// Uninstalls a managed hook block from a specific hook file, restoring backups if present.
fn uninstall_hook_file(hook_path: &Path) -> KnoxResult<()> {
    if !hook_path.exists() {
        return Ok(());
    }

    let existing = fs::read_to_string(hook_path)?;
    if !existing.contains(BLOCK_BEGIN) {
        return Ok(());
    }

    let remaining = strip_managed_block(&existing);
    let backup = backup_path(hook_path);

    if is_empty_hook(&remaining) {
        if backup.exists() {
            let backup_content = fs::read_to_string(&backup)?;
            if is_empty_hook(&backup_content) {
                fs::remove_file(hook_path)?;
            } else {
                fs::copy(&backup, hook_path)?;
                make_executable(hook_path)?;
            }
            fs::remove_file(&backup)?;
        } else {
            fs::remove_file(hook_path)?;
        }
    } else {
        fs::write(hook_path, &remaining)?;
        make_executable(hook_path)?;
        if backup.exists() {
            let _ = fs::remove_file(&backup);
        }
    }
    Ok(())
}

/// Removes a previously installed managed block, returning the remaining text.
fn strip_managed_block(content: &str) -> String {
    let mut out = String::with_capacity(content.len());
    let mut in_block = false;
    for line in content.lines() {
        if line.trim() == BLOCK_BEGIN {
            in_block = true;
            continue;
        }
        if line.trim() == BLOCK_END {
            in_block = false;
            continue;
        }
        if !in_block {
            out.push_str(line);
            out.push('\n');
        }
    }
    out
}

/// Returns true when the text has no meaningful content besides a shebang.
fn is_empty_hook(content: &str) -> bool {
    content.lines().all(|line| {
        let line = line.trim();
        line.is_empty() || line.starts_with("#!")
    })
}

fn backup_path(hook_path: &Path) -> PathBuf {
    let mut name = hook_path.file_name().unwrap_or_default().to_os_string();
    name.push(BACKUP_SUFFIX);
    hook_path.with_file_name(name)
}

#[cfg(unix)]
fn make_executable(path: &Path) -> KnoxResult<()> {
    use std::os::unix::fs::PermissionsExt;
    let mut perms = fs::metadata(path)?.permissions();
    perms.set_mode(0o755);
    fs::set_permissions(path, perms)?;
    Ok(())
}

#[cfg(not(unix))]
fn make_executable(_path: &Path) -> KnoxResult<()> {
    Ok(())
}

/// Installs Knox git hooks (`pre-commit` and `post-commit`) into the project's `.git/hooks` directory.
///
/// Pre-commit scans staged human changes; post-commit captures the commit message and SHA into project memory.
/// Existing hooks are preserved: their contents are backed up and the Knox block is appended idempotently.
pub fn cmd_hook_install(project_path: &Path) -> KnoxResult<()> {
    let git_dir = project_path.join(".git");
    if !git_dir.exists() {
        return Err(KnoxError::Git(format!(
            "Directory is not a git repository (.git not found at {})",
            project_path.display()
        )));
    }

    let hooks_dir = git_dir.join("hooks");
    fs::create_dir_all(&hooks_dir)?;

    install_hook_file(&hooks_dir.join("pre-commit"), &pre_commit_block())?;
    install_hook_file(&hooks_dir.join("post-commit"), &post_commit_block())?;

    Ok(())
}

/// Uninstalls Knox git hooks, restoring any pre-existing hooks.
pub fn cmd_hook_uninstall(project_path: &Path) -> KnoxResult<()> {
    let git_dir = project_path.join(".git");
    if !git_dir.exists() {
        return Ok(());
    }

    let hooks_dir = git_dir.join("hooks");
    uninstall_hook_file(&hooks_dir.join("pre-commit"))?;
    uninstall_hook_file(&hooks_dir.join("post-commit"))?;

    Ok(())
}

/// Extracts, parses, and summarizes the latest git commit message into a scoped
/// `KnoxAction` in the knowledge graph.
///
/// Acceptance: Commit text appears in the record, scoped to the project.
pub fn cmd_hook_record_commit(
    project_path: &Path,
    db: &crate::storage::db::KnoxDb,
    msg_file: Option<&Path>,
    msg_text: Option<&str>,
) -> KnoxResult<Option<crate::core::ids::ActionId>> {
    // 1. Determine raw commit message
    let raw_message = if let Some(text) = msg_text {
        text.to_string()
    } else if let Some(file) = msg_file {
        if file.exists() {
            fs::read_to_string(file)?
        } else {
            return Ok(None);
        }
    } else {
        let out = std::process::Command::new("git")
            .args(["log", "-1", "--pretty=%B"])
            .current_dir(project_path)
            .output()
            .ok();
        match out {
            Some(o) if o.status.success() => String::from_utf8_lossy(&o.stdout).to_string(),
            _ => return Ok(None),
        }
    };

    // 2. Filter git comments and leading/trailing whitespace
    let non_comment_lines: Vec<&str> = raw_message
        .lines()
        .map(str::trim)
        .filter(|line| !line.starts_with('#'))
        .collect();

    if non_comment_lines.is_empty() {
        return Ok(None);
    }

    let subject = non_comment_lines[0].trim();
    if subject.is_empty() {
        return Ok(None);
    }

    let body = if non_comment_lines.len() > 1 {
        let b = non_comment_lines[1..].join("\n").trim().to_string();
        if b.is_empty() {
            None
        } else {
            Some(b)
        }
    } else {
        None
    };

    // 3. Extract commit SHA
    let commit_sha = std::process::Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(project_path)
        .output()
        .ok()
        .and_then(|o| {
            if o.status.success() {
                let s = String::from_utf8_lossy(&o.stdout).trim().to_string();
                if s.is_empty() { None } else { Some(s) }
            } else {
                None
            }
        });

    // 4. Extract author
    let actor = std::process::Command::new("git")
        .args(["log", "-1", "--pretty=%an"])
        .current_dir(project_path)
        .output()
        .ok()
        .and_then(|o| {
            if o.status.success() {
                let a = String::from_utf8_lossy(&o.stdout).trim().to_string();
                if a.is_empty() { None } else { Some(a) }
            } else {
                None
            }
        })
        .unwrap_or_else(|| "human".to_string());

    // 5. Extract files touched in this commit
    let mut files: Vec<String> = Vec::new();
    if let Ok(o) = std::process::Command::new("git")
        .args(["diff-tree", "--no-commit-id", "--name-only", "-r", "--root", "HEAD"])
        .current_dir(project_path)
        .output()
    {
        if o.status.success() {
            for line in String::from_utf8_lossy(&o.stdout).lines() {
                let trimmed = line.trim();
                if !trimmed.is_empty() {
                    files.push(trimmed.replace('\\', "/"));
                }
            }
        }
    }
    if files.is_empty() {
        if let Ok(o) = std::process::Command::new("git")
            .args(["diff", "--cached", "--name-only"])
            .current_dir(project_path)
            .output()
        {
            if o.status.success() {
                for line in String::from_utf8_lossy(&o.stdout).lines() {
                    let trimmed = line.trim();
                    if !trimmed.is_empty() {
                        files.push(trimmed.replace('\\', "/"));
                    }
                }
            }
        }
    }

    // 6. Resolve project scope
    let project_id = Some(crate::storage::sessions::resolve_or_create_project(
        db,
        project_path,
    )?);

    // 7. Check if a recent staged human edit action exists to enhance
    // rather than duplicating the record
    let conn = db.conn();
    let cutoff_ms = chrono::Utc::now().timestamp_millis() - 15 * 60 * 1000;
    let proj_str = project_id.as_ref().map(|p| p.as_str());

    let recent_staged_id: Option<String> = conn
        .query_row(
            "SELECT a.id FROM actions a
             WHERE a.actor = 'human'
               AND (?1 IS NULL OR a.project_id = ?1)
               AND a.id IN (SELECT at.action_id FROM action_tags at WHERE at.tag = 'staged')
               AND a.timestamp_ms > ?2
             ORDER BY a.timestamp_ms DESC LIMIT 1",
            rusqlite::params![proj_str, cutoff_ms],
            |row| row.get(0),
        )
        .ok();

    if let Some(existing_id) = recent_staged_id {
        conn.execute(
            "UPDATE actions
             SET summary = ?1,
                 rationale = COALESCE(?2, rationale),
                 commit_sha = COALESCE(?3, commit_sha)
             WHERE id = ?4",
            rusqlite::params![subject, body, commit_sha, existing_id],
        )?;

        conn.execute(
            "INSERT OR IGNORE INTO action_tags (action_id, tag) VALUES (?1, 'commit')",
            rusqlite::params![existing_id],
        )?;
        for file in &files {
            conn.execute(
                "INSERT OR IGNORE INTO action_files (action_id, file_path) VALUES (?1, ?2)",
                rusqlite::params![existing_id, file],
            )?;
        }

        return Ok(Some(crate::core::ids::ActionId::from(existing_id)));
    }

    let action_id = crate::core::ids::ActionId::new();
    let action = crate::core::types::KnoxAction {
        id: action_id.clone(),
        actor,
        action_type: "human_write".to_string(),
        summary: subject.to_string(),
        rationale: body,
        files,
        tags: vec!["git".to_string(), "commit".to_string()],
        confidence: Some(1.0),
        timestamp: chrono::Utc::now(),
        session_id: None,
        project_id,
        parent_action_id: None,
        supersedes: None,
        is_current: true,
        relations: Vec::new(),
        symbols: Vec::new(),
        rejected_alternatives: Vec::new(),
        context_summary: Some("Git commit captured by Knox hook".to_string()),
        significance: Some("minor".to_string()),
        is_resolved: Some(true),
        resolution_note: None,
        commit_sha,
        delta_summary: None,
        dedupe_key: None,
        seen_count: 1,
    };

    crate::storage::nodes::insert_action(db, &action)?;
    Ok(Some(action_id))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn pre_hook_path(root: &Path) -> PathBuf {
        root.join(".git").join("hooks").join("pre-commit")
    }

    fn post_hook_path(root: &Path) -> PathBuf {
        root.join(".git").join("hooks").join("post-commit")
    }

    #[test]
    fn test_hook_install_and_uninstall() {
        let dir = tempdir().unwrap();
        let git_dir = dir.path().join(".git");
        fs::create_dir_all(&git_dir).unwrap();

        cmd_hook_install(dir.path()).unwrap();
        let pre_file = pre_hook_path(dir.path());
        let post_file = post_hook_path(dir.path());

        assert!(pre_file.exists());
        assert!(post_file.exists());

        let pre_content = fs::read_to_string(&pre_file).unwrap();
        assert!(pre_content.contains("knox scan"));
        assert!(pre_content.contains(BLOCK_BEGIN));
        assert!(pre_content.contains("|| true"));

        let post_content = fs::read_to_string(&post_file).unwrap();
        assert!(post_content.contains("knox hook record-commit"));
        assert!(post_content.contains(BLOCK_BEGIN));
        assert!(post_content.contains("|| true"));

        cmd_hook_uninstall(dir.path()).unwrap();
        assert!(!pre_file.exists());
        assert!(!post_file.exists());
    }

    #[test]
    fn test_hook_install_preserves_existing_hook() {
        let dir = tempdir().unwrap();
        let git_dir = dir.path().join(".git");
        let hooks_dir = git_dir.join("hooks");
        fs::create_dir_all(&hooks_dir).unwrap();
        let existing = "#!/bin/sh\nnpm test\n";
        fs::write(hooks_dir.join("pre-commit"), existing).unwrap();

        cmd_hook_install(dir.path()).unwrap();
        let installed = fs::read_to_string(pre_hook_path(dir.path())).unwrap();
        assert!(installed.contains("npm test"));
        assert!(installed.contains(BLOCK_BEGIN));
        assert!(hooks_dir.join("pre-commit.knox-backup").exists());

        // Re-install must not duplicate the managed block.
        cmd_hook_install(dir.path()).unwrap();
        let reinstalled = fs::read_to_string(pre_hook_path(dir.path())).unwrap();
        assert_eq!(reinstalled.matches(BLOCK_BEGIN).count(), 1);

        cmd_hook_uninstall(dir.path()).unwrap();
        let restored = fs::read_to_string(pre_hook_path(dir.path())).unwrap();
        assert_eq!(restored, existing);
        assert!(!hooks_dir.join("pre-commit.knox-backup").exists());
    }

    #[test]
    fn test_hook_install_fails_if_not_git_repo() {
        let dir = tempdir().unwrap();
        let res = cmd_hook_install(dir.path());
        assert!(res.is_err());
    }

    #[test]
    fn test_cmd_hook_record_commit_with_text() {
        let db = crate::storage::db::KnoxDb::open_memory().unwrap();
        let dir = tempdir().unwrap();

        let commit_msg = "feat(auth): add OAuth2 refresh token rotation\n\n\
                          Rotating refresh tokens prevents replay attacks and ensures zero-trust compliance.\n\
                          # Ignored git comment line";

        let action_id = cmd_hook_record_commit(dir.path(), &db, None, Some(commit_msg))
            .unwrap()
            .expect("must record an action");

        let action = crate::storage::nodes::get_action(&db, &action_id)
            .unwrap()
            .expect("action must be retrievable");

        assert_eq!(action.summary, "feat(auth): add OAuth2 refresh token rotation");
        assert!(action.rationale.as_ref().unwrap().contains("Rotating refresh tokens prevents replay attacks"));
        assert!(!action.rationale.as_ref().unwrap().contains("# Ignored git comment line"));
        assert_eq!(action.actor, "human");
        assert!(action.tags.contains(&"commit".to_string()));
    }
}
