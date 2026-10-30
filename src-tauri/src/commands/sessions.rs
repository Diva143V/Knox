//! IPC command handlers for Knox session management.

use knox::core::ids::SessionId;
use knox::core::types::Session;
use knox::storage::sessions::{
    end_session as db_end_session, list_recent_sessions as db_list_recent_sessions,
    start_session as db_start_session,
};
use tauri::State;

use crate::AppState;

/// Starts a new session in Knox for a given actor and project.
#[tauri::command]
pub async fn start_session(
    state: State<'_, AppState>,
    actor: String,
    project: Option<String>,
) -> Result<Session, String> {
    let db = state.db.lock().await;
    db_start_session(&db, &actor, project.as_deref()).map_err(|e| e.to_string())
}

/// Ends an active session in Knox.
#[tauri::command]
pub async fn end_session(
    state: State<'_, AppState>,
    session_id: String,
) -> Result<Session, String> {
    let db = state.db.lock().await;
    db_end_session(&db, &SessionId::from(session_id)).map_err(|e| e.to_string())
}

/// Lists open and recent sessions in Knox.
#[tauri::command]
pub async fn get_sessions(
    state: State<'_, AppState>,
    limit: Option<usize>,
) -> Result<Vec<Session>, String> {
    let db = state.db.lock().await;
    let max_limit = limit.unwrap_or(20).clamp(1, 100);
    db_list_recent_sessions(&db, max_limit).map_err(|e| e.to_string())
}
