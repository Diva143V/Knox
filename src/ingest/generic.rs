//! Generic transcript mapper implementation.
//! Provides a resilient heuristic fallback for Markdown chat notes, plain text conversations,
//! and generic JSON arrays `[{ role, content }]`.

use chrono::Utc;
use serde_json::Value;

use super::action_group::{ActionGrouper, TranscriptStep};
use super::mapper::TranscriptMapper;
use crate::core::error::{KnoxError, KnoxResult};
use crate::core::types::KnoxAction;

/// Flexible fallback mapper for generic Markdown, plain text, or arbitrary JSON chat logs.
pub struct GenericTranscriptMapper;

impl TranscriptMapper for GenericTranscriptMapper {
    fn tool_name(&self) -> &str {
        "generic"
    }

    fn can_parse(&self, _raw: &[u8]) -> bool {
        // As the fallback mapper registered last, it can parse any non-empty UTF-8 text
        true
    }

    fn map(&self, raw: &[u8]) -> KnoxResult<Vec<KnoxAction>> {
        let text = std::str::from_utf8(raw)
            .map_err(|e| KnoxError::ParseError(format!("Invalid UTF-8 in transcript: {e}")))?;

        let trimmed = text.trim();
        if trimmed.is_empty() {
            return Ok(Vec::new());
        }

        let mut grouper = ActionGrouper::new(self.tool_name());
        let mut all_actions = Vec::new();

        // 1. Check if it's generic JSON
        if trimmed.starts_with('{') || trimmed.starts_with('[') {
            if let Ok(val) = serde_json::from_str::<Value>(trimmed) {
                let messages = if let Some(arr) = val.as_array() {
                    arr.as_slice()
                } else if let Some(arr) = val.get("messages").and_then(|m| m.as_array()) {
                    arr.as_slice()
                } else if let Some(arr) = val.get("conversation").and_then(|c| c.as_array()) {
                    arr.as_slice()
                } else {
                    &[]
                };

                if !messages.is_empty() {
                    for (idx, item) in messages.iter().enumerate() {
                        if let Some(step) = parse_generic_json_item(item, idx as i64) {
                            all_actions.extend(grouper.process_step(&step));
                        }
                    }
                    all_actions.extend(grouper.finish());
                    if !all_actions.is_empty() {
                        return Ok(all_actions);
                    }
                }
            }
        }

        // 2. Markdown or Plain Text conversation heuristic
        let steps = parse_markdown_or_plaintext(trimmed);
        for step in steps {
            all_actions.extend(grouper.process_step(&step));
        }

        all_actions.extend(grouper.finish());
        Ok(all_actions)
    }
}

fn parse_generic_json_item(val: &Value, idx: i64) -> Option<TranscriptStep> {
    let role = val
        .get("role")
        .or_else(|| val.get("sender"))
        .or_else(|| val.get("author"))
        .or_else(|| val.get("type"))
        .and_then(|r| {
            if let Some(s) = r.as_str() {
                Some(s)
            } else if let Some(obj) = r.as_object() {
                obj.get("role").and_then(|s| s.as_str())
            } else {
                None
            }
        })
        .unwrap_or("");

    let is_user = role.eq_ignore_ascii_case("user") || role.eq_ignore_ascii_case("human");

    let content = val
        .get("content")
        .or_else(|| val.get("text"))
        .or_else(|| val.get("message"))
        .and_then(|c| {
            if let Some(s) = c.as_str() {
                Some(s.to_string())
            } else if let Some(arr) = c.as_array() {
                let parts: Vec<&str> = arr
                    .iter()
                    .filter_map(|p| p.get("text").and_then(|t| t.as_str()).or_else(|| p.as_str()))
                    .collect();
                Some(parts.join("\n"))
            } else {
                None
            }
        })
        .unwrap_or_default();

    if content.trim().is_empty() {
        return None;
    }

    if is_user {
        Some(TranscriptStep {
            step_index: Some(idx),
            source: Some("USER_EXPLICIT".to_string()),
            step_type: Some("USER_INPUT".to_string()),
            status: Some("DONE".to_string()),
            created_at: Some(Utc::now().to_rfc3339()),
            content: Some(content),
            thinking: None,
            tool_calls: None,
        })
    } else {
        Some(TranscriptStep {
            step_index: Some(idx),
            source: Some("MODEL".to_string()),
            step_type: Some("PLANNER_RESPONSE".to_string()),
            status: Some("DONE".to_string()),
            created_at: Some(Utc::now().to_rfc3339()),
            content: Some(content),
            thinking: None,
            tool_calls: None,
        })
    }
}

fn parse_markdown_or_plaintext(text: &str) -> Vec<TranscriptStep> {
    let mut steps = Vec::new();
    let mut current_role = "user";
    let mut current_lines: Vec<&str> = Vec::new();
    let mut step_counter: i64 = 0;

    let flush_current = |steps: &mut Vec<TranscriptStep>, role: &str, lines: &[&str], counter: &mut i64| {
        let content = lines.join("\n").trim().to_string();
        if !content.is_empty() {
            let is_user = role == "user";
            steps.push(TranscriptStep {
                step_index: Some(*counter),
                source: Some(if is_user { "USER_EXPLICIT".to_string() } else { "MODEL".to_string() }),
                step_type: Some(if is_user { "USER_INPUT".to_string() } else { "PLANNER_RESPONSE".to_string() }),
                status: Some("DONE".to_string()),
                created_at: Some(Utc::now().to_rfc3339()),
                content: Some(content),
                thinking: None,
                tool_calls: None,
            });
            *counter += 1;
        }
    };

    for line in text.lines() {
        let trimmed = line.trim();
        let lower = trimmed.to_lowercase();

        let is_user_heading = lower.starts_with("# user")
            || lower.starts_with("## user")
            || lower.starts_with("### user")
            || lower.starts_with("**user:**")
            || lower.starts_with("> user:")
            || lower.starts_with("user:");

        let is_agent_heading = lower.starts_with("# assistant")
            || lower.starts_with("## assistant")
            || lower.starts_with("### assistant")
            || lower.starts_with("# agent")
            || lower.starts_with("## agent")
            || lower.starts_with("**assistant:**")
            || lower.starts_with("**agent:**")
            || lower.starts_with("assistant:")
            || lower.starts_with("agent:");

        if is_user_heading {
            flush_current(&mut steps, current_role, &current_lines, &mut step_counter);
            current_lines.clear();
            current_role = "user";
        } else if is_agent_heading {
            flush_current(&mut steps, current_role, &current_lines, &mut step_counter);
            current_lines.clear();
            current_role = "assistant";
        } else {
            current_lines.push(line);
        }
    }

    flush_current(&mut steps, current_role, &current_lines, &mut step_counter);
    steps
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_map_markdown_chat() {
        let mapper = GenericTranscriptMapper;
        let md = b"## User\nRefactor authentication layer\n\n## Assistant\nRefactored token verification in src/auth.rs\n";
        let actions = mapper.map(md).unwrap();
        assert!(!actions.is_empty());
        assert_eq!(actions[0].action_type, "plan");
        assert!(actions[0].summary.contains("Refactor authentication layer"));
    }
}

