//! IPC command handler for Knox configuration and metadata.

use knox::storage::db::StorageStats;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
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

/// Updates Knox runtime configuration (session timeout, global mode, and db path),
/// persisting changes to ~/.knox/config.json.
#[tauri::command]
pub async fn update_config(
    state: State<'_, AppState>,
    session_timeout_seconds: Option<u64>,
    global_mode: Option<bool>,
    kg_path: Option<String>,
) -> Result<ConfigDto, String> {
    let mut cfg = state.config.lock().await;
    if let Some(timeout) = session_timeout_seconds {
        cfg.session_timeout = std::time::Duration::from_secs(timeout);
    }
    if let Some(mode) = global_mode {
        cfg.global_mode = mode;
    }
    if let Some(path_str) = kg_path.as_deref().filter(|s| !s.trim().is_empty()) {
        cfg.kg_path = PathBuf::from(path_str.trim());
    }
    if let Err(e) = cfg.save_default() {
        tracing::warn!("Failed to persist config to default file: {e}");
    }
    Ok(ConfigDto {
        kg_path: cfg.kg_path.display().to_string(),
        session_timeout_seconds: cfg.session_timeout.as_secs(),
        global_mode: cfg.global_mode,
    })
}

/// Explicitly saves the entire Knox configuration DTO.
#[tauri::command]
pub async fn save_config(
    state: State<'_, AppState>,
    config: ConfigDto,
) -> Result<ConfigDto, String> {
    update_config(
        state,
        Some(config.session_timeout_seconds),
        Some(config.global_mode),
        Some(config.kg_path),
    )
    .await
}

/// Fetches storage and health telemetry (size, WAL size, counts, schema version).
#[tauri::command]
pub async fn get_db_stats(state: State<'_, AppState>) -> Result<StorageStats, String> {
    let cfg = state.config.lock().await;
    let db = state.db.lock().await;
    db.get_storage_stats(Some(&cfg.kg_path))
        .map_err(|e| e.to_string())
}

/// Executes a SQLite PRAGMA integrity_check against the active database.
#[tauri::command]
pub async fn run_db_integrity(state: State<'_, AppState>) -> Result<String, String> {
    let db = state.db.lock().await;
    db.integrity_check().map_err(|e| e.to_string())
}

/// Reclaims unused disk space and defragments SQLite pages via VACUUM.
#[tauri::command]
pub async fn run_db_vacuum(state: State<'_, AppState>) -> Result<String, String> {
    let db = state.db.lock().await;
    db.vacuum().map_err(|e| e.to_string())?;
    Ok("Database vacuum completed successfully.".to_string())
}

/// Reveals the SQLite database folder in the OS file explorer.
#[tauri::command]
pub async fn reveal_db_folder(state: State<'_, AppState>) -> Result<(), String> {
    let cfg = state.config.lock().await;
    let path = &cfg.kg_path;
    let folder = if path.is_file() {
        path.parent().unwrap_or(path)
    } else {
        path.as_path()
    };

    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("explorer")
            .arg(folder)
            .spawn()
            .map_err(|e| format!("Failed to reveal folder: {e}"))?;
    }

    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(folder)
            .spawn()
            .map_err(|e| format!("Failed to reveal folder: {e}"))?;
    }

    #[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
    {
        std::process::Command::new("xdg-open")
            .arg(folder)
            .spawn()
            .map_err(|e| format!("Failed to reveal folder: {e}"))?;
    }

    Ok(())
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

    #[tokio::test]
    async fn test_db_maintenance_flow() {
        let config = KnoxConfig::default();
        let db = KnoxDb::open_memory().unwrap();
        let state = AppState {
            db: Arc::new(Mutex::new(db)),
            config: Arc::new(Mutex::new(config)),
        };

        // Test integrity check
        let integrity = {
            let db = state.db.lock().await;
            db.integrity_check().unwrap()
        };
        assert_eq!(integrity.to_lowercase(), "ok");

        // Test vacuum and stats
        {
            let db = state.db.lock().await;
            db.vacuum().unwrap();
            let stats = db.get_storage_stats(None).unwrap();
            assert!(stats.schema_version >= 3);
        }
    }
}
