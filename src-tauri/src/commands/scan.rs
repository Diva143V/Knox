//! IPC command handler for scanning staged git changes and detecting human edits.

use std::path::PathBuf;

use knox::commands::cmd_scan;
use knox::core::types::ScanReport;
use tauri::State;

use crate::AppState;

/// Scans staged git diffs in a project to detect unrecorded human changes.
#[tauri::command]
pub async fn scan_staged(
    state: State<'_, AppState>,
    project: Option<String>,
) -> Result<ScanReport, String> {
    let db = state.db.lock().await;
    let path = project
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));

    cmd_scan(&path, &db).map_err(|e| e.to_string())
}
