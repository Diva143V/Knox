//! Git pre-commit hook installer and uninstaller for Knox.
//! Automatically invokes `knox scan` when developers commit staged changes.

use std::fs;
use std::path::Path;

use crate::core::error::{KnoxError, KnoxResult};

const PRE_COMMIT_SCRIPT: &str = r#"#!/bin/sh
# Knox Git Hook — automatically scans staged changes and records human edits
knox scan --project .
"#;

/// Installs the Knox pre-commit hook into the project's `.git/hooks` directory.
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
    fs::write(&hook_path, PRE_COMMIT_SCRIPT)?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&hook_path)?.permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&hook_path, perms)?;
    }

    Ok(())
}

/// Uninstalls the Knox pre-commit hook from the project's `.git/hooks` directory.
pub fn cmd_hook_uninstall(project_path: &Path) -> KnoxResult<()> {
    let hook_path = project_path.join(".git").join("hooks").join("pre-commit");
    if hook_path.exists() {
        fs::remove_file(&hook_path)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_hook_install_and_uninstall() {
        let dir = tempdir().unwrap();
        let git_dir = dir.path().join(".git");
        fs::create_dir_all(&git_dir).unwrap();

        cmd_hook_install(dir.path()).unwrap();
        let hook_file = git_dir.join("hooks").join("pre-commit");
        assert!(hook_file.exists());
        let content = fs::read_to_string(&hook_file).unwrap();
        assert!(content.contains("knox scan"));

        cmd_hook_uninstall(dir.path()).unwrap();
        assert!(!hook_file.exists());
    }

    #[test]
    fn test_hook_install_fails_if_not_git_repo() {
        let dir = tempdir().unwrap();
        let res = cmd_hook_install(dir.path());
        assert!(res.is_err());
    }
}
