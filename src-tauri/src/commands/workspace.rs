//! IPC command handlers for project initialization, knox.md, and git hooks.

use std::path::{Path, PathBuf};

use knox::commands::{cmd_hook_install, cmd_hook_uninstall, cmd_init};
use serde::{Deserialize, Serialize};
use tauri::State;

use crate::AppState;

/// Project representation sent to the frontend with egress permission state.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProjectDto {
    pub id: String,
    pub name: String,
    pub path: String,
    pub remote: Option<String>,
    pub created_at: String,
    pub egress: bool,
}

/// Initializes Knox in a project workspace, generating knox.md and registering in SQLite.
#[tauri::command]
pub async fn init_project(state: State<'_, AppState>, path: String) -> Result<String, String> {
    let path_buf = PathBuf::from(&path);
    let config = state.config.clone();
    tokio::task::spawn_blocking(move || {
        let config = config.blocking_lock();
        cmd_init(&path_buf, &config).map_err(|e| e.to_string())?;
        Ok(format!("Initialized Knox in {}", path_buf.display()))
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Installs the Knox pre-commit git hook in the specified project repository.
#[tauri::command]
pub async fn install_hook(path: String) -> Result<String, String> {
    let path_buf = PathBuf::from(&path);
    tokio::task::spawn_blocking(move || {
        cmd_hook_install(&path_buf).map_err(|e| e.to_string())?;
        Ok(format!("Installed git hook in {}", path_buf.display()))
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Uninstalls the Knox pre-commit git hook from the specified project repository.
#[tauri::command]
pub async fn uninstall_hook(path: String) -> Result<String, String> {
    let path_buf = PathBuf::from(&path);
    tokio::task::spawn_blocking(move || {
        cmd_hook_uninstall(&path_buf).map_err(|e| e.to_string())?;
        Ok(format!("Uninstalled git hook from {}", path_buf.display()))
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Reads the existing `knox.md` file in the specified project path if it exists.
#[tauri::command]
pub async fn get_knox_md(path: String) -> Result<Option<String>, String> {
    tokio::task::spawn_blocking(move || {
        let knox_md_path = Path::new(&path).join("knox.md");
        if knox_md_path.exists() {
            std::fs::read_to_string(&knox_md_path)
                .map(Some)
                .map_err(|e| e.to_string())
        } else {
            Ok(None)
        }
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Checks whether the Knox pre-commit hook is installed in the given project repository.
#[tauri::command]
pub async fn get_hook_status(path: String) -> Result<bool, String> {
    tokio::task::spawn_blocking(move || {
        let hook_path = Path::new(&path)
            .join(".git")
            .join("hooks")
            .join("pre-commit");
        if let Ok(content) = std::fs::read_to_string(&hook_path) {
            return Ok(content.contains("knox scan"));
        }
        Ok(false)
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Fetches all tracked projects from the database with egress permission state.
#[tauri::command]
pub async fn get_projects(
    state: State<'_, AppState>,
) -> Result<Vec<ProjectDto>, String> {
    let db = state.db.clone();
    tokio::task::spawn_blocking(move || {
        let db = db.blocking_lock();
        db.with_conn(|conn| {
            let mut stmt = conn
                .prepare("SELECT id, name, path, remote, created_at, egress FROM projects ORDER BY created_at ASC")
                .map_err(knox::core::error::KnoxError::Database)?;
            let rows = stmt
                .query_map([], |r| {
                    let mut path: String = r.get(2)?;
                    if let Some(stripped) = path.strip_prefix(r"\\?\") {
                        path = stripped.to_string();
                    }
                    Ok(ProjectDto {
                        id: r.get(0)?,
                        name: r.get(1)?,
                        path,
                        remote: r.get(3)?,
                        created_at: r.get(4)?,
                        egress: r.get::<_, i64>(5).unwrap_or(0) == 1,
                    })
                })
                .map_err(knox::core::error::KnoxError::Database)?;
            let mut projs = Vec::new();
            for r in rows {
                projs.push(r.map_err(knox::core::error::KnoxError::Database)?);
            }
            Ok(projs)
        })
        .map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Sets or revokes outbound cloud model egress permission for a project.
#[tauri::command]
pub async fn set_project_egress(
    state: State<'_, AppState>,
    project_id: String,
    allowed: bool,
) -> Result<(), String> {
    let db = state.db.clone();
    tokio::task::spawn_blocking(move || {
        let db = db.blocking_lock();
        knox::storage::set_project_egress(&db, &project_id, allowed)
            .map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_project_egress_toggle() {
        let db = knox::storage::KnoxDb::open_memory().unwrap();
        let proj = knox::core::types::Project {
            id: knox::core::ProjectId::new(),
            name: "Egress Test".to_string(),
            path: "/path/to/egress_test".to_string(),
            remote: None,
            created_at: chrono::Utc::now(),
        };
        knox::storage::insert_project(&db, &proj).unwrap();

        // Default is false
        assert!(!knox::storage::is_project_egress_allowed(&db, proj.id.as_str()).unwrap());

        // Toggle to true
        knox::storage::set_project_egress(&db, proj.id.as_str(), true).unwrap();
        assert!(knox::storage::is_project_egress_allowed(&db, proj.id.as_str()).unwrap());

        // Toggle back to false
        knox::storage::set_project_egress(&db, proj.id.as_str(), false).unwrap();
        assert!(!knox::storage::is_project_egress_allowed(&db, proj.id.as_str()).unwrap());
    }

    #[tokio::test]
    async fn test_hook_status_lifecycle() {
        let unique_id = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let temp = std::env::temp_dir().join(format!("knox_test_hook_{unique_id}"));
        let git_hooks = temp.join(".git").join("hooks");
        std::fs::create_dir_all(&git_hooks).unwrap();

        let path_str = temp.to_string_lossy().to_string();

        // 1. Initially false
        assert!(!get_hook_status(path_str.clone()).await.unwrap());

        // 2. After install -> true
        install_hook(path_str.clone()).await.unwrap();
        assert!(get_hook_status(path_str.clone()).await.unwrap());

        // 3. After uninstall -> false
        uninstall_hook(path_str.clone()).await.unwrap();
        assert!(!get_hook_status(path_str.clone()).await.unwrap());

        let _ = std::fs::remove_dir_all(&temp);
    }
}
