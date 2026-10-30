//! IPC command handler for Knox status reports.

use knox::commands::cmd_status;
use knox::core::types::StatusReport;
use tauri::State;

use crate::AppState;

/// Fetches the current Knox knowledge graph status report.
#[tauri::command]
pub async fn get_status(state: State<'_, AppState>) -> Result<StatusReport, String> {
    let db = state.db.lock().await;
    cmd_status(&db).map_err(|e| e.to_string())
}
