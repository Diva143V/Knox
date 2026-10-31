//! MCP Prompts handler (`prompts/list`, `prompts/get`) for Knox.

use serde::Deserialize;
use serde_json::{Value, json};

use crate::core::error::{KnoxError, KnoxResult};
use crate::storage::db::KnoxDb;

/// Definition of an exposed Knox MCP prompt template.
#[derive(Debug, Clone, serde::Serialize)]
pub struct PromptInfo {
    pub name: &'static str,
    pub description: &'static str,
    pub arguments: Vec<PromptArgument>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct PromptArgument {
    pub name: &'static str,
    pub description: &'static str,
    pub required: bool,
}

pub fn all_prompts() -> Vec<PromptInfo> {
    vec![
        PromptInfo {
            name: "architectural-audit",
            description: "Audit the codebase against recorded Knox architectural decisions to detect drift or rule violations.",
            arguments: vec![
                PromptArgument {
                    name: "target_path",
                    description: "Target directory or file path to audit (defaults to project root)",
                    required: false,
                }
            ],
        },
        PromptInfo {
            name: "session-handover",
            description: "Generate a structured handover summary of decisions, actions, and code changes made in this session.",
            arguments: vec![
                PromptArgument {
                    name: "session_id",
                    description: "Optional session ID to summarize (defaults to most recent session)",
                    required: false,
                }
            ],
        },
    ]
}

/// Handles the `prompts/list` MCP request.
pub fn handle_prompts_list() -> Value {
    json!({
        "prompts": all_prompts()
    })
}

#[derive(Debug, Deserialize)]
struct GetPromptParams {
    name: String,
    #[serde(default)]
    arguments: Option<Value>,
}

/// Handles the `prompts/get` MCP request.
pub fn handle_prompts_get(db: &KnoxDb, params: Value) -> KnoxResult<Value> {
    let input: GetPromptParams = serde_json::from_value(params)
        .map_err(|e| KnoxError::InvalidInput(format!("Invalid prompts/get params: {e}")))?;

    match input.name.as_str() {
        "architectural-audit" => {
            let filter = crate::core::types::QueryFilter {
                action_type: Some("decide".to_string()),
                current_only: true,
                limit: 50,
                ..Default::default()
            };
            let decisions = crate::storage::queries::query_actions(db, &filter)?;
            let decisions_summary = crate::mcp::format::format_markdown_digest(
                "Active Architectural Decisions to Audit Against",
                &decisions.results,
                true,
            );

            let prompt_text = format!(
                "You are an expert software architect conducting an architectural audit.\n\n\
                Review the active codebase against the following Knox architectural decisions:\n\n\
                {}\n\n\
                Identify any code patterns that violate these decisions, locate files that require updates, \
                and recommend corrective actions. If an architectural decision needs to be updated or superseded, \
                document the reasons explicitly using `knox_record_decision`.",
                decisions_summary
            );

            Ok(json!({
                "description": "Architectural Audit Prompt",
                "messages": [{
                    "role": "user",
                    "content": {
                        "type": "text",
                        "text": prompt_text
                    }
                }]
            }))
        }
        "session-handover" => {
            let session_id_opt = input
                .arguments
                .as_ref()
                .and_then(|a| a.get("session_id"))
                .and_then(|v| v.as_str())
                .map(|s| crate::core::ids::SessionId::from(s.trim()));

            let filter = crate::core::types::QueryFilter {
                session_id: session_id_opt,
                current_only: false,
                limit: 30,
                ..Default::default()
            };
            let actions = crate::storage::queries::query_actions(db, &filter)?;
            let digest = crate::mcp::format::format_markdown_digest(
                "Session Activity Digest",
                &actions.results,
                true,
            );

            let prompt_text = format!(
                "You are preparing a concise session handover recap for your team or the next AI agent.\n\n\
                Here is the recent session activity recorded in Knox:\n\n\
                {}\n\n\
                Synthesize this activity into an executive handover summary with:\n\
                1. What was accomplished\n\
                2. Key architectural decisions made and why\n\
                3. Open questions or next steps for the incoming agent.",
                digest
            );

            Ok(json!({
                "description": "Session Handover Prompt",
                "messages": [{
                    "role": "user",
                    "content": {
                        "type": "text",
                        "text": prompt_text
                    }
                }]
            }))
        }
        other => Err(KnoxError::InvalidInput(format!("Prompt not found: {other}"))),
    }
}

