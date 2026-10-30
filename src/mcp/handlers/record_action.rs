//! MCP tool handler for `knox_record_action`.

use chrono::Utc;
use serde::Deserialize;
use serde_json::{Value, json};

use crate::core::error::{KnoxError, KnoxResult};
use crate::core::ids::{ActionId, SessionId};
use crate::core::types::KnoxAction;
use crate::ingest::summary::normalize_file_path;
use crate::storage::db::KnoxDb;

/// Input parameters for `knox_record_action`.
#[derive(Debug, Deserialize)]
pub struct RecordActionParams {
    /// Active session identifier.
    pub session_id: String,
    /// Software or agent name performing the action.
    pub actor: String,
    /// Semantic action type (write, debug, read, review, plan, change).
    pub action_type: String,
    /// Human-readable summary of what was performed.
    pub summary: String,
    /// Optional relative file paths modified or referenced.
    #[serde(default)]
    pub files: Vec<String>,
    /// Optional causal parent action ID.
    #[serde(default)]
    pub parent_action_id: Option<String>,
    /// Optional tags for indexing.
    #[serde(default)]
    pub tags: Vec<String>,
    /// Optional confidence score hint (0.0 to 1.0).
    #[serde(default)]
    pub confidence: Option<f64>,
    /// Optional prior action ID that this action supersedes.
    #[serde(default)]
    pub supersedes: Option<String>,
}

/// Handles the `knox_record_action` tool invocation.
pub fn handle_record_action(db: &KnoxDb, params: Value) -> KnoxResult<Value> {
    let input: RecordActionParams = serde_json::from_value(params)
        .map_err(|e| KnoxError::InvalidInput(format!("Invalid record_action params: {e}")))?;

    let session_id = validate_action_params(&input, db)?;

    let norm_files = input.files.iter().map(|f| normalize_file_path(f)).collect();
    let action_id = ActionId::new();
    let now = Utc::now();

    let action = KnoxAction {
        id: action_id.clone(),
        actor: input.actor.trim().to_string(),
        action_type: input.action_type.trim().to_string(),
        summary: input.summary.trim().to_string(),
        files: norm_files,
        tags: input.tags,
        confidence: input.confidence,
        is_current: true,
        timestamp: now,
        session_id: Some(session_id),
        parent_action_id: input.parent_action_id.map(ActionId::from),
        supersedes: input.supersedes.map(ActionId::from),
    };

    crate::storage::nodes::insert_action(db, &action)?;

    Ok(json!({
        "id": action_id.to_string(),
        "status": "recorded",
        "timestamp": now.to_rfc3339()
    }))
}

/// Validates required parameters, session state, and confidence range.
fn validate_action_params(input: &RecordActionParams, db: &KnoxDb) -> KnoxResult<SessionId> {
    if input.session_id.trim().is_empty() {
        return Err(KnoxError::InvalidInput(
            "session_id must not be empty".to_string(),
        ));
    }
    if input.actor.trim().is_empty() {
        return Err(KnoxError::InvalidInput(
            "actor must not be empty".to_string(),
        ));
    }
    if input.action_type.trim().is_empty() {
        return Err(KnoxError::InvalidInput(
            "action_type must not be empty".to_string(),
        ));
    }
    if input.summary.trim().is_empty() {
        return Err(KnoxError::InvalidInput(
            "summary must not be empty".to_string(),
        ));
    }

    if let Some(c) = input.confidence {
        if !(0.0..=1.0).contains(&c) {
            return Err(KnoxError::InvalidInput(
                "confidence must be between 0.0 and 1.0".to_string(),
            ));
        }
    }

    let session_id = SessionId::from(input.session_id.trim());
    let session = crate::storage::sessions::get_session(db, &session_id)?
        .ok_or_else(|| KnoxError::SessionNotFound(session_id.to_string()))?;

    if !session.is_open {
        return Err(KnoxError::SessionClosed(session_id.to_string()));
    }

    Ok(session_id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_handle_record_action_success() {
        let db = KnoxDb::open_memory().unwrap();
        let session = crate::storage::sessions::start_session(&db, "agent-1", None).unwrap();

        let params = json!({
            "session_id": session.id.as_str(),
            "actor": "agent-1",
            "action_type": "write",
            "summary": "Implemented parser",
            "files": ["src/parser.rs"],
            "tags": ["parser", "refactor"],
            "confidence": 0.95
        });

        let res = handle_record_action(&db, params).unwrap();
        assert_eq!(res.get("status").unwrap(), "recorded");
        assert!(res.get("id").is_some());
    }

    #[test]
    fn test_handle_record_action_closed_session() {
        let db = KnoxDb::open_memory().unwrap();
        let session = crate::storage::sessions::start_session(&db, "agent-1", None).unwrap();
        crate::storage::sessions::end_session(&db, &session.id).unwrap();

        let params = json!({
            "session_id": session.id.as_str(),
            "actor": "agent-1",
            "action_type": "write",
            "summary": "Should fail"
        });

        let err = handle_record_action(&db, params).unwrap_err();
        assert!(matches!(err, KnoxError::SessionClosed(_)));
    }
}
