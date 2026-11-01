//! IPC command handler for Knowledge Graph data.

use knox::core::types::{KnoxAction, QueryFilter};
use knox::storage::queries::query_actions;
use tauri::State;

use crate::AppState;

/// Fetches graph actions for rendering on the React Flow canvas.
#[tauri::command]
pub async fn get_graph_data(
    state: State<'_, AppState>,
    current_only: Option<bool>,
    limit: Option<usize>,
) -> Result<Vec<KnoxAction>, String> {
    let db = state.db.lock().await;

    let filter = QueryFilter {
        current_only: current_only.unwrap_or(false),
        limit: limit.unwrap_or(200).clamp(1, 1000),
        ..Default::default()
    };

    let result = query_actions(&db, &filter).map_err(|e| e.to_string())?;
    Ok(result.results)
}

/// Updates an existing graph action or decision node.
#[tauri::command]
pub async fn update_action(
    state: State<'_, AppState>,
    params: knox::core::types::UpdateActionParams,
) -> Result<KnoxAction, String> {
    let db = state.db.lock().await;
    knox::storage::nodes::update_action(&db, &params).map_err(|e| e.to_string())
}
