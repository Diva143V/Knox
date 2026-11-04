//! Session begin command.
//!
//! Business logic for `knox_begin`: finds or creates a scoped session, links
//! it to the previous session for the same actor/project, computes budgeted
//! context, and returns the scope + context for the turn.

use serde::Deserialize;
use serde_json::{Value, json};

use crate::core::budget::DEFAULT_CONTEXT_BUDGET;
use crate::core::error::{KnoxError, KnoxResult};
use crate::storage::db::KnoxDb;

/// Input parameters for `knox_begin`.
#[derive(Debug, Deserialize)]
pub struct BeginParams {
    /// Project scope filter (required — cross-project sessions are rejected).
    pub workspace: String,
    pub actor: String,
    pub task: Option<String>,
    pub budget: Option<usize>,
}

/// Runs the session begin workflow for `knox_begin`.
///
/// Finds the most recent open session for the actor/workspace, or creates a
/// new one linked to the previous session for the same actor/project.
/// Returns `scope` (session metadata) and `context` (budgeted task context).
pub fn cmd_begin(db: &KnoxDb, params: Value) -> KnoxResult<Value> {
    let input: BeginParams = serde_json::from_value(params)
        .map_err(|e| KnoxError::InvalidInput(format!("Invalid begin params: {e}")))?;
    let actor = input.actor.trim();
    if actor.is_empty() {
        return Err(KnoxError::InvalidInput(
            "Actor must not be empty".to_string(),
        ));
    }

    let existing =
        crate::storage::sessions::find_open_session(db, actor, Some(&input.workspace))?;
    let (session, resumed, parent_session) = match existing {
        Some(session) => (session, true, None),
        None => {
            let session =
                crate::storage::sessions::start_session(db, actor, Some(&input.workspace))?;
            // Link the new session to the most recent session for the same
            // actor and project so handoff digests form a chain.
            let previous = crate::storage::sessions::list_recent_sessions(db, 50)?
                .into_iter()
                .find(|candidate| {
                    candidate.id != session.id
                        && candidate.actor == actor
                        && candidate.project_id == session.project_id
                });
            if let Some(previous) = previous {
                crate::storage::sessions::set_parent_session(db, &session.id, &previous.id)?;
                (session, false, Some(previous.id))
            } else {
                (session, false, None)
            }
        }
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

    let context = crate::commands::context::cmd_context(db, context)?;
    Ok(json!({
        "scope": {
            "session_id": session.id.to_string(),
            "project": session.project,
            "actor": actor,
            "resumed": resumed,
            "parent_session_id": parent_session.map(|id| id.to_string())
        },
        "context": context
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn resumes_same_open_actor_and_project_session() {
        let db = KnoxDb::open_memory().unwrap();
        let first = cmd_begin(
            &db,
            json!({"actor": "gemini", "workspace": "knox", "task": "inspect storage"}),
        )
        .unwrap();
        let second = cmd_begin(
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
        let first = cmd_begin(&db, json!({"actor": "gemini", "workspace": "demo"})).unwrap();
        let second = cmd_begin(&db, json!({"actor": "claude", "workspace": "demo"})).unwrap();
        assert_ne!(first["scope"]["session_id"], second["scope"]["session_id"]);
    }

    #[test]
    fn new_session_links_to_the_previous_one() {
        let db = KnoxDb::open_memory().unwrap();
        let first = cmd_begin(&db, json!({"actor": "gemini", "workspace": "demo"})).unwrap();
        let first_id = first["scope"]["session_id"].as_str().unwrap().to_string();
        crate::commands::handoff::cmd_handoff(
            &db,
            json!({"session_id": first_id}),
        )
        .unwrap();

        let second = cmd_begin(&db, json!({"actor": "gemini", "workspace": "demo"})).unwrap();
        assert_eq!(second["scope"]["resumed"], false);
        assert_eq!(
            second["scope"]["parent_session_id"].as_str().unwrap(),
            first_id
        );
    }
}
