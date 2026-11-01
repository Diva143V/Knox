//! IPC command handlers for recording quick thoughts, doubts, micro-decisions, and resolving doubts.

use chrono::Utc;
use knox::core::ids::ActionId;
use knox::core::types::KnoxAction;
use knox::storage::db::KnoxDb;
use knox::storage::nodes::{insert_action, resolve_doubt as db_resolve_doubt};
use tauri::State;

use crate::AppState;

/// Core business logic for recording a quick thought into the Knox database.
pub fn record_quick_thought_impl(
    db: &KnoxDb,
    summary: String,
    category: String,
    actor: Option<String>,
    context_summary: Option<String>,
    files: Option<Vec<String>>,
) -> Result<String, String> {
    let trimmed_summary = summary.trim();
    if trimmed_summary.is_empty() {
        return Err("Summary must not be empty".to_string());
    }

    let cat = category.trim().to_lowercase();
    let (action_type, significance, is_resolved) = match cat.as_str() {
        "doubt" | "question" => ("doubt".to_string(), "minor".to_string(), false),
        "minor_decision" | "minor" => ("decide".to_string(), "minor".to_string(), true),
        "major_decision" | "decision" | "decide" => {
            ("decide".to_string(), "major".to_string(), true)
        }
        "dialogue" | "conversation" => ("dialogue".to_string(), "minor".to_string(), true),
        _ => ("dialogue".to_string(), "minor".to_string(), true),
    };

    let action_id = ActionId::new();
    let actor_str = actor
        .map(|a| a.trim().to_string())
        .filter(|a| !a.is_empty())
        .unwrap_or_else(|| "human".to_string());

    let action = KnoxAction {
        id: action_id.clone(),
        actor: actor_str,
        action_type,
        summary: trimmed_summary.to_string(),
        files: files.unwrap_or_default(),
        tags: vec![cat],
        confidence: Some(1.0),
        is_current: true,
        timestamp: Utc::now(),
        session_id: None,
        parent_action_id: None,
        supersedes: None,
        relations: Vec::new(),
        symbols: Vec::new(),
        rationale: None,
        rejected_alternatives: Vec::new(),
        context_summary,
        significance: Some(significance),
        is_resolved: Some(is_resolved),
        resolution_note: None,
    };

    insert_action(db, &action).map_err(|e| e.to_string())?;

    Ok(action_id.to_string())
}

/// Core business logic for resolving an open doubt in the Knox database.
pub fn resolve_doubt_impl(
    db: &KnoxDb,
    doubt_id: String,
    resolution_note: String,
    resolver_action_id: Option<String>,
) -> Result<bool, String> {
    let trimmed_id = doubt_id.trim();
    if trimmed_id.is_empty() {
        return Err("doubt_id must not be empty".to_string());
    }
    let trimmed_note = resolution_note.trim();
    if trimmed_note.is_empty() {
        return Err("resolution_note must not be empty".to_string());
    }

    let action_id = ActionId::from(trimmed_id);
    let resolver_id = resolver_action_id
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .map(ActionId::from);

    db_resolve_doubt(db, &action_id, trimmed_note, resolver_id.as_ref())
        .map_err(|e| e.to_string())?;

    Ok(true)
}

/// Records a quick thought, doubt, or decision from the Desktop UI.
#[tauri::command]
pub async fn record_quick_thought(
    state: State<'_, AppState>,
    summary: String,
    category: String,
    actor: Option<String>,
    context_summary: Option<String>,
    files: Option<Vec<String>>,
) -> Result<String, String> {
    let db = state.db.lock().await;
    record_quick_thought_impl(&db, summary, category, actor, context_summary, files)
}

/// Resolves an open doubt in the knowledge graph.
#[tauri::command]
pub async fn resolve_doubt(
    state: State<'_, AppState>,
    doubt_id: String,
    resolution_note: String,
    resolver_action_id: Option<String>,
) -> Result<bool, String> {
    let db = state.db.lock().await;
    resolve_doubt_impl(&db, doubt_id, resolution_note, resolver_action_id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_record_quick_thought_and_resolve() {
        let db = KnoxDb::open_memory().unwrap();

        // Test recording a doubt
        let id_res = record_quick_thought_impl(
            &db,
            "Will SQLite connection pool block async UI queries?".to_string(),
            "doubt".to_string(),
            Some("engineer".to_string()),
            Some("Architecture review".to_string()),
            None,
        );
        assert!(id_res.is_ok());
        let doubt_id = id_res.unwrap();

        // Verify doubt is stored as unresolved
        let action = knox::storage::nodes::get_action(&db, &ActionId::from(doubt_id.clone()))
            .unwrap()
            .unwrap();
        assert_eq!(action.action_type, "doubt");
        assert_eq!(action.is_resolved, Some(false));
        assert_eq!(action.significance.as_deref(), Some("minor"));

        // Test resolving the doubt
        let resolve_res = resolve_doubt_impl(
            &db,
            doubt_id.clone(),
            "Verified SQLite in WAL mode handles readers concurrently with one writer.".to_string(),
            None,
        );
        assert!(resolve_res.is_ok());

        // Verify doubt is marked resolved
        let action = knox::storage::nodes::get_action(&db, &ActionId::from(doubt_id))
            .unwrap()
            .unwrap();
        assert_eq!(action.is_resolved, Some(true));
        assert!(action.resolution_note.is_some());
    }
}
