//! Antigravity IDE transcript mapper implementation.
//! Parses Antigravity `transcript.jsonl` files into Knox actions.

use serde_json::Value;

use super::action_group::{ActionGrouper, TranscriptStep};
use super::mapper::TranscriptMapper;
use crate::core::error::{KnoxError, KnoxResult};
use crate::core::types::KnoxAction;

/// Mapper for Antigravity conversation transcripts (`transcript.jsonl`).
pub struct AntigravityMapper;

impl TranscriptMapper for AntigravityMapper {
    fn tool_name(&self) -> &str {
        "antigravity"
    }

    fn can_parse(&self, raw: &[u8]) -> bool {
        let Ok(text) = std::str::from_utf8(raw) else {
            return false;
        };

        for line in text.lines() {
            let trimmed = line.trim_start_matches('\u{feff}').trim();
            if trimmed.is_empty() {
                continue;
            }

            if let Ok(Value::Object(map)) = serde_json::from_str::<Value>(trimmed) {
                let has_step = map.contains_key("step_index");
                let has_type = map.contains_key("type");
                return has_step && has_type;
            }
            return false;
        }

        false
    }

    fn map(&self, raw: &[u8]) -> KnoxResult<Vec<KnoxAction>> {
        let text = std::str::from_utf8(raw)
            .map_err(|e| KnoxError::ParseError(format!("Invalid UTF-8 in transcript: {e}")))?;

        let mut grouper = ActionGrouper::new(self.tool_name());
        let mut all_actions = Vec::new();

        for line in text.lines() {
            let trimmed = line.trim_start_matches('\u{feff}').trim();
            if trimmed.is_empty() {
                continue;
            }

            let step: TranscriptStep = serde_json::from_str(trimmed).map_err(|err| {
                KnoxError::ParseError(format!("Failed to deserialize Antigravity step: {err}"))
            })?;

            let flushed = grouper.process_step(&step);
            all_actions.extend(flushed);
        }

        let final_flushed = grouper.finish();
        all_actions.extend(final_flushed);

        Ok(all_actions)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_can_parse_valid_antigravity_transcript() {
        let mapper = AntigravityMapper;
        let valid =
            br#"{"step_index":0,"source":"USER_EXPLICIT","type":"USER_INPUT","content":"Hello"}"#;
        assert!(mapper.can_parse(valid));

        // Test with UTF-8 BOM prefix
        let mut bom_prefixed = b"\xef\xbb\xbf".to_vec();
        bom_prefixed.extend_from_slice(valid);
        assert!(mapper.can_parse(&bom_prefixed));
    }

    #[test]
    fn test_can_parse_invalid_format() {
        let mapper = AntigravityMapper;
        let invalid = br#"{"message": "Hello from unknown log"}"#;
        assert!(!mapper.can_parse(invalid));

        let non_json = b"plain text line";
        assert!(!mapper.can_parse(non_json));
    }

    #[test]
    fn test_map_simple_transcript() {
        let mapper = AntigravityMapper;
        let raw = br#"
{"step_index":0,"source":"USER_EXPLICIT","type":"USER_INPUT","content":"Do work"}
{"step_index":1,"source":"MODEL","type":"PLANNER_RESPONSE","content":"Done work"}
"#;
        let actions = mapper.map(raw).unwrap();
        assert_eq!(actions.len(), 2);
        assert_eq!(actions[0].action_type, "plan");
        assert_eq!(actions[1].action_type, "decide");
    }
}
