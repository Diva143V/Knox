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

/// Installs the Knox pre-commit hook into the project's `.git/hooks` directory.
///
/// An existing hook is preserved: its contents are backed up and the Knox block
/// is appended. Re-running install replaces the block instead of duplicating it.
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

    let hook_path = hooks_dir.join("pre-commit");
    let block = pre_commit_block();

    if hook_path.exists() {
        let existing = fs::read_to_string(&hook_path)?;
        let updated = if existing.contains(BLOCK_BEGIN) {
            // Idempotent re-install: replace the old block.
            format!("{}{}", strip_managed_block(&existing), block)
        } else {
            // Preserve the user's hook and keep a pristine copy for uninstall.
            let backup = backup_path(&hook_path);
            if !backup.exists() {
                fs::copy(&hook_path, &backup)?;
            }
            format!("{}\n{}", existing.trim_end(), block)
        };
        fs::write(&hook_path, updated)?;
    } else {
        fs::write(&hook_path, format!("#!/bin/sh\n{block}"))?;
    }

    make_executable(&hook_path)?;

    Ok(())
}

/// Uninstalls the Knox pre-commit hook, restoring any pre-existing hook.
pub fn cmd_hook_uninstall(project_path: &Path) -> KnoxResult<()> {
    let hook_path = project_path.join(".git").join("hooks").join("pre-commit");
    if !hook_path.exists() {
        return Ok(());
    }

    let backup = backup_path(&hook_path);
    if backup.exists() {
        // Restore the hook exactly as it was before Knox touched it.
        fs::copy(&backup, &hook_path)?;
        fs::remove_file(&backup)?;
        make_executable(&hook_path)?;
        return Ok(());
    }

    let existing = fs::read_to_string(&hook_path)?;
    if !existing.contains(BLOCK_BEGIN) {
        // Not ours; leave it alone.
        return Ok(());
    }

    let remaining = strip_managed_block(&existing);
    if is_empty_hook(&remaining) {
        fs::remove_file(&hook_path)?;
    } else {
        fs::write(&hook_path, remaining)?;
        make_executable(&hook_path)?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn hook_path(root: &Path) -> PathBuf {
        root.join(".git").join("hooks").join("pre-commit")
    }

    #[test]
    fn test_hook_install_and_uninstall() {
        let dir = tempdir().unwrap();
        let git_dir = dir.path().join(".git");
        fs::create_dir_all(&git_dir).unwrap();

        cmd_hook_install(dir.path()).unwrap();
        let hook_file = hook_path(dir.path());
        assert!(hook_file.exists());
        let content = fs::read_to_string(&hook_file).unwrap();
        assert!(content.contains("knox scan"));
        assert!(content.contains(BLOCK_BEGIN));
        assert!(content.contains("|| true"));

        cmd_hook_uninstall(dir.path()).unwrap();
        assert!(!hook_file.exists());
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
        let installed = fs::read_to_string(hook_path(dir.path())).unwrap();
        assert!(installed.contains("npm test"));
        assert!(installed.contains(BLOCK_BEGIN));
        assert!(hooks_dir.join("pre-commit.knox-backup").exists());

        // Re-install must not duplicate the managed block.
        cmd_hook_install(dir.path()).unwrap();
        let reinstalled = fs::read_to_string(hook_path(dir.path())).unwrap();
        assert_eq!(reinstalled.matches(BLOCK_BEGIN).count(), 1);

        cmd_hook_uninstall(dir.path()).unwrap();
        let restored = fs::read_to_string(hook_path(dir.path())).unwrap();
        assert_eq!(restored, existing);
        assert!(!hooks_dir.join("pre-commit.knox-backup").exists());
    }

    #[test]
    fn test_hook_install_fails_if_not_git_repo() {
        let dir = tempdir().unwrap();
        let res = cmd_hook_install(dir.path());
        assert!(res.is_err());
    }
}
