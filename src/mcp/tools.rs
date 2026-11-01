//! MCP tool declarations and JSON Schema definitions for Knox.

use serde_json::json;

use super::protocol::ToolDescription;

/// All supported Knox MCP tool names (17 tools).
pub const TOOL_NAMES: [&str; 17] = [
    "knox_start_session",
    "knox_end_session",
    "knox_record_action",
    "knox_record_decision",
    "knox_record_doubt",
    "knox_resolve_doubt",
    "knox_record_batch",
    "knox_get_context",
    "knox_get_history",
    "knox_get_lineage",
    "knox_get_impact",
    "knox_get_path",
    "knox_check_guardrails",
    "knox_search",
    "knox_list_stale",
    "knox_scan_staged",
    "knox_ingest",
];

/// Returns the complete list of all 17 Knox tool definitions.
pub fn all_tools() -> Vec<ToolDescription> {
    vec![
        tool_start_session(),
        tool_end_session(),
        tool_record_action(),
        tool_record_decision(),
        tool_record_doubt(),
        tool_resolve_doubt(),
        tool_record_batch(),
        tool_get_context(),
        tool_get_history(),
        tool_get_lineage(),
        tool_get_impact(),
        tool_get_path(),
        tool_check_guardrails(),
        tool_search(),
        tool_list_stale(),
        tool_scan_staged(),
        tool_ingest(),
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
            "description": "Session ID (optional; auto-attaches to latest active session if omitted)"
        },
        "actor": {
            "type": "string",
            "description": "Software or agent name (e.g. gemini, cursor, claude)"
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
        },
        "relations": {
            "type": "array",
            "description": "Explicit semantic relationships (target_id + relation_type)",
            "items": {
                "type": "object",
                "properties": {
                    "target_id": { "type": "string", "description": "Target action ID" },
                    "relation_type": {
                        "type": "string",
                        "enum": ["depends_on", "implements", "verifies", "reverts", "relates_to", "resolves"],
                        "description": "Semantic type of the relation"
                    }
                },
                "required": ["target_id"]
            }
        },
        "symbols": {
            "type": "array",
            "items": { "type": "string" },
            "description": "Source code symbols touched (structs, functions, traits, modules)"
        },
        "rationale": {
            "type": "string",
            "description": "Deep architectural justification or technical reasoning"
        },
        "rejected_alternatives": {
            "type": "array",
            "items": { "type": "string" },
            "description": "Alternative approaches considered and why they were rejected"
        },
        "context_summary": {
            "type": "string",
            "description": "High-level summary of active task and state at execution time"
        },
        "significance": {
            "type": "string",
            "enum": ["major", "minor"],
            "description": "Significance level: 'major' (ADR, architectural decision) or 'minor' (implementation detail, styling, micro-tradeoff)"
        },
        "is_resolved": {
            "type": "boolean",
            "description": "Resolution status (false for open doubts, true for resolved/standard actions)"
        },
        "resolution_note": {
            "type": "string",
            "description": "Explanation of how a doubt was resolved or answered"
        },
        "link_to_latest": {
            "type": "boolean",
            "description": "Convenience helper: automatically link to immediate predecessor in session"
        },
        "implements_latest_decision": {
            "type": "boolean",
            "description": "Convenience helper: automatically link `implements` to latest decision in session"
        }
    })
}

fn tool_record_doubt() -> ToolDescription {
    ToolDescription::new(
        "knox_record_doubt",
        "Records a user doubt, open technical question, or uncertainty to be tracked until resolved.",
        json!({
            "type": "object",
            "properties": {
                "actor": {
                    "type": "string",
                    "description": "Software or agent name (e.g. gemini, cursor, claude)"
                },
                "summary": {
                    "type": "string",
                    "description": "The exact question, doubt, or hypothesis being tracked"
                },
                "session_id": {
                    "type": "string",
                    "description": "Session ID (optional; auto-attaches to latest active session if omitted)"
                },
                "files": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "Affected or referenced file paths"
                },
                "context_summary": {
                    "type": "string",
                    "description": "Context or background surrounding why this doubt arose"
                },
                "tags": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "Optional search tags"
                }
            },
            "required": ["actor", "summary"]
        }),
    )
}

fn tool_resolve_doubt() -> ToolDescription {
    ToolDescription::new(
        "knox_resolve_doubt",
        "Marks a previously recorded doubt or question as resolved with an explanation note and optional answering action link.",
        json!({
            "type": "object",
            "properties": {
                "doubt_id": {
                    "type": "string",
                    "description": "The unique ID of the doubt action being marked as resolved"
                },
                "resolution_note": {
                    "type": "string",
                    "description": "Detailed explanation of the answer, solution, or reason why the doubt was resolved"
                },
                "resolved_by_action_id": {
                    "type": "string",
                    "description": "Optional action ID of the decision, code write, or verification that resolved the doubt"
                }
            },
            "required": ["doubt_id", "resolution_note"]
        }),
    )
}

fn tool_record_action() -> ToolDescription {
    ToolDescription::new(
        "knox_record_action",
        "Records an action (write, debug, read, review, plan, change) linked to the session.",
        json!({
            "type": "object",
            "properties": action_schema_properties("Action type (write, debug, read, review, plan, change)"),
            "required": ["actor", "action_type", "summary"]
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
            "required": ["actor", "summary"]
        }),
    )
}

fn tool_record_batch() -> ToolDescription {
    ToolDescription::new(
        "knox_record_batch",
        "Atomically records multiple related actions and decisions in a single round trip.",
        json!({
            "type": "object",
            "properties": {
                "actor": {
                    "type": "string",
                    "description": "Software or agent name"
                },
                "session_id": {
                    "type": "string",
                    "description": "Optional session ID"
                },
                "project": {
                    "type": "string",
                    "description": "Optional project scope identifier"
                },
                "actions": {
                    "type": "array",
                    "description": "List of actions to record atomically",
                    "items": {
                        "type": "object",
                        "properties": {
                            "action_type": { "type": "string" },
                            "summary": { "type": "string" },
                            "files": { "type": "array", "items": { "type": "string" } },
                            "parent_action_id": { "type": "string" },
                            "tags": { "type": "array", "items": { "type": "string" } },
                            "confidence": { "type": "number" },
                            "supersedes": { "type": "string" },
                            "symbols": { "type": "array", "items": { "type": "string" } },
                            "rationale": { "type": "string" },
                            "rejected_alternatives": { "type": "array", "items": { "type": "string" } },
                            "context_summary": { "type": "string" },
                            "link_to_previous": { "type": "boolean", "description": "Link parent_action_id to predecessor in batch" }
                        },
                        "required": ["action_type", "summary"]
                    }
                }
            },
            "required": ["actor", "actions"]
        }),
    )
}

fn tool_get_context() -> ToolDescription {
    ToolDescription::new(
        "knox_get_context",
        "Retrieves current context for a file, symbol, or project, excluding superseded actions by default.",
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
                "action_type": {
                    "type": "string",
                    "description": "Filter by action type (e.g. decide, write, plan)"
                },
                "symbols": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "Filter by code symbols touched"
                },
                "query": {
                    "type": "string",
                    "description": "In-context keyword query across summaries and rationales"
                },
                "tags": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "Filter by tags"
                },
                "format": {
                    "type": "string",
                    "enum": ["markdown", "json", "compact"],
                    "default": "markdown",
                    "description": "Output representation format (default: markdown, saves 70%+ tokens)"
                },
                "synthesis": {
                    "type": "boolean",
                    "default": false,
                    "description": "Group into categorized sections: decisions, symbols, and constraints"
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
                "action_type": {
                    "type": "string",
                    "description": "Filter by action type"
                },
                "symbols": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "Filter by code symbols"
                },
                "tags": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "Filter by tags"
                },
                "format": {
                    "type": "string",
                    "enum": ["markdown", "json"],
                    "default": "markdown"
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

fn tool_get_lineage() -> ToolDescription {
    ToolDescription::new(
        "knox_get_lineage",
        "Traces causal ancestry and parent lineage ('Why was this code or decision made?') back to the root plan.",
        json!({
            "type": "object",
            "properties": {
                "action_id": {
                    "type": "string",
                    "description": "Action ID to trace backwards"
                },
                "file_path": {
                    "type": "string",
                    "description": "Target file path to trace the latest action for"
                },
                "max_depth": {
                    "type": "integer",
                    "default": 10,
                    "description": "Maximum hops backwards in the causal DAG"
                },
                "format": {
                    "type": "string",
                    "enum": ["markdown", "json"],
                    "default": "markdown"
                }
            }
        }),
    )
}

fn tool_get_impact() -> ToolDescription {
    ToolDescription::new(
        "knox_get_impact",
        "Traces downstream descendant effects of an action or decision ('What depends on or implements this?').",
        json!({
            "type": "object",
            "properties": {
                "action_id": {
                    "type": "string",
                    "description": "Root action or decision to compute downstream impact for"
                },
                "file_path": {
                    "type": "string",
                    "description": "Target file path to compute downstream impact for (resolves to latest action)"
                },
                "max_depth": {
                    "type": "integer",
                    "default": 5,
                    "description": "Maximum traversal depth forward in the DAG"
                },
                "format": {
                    "type": "string",
                    "enum": ["markdown", "json"],
                    "default": "markdown"
                }
            }
        }),
    )
}

fn tool_get_path() -> ToolDescription {
    ToolDescription::new(
        "knox_get_path",
        "Finds the shortest directed causal path between two actions in the graph ('How did A lead to B?').",
        json!({
            "type": "object",
            "properties": {
                "from_action_id": {
                    "type": "string",
                    "description": "Starting action ID"
                },
                "from_file": {
                    "type": "string",
                    "description": "Starting file path (resolves to latest action on this file)"
                },
                "to_action_id": {
                    "type": "string",
                    "description": "Destination action ID"
                },
                "to_file": {
                    "type": "string",
                    "description": "Destination file path (resolves to latest action on this file)"
                },
                "max_depth": {
                    "type": "integer",
                    "default": 10
                },
                "format": {
                    "type": "string",
                    "enum": ["markdown", "json"],
                    "default": "markdown"
                }
            }
        }),
    )
}

fn tool_check_guardrails() -> ToolDescription {
    ToolDescription::new(
        "knox_check_guardrails",
        "Pre-flight safety check before editing files. Retrieves active architectural constraints, tests proposed changes, and flags superseded anti-patterns.",
        json!({
            "type": "object",
            "properties": {
                "files": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "List of files about to be modified"
                },
                "proposed_action": {
                    "type": "string",
                    "description": "Proposed action or change description to check against architectural constraints and rejected alternatives"
                },
                "strict": {
                    "type": "boolean",
                    "default": false,
                    "description": "Enforce strict checking; fails if any superseded anti-patterns or constraint conflicts are present"
                },
                "project": {
                    "type": "string",
                    "description": "Optional project scope identifier"
                },
                "format": {
                    "type": "string",
                    "enum": ["markdown", "json"],
                    "default": "markdown"
                }
            },
            "required": ["files"]
        }),
    )
}

fn tool_search() -> ToolDescription {
    ToolDescription::new(
        "knox_search",
        "Full-text keyword and tag search across action summaries and rationales.",
        json!({
            "type": "object",
            "properties": {
                "query": {
                    "type": "string",
                    "description": "Keyword search across action summaries and rationales"
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
        "Lists superseded (stale) entries for cleanup and architectural review.",
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

fn tool_ingest() -> ToolDescription {
    ToolDescription::new(
        "knox_ingest",
        "Ingests conversational transcripts, human chat notes, or raw dialogue into the Knox knowledge graph. Automatically classifies human inputs into plans, doubts, and dialogues, records writes and decisions, and establishes causal lineage.",
        json!({
            "type": "object",
            "properties": {
                "file_path": {
                    "type": "string",
                    "description": "Optional filesystem path to a conversation transcript file (JSONL, JSON, Markdown, or git repo)."
                },
                "raw_transcript": {
                    "type": "string",
                    "description": "Optional raw conversation text, Markdown chat, or JSON string to ingest directly."
                },
                "actor": {
                    "type": "string",
                    "description": "Optional actor/tool override for actions emitted during ingestion (defaults to auto-detected format)."
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
        assert_eq!(tools.len(), 17);
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
        assert!(find_tool("knox_get_lineage").is_some());
        assert!(find_tool("knox_check_guardrails").is_some());
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
        // session_id is now optional for zero-friction auto-sessions
        assert_eq!(record_req.len(), 3);
    }

    #[test]
    fn test_export_antigravity_schemas() {
        let dest = std::path::Path::new("C:/Users/Diwa/.gemini/antigravity/mcp/knox");
        if dest.exists() {
            for tool in all_tools() {
                let schema_obj = json!({
                    "name": tool.name,
                    "description": tool.description,
                    "parameters": tool.input_schema,
                });
                let file_path = dest.join(format!("{}.json", tool.name));
                let _ = std::fs::write(
                    file_path,
                    serde_json::to_string_pretty(&schema_obj).unwrap(),
                );
            }
            let instructions = r#"# Knox MCP Best Practices for AI Agents

1. **Smart Auto-Sessions**: You do NOT need to call `knox_start_session` before logging actions. Knox will automatically reuse your active session or spawn one transparently.
2. **Pre-flight Guardrail Check**: Before modifying critical files or architecture, call `knox_check_guardrails` with `files: ["path/to/file"]` to inspect active architectural decisions and avoid repeating superseded patterns.
3. **Graph Lineage & Impact**: Use `knox_get_lineage` to trace the causal chain of decisions and code modifications, and `knox_get_impact` to assess downstream implications of changing a file or decision.
4. **Batch Operations**: When performing multiple atomic steps, use `knox_record_batch` with `link_to_previous: true` to record an interconnected causal chain in a single turn.
5. **Rich Relations & Symbols**: When recording actions, specify `symbols: ["fn_name", "StructName"]` and `relations: [{"target_id": "...", "relation_type": "implements"}]` to enrich Knox's petgraph knowledge graph.
"#;
            let _ = std::fs::write(dest.join("instructions.md"), instructions);
        }
    }
}
