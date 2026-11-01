//! Git repository commit history transcript mapper implementation.
//! Parses Git commit logs (`git log`) and turns commits into Knox knowledge graph actions.

use std::path::Path;
use std::process::Command;

use chrono::{DateTime, Utc};

use super::mapper::TranscriptMapper;
use crate::core::error::{KnoxError, KnoxResult};
use crate::core::ids::ActionId;
use crate::core::types::{ActionRelation, KnoxAction, RelationType};

/// Mapper for Git repository commit logs.
pub struct GitLogMapper;

impl TranscriptMapper for GitLogMapper {
    fn tool_name(&self) -> &str {
        "git"
    }

    fn can_parse(&self, raw: &[u8]) -> bool {
        let Ok(text) = std::str::from_utf8(raw) else {
            return false;
        };

        if text.contains("COMMIT_START") {
            return true;
        }

        // Standard git log format: "commit [0-9a-f]{40}" followed by Author/Date
        let lines: Vec<&str> = text.lines().take(15).collect();
        for window in lines.windows(3) {
            let line0 = window[0].trim();
            let line1 = window[1].trim();
            let line2 = window[2].trim();

            if line0.starts_with("commit ") && line0.len() >= 47 {
                let hash_candidate = &line0[7..];
                if hash_candidate.chars().take(40).all(|c| c.is_ascii_hexdigit())
                    && (line1.starts_with("Author:") || line2.starts_with("Author:") || line1.starts_with("Merge:"))
                {
                    return true;
                }
            }
        }

        false
    }

    fn map(&self, raw: &[u8]) -> KnoxResult<Vec<KnoxAction>> {
        let text = std::str::from_utf8(raw)
            .map_err(|e| KnoxError::ParseError(format!("Invalid UTF-8 in Git log: {e}")))?;

        let mut actions = Vec::new();
        let commits = parse_git_log_text(text);

        let mut previous_action_id: Option<ActionId> = None;

        for commit in commits {
            let is_merge = commit.subject.starts_with("Merge ") || commit.subject.contains("pull request");
            let action_type = if is_merge { "decide" } else { "plan" };

            let plan_id = ActionId::new();
            let mut relations = Vec::new();

            if let Some(ref prev) = previous_action_id {
                relations.push(ActionRelation {
                    target_id: prev.clone(),
                    relation_type: RelationType::DependsOn,
                });
            }

            let summary = if commit.subject.is_empty() {
                format!("Commit {}", &commit.hash[..commit.hash.len().min(8)])
            } else {
                commit.subject.clone()
            };

            let rationale = if commit.body.is_empty() {
                None
            } else {
                Some(commit.body.clone())
            };

            let plan_action = KnoxAction {
                id: plan_id.clone(),
                actor: "git".to_string(),
                action_type: action_type.to_string(),
                summary,
                confidence: Some(1.0),
                timestamp: commit.timestamp,
                session_id: None,
                parent_action_id: previous_action_id.clone(),
                files: commit.files.clone(),
                tags: vec!["git-commit".to_string()],
                symbols: Vec::new(),
                relations,
                supersedes: None,
                is_current: true,
                rationale,
                context_summary: Some(format!("Git commit {} by {}", &commit.hash[..commit.hash.len().min(8)], commit.author)),
                rejected_alternatives: Vec::new(),
                significance: Some("major".to_string()),
                is_resolved: Some(true),
                resolution_note: None,
            };

            actions.push(plan_action);

            // If the commit modified files, create a complementary "write" action linked to the plan
            if !commit.files.is_empty() && !is_merge {
                let write_id = ActionId::new();
                let write_action = KnoxAction {
                    id: write_id.clone(),
                    actor: "git".to_string(),
                    action_type: "write".to_string(),
                    summary: format!("Modify {} file(s) in commit {}", commit.files.len(), &commit.hash[..commit.hash.len().min(8)]),
                    confidence: Some(1.0),
                    timestamp: commit.timestamp,
                    session_id: None,
                    parent_action_id: Some(plan_id.clone()),
                    files: commit.files.clone(),
                    tags: vec!["git-write".to_string()],
                    symbols: Vec::new(),
                    relations: vec![ActionRelation {
                        target_id: plan_id.clone(),
                        relation_type: RelationType::Implements,
                    }],
                    supersedes: None,
                    is_current: true,
                    rationale: None,
                    context_summary: None,
                    rejected_alternatives: Vec::new(),
                    significance: Some("minor".to_string()),
                    is_resolved: Some(true),
                    resolution_note: None,
                };
                actions.push(write_action);
                previous_action_id = Some(write_id);
            } else {
                previous_action_id = Some(plan_id);
            }
        }

        Ok(actions)
    }
}

/// Helper to execute `git log` directly inside a repository directory.
pub fn run_git_log(repo_path: &Path, max_commits: usize) -> KnoxResult<Vec<KnoxAction>> {
    let limit_arg = if max_commits == 0 {
        "-n 500".to_string()
    } else {
        format!("-n {max_commits}")
    };

    let output = Command::new("git")
        .current_dir(repo_path)
        .args([
            "log",
            &limit_arg,
            "--raw",
            "--date=iso-strict",
            "--pretty=format:COMMIT_START%n%H%n%an%n%ad%n%s%n%b%nFILES_START",
        ])
        .output()
        .map_err(|e| KnoxError::Git(format!("Failed to execute git command: {e}")))?;

    if !output.status.success() {
        let err_msg = String::from_utf8_lossy(&output.stderr);
        return Err(KnoxError::Git(format!("Git log failed: {err_msg}")));
    }

    let mapper = GitLogMapper;
    mapper.map(&output.stdout)
}

#[derive(Clone, Debug, Default)]
struct ParsedCommit {
    hash: String,
    author: String,
    timestamp: DateTime<Utc>,
    subject: String,
    body: String,
    files: Vec<String>,
}

fn parse_git_log_text(text: &str) -> Vec<ParsedCommit> {
    let mut commits = Vec::new();

    if text.contains("COMMIT_START") {
        for block in text.split("COMMIT_START\n") {
            let trimmed = block.trim();
            if trimmed.is_empty() {
                continue;
            }

            let mut lines = trimmed.lines();
            let hash = lines.next().unwrap_or("").trim().to_string();
            let author = lines.next().unwrap_or("").trim().to_string();
            let date_str = lines.next().unwrap_or("").trim();
            let subject = lines.next().unwrap_or("").trim().to_string();

            let timestamp = DateTime::parse_from_rfc3339(date_str)
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now());

            let mut body_lines = Vec::new();
            let mut files = Vec::new();
            let mut in_files = false;

            for line in lines {
                let l = line.trim();
                if l == "FILES_START" {
                    in_files = true;
                    continue;
                }

                if in_files {
                    if l.starts_with(':') {
                        // Raw format: :100644 100644 bcd1234 0123456 M  file.rs
                        if let Some(tab_pos) = l.find('\t') {
                            let file_path = l[tab_pos + 1..].trim();
                            if !file_path.is_empty() {
                                files.push(file_path.to_string());
                            }
                        }
                    } else if !l.is_empty() {
                        files.push(l.to_string());
                    }
                } else if !l.is_empty() {
                    body_lines.push(l);
                }
            }

            commits.push(ParsedCommit {
                hash,
                author,
                timestamp,
                subject,
                body: body_lines.join("\n"),
                files,
            });
        }
    } else {
        // Fallback for standard git log: "commit <hash>\nAuthor: <author>\nDate: <date>\n\n<message>\n"
        let mut current: Option<ParsedCommit> = None;

        for line in text.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("commit ") && trimmed.len() >= 47 {
                if let Some(c) = current.take() {
                    commits.push(c);
                }
                current = Some(ParsedCommit {
                    hash: trimmed[7..].trim().to_string(),
                    timestamp: Utc::now(),
                    ..Default::default()
                });
            } else if let Some(ref mut c) = current {
                if let Some(author) = trimmed.strip_prefix("Author:") {
                    c.author = author.trim().to_string();
                } else if let Some(date_str) = trimmed.strip_prefix("Date:") {
                    let d = date_str.trim();
                    if let Ok(dt) = DateTime::parse_from_rfc2822(d) {
                        c.timestamp = dt.with_timezone(&Utc);
                    }
                } else if !trimmed.is_empty() && c.subject.is_empty() {
                    c.subject = trimmed.to_string();
                } else if !trimmed.is_empty() {
                    if !c.body.is_empty() {
                        c.body.push('\n');
                    }
                    c.body.push_str(trimmed);
                }
            }
        }

        if let Some(c) = current {
            commits.push(c);
        }
    }

    commits
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_can_parse_git_raw_log() {
        let mapper = GitLogMapper;
        let log = b"COMMIT_START\na1b2c3d4e5f6\nAlice\n2026-10-30T10:00:00Z\nAdd login form\n\nFILES_START\nsrc/login.rs\n";
        assert!(mapper.can_parse(log));
    }

    #[test]
    fn test_map_git_log() {
        let mapper = GitLogMapper;
        let log = b"COMMIT_START\na1b2c3d4e5f6\nAlice\n2026-10-30T10:00:00Z\nAdd login form\nImplements user auth\nFILES_START\nsrc/login.rs\n";
        let actions = mapper.map(log).unwrap();
        assert_eq!(actions.len(), 2);
        assert_eq!(actions[0].action_type, "plan");
        assert_eq!(actions[0].summary, "Add login form");
        assert_eq!(actions[1].action_type, "write");
        assert_eq!(actions[1].files, vec!["src/login.rs"]);
    }
}
