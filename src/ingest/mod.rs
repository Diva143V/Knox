//! Transcript ingestion and mapping pipelines for Knox.
//!
//! Provides the `TranscriptMapper` trait and tool-specific mappers (Antigravity, Claude,
//! Cursor, OpenAI/ChatGPT, Git logs, and Generic Markdown/JSON) to ingest conversational
//! transcripts into persistent knowledge graph actions.

pub mod action_group;
pub mod antigravity;
pub mod claude;
pub mod cursor;
pub mod generic;
pub mod git_log;
pub mod mapper;
pub mod openai;
pub mod summary;

use std::path::Path;

use crate::core::error::{KnoxError, KnoxResult};
use crate::core::types::{DetectedTranscript, KnoxAction};

pub use action_group::*;
pub use antigravity::*;
pub use claude::*;
pub use cursor::*;
pub use generic::*;
pub use git_log::*;
pub use mapper::*;
pub use openai::*;
pub use summary::*;

/// Returns an ordered list of all registered transcript mappers.
pub fn all_mappers() -> Vec<Box<dyn TranscriptMapper>> {
    vec![
        Box::new(antigravity::AntigravityMapper),
        Box::new(claude::ClaudeMapper),
        Box::new(cursor::CursorMapper),
        Box::new(openai::OpenAiMapper),
        Box::new(git_log::GitLogMapper),
        Box::new(generic::GenericTranscriptMapper),
    ]
}

/// Automatically detects the appropriate mapper for a transcript file and maps it into actions.
pub fn ingest_file(path: &Path) -> KnoxResult<Vec<KnoxAction>> {
    let (_, actions) = ingest_file_with_tool(path)?;
    Ok(actions)
}

/// Automatically detects the appropriate mapper for a transcript file, returning tool name and actions.
pub fn ingest_file_with_tool(path: &Path) -> KnoxResult<(String, Vec<KnoxAction>)> {
    let bytes = std::fs::read(path)?;
    ingest_bytes_with_tool(&bytes)
}

/// Maps in-memory raw transcript bytes using the first suitable mapper, returning tool name and actions.
pub fn ingest_bytes_with_tool(bytes: &[u8]) -> KnoxResult<(String, Vec<KnoxAction>)> {
    for mapper in all_mappers() {
        if mapper.can_parse(bytes) {
            let tool = mapper.tool_name().to_string();
            let actions = mapper.map(bytes)?;
            return Ok((tool, actions));
        }
    }

    Err(KnoxError::ParseError(
        "No suitable transcript mapper found for provided content".to_string(),
    ))
}

/// Scans local workspace and agent storage directories for available conversation transcripts.
pub fn detect_local_transcripts(project_root: Option<&Path>) -> Vec<DetectedTranscript> {
    let mut detected = Vec::new();

    // 1. Check Git history in current project root
    if let Some(root) = project_root {
        let git_dir = root.join(".git");
        if git_dir.exists() {
            detected.push(DetectedTranscript {
                path: root.display().to_string(),
                source: "Git History".to_string(),
                size_bytes: 0,
                last_modified: chrono::Utc::now().to_rfc3339(),
                estimated_actions: 50,
            });
        }

        // Check for local project transcripts (.jsonl, transcript.jsonl)
        if let Ok(entries) = std::fs::read_dir(root) {
            for entry in entries.flatten() {
                let p = entry.path();
                if let Some(name) = p.file_name().and_then(|n| n.to_str()) {
                    let lower = name.to_lowercase();
                    if lower.ends_with(".jsonl") || lower.contains("transcript") {
                        let size = entry.metadata().map(|m| m.len()).unwrap_or(0);
                        detected.push(DetectedTranscript {
                            path: p.display().to_string(),
                            source: if lower.contains("antigravity") {
                                "Antigravity".to_string()
                            } else if lower.contains("claude") {
                                "Claude Code".to_string()
                            } else if lower.contains("cursor") {
                                "Cursor".to_string()
                            } else {
                                "Generic Transcript".to_string()
                            },
                            size_bytes: size,
                            last_modified: chrono::Utc::now().to_rfc3339(),
                            estimated_actions: (size / 500).max(1) as usize,
                        });
                    }
                }
            }
        }
    }

    // 2. Check Antigravity brain transcripts in user home directory
    if let Some(home) = crate::core::config::home_dir() {
        let antigravity_brain = home.join(".gemini").join("antigravity").join("brain");
        if antigravity_brain.exists() {
            if let Ok(conv_dirs) = std::fs::read_dir(&antigravity_brain) {
                for conv in conv_dirs.flatten().take(5) {
                    let transcript_path = conv
                        .path()
                        .join(".system_generated")
                        .join("logs")
                        .join("transcript.jsonl");
                    if transcript_path.exists() {
                        let size = std::fs::metadata(&transcript_path)
                            .map(|m| m.len())
                            .unwrap_or(0);
                        detected.push(DetectedTranscript {
                            path: transcript_path.display().to_string(),
                            source: "Antigravity".to_string(),
                            size_bytes: size,
                            last_modified: chrono::Utc::now().to_rfc3339(),
                            estimated_actions: (size / 400).max(1) as usize,
                        });
                    }
                }
            }
        }

        // 3. Check Claude Code project logs
        let claude_projects = home.join(".claude").join("projects");
        if claude_projects.exists() {
            if let Ok(projects) = std::fs::read_dir(&claude_projects) {
                for proj in projects.flatten().take(5) {
                    let p = proj.path();
                    let size = std::fs::metadata(&p).map(|m| m.len()).unwrap_or(0);
                    detected.push(DetectedTranscript {
                        path: p.display().to_string(),
                        source: "Claude Code".to_string(),
                        size_bytes: size,
                        last_modified: chrono::Utc::now().to_rfc3339(),
                        estimated_actions: (size / 500).max(1) as usize,
                    });
                }
            }
        }
    }

    detected
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_all_mappers_contains_all_agent_suites() {
        let mappers = all_mappers();
        let tools: Vec<&str> = mappers.iter().map(|m| m.tool_name()).collect();
        assert!(tools.contains(&"antigravity"));
        assert!(tools.contains(&"claude"));
        assert!(tools.contains(&"cursor"));
        assert!(tools.contains(&"chatgpt"));
        assert!(tools.contains(&"git"));
        assert!(tools.contains(&"generic"));
    }

    #[test]
    fn test_ingest_file_auto_detection() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(
            file,
            r#"{{"step_index":0,"source":"USER_EXPLICIT","type":"USER_INPUT","content":"Build feature"}}"#
        )
        .unwrap();
        writeln!(
            file,
            r#"{{"step_index":1,"source":"MODEL","type":"PLANNER_RESPONSE","content":"Feature built"}}"#
        )
        .unwrap();

        let (tool, actions) = ingest_file_with_tool(file.path()).unwrap();
        assert_eq!(tool, "antigravity");
        assert_eq!(actions.len(), 2);
        assert_eq!(actions[0].summary, "Build feature");
        assert_eq!(actions[1].summary, "Feature built");
    }
}
