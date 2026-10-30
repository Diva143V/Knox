//! IPC command handler for Knox configuration and metadata.

use serde::{Deserialize, Serialize};
use tauri::State;

use crate::AppState;

/// Safe configuration DTO returned to the frontend.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ConfigDto {
    pub kg_path: String,
    pub session_timeout_seconds: u64,
    pub global_mode: bool,
}

/// Fetches current Knox runtime configuration as a frontend-safe DTO.
#[tauri::command]
pub async fn get_config(state: State<'_, AppState>) -> Result<ConfigDto, String> {
    Ok(ConfigDto {
        kg_path: state.config.kg_path.display().to_string(),
        session_timeout_seconds: state.config.session_timeout.as_secs(),
        global_mode: state.config.global_mode,
    })
}
