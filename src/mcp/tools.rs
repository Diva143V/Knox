//! MCP tool declarations and JSON Schema definitions for Knox.

use serde_json::json;

use super::protocol::ToolDescription;

/// All supported Knox MCP tool names.
pub const TOOL_NAMES: [&str; 9] = [
    "knox_start_session",
    "knox_end_session",
    "knox_record_action",
    "knox_record_decision",
    "knox_get_context",
    "knox_get_history",
    "knox_search",
    "knox_list_stale",
    "knox_scan_staged",
];

/// Returns the complete list of all 9 Knox tool definitions.
pub fn all_tools() -> Vec<ToolDescription> {
    vec![
        tool_start_session(),
        tool_end_session(),
        tool_record_action(),
        tool_record_decision(),
        tool_get_context(),
        tool_get_history(),
        tool_search(),
        tool_list_stale(),
        tool_scan_staged(),
    ]
}

/// Finds a tool definition by name.
pub fn find_tool(name: &str) -> Option<ToolDescription> {
    all_tools().into_iter().find(|t| t.name == name)
}

fn tool_start_session() -> ToolDescription {
    ToolDescription::new(
        "knox_start_session",
        "Starts a new work session in Knox. Returns a session_id that subsequent actions reference.",
        json!({
            "type": "object",
            "properties": {
                "actor": {
                    "type": "string",
                    "description": "Software or agent name (e.g. gemini, cursor, claude)"
                },
                "project": {
                    "type": "string",
                    "description": "Optional project identifier, name, or path"
                },
                "metadata": {
                    "type": "object",
                    "description": "Optional arbitrary key-value metadata pairs"
                }
            },
            "required": ["actor"]
        }),
    )
}

fn tool_end_session() -> ToolDescription {
    ToolDescription::new(
        "knox_end_session",
        "Explicitly closes an active Knox session.",
        json!({
            "type": "object",
            "properties": {
                "session_id": {
                    "type": "string",
                    "description": "The unique identifier of the session to close"
                }
            },
            "required": ["session_id"]
        }),
    )
}

fn action_schema_properties(action_type_desc: &str) -> serde_json::Value {
    json!({
        "session_id": {
            "type": "string",
            "description": "Session ID returned from knox_start_session"
        },
        "actor": {
            "type": "string",
            "description": "Software or agent name"
        },
        "action_type": {
            "type": "string",
            "description": action_type_desc
        },
        "summary": {
            "type": "string",
            "description": "Human-readable summary or design decision"
        },
        "files": {
            "type": "array",
            "items": { "type": "string" },
            "description": "Affected file paths relative to project root"
        },
        "parent_action_id": {
            "type": "string",
            "description": "Causal link to a prior action ID"
        },
        "tags": {
            "type": "array",
            "items": { "type": "string" },
            "description": "Free-form tags for search and categorization"
        },
        "confidence": {
            "type": "number",
            "minimum": 0.0,
            "maximum": 1.0,
            "description": "Optional confidence hint (0.0 to 1.0)"
        },
        "supersedes": {
            "type": "string",
            "description": "ID of a prior decision or action this replaces"
        }
    })
}

fn tool_record_action() -> ToolDescription {
    ToolDescription::new(
        "knox_record_action",
        "Records an action (write, debug, read, review, plan, change) linked to the current session.",
        json!({
            "type": "object",
            "properties": action_schema_properties("Action type (write, debug, read, review, plan, change)"),
            "required": ["session_id", "actor", "action_type", "summary"]
        }),
    )
}

fn tool_record_decision() -> ToolDescription {
    ToolDescription::new(
        "knox_record_decision",
        "Records a design decision or architectural approach choice (action_type forced to 'decide').",
        json!({
            "type": "object",
            "properties": action_schema_properties("Action type (forced to 'decide' if omitted or specified)"),
            "required": ["session_id", "actor", "summary"]
        }),
    )
}

fn tool_get_context() -> ToolDescription {
    ToolDescription::new(
        "knox_get_context",
        "Retrieves current context for a file or project, excluding superseded actions by default.",
        json!({
            "type": "object",
            "properties": {
                "file_path": {
                    "type": "string",
                    "description": "Filter by affected file path"
                },
                "project": {
                    "type": "string",
                    "description": "Scope to project identifier"
                },
                "session_id": {
                    "type": "string",
                    "description": "Scope to a specific session"
                },
                "actor": {
                    "type": "string",
                    "description": "Filter by software or agent name"
                },
                "since": {
                    "type": "string",
                    "description": "ISO 8601 timestamp; only actions after this time"
                },
                "current_only": {
                    "type": "boolean",
                    "default": true,
                    "description": "Exclude superseded actions (default true)"
                },
                "tags": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "Filter by tags"
                },
                "limit": {
                    "type": "integer",
                    "default": 20,
                    "description": "Maximum number of results to return"
                }
            }
        }),
    )
}

fn tool_get_history() -> ToolDescription {
    ToolDescription::new(
        "knox_get_history",
        "Full append-only history for a file or project, including superseded entries.",
        json!({
            "type": "object",
            "properties": {
                "file_path": {
                    "type": "string",
                    "description": "Filter by affected file path"
                },
                "project": {
                    "type": "string",
                    "description": "Scope to project identifier"
                },
                "session_id": {
                    "type": "string",
                    "description": "Scope to a specific session"
                },
                "actor": {
                    "type": "string",
                    "description": "Filter by software or agent name"
                },
                "since": {
                    "type": "string",
                    "description": "ISO 8601 timestamp; only actions after this time"
                },
                "current_only": {
                    "type": "boolean",
                    "default": false,
                    "description": "Include superseded actions (default false)"
                },
                "tags": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "Filter by tags"
                },
                "limit": {
                    "type": "integer",
                    "default": 20,
                    "description": "Maximum number of results to return"
                }
            }
        }),
    )
}

fn tool_search() -> ToolDescription {
    ToolDescription::new(
        "knox_search",
        "Full-text keyword and tag search across the entire Knox knowledge graph.",
        json!({
            "type": "object",
            "properties": {
                "query": {
                    "type": "string",
                    "description": "Keyword search across action summaries"
                },
                "project": {
                    "type": "string",
                    "description": "Scope search to project identifier"
                },
                "actor": {
                    "type": "string",
                    "description": "Filter by software or agent name"
                },
                "tags": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "Filter by tags"
                },
                "since": {
                    "type": "string",
                    "description": "ISO 8601 timestamp filter"
                },
                "limit": {
                    "type": "integer",
                    "default": 20,
                    "description": "Maximum results to return"
                }
            },
            "required": ["query"]
        }),
    )
}

fn tool_list_stale() -> ToolDescription {
    ToolDescription::new(
        "knox_list_stale",
        "Lists superseded (stale) entries for cleanup and review.",
        json!({
            "type": "object",
            "properties": {
                "project": {
                    "type": "string",
                    "description": "Scope to project identifier"
                },
                "file_path": {
                    "type": "string",
                    "description": "Filter by affected file path"
                },
                "limit": {
                    "type": "integer",
                    "default": 50,
                    "description": "Maximum results to return"
                }
            }
        }),
    )
}

fn tool_scan_staged() -> ToolDescription {
    ToolDescription::new(
        "knox_scan_staged",
        "Scans git staged changes and records human edits not covered by AI actions.",
        json!({
            "type": "object",
            "properties": {
                "project": {
                    "type": "string",
                    "description": "Project directory or identifier (defaults to current directory)"
                }
            }
        }),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_all_tools_count_and_names() {
        let tools = all_tools();
        assert_eq!(tools.len(), 9);
        for expected_name in TOOL_NAMES {
            assert!(
                tools.iter().any(|t| t.name == expected_name),
                "Missing tool: {}",
                expected_name
            );
        }
    }

    #[test]
    fn test_find_tool_existing_and_missing() {
        assert!(find_tool("knox_start_session").is_some());
        assert!(find_tool("knox_search").is_some());
        assert!(find_tool("knox_unknown_tool").is_none());
    }

    #[test]
    fn test_tool_schemas_have_properties() {
        for tool in all_tools() {
            assert!(!tool.description.is_empty());
            assert!(tool.input_schema.get("type").is_some());
            assert!(tool.input_schema.get("properties").is_some());
        }
    }

    #[test]
    fn test_required_fields_presence() {
        let start = find_tool("knox_start_session").unwrap();
        let required = start
            .input_schema
            .get("required")
            .unwrap()
            .as_array()
            .unwrap();
        assert!(required.iter().any(|v| v == "actor"));

        let record = find_tool("knox_record_action").unwrap();
        let record_req = record
            .input_schema
            .get("required")
            .unwrap()
            .as_array()
            .unwrap();
        assert_eq!(record_req.len(), 4);
    }
}
