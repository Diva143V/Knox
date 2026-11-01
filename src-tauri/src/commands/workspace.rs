//! IPC command handlers for project initialization, knox.md, and git hooks.

use std::path::{Path, PathBuf};

use knox::commands::{cmd_hook_install, cmd_hook_uninstall, cmd_init};
use tauri::State;

use crate::AppState;

/// Initializes Knox in a project workspace, generating knox.md and registering in SQLite.
#[tauri::command]
pub async fn init_project(state: State<'_, AppState>, path: String) -> Result<String, String> {
    let path_buf = PathBuf::from(&path);
    let config = state.config.lock().await;
    cmd_init(&path_buf, &config).map_err(|e| e.to_string())?;
    Ok(format!("Initialized Knox in {}", path_buf.display()))
}

/// Installs the Knox pre-commit git hook in the specified project repository.
#[tauri::command]
pub async fn install_hook(path: String) -> Result<String, String> {
    let path_buf = PathBuf::from(&path);
    cmd_hook_install(&path_buf).map_err(|e| e.to_string())?;
    Ok(format!("Installed git hook in {}", path_buf.display()))
}

/// Uninstalls the Knox pre-commit git hook from the specified project repository.
#[tauri::command]
pub async fn uninstall_hook(path: String) -> Result<String, String> {
    let path_buf = PathBuf::from(&path);
    cmd_hook_uninstall(&path_buf).map_err(|e| e.to_string())?;
    Ok(format!("Uninstalled git hook from {}", path_buf.display()))
}

/// Reads the existing `knox.md` file in the specified project path if it exists.
#[tauri::command]
pub async fn get_knox_md(path: String) -> Result<Option<String>, String> {
    let knox_md_path = Path::new(&path).join("knox.md");
    if knox_md_path.exists() {
        std::fs::read_to_string(&knox_md_path)
            .map(Some)
            .map_err(|e| e.to_string())
    } else {
        Ok(None)
    }
}

/// Checks whether the Knox pre-commit hook is installed in the given project repository.
#[tauri::command]
pub async fn get_hook_status(path: String) -> Result<bool, String> {
    let hook_path = Path::new(&path)
        .join(".git")
        .join("hooks")
        .join("pre-commit");
    if let Ok(content) = std::fs::read_to_string(&hook_path) {
        return Ok(content.contains("knox scan"));
    }
    Ok(false)
}

/// Fetches all tracked projects from the database.
#[tauri::command]
pub async fn get_projects(
    state: State<'_, AppState>,
) -> Result<Vec<knox::core::types::Project>, String> {
    let db = state.db.lock().await;
    knox::storage::list_projects(&db).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

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
