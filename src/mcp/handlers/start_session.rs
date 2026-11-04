//! Legacy `knox_start_session` adapter.
//!
//! Delegates to the high-level `knox_begin` workflow so resumption rules are
//! shared, while keeping the original response keys for existing clients.

use serde::Deserialize;
use serde_json::{Value, json};

use crate::core::error::{KnoxError, KnoxResult};
use crate::core::ids::SessionId;
use crate::storage::db::KnoxDb;

/// Input parameters for `knox_start_session`.
#[derive(Debug, Deserialize)]
pub struct StartSessionParams {
    /// Agent or software name starting the session (e.g. "gemini", "cursor").
    pub actor: String,
    /// Optional project identifier, name, or path.
    #[serde(default)]
    pub project: Option<String>,
    /// Optional arbitrary metadata dictionary (accepted for compatibility).
    #[serde(default)]
    pub metadata: Option<Value>,
}

/// Handles the `knox_start_session` tool invocation.
pub fn handle_start_session(db: &KnoxDb, params: Value) -> KnoxResult<Value> {
    let input: StartSessionParams = serde_json::from_value(params)
        .map_err(|e| KnoxError::InvalidInput(format!("Invalid start_session params: {e}")))?;

    if input.actor.trim().is_empty() {
        return Err(KnoxError::InvalidInput(
            "Actor must not be empty".to_string(),
        ));
    }

    let began = super::handle_begin(
        db,
        json!({
            "actor": input.actor.trim(),
            "workspace": input.project,
        }),
    )?;

    let session_id = began
        .get("scope")
        .and_then(|scope| scope.get("session_id"))
        .and_then(Value::as_str)
        .ok_or_else(|| KnoxError::InvalidInput("begin did not return a session".to_string()))?;
    let session = crate::storage::sessions::get_session(db, &SessionId::from(session_id))?
        .ok_or_else(|| KnoxError::SessionNotFound(session_id.to_string()))?;

    Ok(json!({
        "session_id": session.id.to_string(),
        "created_at": session.started_at.to_rfc3339(),
        "resumed": began.get("scope").and_then(|scope| scope.get("resumed")).cloned().unwrap_or(json!(false)),
        "deprecated": "knox_start_session is kept for compatibility; prefer knox_begin"
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_handle_start_session_success() {
        let db = KnoxDb::open_memory().unwrap();
        let params = json!({
            "actor": "gemini",
            "project": "knox"
        });

        let res = handle_start_session(&db, params).unwrap();
        assert!(res.get("session_id").is_some());
        assert!(res.get("created_at").is_some());
    }

    #[test]
    fn test_handle_start_session_resumes_open_session() {
        let db = KnoxDb::open_memory().unwrap();
        let first = handle_start_session(&db, json!({"actor": "gemini"})).unwrap();
        let second = handle_start_session(&db, json!({"actor": "gemini"})).unwrap();
        assert_eq!(first["session_id"], second["session_id"]);
        assert_eq!(second["resumed"], true);
    }

    #[test]
    fn test_handle_start_session_empty_actor() {
        let db = KnoxDb::open_memory().unwrap();
        let params = json!({
            "actor": "   "
        });

        let err = handle_start_session(&db, params).unwrap_err();
        assert!(matches!(err, KnoxError::InvalidInput(_)));
    }
}
