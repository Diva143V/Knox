//! Claude (Code & Desktop) transcript mapper implementation.
//! Parses Claude Code JSONL/JSON logs and Claude Desktop chat exports into Knox actions.

use chrono::Utc;
use serde_json::Value;

use super::action_group::{ActionGrouper, TranscriptStep, TranscriptToolCall};
use super::mapper::TranscriptMapper;
use crate::core::error::{KnoxError, KnoxResult};
use crate::core::types::KnoxAction;

/// Mapper for Claude Code CLI and Claude Desktop conversation transcripts.
pub struct ClaudeMapper;

impl TranscriptMapper for ClaudeMapper {
    fn tool_name(&self) -> &str {
        "claude"
    }

    fn can_parse(&self, raw: &[u8]) -> bool {
        let Ok(text) = std::str::from_utf8(raw) else {
            return false;
        };

        // Check for Claude Desktop export signature
        if text.contains("\"chat_messages\"") || (text.contains("\"sender\"") && text.contains("\"human\"")) {
            return true;
        }

        // Check for Claude Code tool_use signature
        if text.contains("\"tool_use\"") && (text.contains("\"Edit\"") || text.contains("\"Write\"") || text.contains("\"Bash\"")) {
            return true;
        }

        // Check for JSONL with Claude model or role with claude metadata
        for line in text.lines().take(20) {
            let trimmed = line.trim_start_matches('\u{feff}').trim();
            if trimmed.is_empty() {
                continue;
            }
            if let Ok(Value::Object(map)) = serde_json::from_str::<Value>(trimmed) {
                if let Some(model) = map.get("model").and_then(|m| m.as_str()) {
                    if model.contains("claude") {
                        return true;
                    }
                }
                if let Some(tool) = map.get("tool").and_then(|t| t.as_str()) {
                    if tool.eq_ignore_ascii_case("claude") {
                        return true;
                    }
                }
            }
        }

        false
    }

    fn map(&self, raw: &[u8]) -> KnoxResult<Vec<KnoxAction>> {
        let text = std::str::from_utf8(raw)
            .map_err(|e| KnoxError::ParseError(format!("Invalid UTF-8 in Claude transcript: {e}")))?;

        let mut grouper = ActionGrouper::new(self.tool_name());
        let mut all_actions = Vec::new();

        // 1. Try JSON Array (Claude Desktop export or array of messages)
        if let Ok(Value::Array(arr)) = serde_json::from_str::<Value>(text.trim()) {
            for item in arr {
                let steps = extract_steps_from_claude_value(&item);
                for step in steps {
                    all_actions.extend(grouper.process_step(&step));
                }
            }
            all_actions.extend(grouper.finish());
            if !all_actions.is_empty() {
                return Ok(all_actions);
            }
        }

        // 2. Try JSON Object with chat_messages or messages array
        if let Ok(Value::Object(root)) = serde_json::from_str::<Value>(text.trim()) {
            if let Some(Value::Array(messages)) = root.get("chat_messages").or_else(|| root.get("messages")) {
                for item in messages {
                    let steps = extract_steps_from_claude_value(item);
                    for step in steps {
                        all_actions.extend(grouper.process_step(&step));
                    }
                }
                all_actions.extend(grouper.finish());
                if !all_actions.is_empty() {
                    return Ok(all_actions);
                }
            }
        }

        // 3. Line-delimited JSON (JSONL - Claude Code)
        for (idx, line) in text.lines().enumerate() {
            let trimmed = line.trim_start_matches('\u{feff}').trim();
            if trimmed.is_empty() {
                continue;
            }

            if let Ok(val) = serde_json::from_str::<Value>(trimmed) {
                let steps = extract_steps_from_claude_value(&val);
                for mut step in steps {
                    if step.step_index.is_none() {
                        step.step_index = Some(idx as i64);
                    }
                    all_actions.extend(grouper.process_step(&step));
                }
            }
        }

        all_actions.extend(grouper.finish());
        Ok(all_actions)
    }
}

/// Converts a Claude JSON message value into standardized `TranscriptStep` items.
fn extract_steps_from_claude_value(val: &Value) -> Vec<TranscriptStep> {
    let mut steps = Vec::new();

    let role = val
        .get("role")
        .or_else(|| val.get("sender"))
        .and_then(|r| r.as_str())
        .unwrap_or("");

    let is_user = role.eq_ignore_ascii_case("user") || role.eq_ignore_ascii_case("human");
    let is_assistant = role.eq_ignore_ascii_case("assistant") || role.eq_ignore_ascii_case("bot");

    let timestamp = val
        .get("created_at")
        .or_else(|| val.get("timestamp"))
        .and_then(|t| t.as_str())
        .map(String::from)
        .unwrap_or_else(|| Utc::now().to_rfc3339());

    if is_user {
        let content = val
            .get("text")
            .or_else(|| val.get("content"))
            .and_then(|c| {
                if let Some(s) = c.as_str() {
                    Some(s.to_string())
                } else if let Some(arr) = c.as_array() {
                    // Extract text blocks
                    let parts: Vec<&str> = arr
                        .iter()
                        .filter_map(|b| {
                            if b.get("type").and_then(|t| t.as_str()) == Some("text") {
                                b.get("text").and_then(|t| t.as_str())
                            } else {
                                None
                            }
                        })
                        .collect();
                    Some(parts.join("\n"))
                } else {
                    None
                }
            })
            .unwrap_or_default();

        if !content.trim().is_empty() {
            steps.push(TranscriptStep {
                step_index: None,
                source: Some("USER_EXPLICIT".to_string()),
                step_type: Some("USER_INPUT".to_string()),
                status: Some("DONE".to_string()),
                created_at: Some(timestamp),
                content: Some(content),
                thinking: None,
                tool_calls: None,
            });
        }
    } else if is_assistant {
        let mut tool_calls = Vec::new();
        let mut text_parts = Vec::new();

        if let Some(content_arr) = val.get("content").and_then(|c| c.as_array()) {
            for block in content_arr {
                let block_type = block.get("type").and_then(|t| t.as_str()).unwrap_or("");
                if block_type == "text" {
                    if let Some(t) = block.get("text").and_then(|t| t.as_str()) {
                        text_parts.push(t.to_string());
                    }
                } else if block_type == "tool_use" {
                    let tool_name = block.get("name").and_then(|n| n.as_str()).unwrap_or("unknown");
                    let input = block.get("input").cloned().unwrap_or(Value::Null);

                    // Map Claude tool names to standard Knox tool call arguments
                    let normalized_name = match tool_name {
                        "Edit" | "replace" => "edit_file",
                        "Write" | "write_file" => "write_file",
                        "Bash" | "bash" | "execute" => "run_command",
                        other => other,
                    };

                    let mut mapped_args = serde_json::Map::new();
                    if let Value::Object(inp) = input {
                        for (k, v) in inp {
                            if k == "file_path" || k == "path" {
                                mapped_args.insert("Path".to_string(), v);
                            } else if k == "command" {
                                mapped_args.insert("CommandLine".to_string(), v);
                            } else {
                                mapped_args.insert(k, v);
                            }
                        }
                    }

                    tool_calls.push(TranscriptToolCall {
                        name: normalized_name.to_string(),
                        args: Some(Value::Object(mapped_args)),
                    });
                }
            }
        } else if let Some(t) = val.get("text").and_then(|t| t.as_str()) {
            text_parts.push(t.to_string());
        }

        steps.push(TranscriptStep {
            step_index: None,
            source: Some("MODEL".to_string()),
            step_type: Some("PLANNER_RESPONSE".to_string()),
            status: Some("DONE".to_string()),
            created_at: Some(timestamp),
            content: if text_parts.is_empty() { None } else { Some(text_parts.join("\n")) },
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
    fn test_can_parse_claude_desktop_export() {
        let mapper = ClaudeMapper;
        let desktop = br#"{"chat_messages":[{"sender":"human","text":"Refactor the DB layer"},{"sender":"assistant","text":"I will refactor SQLite."}]}"#;
        assert!(mapper.can_parse(desktop));
    }

    #[test]
    fn test_can_parse_claude_code_tool_use() {
        let mapper = ClaudeMapper;
        let code_jsonl = br#"{"role":"assistant","content":[{"type":"tool_use","name":"Edit","input":{"file_path":"src/db.rs"}}]}"#;
        assert!(mapper.can_parse(code_jsonl));
    }

    #[test]
    fn test_map_claude_conversation() {
        let mapper = ClaudeMapper;
        let conversation = br#"{
            "chat_messages": [
                {
                    "sender": "human",
                    "text": "Fix connection timeout in db.rs"
                },
                {
                    "sender": "assistant",
                    "content": [
                        {
                            "type": "text",
                            "text": "Modifying connection timeout configuration."
                        },
                        {
                            "type": "tool_use",
                            "name": "Edit",
                            "input": {
                                "file_path": "src/db.rs"
                            }
                        }
                    ]
                }
            ]
        }"#;

        let actions = mapper.map(conversation).unwrap();
        assert!(!actions.is_empty());
        assert_eq!(actions[0].actor, "human");
        assert_eq!(actions[0].action_type, "plan");
        assert!(actions[0].summary.contains("Fix connection timeout"));
        assert!(actions.iter().any(|a| a.actor == "claude"));
    }
}
