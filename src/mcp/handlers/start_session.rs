//! MCP tool handler for `knox_start_session`.

use serde::Deserialize;
use serde_json::{Value, json};

use crate::core::error::{KnoxError, KnoxResult};
use crate::storage::db::KnoxDb;

/// Input parameters for `knox_start_session`.
#[derive(Debug, Deserialize)]
pub struct StartSessionParams {
    /// Agent or software name starting the session (e.g. "gemini", "cursor").
    pub actor: String,
    /// Optional project identifier, name, or path.
    #[serde(default)]
    pub project: Option<String>,
    /// Optional arbitrary metadata dictionary.
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

    let session =
        crate::storage::sessions::start_session(db, input.actor.trim(), input.project.as_deref())?;

    Ok(json!({
        "session_id": session.id.to_string(),
        "created_at": session.started_at.to_rfc3339()
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
    fn test_handle_start_session_empty_actor() {
        let db = KnoxDb::open_memory().unwrap();
        let params = json!({
            "actor": "   "
        });

        let err = handle_start_session(&db, params).unwrap_err();
        assert!(matches!(err, KnoxError::InvalidInput(_)));
    }
}
