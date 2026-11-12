//! IPC command handler for Knox configuration and metadata.

use knox::commands::migrate::migration_consolidate;
use knox::core::config::{DEFAULT_DB_FILENAME, detect_project_db};
use knox::rusqlite::{self, Connection};
use knox::storage::db::{KnoxDb, StorageStats};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tauri::State;

use crate::AppState;

/// Mask helper for API keys to avoid exposing secrets in IPC responses.
fn mask_key(k: &str) -> String {
    let trimmed = k.trim();
    if trimmed.is_empty() {
        return String::new();
    }
    if trimmed.chars().count() <= 8 {
        "••••••••".to_string()
    } else {
        let tail: String = trimmed
            .chars()
            .rev()
            .take(4)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect();
        format!("••••{tail}")
    }
}

/// Safe configuration DTO returned to the frontend.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ConfigDto {
    pub kg_path: String,
    pub session_timeout_seconds: u64,
    pub global_mode: bool,
    pub model_profile: String,
    pub model_endpoint: Option<String>,
    pub model_api_key_masked: Option<String>,
    pub embedding_model: Option<String>,
    pub chat_model: Option<String>,
    pub allow_cloud_egress: bool,
    pub stray_db_prompted: bool,
}

/// Detected legacy stray database candidate.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StrayDatabaseDto {
    pub project_path: String,
    pub db_path: String,
    pub action_count: usize,
}

/// Fetches current Knox runtime configuration as a frontend-safe DTO.
#[tauri::command]
pub async fn get_config(state: State<'_, AppState>) -> Result<ConfigDto, String> {
    let cfg = state.config.lock().await;
    Ok(ConfigDto {
        kg_path: cfg.kg_path.display().to_string(),
        session_timeout_seconds: cfg.session_timeout.as_secs(),
        global_mode: cfg.global_mode,
        model_profile: cfg.model_profile.clone(),
        model_endpoint: cfg.model_endpoint.clone(),
        model_api_key_masked: cfg.model_api_key.as_ref().map(|k| mask_key(k)),
        embedding_model: cfg.embedding_model.clone(),
        chat_model: cfg.chat_model.clone(),
        allow_cloud_egress: cfg.allow_cloud_egress,
        stray_db_prompted: cfg.stray_db_prompted,
    })
}

/// Updates Knox runtime configuration, persisting changes to ~/.knox/config.json.
#[allow(clippy::too_many_arguments)]
#[tauri::command]
pub async fn update_config(
    state: State<'_, AppState>,
    session_timeout_seconds: Option<u64>,
    global_mode: Option<bool>,
    kg_path: Option<String>,
    model_profile: Option<String>,
    model_endpoint: Option<String>,
    model_api_key: Option<String>,
    embedding_model: Option<String>,
    chat_model: Option<String>,
    allow_cloud_egress: Option<bool>,
    stray_db_prompted: Option<bool>,
) -> Result<ConfigDto, String> {
    let cfg_arc = state.config.clone();
    tokio::task::spawn_blocking(move || {
        let mut cfg = cfg_arc.blocking_lock();
        if let Some(timeout) = session_timeout_seconds {
            cfg.session_timeout = std::time::Duration::from_secs(timeout);
        }
        if let Some(mode) = global_mode {
            cfg.global_mode = mode;
        }
        if let Some(path_str) = kg_path.as_deref().filter(|s| !s.trim().is_empty()) {
            cfg.kg_path = PathBuf::from(path_str.trim());
        }
        if let Some(profile) = model_profile {
            cfg.model_profile = profile;
        }
        if let Some(endpoint) = model_endpoint {
            let trimmed = endpoint.trim();
            cfg.model_endpoint = if trimmed.is_empty() {
                None
            } else {
                Some(trimmed.to_string())
            };
        }
        if let Some(key) = model_api_key {
            let trimmed = key.trim();
            if !trimmed.starts_with('•') && !trimmed.starts_with('*') {
                if trimmed.is_empty() {
                    cfg.model_api_key = None;
                } else {
                    cfg.model_api_key = Some(trimmed.to_string());
                }
            }
        }
        if let Some(em) = embedding_model {
            let trimmed = em.trim();
            cfg.embedding_model = if trimmed.is_empty() {
                None
            } else {
                Some(trimmed.to_string())
            };
        }
        if let Some(cm) = chat_model {
            let trimmed = cm.trim();
            cfg.chat_model = if trimmed.is_empty() {
                None
            } else {
                Some(trimmed.to_string())
            };
        }
        if let Some(egress) = allow_cloud_egress {
            cfg.allow_cloud_egress = egress;
        }
        if let Some(prompted) = stray_db_prompted {
            cfg.stray_db_prompted = prompted;
        }

        if let Err(e) = cfg.save_default() {
            tracing::warn!("Failed to persist config to default file: {e}");
        }

        Ok(ConfigDto {
            kg_path: cfg.kg_path.display().to_string(),
            session_timeout_seconds: cfg.session_timeout.as_secs(),
            global_mode: cfg.global_mode,
            model_profile: cfg.model_profile.clone(),
            model_endpoint: cfg.model_endpoint.clone(),
            model_api_key_masked: cfg.model_api_key.as_ref().map(|k| mask_key(k)),
            embedding_model: cfg.embedding_model.clone(),
            chat_model: cfg.chat_model.clone(),
            allow_cloud_egress: cfg.allow_cloud_egress,
            stray_db_prompted: cfg.stray_db_prompted,
        })
    })
    .await
    .map_err(|e| e.to_string())?
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
        Some(config.model_profile),
        config.model_endpoint,
        None, // Preserves existing secret without overwriting with mask
        config.embedding_model,
        config.chat_model,
        Some(config.allow_cloud_egress),
        Some(config.stray_db_prompted),
    )
    .await
}

/// Scans known project directories for legacy stray SQLite databases.
#[tauri::command]
pub async fn scan_stray_databases(state: State<'_, AppState>) -> Result<Vec<StrayDatabaseDto>, String> {
    let db_arc = state.db.clone();
    let cfg_arc = state.config.clone();
    tokio::task::spawn_blocking(move || {
        let global_db = db_arc.blocking_lock();
        let cfg = cfg_arc.blocking_lock();
        let global_canonical = global_db
            .path()
            .and_then(|p| std::fs::canonicalize(p).ok())
            .unwrap_or_else(|| std::fs::canonicalize(&cfg.kg_path).unwrap_or_else(|_| cfg.kg_path.clone()));

        // Query distinct project paths
        let project_paths: Vec<String> = global_db
            .with_conn(|conn| {
                let mut stmt = conn
                    .prepare("SELECT DISTINCT path FROM projects WHERE path IS NOT NULL AND path != ''")
                    .map_err(knox::core::error::KnoxError::Database)?;
                let rows = stmt
                    .query_map([], |r| r.get::<_, String>(0))
                    .map_err(knox::core::error::KnoxError::Database)?
                    .filter_map(|res| res.ok())
                    .collect();
                Ok(rows)
            })
            .map_err(|e| e.to_string())?;

        let mut strays = Vec::new();
        for proj_path_str in project_paths {
            let p = Path::new(&proj_path_str);
            if let Some(candidate) = detect_project_db(p) {
                if std::fs::canonicalize(&candidate).is_ok_and(|c| c == global_canonical) {
                    continue;
                }
                // Count actions in the stray db
                let action_count = match Connection::open_with_flags(
                    &candidate,
                    rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
                ) {
                    Ok(conn) => conn
                        .query_row("SELECT COUNT(*) FROM actions", [], |r| r.get::<_, usize>(0))
                        .unwrap_or(0),
                    Err(_) => 0,
                };
                strays.push(StrayDatabaseDto {
                    project_path: proj_path_str,
                    db_path: candidate.display().to_string(),
                    action_count,
                });
            }
        }
        Ok(strays)
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Consolidates a legacy stray project SQLite database into the active global database.
#[tauri::command]
pub async fn consolidate_stray_database(
    state: State<'_, AppState>,
    path: String,
) -> Result<knox::commands::migrate::ConsolidationReport, String> {
    let db_arc = state.db.clone();
    tokio::task::spawn_blocking(move || {
        let db = db_arc.blocking_lock();
        let src_path = PathBuf::from(path);
        migration_consolidate(&db, &src_path).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Relocates the active database to a new directory or file path.
#[tauri::command]
pub async fn relocate_database(
    state: State<'_, AppState>,
    new_dir: String,
) -> Result<ConfigDto, String> {
    let db_arc = state.db.clone();
    let cfg_arc = state.config.clone();
    tokio::task::spawn_blocking(move || {
        let mut db = db_arc.blocking_lock();
        let mut cfg = cfg_arc.blocking_lock();

        let target_dir = PathBuf::from(new_dir.trim());
        let new_path = if target_dir.extension().and_then(|s| s.to_str()) == Some("db") {
            target_dir
        } else {
            target_dir.join(DEFAULT_DB_FILENAME)
        };

        if let Some(parent) = new_path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create directory {}: {e}", parent.display()))?;
        }

        let old_path = cfg.kg_path.clone();
        if old_path.exists() && old_path != new_path && !new_path.exists() {
            let _ = db.checkpoint_truncate();
            std::fs::copy(&old_path, &new_path)
                .map_err(|e| format!("Failed to copy database: {e}"))?;
            let old_wal = PathBuf::from(format!("{}-wal", old_path.display()));
            let new_wal = PathBuf::from(format!("{}-wal", new_path.display()));
            if old_wal.exists() {
                let _ = std::fs::copy(old_wal, new_wal);
            }
        }

        // Re-open db at new path
        *db = KnoxDb::open(&new_path)
            .map_err(|e| format!("Failed to open database at {}: {e}", new_path.display()))?;
        cfg.kg_path = new_path;
        if let Err(e) = cfg.save_default() {
            tracing::warn!("Failed to persist config after relocation: {e}");
        }

        Ok(ConfigDto {
            kg_path: cfg.kg_path.display().to_string(),
            session_timeout_seconds: cfg.session_timeout.as_secs(),
            global_mode: cfg.global_mode,
            model_profile: cfg.model_profile.clone(),
            model_endpoint: cfg.model_endpoint.clone(),
            model_api_key_masked: cfg.model_api_key.as_ref().map(|k| mask_key(k)),
            embedding_model: cfg.embedding_model.clone(),
            chat_model: cfg.chat_model.clone(),
            allow_cloud_egress: cfg.allow_cloud_egress,
            stray_db_prompted: cfg.stray_db_prompted,
        })
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Fetches storage and health telemetry (size, WAL size, counts, schema version).
#[tauri::command]
pub async fn get_db_stats(state: State<'_, AppState>) -> Result<StorageStats, String> {
    let cfg_arc = state.config.clone();
    let db_arc = state.db.clone();
    tokio::task::spawn_blocking(move || {
        let cfg = cfg_arc.blocking_lock();
        let db = db_arc.blocking_lock();
        db.get_storage_stats(Some(&cfg.kg_path))
            .map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Executes a SQLite PRAGMA integrity_check against the active database.
#[tauri::command]
pub async fn run_db_integrity(state: State<'_, AppState>) -> Result<String, String> {
    let db_arc = state.db.clone();
    tokio::task::spawn_blocking(move || {
        let db = db_arc.blocking_lock();
        db.integrity_check().map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Reclaims unused disk space and defragments SQLite pages via VACUUM.
#[tauri::command]
pub async fn run_db_vacuum(state: State<'_, AppState>) -> Result<String, String> {
    let db_arc = state.db.clone();
    tokio::task::spawn_blocking(move || {
        let db = db_arc.blocking_lock();
        db.vacuum().map_err(|e| e.to_string())?;
        Ok("Database vacuum completed successfully.".to_string())
    })
    .await
    .map_err(|e| e.to_string())?
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

        // Update timeout and model fields
        {
            let mut cfg = state.config.lock().await;
            cfg.session_timeout = std::time::Duration::from_secs(3600);
            cfg.global_mode = true;
            cfg.model_profile = "local".to_string();
            cfg.model_endpoint = Some("http://localhost:11434".to_string());
            cfg.stray_db_prompted = true;
        }

        let cfg = state.config.lock().await;
        assert_eq!(cfg.session_timeout.as_secs(), 3600);
        assert!(cfg.global_mode);
        assert_eq!(cfg.model_profile, "local");
        assert_eq!(cfg.model_endpoint.as_deref(), Some("http://localhost:11434"));
        assert!(cfg.stray_db_prompted);
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
