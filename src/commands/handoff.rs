//! Session handoff command.
//!
//! Business logic for `knox_handoff`: computes a context digest for the
//! session, closes it if open, and returns the digest for the next session.

use serde::Deserialize;
use serde_json::{Value, json};

use crate::core::budget::DEFAULT_HANDOFF_BUDGET;
use crate::core::error::{KnoxError, KnoxResult};
use crate::core::ids::SessionId;
use crate::storage::db::KnoxDb;

/// Input parameters for `knox_handoff`.
#[derive(Debug, Deserialize)]
pub struct HandoffParams {
    pub session_id: String,
    pub budget: Option<usize>,
}

/// Runs the session handoff for `knox_handoff`.
///
/// Produces a context digest for the session, ends the session if it is
/// still open, and returns the digest along with close metadata.
pub fn cmd_handoff(db: &KnoxDb, params: Value) -> KnoxResult<Value> {
    let start_instant = std::time::Instant::now();
    let input: HandoffParams = serde_json::from_value(params)
        .map_err(|e| KnoxError::InvalidInput(format!("Invalid handoff params: {e}")))?;
    let session_id = input.session_id.trim();
    if session_id.is_empty() {
        return Err(KnoxError::InvalidInput("Session ID must not be empty".to_string()));
    }

    let id = SessionId::from(session_id);
    let session = crate::storage::sessions::get_session(db, &id)?
        .ok_or_else(|| KnoxError::SessionNotFound(session_id.to_string()))?;
    let project_id_str = session.project_id.as_ref().map(|p| p.to_string());
    let digest = {
        let project_param = project_id_str.clone();
        crate::mcp::handlers::get_context::handle_context(
            db,
            json!({
                "session_id": session_id,
                "project": project_param,
                "budget": input.budget.unwrap_or(DEFAULT_HANDOFF_BUDGET),
                "format": "compact",
                "current_only": true
            }),
        )?
    };
    let closed = if session.is_open {
        crate::storage::sessions::end_session(db, &id)?
    } else {
        session
    };

    let reason = digest.get("reason").and_then(Value::as_str);
    let latency_ms = start_instant.elapsed().as_millis() as u64;
    let budget_used = digest
        .get("budget_used")
        .and_then(Value::as_u64)
        .unwrap_or(0) as usize;
    let rows = digest
        .get("results")
        .and_then(Value::as_array)
        .map(|a| a.len())
        .unwrap_or(0);

    crate::storage::record_query_log_best_effort(
        db,
        &crate::core::types::QueryLogEntry {
            id: format!("ql_{}", uuid::Uuid::new_v4().simple()),
            ts_ms: chrono::Utc::now().timestamp_millis(),
            tool: "knox_handoff".to_string(),
            project_id: project_id_str,
            rows,
            tokens: budget_used,
            latency_ms,
            reason: reason.map(String::from),
        },
    );

    let contradiction_count: usize = db
        .conn()
        .query_row(
            "SELECT COUNT(*) FROM edges WHERE edge_type = 'contradicts' AND from_action_id IN (SELECT id FROM actions WHERE session_id = ?1)",
            rusqlite::params![session_id],
            |row| row.get(0),
        )
        .unwrap_or(0);

    let mut response = json!({
        "status": "closed",
        "session_id": session_id,
        "ended_at": closed.ended_at.map(|time| time.to_rfc3339()),
        "contradictions": contradiction_count,
        "digest": digest
    });
    if let Some(r) = reason {
        response["reason"] = json!(r);
    }

    Ok(response)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn returns_digest_and_closes_session() {
        let db = KnoxDb::open_memory().unwrap();
        let session = crate::storage::sessions::start_session(&db, "gemini", Some("demo")).unwrap();
        let result = cmd_handoff(
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
