//! Tauri IPC command for Knox doctor diagnostics.

use tauri::State;

use crate::AppState;
use knox::commands::doctor::{DoctorReport, cmd_doctor};

/// Runs operational diagnostics and returns a `DoctorReport`.
#[tauri::command]
pub async fn run_doctor(
    state: State<'_, AppState>,
    project: Option<String>,
) -> Result<DoctorReport, String> {
    let db = state.db.lock().await;
    cmd_doctor(&db, project.as_deref()).map_err(|e| e.to_string())
}

