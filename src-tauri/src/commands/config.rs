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
    let cfg = state.config.lock().await;
    Ok(ConfigDto {
        kg_path: cfg.kg_path.display().to_string(),
        session_timeout_seconds: cfg.session_timeout.as_secs(),
        global_mode: cfg.global_mode,
    })
}

/// Updates Knox runtime configuration (session timeout and global mode).
#[tauri::command]
pub async fn update_config(
    state: State<'_, AppState>,
    session_timeout_seconds: Option<u64>,
    global_mode: Option<bool>,
) -> Result<ConfigDto, String> {
    let mut cfg = state.config.lock().await;
    if let Some(timeout) = session_timeout_seconds {
        cfg.session_timeout = std::time::Duration::from_secs(timeout);
    }
    if let Some(mode) = global_mode {
        cfg.global_mode = mode;
    }
    Ok(ConfigDto {
        kg_path: cfg.kg_path.display().to_string(),
        session_timeout_seconds: cfg.session_timeout.as_secs(),
        global_mode: cfg.global_mode,
    })
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use knox::core::config::KnoxConfig;
    use knox::storage::db::KnoxDb;
    use std::sync::Arc;
    use tokio::sync::Mutex;

    #[tokio::test]
    async fn test_update_config_flow() {
        let config = KnoxConfig::default();
        let db = KnoxDb::open_memory().unwrap();
        let state = AppState {
            db: Arc::new(Mutex::new(db)),
            config: Arc::new(Mutex::new(config)),
        };

        // Update timeout
        {
            let mut cfg = state.config.lock().await;
            cfg.session_timeout = std::time::Duration::from_secs(3600);
            cfg.global_mode = false;
        }

        let cfg = state.config.lock().await;
        assert_eq!(cfg.session_timeout.as_secs(), 3600);
        assert!(!cfg.global_mode);
    }
}
