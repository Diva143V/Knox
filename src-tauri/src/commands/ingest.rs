//! IPC command handler for transcript ingestion.

use std::path::PathBuf;

use knox::commands::cmd_ingest;
use knox::core::types::IngestReport;
use tauri::State;

use crate::AppState;

/// Ingests a conversation transcript file (such as Antigravity `transcript.jsonl`)
/// into the persistent knowledge graph.
#[tauri::command]
pub async fn ingest_transcript(
    state: State<'_, AppState>,
    path: String,
) -> Result<IngestReport, String> {
    let db = state.db.lock().await;
    let path_buf = PathBuf::from(path);

    if !path_buf.exists() {
        return Err(format!("File does not exist: {}", path_buf.display()));
    }

    cmd_ingest(&path_buf, &db).map_err(|e| e.to_string())
}
