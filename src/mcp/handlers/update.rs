//! MCP tool handler for `knox_update`.

use std::path::Path;
use serde::Deserialize;
use serde_json::{Value, json};

use crate::core::error::{KnoxError, KnoxResult};
use crate::storage::db::KnoxDb;

/// Parameters for `knox_update`.
#[derive(Debug, Deserialize, Default)]
pub struct UpdateParams {
    /// Explicit transcript file path
    pub transcript_path: Option<String>,
    /// Project identifier or root directory
    pub project: Option<String>,
    /// Auto-discover transcript if path is omitted (default: true)
    #[serde(default = "default_discover")]
    pub discover: Option<bool>,
}

fn default_discover() -> Option<bool> {
    Some(true)
}

/// Executes the `knox_update` tool call.
pub fn handle_update(db: &KnoxDb, params: Value) -> KnoxResult<Value> {
    let input: UpdateParams = serde_json::from_value(params)
        .map_err(|e| KnoxError::InvalidInput(format!("Invalid knox_update params: {e}")))?;

    let path = input.transcript_path.as_deref().map(Path::new);
    let discover = input.discover.unwrap_or(true);

    let report = crate::commands::update::cmd_update(
        db,
        path,
        input.project.as_deref(),
        discover,
    )?;

    Ok(json!({
        "actions_imported": report.actions_imported,
        "duplicates_skipped": report.duplicates_skipped,
        "files_touched": report.files_touched,
        "tool_detected": report.tool_detected,
        "source": report.source,
        "offset": report.offset,
        "markdown": report.markdown,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_handle_update_deserialization_defaults() {
        let val = json!({});
        let p: UpdateParams = serde_json::from_value(val).unwrap();
        assert_eq!(p.discover, Some(true));
        assert!(p.transcript_path.is_none());
        assert!(p.project.is_none());
    }
}

