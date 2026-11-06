//! MCP tool declarations and JSON Schema definitions for Knox.
//!
//! Only the six high-level workflow tools are defined. Legacy tool names are
//! no longer accepted by `tools/list` or `execute_tool`.

use serde_json::json;

use super::protocol::ToolDescription;

/// Tools advertised by `tools/list`. The high-level workflow surface is small
/// on purpose: agents get one obvious entry point per phase of a turn.
pub const LISTED_TOOL_NAMES: [&str; 7] = [
    "knox_begin",
    "knox_context",
    "knox_check",
    "knox_trace",
    "knox_record",
    "knox_handoff",
    "knox_ask",
];

/// Canonical number of advertised Knox tools. Documentation and tests derive
/// from this constant so the count cannot drift from the implementation.
pub const TOOL_COUNT: usize = LISTED_TOOL_NAMES.len();

/// Returns the tool definitions advertised over `tools/list`.
pub fn all_tools() -> Vec<ToolDescription> {
    vec![
        tool_begin(),
        tool_context(),
        tool_check(),
        tool_trace(),
        tool_record(),
        tool_handoff(),
        tool_ask(),
    ]
}

/// Finds a tool definition by name among the six advertised tools.
pub fn find_tool(name: &str) -> Option<ToolDescription> {
    all_tools().into_iter().find(|t| t.name == name)
}

fn tool_record() -> ToolDescription {
    ToolDescription::new(
        "knox_record",
        "Atomically records structured engineering entries and returns committed IDs.",
        json!({
            "type": "object",
            "properties": {
                "actor": {"type": "string"},
                "project": {"type": "string"},
                "session_id": {"type": "string"},
                "entries": {"type": "array", "items": {"type": "object"}}
            },
            "required": ["actor", "entries"]
        }),
    )
}

fn tool_begin() -> ToolDescription {
    ToolDescription::new(
        "knox_begin",
        "Resumes or creates a scoped session and returns compact task context.",
        json!({
            "type": "object",
            "properties": {
                "workspace": { "type": "string", "description": "Project path, ID, or unique name" },
                "actor": { "type": "string", "description": "Agent or software identity" },
                "task": { "type": "string", "description": "Current task used to focus context" },
                "budget": { "type": "integer", "minimum": 1, "maximum": 4000, "default": 1200 },
                "transcript_path": { "type": "string", "description": "Optional path to conversation transcript to auto-capture memory since last offset" }
            },
            "required": ["actor"]
        }),
    )
}

fn tool_context() -> ToolDescription {
    ToolDescription::new(
        "knox_context",
        "Returns compact, token-budgeted engineering context for a file, symbol, query, or project.",
        json!({
            "type": "object",
            "properties": {
                "view": { "type": "string", "description": "context, file, symbol, or query" },
                "files": { "type": "array", "items": { "type": "string" } },
                "symbols": { "type": "array", "items": { "type": "string" } },
                "query": { "type": "string" },
                "cursor": { "type": "string", "description": "Pagination cursor from previous next field" },
                "budget": { "type": "integer", "minimum": 1, "maximum": 4000, "default": 1200 }
            },
            "required": ["view"]
        }),
    )
}

fn tool_check() -> ToolDescription {
    ToolDescription::new(
        "knox_check",
        "Checks proposed work against scoped architectural guardrails and returns overlap-rated violations.",
        json!({
            "type": "object",
            "properties": {
                "files": {"type": "array", "items": {"type": "string"}},
                "symbols": {"type": "array", "items": {"type": "string"}},
                "intent": {"type": "string"},
                "project": {"type": "string"},
                "strict": {"type": "boolean"}
            },
            "required": ["files"]
        }),
    )
}

fn tool_trace() -> ToolDescription {
    ToolDescription::new(
        "knox_trace",
        "Traces causal ancestry from an action or file using only causal graph edges.",
        json!({
            "type": "object",
            "properties": {
                "from": {"type": "string"},
                "file_path": {"type": "string"},
                "depth": {"type": "integer", "minimum": 1, "default": 10},
                "max_nodes": {"type": "integer", "minimum": 1, "default": 200}
            },
            "anyOf": [
                {"required": ["from"]},
                {"required": ["file_path"]}
            ]
        }),
    )
}

fn tool_handoff() -> ToolDescription {
    ToolDescription::new(
        "knox_handoff",
        "Closes a session and returns a compact digest for the next session.",
        json!({
            "type": "object",
            "properties": {
                "session_id": { "type": "string", "description": "Open session to close" },
                "budget": { "type": "integer", "minimum": 1, "maximum": 4000, "default": 800 }
            },
            "required": ["session_id"]
        }),
    )
}

fn tool_ask() -> ToolDescription {
    ToolDescription::new(
        "knox_ask",
        "Answers architectural questions across project memory using grounded context with verifiable citations.",
        json!({
            "type": "object",
            "properties": {
                "question": { "type": "string", "description": "Technical or architectural question to ask project memory" },
                "project": { "type": "string", "description": "Project scope (canonical path, remote URL, or ID)" },
                "file_path": { "type": "string", "description": "Optional file path anchor for focused context" },
                "strict_citations": { "type": "boolean", "description": "Enforce strict citation verification (default: true)" }
            },
            "required": ["question"]
        }),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_all_tools_count_and_names() {
        let tools = all_tools();
        assert_eq!(tools.len(), TOOL_COUNT);
        assert_eq!(LISTED_TOOL_NAMES.len(), TOOL_COUNT);
        for expected_name in LISTED_TOOL_NAMES {
            assert!(
                tools.iter().any(|t| t.name == expected_name),
                "Missing tool: {}",
                expected_name
            );
        }
    }

    #[test]
    fn test_find_tool_existing_and_missing() {
        assert!(find_tool("knox_begin").is_some());
        assert!(find_tool("knox_record").is_some());
        assert!(find_tool("knox_context").is_some());
        assert!(find_tool("knox_unknown_tool").is_none());
        // Legacy tools are no longer resolvable.
        assert!(find_tool("knox_start_session").is_none());
        assert!(find_tool("knox_record_action").is_none());
        assert!(find_tool("knox_get_context").is_none());
    }

    #[test]
    fn test_tool_schemas_have_properties() {
        for tool in all_tools() {
            assert!(!tool.description.is_empty());
            assert!(tool.input_schema.is_object());
        }
    }
}
