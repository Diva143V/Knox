//! IPC command handler for native file dialog selection.

use tauri::AppHandle;
use tauri_plugin_dialog::DialogExt;

/// Opens a native file dialog for selecting a file path.
#[tauri::command]
pub async fn open_file_dialog(
    app: AppHandle,
    name: Option<String>,
    extensions: Option<Vec<String>>,
) -> Result<Option<String>, String> {
    let (tx, rx) = tokio::sync::oneshot::channel();
    let mut builder = app.dialog().file();

    if let Some(exts) = extensions {
        let filter_name = name.unwrap_or_else(|| "Files".to_string());
        let ext_refs: Vec<&str> = exts.iter().map(|s| s.as_str()).collect();
        builder = builder.add_filter(filter_name, &ext_refs);
    }

    builder.pick_file(move |file_path| {
        let _ = tx.send(file_path.map(|p| p.to_string()));
    });

    rx.await.map_err(|e| e.to_string())
}

/// Opens a native folder dialog for selecting a workspace or repository directory.
#[tauri::command]
pub async fn open_folder_dialog(app: AppHandle) -> Result<Option<String>, String> {
    let (tx, rx) = tokio::sync::oneshot::channel();
    app.dialog().file().pick_folder(move |folder_path| {
        let _ = tx.send(folder_path.map(|p| p.to_string()));
    });
    rx.await.map_err(|e| e.to_string())
}
