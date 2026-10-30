//! MCP tool handler for `knox_scan_staged`.

use serde::Deserialize;
use serde_json::{Value, json};
use std::path::Path;

use crate::core::error::{KnoxError, KnoxResult};
use crate::storage::db::KnoxDb;

/// Input parameters for `knox_scan_staged`.
#[derive(Debug, Deserialize, Default)]
pub struct ScanStagedParams {
    /// Optional project filesystem path (defaults to current directory `"."`).
    pub project: Option<String>,
}

/// Handles the `knox_scan_staged` tool invocation.
pub fn handle_scan_staged(db: &KnoxDb, params: Value) -> KnoxResult<Value> {
    let input: ScanStagedParams = serde_json::from_value(params)
        .map_err(|e| KnoxError::InvalidInput(format!("Invalid scan_staged params: {e}")))?;

    let target_path = input.project.as_deref().unwrap_or(".");
    let path = Path::new(target_path);

    let report = crate::commands::scan::cmd_scan(path, db)?;

    Ok(json!({
        "human_edits_detected": report.human_edits_detected,
        "actions_recorded": report.actions_recorded
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_handle_scan_staged_in_clean_repo() {
        let db = KnoxDb::open_memory().unwrap();
        // Running in current workspace directory (which is a valid git repository)
        let res = handle_scan_staged(&db, json!({})).unwrap();
        assert!(res.get("human_edits_detected").is_some());
        assert!(res.get("actions_recorded").is_some());
    }
}
