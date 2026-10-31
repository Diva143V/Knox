//! MCP Resources handler (`resources/list`, `resources/read`) for Knox.

use serde::Deserialize;
use serde_json::{Value, json};

use crate::core::error::{KnoxError, KnoxResult};
use crate::core::types::QueryFilter;
use crate::storage::db::KnoxDb;

/// Definition of an exposed Knox MCP resource.
#[derive(Debug, Clone, serde::Serialize)]
pub struct ResourceInfo {
    pub uri: &'static str,
    pub name: &'static str,
    pub description: &'static str,
    #[serde(rename = "mimeType")]
    pub mime_type: &'static str,
}

pub const KNOWN_RESOURCES: &[ResourceInfo] = &[
    ResourceInfo {
        uri: "knox://project/overview",
        name: "Project Overview & Knowledge Graph Status",
        description: "Summary statistics, open sessions, and total tracked actions across Knox.",
        mime_type: "text/markdown",
    },
    ResourceInfo {
        uri: "knox://decisions/active",
        name: "Active Architectural Decisions",
        description: "All non-superseded architectural decisions, rationales, and constraints.",
        mime_type: "text/markdown",
    },
];

/// Handles the `resources/list` MCP request.
pub fn handle_resources_list() -> Value {
    json!({
        "resources": KNOWN_RESOURCES
    })
}

#[derive(Debug, Deserialize)]
struct ReadResourceParams {
    uri: String,
}

/// Handles the `resources/read` MCP request.
pub fn handle_resources_read(db: &KnoxDb, params: Value) -> KnoxResult<Value> {
    let input: ReadResourceParams = serde_json::from_value(params)
        .map_err(|e| KnoxError::InvalidInput(format!("Invalid resources/read params: {e}")))?;

    let uri = input.uri.trim();
    let (mime_type, text) = match uri {
        "knox://project/overview" => {
            let status = crate::storage::queries::get_status_report(db)?;
            let text = format!(
                "# Knox Knowledge Graph Overview & Status\n\n- **Total Actions**: {}\n- **Recorded Sessions**: {}\n- **Projects Tracked**: {}\n- **Last Action**: {}\n\n### Actions by Actor\n{}\n### Actions by Type\n{}",
                status.total_actions,
                status.total_sessions,
                status.projects_tracked,
                status
                    .last_action_timestamp
                    .map(|ts| ts.to_rfc3339())
                    .unwrap_or_else(|| "None".to_string()),
                status
                    .actions_by_actor
                    .iter()
                    .map(|(a, c)| format!("- `{}`: {}", a, c))
                    .collect::<Vec<_>>()
                    .join("\n"),
                status
                    .actions_by_type
                    .iter()
                    .map(|(t, c)| format!("- `{}`: {}", t, c))
                    .collect::<Vec<_>>()
                    .join("\n")
            );
            ("text/markdown", text)
        }
        "knox://decisions/active" => {
            let filter = QueryFilter {
                action_type: Some("decide".to_string()),
                current_only: true,
                limit: 50,
                ..Default::default()
            };
            let res = crate::storage::queries::query_actions(db, &filter)?;
            let text = crate::mcp::format::format_markdown_digest(
                "Active Architectural Decisions",
                &res.results,
                true,
            );
            ("text/markdown", text)
        }
        uri if uri.starts_with("knox://context/") => {
            let file_path = uri.strip_prefix("knox://context/").unwrap_or("");
            let filter = QueryFilter {
                file_path: Some(crate::ingest::summary::normalize_file_path(file_path)),
                current_only: true,
                limit: 20,
                ..Default::default()
            };
            let res = crate::storage::queries::query_actions(db, &filter)?;
            let text = crate::mcp::format::format_markdown_digest(
                &format!("Context for `{file_path}`"),
                &res.results,
                true,
            );
            ("text/markdown", text)
        }
        other => {
            return Err(KnoxError::InvalidInput(format!("Resource not found: {other}")));
        }
    };

    Ok(json!({
        "contents": [{
            "uri": uri,
            "mimeType": mime_type,
            "text": text
        }]
    }))
}
