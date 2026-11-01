//! MCP tool handler for `knox_ingest`.
//! Allows AI agents to ingest conversations, chat transcripts, or raw dialogue into the Knox knowledge graph.

use serde::Deserialize;
use serde_json::{Value, json};
use std::path::Path;

use crate::core::error::{KnoxError, KnoxResult};
use crate::storage::db::KnoxDb;

/// Input parameters for `knox_ingest`.
#[derive(Debug, Deserialize, Default)]
pub struct IngestParams {
    /// Optional filesystem path to a transcript file (JSONL, JSON, Markdown, or git repo).
    pub file_path: Option<String>,
    /// Optional raw conversation or transcript content string.
    pub raw_transcript: Option<String>,
    /// Optional actor override (e.g. "gemini", "claude", "cursor", "antigravity").
    pub actor: Option<String>,
}

/// Handles the `knox_ingest` tool invocation.
pub fn handle_ingest(db: &KnoxDb, params: Value) -> KnoxResult<Value> {
    let input: IngestParams = serde_json::from_value(params)
        .map_err(|e| KnoxError::InvalidInput(format!("Invalid ingest params: {e}")))?;

    if input.file_path.is_none() && input.raw_transcript.is_none() {
        return Err(KnoxError::InvalidInput(
            "At least one of 'file_path' or 'raw_transcript' must be provided to knox_ingest".to_string(),
        ));
    }

    let report = if let Some(ref path_str) = input.file_path {
        let path = Path::new(path_str);
        if !path.exists() {
            return Err(KnoxError::InvalidInput(format!(
                "Transcript file not found: {}",
                path.display()
            )));
        }
        crate::commands::cmd_ingest(path, db)?
    } else if let Some(ref raw) = input.raw_transcript {
        crate::commands::cmd_ingest_raw(raw, db, input.actor.as_deref())?
    } else {
        unreachable!()
    };

    Ok(json!({
        "status": "ingested",
        "tool_detected": report.tool_detected,
        "actions_imported": report.actions_imported,
        "files_touched": report.files_touched,
        "sessions_created": report.sessions_created,
        "duplicates_skipped": report.duplicates_skipped,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_handle_ingest_raw_dialogue_with_doubt_and_decision() {
        let db = KnoxDb::open_memory().unwrap();
        let raw = r#"
## User
Should we use rusqlite bundled or dynamic linking?

## Assistant
Decided on bundled sqlite3 to ensure zero external system dependencies.
"#;
        let res = handle_ingest(
            &db,
            json!({
                "raw_transcript": raw,
                "actor": "gemini"
            }),
        )
        .unwrap();

        assert_eq!(res.get("status").unwrap(), "ingested");
        assert_eq!(res.get("actions_imported").unwrap(), 2);

        // Verify that the user's question was classified as a doubt
        let filter = crate::core::types::QueryFilter {
            action_type: Some("doubt".to_string()),
            ..Default::default()
        };
        let query_res = crate::storage::queries::query_actions(&db, &filter).unwrap();
        assert_eq!(query_res.total_count, 1);
        let doubt = &query_res.results[0];
        assert_eq!(doubt.is_resolved, Some(false));
        assert!(doubt.summary.contains("rusqlite bundled"));
    }
}
