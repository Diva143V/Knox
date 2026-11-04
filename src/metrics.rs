//! Deterministic measurements for Knox's token-efficiency goals.

use serde::Serialize;

use crate::core::budget::estimate_tokens;
use crate::core::types::KnoxAction;
use crate::mcp::format::{format_compact_digest, format_markdown_digest};
use crate::mcp::protocol::ToolDescription;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct TokenEfficiencyReport {
    pub tool_count: usize,
    pub tool_definition_tokens: usize,
    pub compact_context_tokens: usize,
    pub markdown_context_tokens: usize,
    pub json_context_tokens: usize,
}

impl TokenEfficiencyReport {
    pub fn compact_savings_percent(&self) -> usize {
        savings_percent(self.json_context_tokens, self.compact_context_tokens)
    }

    pub fn markdown_savings_percent(&self) -> usize {
        savings_percent(self.json_context_tokens, self.markdown_context_tokens)
    }
}

pub fn measure_token_efficiency(
    tools: &[ToolDescription],
    actions: &[KnoxAction],
) -> TokenEfficiencyReport {
    let tool_definitions = tools
        .iter()
        .map(|tool| {
            serde_json::to_string(tool).expect("ToolDescription must remain JSON serializable")
        })
        .collect::<Vec<_>>()
        .join("\n");
    let compact = format_compact_digest(actions);
    let markdown = format_markdown_digest("Context", actions, false);
    let json = serde_json::to_string(actions).expect("KnoxAction must remain JSON serializable");

    TokenEfficiencyReport {
        tool_count: tools.len(),
        tool_definition_tokens: estimate_tokens(&tool_definitions),
        compact_context_tokens: estimate_tokens(&compact),
        markdown_context_tokens: estimate_tokens(&markdown),
        json_context_tokens: estimate_tokens(&json),
    }
}

fn savings_percent(baseline: usize, candidate: usize) -> usize {
    if baseline == 0 || candidate >= baseline {
        return 0;
    }
    ((baseline - candidate) * 100) / baseline
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mcp::tools::all_tools;
    use chrono::Utc;

    #[test]
    fn measures_registered_tools_and_context_formats() {
        let actions = vec![KnoxAction {
            actor: "human".to_string(),
            action_type: "decide".to_string(),
            summary: "Use compact context retrieval".to_string(),
            rationale: Some("Reduce repeated MCP round trips".to_string()),
            files: vec!["src/mcp/handlers/get_context.rs".to_string()],
            timestamp: Utc::now(),
            ..Default::default()
        }];
        let report = measure_token_efficiency(&all_tools(), &actions);

        assert_eq!(report.tool_count, crate::mcp::tools::TOOL_COUNT);
        assert!(report.tool_definition_tokens > 0);
        assert!(report.compact_context_tokens < report.json_context_tokens);
        assert!(report.compact_savings_percent() > 0);
    }

    #[test]
    fn savings_are_safe_for_empty_or_non_smaller_candidates() {
        assert_eq!(savings_percent(0, 0), 0);
        assert_eq!(savings_percent(10, 10), 0);
        assert_eq!(savings_percent(10, 12), 0);
        assert_eq!(savings_percent(10, 5), 50);
    }
}
