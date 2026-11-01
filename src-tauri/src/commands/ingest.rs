//! IPC command handlers for multi-agent transcript ingestion, previews, and discovery.

use std::path::PathBuf;

use knox::commands::{cmd_ingest, cmd_ingest_git, cmd_ingest_raw, cmd_preview};
use knox::core::types::{DetectedTranscript, IngestPreview, IngestReport};
use knox::ingest::detect_local_transcripts as detect_transcripts_core;
use tauri::State;

use crate::AppState;

/// Ingests a conversation transcript file (Antigravity, Claude, Cursor, ChatGPT, etc.)
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

/// Ingests raw transcript content (JSON, JSONL, Markdown, or plain text) directly.
#[tauri::command]
pub async fn ingest_raw_transcript(
    state: State<'_, AppState>,
    content: String,
    actor_override: Option<String>,
) -> Result<IngestReport, String> {
    let db = state.db.lock().await;
    cmd_ingest_raw(&content, &db, actor_override.as_deref()).map_err(|e| e.to_string())
}

/// Generates a dry-run preview of a transcript file or raw text before committing to SQLite.
#[tauri::command]
pub async fn preview_transcript(
    state: State<'_, AppState>,
    path: Option<String>,
    content: Option<String>,
) -> Result<IngestPreview, String> {
    let db = state.db.lock().await;
    let path_buf = path.map(PathBuf::from);

    cmd_preview(path_buf.as_deref(), content.as_deref(), &db).map_err(|e| e.to_string())
}

/// Ingests commit history from a local Git repository.
#[tauri::command]
pub async fn ingest_git_history(
    state: State<'_, AppState>,
    project_path: String,
    max_commits: Option<usize>,
) -> Result<IngestReport, String> {
    let db = state.db.lock().await;
    let path_buf = PathBuf::from(project_path);

    if !path_buf.exists() {
        return Err(format!("Project directory does not exist: {}", path_buf.display()));
    }

    cmd_ingest_git(&path_buf, max_commits.unwrap_or(50), &db).map_err(|e| e.to_string())
}

/// Scans local directories and active workspace for discoverable agent transcripts.
#[tauri::command]
pub async fn detect_local_transcripts(
    _state: State<'_, AppState>,
    project_path: Option<String>,
) -> Result<Vec<DetectedTranscript>, String> {
    let path_buf = project_path.map(PathBuf::from);
    Ok(detect_transcripts_core(path_buf.as_deref()))
}
