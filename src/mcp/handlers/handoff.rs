//! High-level MCP session handoff workflow.

use serde::Deserialize;
use serde_json::{Value, json};

use crate::core::budget::DEFAULT_CONTEXT_BUDGET;
use crate::core::error::{KnoxError, KnoxResult};
use crate::core::ids::SessionId;
use crate::storage::db::KnoxDb;

#[derive(Debug, Deserialize)]
pub struct HandoffParams {
    pub session_id: String,
    pub budget: Option<usize>,
}

pub fn handle_handoff(db: &KnoxDb, params: Value) -> KnoxResult<Value> {
    let input: HandoffParams = serde_json::from_value(params)
        .map_err(|e| KnoxError::InvalidInput(format!("Invalid handoff params: {e}")))?;
    let session_id = input.session_id.trim();
    if session_id.is_empty() {
        return Err(KnoxError::InvalidInput("Session ID must not be empty".to_string()));
    }

    let id = SessionId::from(session_id);
    let session = crate::storage::sessions::get_session(db, &id)?
        .ok_or_else(|| KnoxError::SessionNotFound(session_id.to_string()))?;
    let digest = super::handle_context(
        db,
        json!({
            "session_id": session_id,
            "budget": input.budget.unwrap_or(DEFAULT_CONTEXT_BUDGET),
            "format": "compact",
            "current_only": true
        }),
    )?;
    let closed = if session.is_open {
        crate::storage::sessions::end_session(db, &id)?
    } else {
        session
    };
    Ok(json!({
        "status": "closed",
        "session_id": session_id,
        "ended_at": closed.ended_at.map(|time| time.to_rfc3339()),
        "digest": digest
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn returns_digest_and_closes_session() {
        let db = KnoxDb::open_memory().unwrap();
        let session = crate::storage::sessions::start_session(&db, "gemini", None).unwrap();
        let result = handle_handoff(
            &db,
            json!({"session_id": session.id.to_string(), "budget": 100}),
        )
        .unwrap();
        assert_eq!(result["status"], "closed");
        assert!(!crate::storage::sessions::get_session(&db, &session.id)
            .unwrap()
            .unwrap()
            .is_open);
    }
}
