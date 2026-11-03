//! High-level MCP session begin workflow.

use serde::Deserialize;
use serde_json::{Value, json};

use crate::core::budget::DEFAULT_CONTEXT_BUDGET;
use crate::core::error::{KnoxError, KnoxResult};
use crate::storage::db::KnoxDb;

#[derive(Debug, Deserialize)]
pub struct BeginParams {
    pub workspace: Option<String>,
    pub actor: String,
    pub task: Option<String>,
    pub budget: Option<usize>,
}

pub fn handle_begin(db: &KnoxDb, params: Value) -> KnoxResult<Value> {
    let input: BeginParams = serde_json::from_value(params)
        .map_err(|e| KnoxError::InvalidInput(format!("Invalid begin params: {e}")))?;
    let actor = input.actor.trim();
    if actor.is_empty() {
        return Err(KnoxError::InvalidInput("Actor must not be empty".to_string()));
    }

    let existing = crate::storage::sessions::find_open_session(
        db,
        actor,
        input.workspace.as_deref(),
    )?;
    let (session, resumed) = match existing {
        Some(session) => (session, true),
        None => (
            crate::storage::sessions::start_session(
                db,
                actor,
                input.workspace.as_deref(),
            )?,
            false,
        ),
    };

    let mut context = json!({
        "session_id": session.id.to_string(),
        "project": input.workspace,
        "actor": actor,
        "task": input.task,
        "current_only": true,
        "budget": input.budget.unwrap_or(DEFAULT_CONTEXT_BUDGET),
        "format": "compact"
    });
    if let Some(task) = context.get("task").and_then(Value::as_str) {
        context["query"] = Value::String(task.to_string());
    }

    let context = super::handle_context(db, context)?;
    Ok(json!({
        "scope": {
            "session_id": session.id.to_string(),
            "project": session.project,
            "actor": actor,
            "resumed": resumed
        },
        "context": context
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resumes_same_open_actor_and_project_session() {
        let db = KnoxDb::open_memory().unwrap();
        let first = handle_begin(
            &db,
            json!({"actor": "gemini", "workspace": "knox", "task": "inspect storage"}),
        )
        .unwrap();
        let second = handle_begin(
            &db,
            json!({"actor": "gemini", "workspace": "knox", "task": "inspect storage"}),
        )
        .unwrap();
        assert_eq!(first["scope"]["session_id"], second["scope"]["session_id"]);
        assert_eq!(second["scope"]["resumed"], true);
    }

    #[test]
    fn isolates_open_sessions_by_actor() {
        let db = KnoxDb::open_memory().unwrap();
        let first = handle_begin(&db, json!({"actor": "gemini"})).unwrap();
        let second = handle_begin(&db, json!({"actor": "claude"})).unwrap();
        assert_ne!(first["scope"]["session_id"], second["scope"]["session_id"]);
    }
}
