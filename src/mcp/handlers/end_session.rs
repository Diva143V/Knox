//! Legacy `knox_end_session` adapter.
//!
//! Delegates to `knox_handoff` so ending a session always produces the same
//! digest a resumed session would receive.

use serde::Deserialize;
use serde_json::{Value, json};

use crate::core::error::{KnoxError, KnoxResult};
use crate::core::ids::SessionId;
use crate::storage::db::KnoxDb;

/// Input parameters for `knox_end_session`.
#[derive(Debug, Deserialize)]
pub struct EndSessionParams {
    /// Unique identifier of the session to conclude.
    pub session_id: String,
}

/// Handles the `knox_end_session` tool invocation.
pub fn handle_end_session(db: &KnoxDb, params: Value) -> KnoxResult<Value> {
    let input: EndSessionParams = serde_json::from_value(params)
        .map_err(|e| KnoxError::InvalidInput(format!("Invalid end_session params: {e}")))?;

    let trimmed = input.session_id.trim();
    if trimmed.is_empty() {
        return Err(KnoxError::InvalidInput(
            "Session ID must not be empty".to_string(),
        ));
    }

    let session_id = SessionId::from(trimmed);
    let handed_off = super::handle_handoff(
        db,
        json!({
            "session_id": session_id.as_str(),
        }),
    )?;

    let session = crate::storage::sessions::get_session(db, &session_id)?
        .ok_or_else(|| KnoxError::SessionNotFound(session_id.to_string()))?;
    let actions_recorded =
        crate::storage::sessions::count_session_actions(db, &session_id).unwrap_or(0);
    let duration_seconds = session
        .ended_at
        .map(|ended| (ended - session.started_at).num_seconds().max(0))
        .unwrap_or(0);

    Ok(json!({
        "status": "closed",
        "actions_recorded": actions_recorded,
        "duration_seconds": duration_seconds,
        "session_id": session_id.to_string(),
        "digest": handed_off.get("digest").cloned().unwrap_or(Value::Null),
        "deprecated": "knox_end_session is kept for compatibility; prefer knox_handoff"
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_handle_end_session_success() {
        let db = KnoxDb::open_memory().unwrap();
        let session = crate::storage::sessions::start_session(&db, "gemini", None).unwrap();

        let params = json!({
            "session_id": session.id.as_str()
        });

        let res = handle_end_session(&db, params).unwrap();
        assert_eq!(res.get("status").unwrap(), "closed");
        assert_eq!(res.get("actions_recorded").unwrap(), 0);
        assert!(res.get("digest").is_some());
    }

    #[test]
    fn test_handle_end_session_not_found() {
        let db = KnoxDb::open_memory().unwrap();
        let params = json!({
            "session_id": "non-existent-session"
        });

        let err = handle_end_session(&db, params).unwrap_err();
        assert!(matches!(err, KnoxError::SessionNotFound(_)));
    }
}
