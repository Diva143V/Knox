//! OpenAI & ChatGPT transcript mapper implementation.
//! Parses ChatGPT data export (`conversations.json`) and OpenAI chat completion arrays into Knox actions.

use chrono::Utc;
use serde_json::Value;

use super::action_group::{ActionGrouper, TranscriptStep};
use super::mapper::TranscriptMapper;
use crate::core::error::{KnoxError, KnoxResult};
use crate::core::types::KnoxAction;

/// Mapper for ChatGPT and OpenAI conversation logs.
pub struct OpenAiMapper;

impl TranscriptMapper for OpenAiMapper {
    fn tool_name(&self) -> &str {
        "chatgpt"
    }

    fn can_parse(&self, raw: &[u8]) -> bool {
        let Ok(text) = std::str::from_utf8(raw) else {
            return false;
        };

        // Check for ChatGPT export mapping tree structure
        if text.contains("\"mapping\"")
            && text.contains("\"message\"")
            && text.contains("\"author\"")
        {
            return true;
        }

        // Check for OpenAI messages array
        if (text.contains("\"messages\"") || text.starts_with('['))
            && (text.contains("\"role\"") && text.contains("\"user\""))
            && (text.contains("gpt-")
                || text.contains("o1-")
                || text.contains("o3-")
                || text.contains("\"assistant\""))
        {
            return true;
        }

        false
    }

    fn map(&self, raw: &[u8]) -> KnoxResult<Vec<KnoxAction>> {
        let text = std::str::from_utf8(raw).map_err(|e| {
            KnoxError::ParseError(format!("Invalid UTF-8 in OpenAI transcript: {e}"))
        })?;

        let mut grouper = ActionGrouper::new(self.tool_name());
        let mut all_actions = Vec::new();

        if let Ok(root) = serde_json::from_str::<Value>(text.trim()) {
            // Case 1: ChatGPT conversations export array
            if let Some(convs) = root.as_array() {
                for conv in convs {
                    if let Some(mapping) = conv.get("mapping").and_then(|m| m.as_object()) {
                        let mut nodes: Vec<&Value> = mapping.values().collect();
                        nodes.sort_by_key(|n| {
                            n.get("message")
                                .and_then(|m| m.get("create_time"))
                                .and_then(|t| t.as_f64())
                                .map(|f| (f * 1000.0) as i64)
                                .unwrap_or(0)
                        });

                        for (idx, node) in nodes.into_iter().enumerate() {
                            if let Some(msg) = node.get("message") {
                                if let Some(step) =
                                    extract_step_from_openai_message(msg, idx as i64)
                                {
                                    all_actions.extend(grouper.process_step(&step));
                                }
                            }
                        }
                    } else {
                        // Array of standard messages
                        for (idx, item) in convs.iter().enumerate() {
                            if let Some(step) = extract_step_from_openai_message(item, idx as i64) {
                                all_actions.extend(grouper.process_step(&step));
                            }
                        }
                        break;
                    }
                }
            } else if let Some(mapping) = root.get("mapping").and_then(|m| m.as_object()) {
                // Case 2: Single conversation object with mapping
                let mut nodes: Vec<&Value> = mapping.values().collect();
                nodes.sort_by_key(|n| {
                    n.get("message")
                        .and_then(|m| m.get("create_time"))
                        .and_then(|t| t.as_f64())
                        .map(|f| (f * 1000.0) as i64)
                        .unwrap_or(0)
                });

                for (idx, node) in nodes.into_iter().enumerate() {
                    if let Some(msg) = node.get("message") {
                        if let Some(step) = extract_step_from_openai_message(msg, idx as i64) {
                            all_actions.extend(grouper.process_step(&step));
                        }
                    }
                }
            } else if let Some(messages) = root.get("messages").and_then(|m| m.as_array()) {
                // Case 3: Object with messages array
                for (idx, item) in messages.iter().enumerate() {
                    if let Some(step) = extract_step_from_openai_message(item, idx as i64) {
                        all_actions.extend(grouper.process_step(&step));
                    }
                }
            }
        }

        all_actions.extend(grouper.finish());
        Ok(all_actions)
    }
}

/// Converts an OpenAI / ChatGPT message object into a `TranscriptStep`.
fn extract_step_from_openai_message(msg: &Value, idx: i64) -> Option<TranscriptStep> {
    let role = msg
        .get("author")
        .and_then(|a| a.get("role"))
        .or_else(|| msg.get("role"))
        .and_then(|r| r.as_str())
        .unwrap_or("");

    let is_user = role.eq_ignore_ascii_case("user");
    let is_assistant = role.eq_ignore_ascii_case("assistant");

    if !is_user && !is_assistant {
        return None;
    }

    let timestamp = msg
        .get("create_time")
        .and_then(|t| t.as_f64())
        .and_then(|f| chrono::DateTime::from_timestamp(f as i64, 0))
        .map(|dt| dt.to_rfc3339())
        .unwrap_or_else(|| Utc::now().to_rfc3339());

    let content = if let Some(parts) = msg
        .get("content")
        .and_then(|c| c.get("parts"))
        .and_then(|p| p.as_array())
    {
        let texts: Vec<&str> = parts.iter().filter_map(|p| p.as_str()).collect();
        texts.join("\n")
    } else if let Some(text) = msg.get("content").and_then(|c| c.as_str()) {
        text.to_string()
    } else {
        String::new()
    };

    if content.trim().is_empty() {
        return None;
    }

    if is_user {
        Some(TranscriptStep {
            step_index: Some(idx),
            source: Some("USER_EXPLICIT".to_string()),
            step_type: Some("USER_INPUT".to_string()),
            status: Some("DONE".to_string()),
            created_at: Some(timestamp),
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
            created_at: Some(timestamp),
            content: Some(content),
            thinking: None,
            tool_calls: None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_can_parse_chatgpt_export() {
        let mapper = OpenAiMapper;
        let data = br#"{"mapping":{"1":{"message":{"author":{"role":"user"},"content":{"parts":["Hello"]}}}}}"#;
        assert!(mapper.can_parse(data));
    }

    #[test]
    fn test_map_openai_messages() {
        let mapper = OpenAiMapper;
        let data = br#"{
            "messages": [
                {
                    "role": "user",
                    "content": "Implement session caching in rust"
                },
                {
                    "role": "assistant",
                    "content": "Here is an implementation of session caching..."
                }
            ]
        }"#;

        let actions = mapper.map(data).unwrap();
        assert!(!actions.is_empty());
        assert_eq!(actions[0].actor, "human");
        assert_eq!(actions[0].action_type, "plan");
        assert!(actions[0].summary.contains("Implement session caching"));
        assert!(actions.iter().any(|a| a.actor == "chatgpt"));
    }
}
