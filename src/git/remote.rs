//! Git remote URL normalization and repository remote detection.

use std::fs;
use std::path::Path;

/// Normalizes a Git remote URL into a canonical identifier: `host/owner/repo`.
///
/// Handles:
/// - SSH URLs: `git@github.com:owner/repo.git` -> `github.com/owner/repo`
/// - SSH with protocol: `ssh://git@github.com/owner/repo.git` -> `github.com/owner/repo`
/// - SSH with port: `ssh://git@github.com:22/owner/repo.git` -> `github.com/owner/repo`
/// - HTTPS URLs: `https://github.com/owner/repo.git` -> `github.com/owner/repo`
/// - HTTP URLs: `http://github.com/owner/repo.git` -> `github.com/owner/repo`
/// - Git protocol: `git://github.com/owner/repo.git` -> `github.com/owner/repo`
/// - Trailing slashes and `.git` extensions
/// - Authentication user/token prefixes
/// - Case normalization (lowercased)
///
/// Rejects local filesystem paths (e.g. `C:\path` or `/path`).
pub fn normalize_remote_url(raw: &str) -> Option<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }

    // Exclude local filesystem paths
    if trimmed.starts_with('/') || trimmed.starts_with('\\') || trimmed.starts_with('.') {
        return None;
    }
    if trimmed.len() >= 2 && trimmed.as_bytes()[1] == b':' {
        let first = trimmed.as_bytes()[0];
        if first.is_ascii_alphabetic() {
            // Windows drive path like C:\ or D:/
            return None;
        }
    }

    // 1. Strip protocol scheme (https://, http://, ssh://, git://, etc.)
    let after_scheme = if let Some(idx) = trimmed.find("://") {
        &trimmed[idx + 3..]
    } else {
        trimmed
    };

    // 2. Strip userinfo (e.g. git@ or token:password@) before host
    let after_user = if let Some(at_idx) = after_scheme.find('@') {
        let slash_idx = after_scheme.find('/');
        if slash_idx.is_none() || at_idx < slash_idx.unwrap() {
            &after_scheme[at_idx + 1..]
        } else {
            after_scheme
        }
    } else {
        after_scheme
    };

    // 3. Handle SCP-like colon syntax: `host:owner/repo` -> `host/owner/repo`
    // Or `host:port/path` -> `host/path`
    let with_slashes = if let Some(colon_idx) = after_user.find(':') {
        let slash_idx = after_user.find('/');
        if slash_idx.is_none() || colon_idx < slash_idx.unwrap() {
            let host_part = &after_user[..colon_idx];
            let rest = &after_user[colon_idx + 1..];
            if let Some(port_slash) = rest.find('/') {
                let port_candidate = &rest[..port_slash];
                if !port_candidate.is_empty() && port_candidate.chars().all(|c| c.is_ascii_digit()) {
                    format!("{host_part}/{}", rest[port_slash + 1..].trim_start_matches('/'))
                } else {
                    format!("{host_part}/{}", rest.trim_start_matches('/'))
                }
            } else {
                format!("{host_part}/{}", rest.trim_start_matches('/'))
            }
        } else {
            after_user.to_string()
        }
    } else {
        after_user.to_string()
    };

    // 4. Normalize backslashes to forward slashes and trim trailing slashes
    let mut clean = with_slashes.replace('\\', "/");
    while clean.ends_with('/') {
        clean.pop();
    }

    // 5. Strip trailing .git suffix
    if let Some(stripped) = clean.strip_suffix(".git") {
        clean = stripped.to_string();
    }
    while clean.ends_with('/') {
        clean.pop();
    }

    // 6. Lowercase for case-insensitive matching
    let lower = clean.to_ascii_lowercase();

    // 7. Validate that it contains at least one host and path component
    if !lower.contains('/') || lower.starts_with('/') || lower.ends_with('/') {
        return None;
    }

    Some(lower)
}

/// Reads the `remote.origin.url` directly from the `.git/config` file without running a subprocess.
/// Supports both standard `.git` directories and worktree/submodule `.git` pointer files.
pub fn read_git_remote_config(repo_path: &Path) -> Option<String> {
    let git_dir = repo_path.join(".git");
    let config_path = if git_dir.is_dir() {
        git_dir.join("config")
    } else if git_dir.is_file() {
        // Worktree or submodule pointer: `gitdir: <path>`
        let content = fs::read_to_string(&git_dir).ok()?;
        let pointer = content
            .lines()
            .find_map(|line| line.trim().strip_prefix("gitdir:"))
            .map(str::trim)?;
        let resolved = if Path::new(pointer).is_absolute() {
            std::path::PathBuf::from(pointer)
        } else {
            repo_path.join(pointer)
        };
        // In worktrees, config might be in commondir or adjacent
        let direct_cfg = resolved.join("config");
        if direct_cfg.exists() {
            direct_cfg
        } else {
            // Check commondir
            let commondir_file = resolved.join("commondir");
            if let Ok(c) = fs::read_to_string(&commondir_file) {
                let c_trimmed = c.trim();
                let common = if Path::new(c_trimmed).is_absolute() {
                    std::path::PathBuf::from(c_trimmed)
                } else {
                    resolved.join(c_trimmed)
                };
                common.join("config")
            } else {
                resolved.join("config")
            }
        }
    } else {
        return None;
    };

    if !config_path.exists() {
        return None;
    }

    let content = fs::read_to_string(&config_path).ok()?;
    parse_remote_url_from_ini(&content)
}

/// Parses `url = <value>` inside `[remote "origin"]` (or any `[remote "..."]`) from INI content.
fn parse_remote_url_from_ini(content: &str) -> Option<String> {
    let mut in_origin_remote = false;
    let mut fallback_remote_url = None;

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            let section = &trimmed[1..trimmed.len() - 1].trim();
            in_origin_remote = section.eq_ignore_ascii_case("remote \"origin\"")
                || section.eq_ignore_ascii_case("remote 'origin'");
            continue;
        }

        if in_origin_remote {
            if let Some((key, val)) = trimmed.split_once('=') {
                if key.trim().eq_ignore_ascii_case("url") {
                    let v = val.trim();
                    if !v.is_empty() {
                        return Some(v.to_string());
                    }
                }
            }
        } else if trimmed.to_ascii_lowercase().starts_with("url") && fallback_remote_url.is_none() {
            if let Some((key, val)) = trimmed.split_once('=') {
                if key.trim().eq_ignore_ascii_case("url") {
                    let v = val.trim();
                    if !v.is_empty() {
                        fallback_remote_url = Some(v.to_string());
                    }
                }
            }
        }
    }

    fallback_remote_url
}

/// Detects and normalizes the Git remote URL for a given project directory.
///
/// First tries fast direct `.git/config` parsing; falls back to `git config --get remote.origin.url`.
pub fn detect_remote(repo_path: &Path) -> Option<String> {
    if let Some(raw) = read_git_remote_config(repo_path) {
        if let Some(normalized) = normalize_remote_url(&raw) {
            return Some(normalized);
        }
    }

    // Fallback to git CLI if available
    let output = std::process::Command::new("git")
        .arg("-C")
        .arg(repo_path)
        .args(["config", "--get", "remote.origin.url"])
        .output()
        .ok()?;

    if output.status.success() {
        let raw = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !raw.is_empty() {
            return normalize_remote_url(&raw);
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_remote_url_various_formats() {
        let expected = "github.com/owner/repo";

        // SSH SCP-like
        assert_eq!(normalize_remote_url("git@github.com:owner/repo.git"), Some(expected.to_string()));
        assert_eq!(normalize_remote_url("git@github.com:owner/repo"), Some(expected.to_string()));

        // HTTPS
        assert_eq!(normalize_remote_url("https://github.com/owner/repo.git"), Some(expected.to_string()));
        assert_eq!(normalize_remote_url("https://github.com/owner/repo/"), Some(expected.to_string()));
        assert_eq!(normalize_remote_url("https://github.com/owner/repo"), Some(expected.to_string()));

        // HTTP
        assert_eq!(normalize_remote_url("http://github.com/owner/repo.git"), Some(expected.to_string()));

        // SSH with protocol and port
        assert_eq!(normalize_remote_url("ssh://git@github.com/owner/repo.git"), Some(expected.to_string()));
        assert_eq!(normalize_remote_url("ssh://git@github.com:22/owner/repo.git"), Some(expected.to_string()));

        // Git protocol
        assert_eq!(normalize_remote_url("git://github.com/owner/repo.git"), Some(expected.to_string()));

        // Case insensitivity
        assert_eq!(normalize_remote_url("HTTPS://GitHub.COM/Owner/Repo.git"), Some(expected.to_string()));
        assert_eq!(normalize_remote_url("git@GITHUB.COM:Owner/Repo.git"), Some(expected.to_string()));

        // Nested GitLab group
        assert_eq!(
            normalize_remote_url("git@gitlab.com:deep/subgroup/project.git"),
            Some("gitlab.com/deep/subgroup/project".to_string())
        );

        // SSH with absolute path after colon does not truncate path
        assert_eq!(
            normalize_remote_url("git@server.com:/srv/git/repo.git"),
            Some("server.com/srv/git/repo".to_string())
        );

        // Auth tokens in URL
        assert_eq!(
            normalize_remote_url("https://token:x-oauth-basic@github.com/owner/repo.git"),
            Some(expected.to_string())
        );

        // Local paths rejected
        assert_eq!(normalize_remote_url("C:\\Users\\dev\\project"), None);
        assert_eq!(normalize_remote_url("/home/dev/project"), None);
        assert_eq!(normalize_remote_url("./relative/project"), None);
        assert_eq!(normalize_remote_url("bare_project_name"), None);
    }

    #[test]
    fn test_parse_remote_url_from_ini() {
        let ini = r#"
[core]
	repositoryformatversion = 0
	filemode = true
[remote "origin"]
	url = git@github.com:knox/core.git
	fetch = +refs/heads/*:refs/remotes/origin/*
[branch "main"]
	remote = origin
	merge = refs/heads/main
"#;
        assert_eq!(
            parse_remote_url_from_ini(ini),
            Some("git@github.com:knox/core.git".to_string())
        );
    }
}
