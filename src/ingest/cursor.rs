//! Cursor Composer transcript mapper implementation.
//! Parses Cursor Composer exports and workspace storage JSON into Knox actions.

use chrono::Utc;
use serde_json::Value;

use super::action_group::{ActionGrouper, TranscriptStep, TranscriptToolCall};
use super::mapper::TranscriptMapper;
use crate::core::error::{KnoxError, KnoxResult};
use crate::core::types::KnoxAction;

/// Mapper for Cursor Composer session transcripts.
pub struct CursorMapper;

impl TranscriptMapper for CursorMapper {
    fn tool_name(&self) -> &str {
        "cursor"
    }

    fn can_parse(&self, raw: &[u8]) -> bool {
        let Ok(text) = std::str::from_utf8(raw) else {
            return false;
        };

        // Check for Cursor Composer signatures
        if text.contains("\"composerData\"")
            || text.contains("\"bubbleId\"")
            || (text.contains("\"cursor\"") && text.contains("\"messages\""))
            || text.contains("\"codeBlocks\"")
        {
            return true;
        }

        false
    }

    fn map(&self, raw: &[u8]) -> KnoxResult<Vec<KnoxAction>> {
        let text = std::str::from_utf8(raw)
            .map_err(|e| KnoxError::ParseError(format!("Invalid UTF-8 in Cursor transcript: {e}")))?;

        let mut grouper = ActionGrouper::new(self.tool_name());
        let mut all_actions = Vec::new();

        if let Ok(root) = serde_json::from_str::<Value>(text.trim()) {
            let messages = if let Some(arr) = root.get("messages").and_then(|m| m.as_array()) {
                arr.as_slice()
            } else if let Some(comp) = root.get("composerData").and_then(|c| c.get("messages")).and_then(|m| m.as_array()) {
                comp.as_slice()
            } else if let Some(arr) = root.as_array() {
                arr.as_slice()
            } else {
                &[]
            };

            for (idx, msg) in messages.iter().enumerate() {
                let steps = extract_steps_from_cursor_message(msg, idx as i64);
                for step in steps {
                    all_actions.extend(grouper.process_step(&step));
                }
            }
        }

        all_actions.extend(grouper.finish());
        Ok(all_actions)
    }
}

/// Converts a Cursor message node into `TranscriptStep` items.
fn extract_steps_from_cursor_message(msg: &Value, idx: i64) -> Vec<TranscriptStep> {
    let mut steps = Vec::new();

    let sender = msg
        .get("sender")
        .or_else(|| msg.get("type"))
        .or_else(|| msg.get("role"))
        .and_then(|s| s.as_str())
        .unwrap_or("");

    let is_user = sender.eq_ignore_ascii_case("user") || sender.eq_ignore_ascii_case("human");
    let is_assistant = sender.eq_ignore_ascii_case("ai")
        || sender.eq_ignore_ascii_case("assistant")
        || sender.eq_ignore_ascii_case("bot");

    let timestamp = msg
        .get("timestamp")
        .or_else(|| msg.get("createdAt"))
        .and_then(|t| {
            if let Some(s) = t.as_str() {
                Some(s.to_string())
            } else if let Some(n) = t.as_i64() {
                chrono::DateTime::from_timestamp_millis(n)
                    .map(|dt| dt.to_rfc3339())
            } else {
                None
            }
        })
        .unwrap_or_else(|| Utc::now().to_rfc3339());

    if is_user {
        let content = msg
            .get("text")
            .or_else(|| msg.get("content"))
            .and_then(|t| t.as_str())
            .unwrap_or_default();

        if !content.trim().is_empty() {
            steps.push(TranscriptStep {
                step_index: Some(idx),
                source: Some("USER_EXPLICIT".to_string()),
                step_type: Some("USER_INPUT".to_string()),
                status: Some("DONE".to_string()),
                created_at: Some(timestamp),
                content: Some(content.to_string()),
                thinking: None,
                tool_calls: None,
            });
        }
    } else if is_assistant {
        let content = msg
            .get("text")
            .or_else(|| msg.get("content"))
            .and_then(|t| t.as_str())
            .unwrap_or_default();

        let mut tool_calls = Vec::new();

        // Check for attached codeBlocks or file diffs
        if let Some(blocks) = msg.get("codeBlocks").and_then(|b| b.as_array()) {
            for block in blocks {
                let file_path = block
                    .get("relativePath")
                    .or_else(|| block.get("uri"))
                    .or_else(|| block.get("path"))
                    .and_then(|p| p.as_str())
                    .unwrap_or("");

                if !file_path.is_empty() {
                    let mut args = serde_json::Map::new();
                    args.insert("Path".to_string(), Value::String(file_path.to_string()));
                    tool_calls.push(TranscriptToolCall {
                        name: "edit_file".to_string(),
                        args: Some(Value::Object(args)),
                    });
                }
            }
        }

        // Heuristic extraction of edited files from Markdown codeblocks: ```lang path/to/file
        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("```") {
                let rest = trimmed.trim_start_matches('`').trim();
                let parts: Vec<&str> = rest.split_whitespace().collect();
                for part in parts {
                    if (part.contains('.') || part.contains('/') || part.contains('\\'))
                        && !part.ends_with(':')
                        && part.len() > 3
                    {
                        let mut args = serde_json::Map::new();
                        args.insert("Path".to_string(), Value::String(part.to_string()));
                        tool_calls.push(TranscriptToolCall {
                            name: "edit_file".to_string(),
                            args: Some(Value::Object(args)),
                        });
                    }
                }
            }
        }

        steps.push(TranscriptStep {
            step_index: Some(idx),
            source: Some("MODEL".to_string()),
            step_type: Some("PLANNER_RESPONSE".to_string()),
            status: Some("DONE".to_string()),
            created_at: Some(timestamp),
            content: if content.is_empty() { None } else { Some(content.to_string()) },
            thinking: None,
            tool_calls: if tool_calls.is_empty() { None } else { Some(tool_calls) },
        });
    }

    steps
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_can_parse_cursor_composer_json() {
        let mapper = CursorMapper;
        let data = br#"{"composerData":{"messages":[{"sender":"user","text":"Build layout"}]}}"#;
        assert!(mapper.can_parse(data));
    }

    #[test]
    fn test_map_cursor_messages() {
        let mapper = CursorMapper;
        let data = br#"{
            "messages": [
                {
                    "sender": "user",
                    "text": "Optimize SQL queries for graph canvas"
                },
                {
                    "sender": "ai",
                    "text": "Modifying queries in sqlite.rs",
                    "codeBlocks": [
                        {
                            "relativePath": "src/storage/db.rs"
                        }
                    ]
                }
            ]
        }"#;

        let actions = mapper.map(data).unwrap();
        assert!(!actions.is_empty());
        assert_eq!(actions[0].actor, "human");
        assert_eq!(actions[0].action_type, "plan");
        assert!(actions[0].summary.contains("Optimize SQL queries"));
        assert!(actions.iter().any(|a| a.actor == "cursor"));
    }
}
