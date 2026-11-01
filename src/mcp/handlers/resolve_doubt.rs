//! MCP tool handler for `knox_resolve_doubt`.

use serde::Deserialize;
use serde_json::{Value, json};

use crate::core::error::{KnoxError, KnoxResult};
use crate::core::ids::ActionId;
use crate::storage::db::KnoxDb;

/// Input parameters for `knox_resolve_doubt`.
#[derive(Debug, Deserialize)]
pub struct ResolveDoubtParams {
    /// ID of the doubt action being resolved.
    pub doubt_id: String,
    /// Explanation of how the doubt was resolved or answer discovered.
    pub resolution_note: String,
    /// Optional action ID that answered or resolved this doubt.
    pub resolved_by_action_id: Option<String>,
}

/// Handles the `knox_resolve_doubt` tool invocation.
pub fn handle_resolve_doubt(db: &KnoxDb, params: Value) -> KnoxResult<Value> {
    let input: ResolveDoubtParams = serde_json::from_value(params)
        .map_err(|e| KnoxError::InvalidInput(format!("Invalid resolve_doubt params: {e}")))?;

    if input.doubt_id.trim().is_empty() {
        return Err(KnoxError::InvalidInput(
            "doubt_id must not be empty".to_string(),
        ));
    }
    if input.resolution_note.trim().is_empty() {
        return Err(KnoxError::InvalidInput(
            "resolution_note must not be empty".to_string(),
        ));
    }

    let doubt_id = ActionId::from(input.doubt_id.trim());
    let resolver_id = input.resolved_by_action_id.as_deref().map(ActionId::from);

    crate::storage::nodes::resolve_doubt(
        db,
        &doubt_id,
        input.resolution_note.trim(),
        resolver_id.as_ref(),
    )?;

    Ok(json!({
        "status": "resolved",
        "doubt_id": doubt_id.to_string(),
        "resolution_note": input.resolution_note.trim(),
        "resolved_by_action_id": resolver_id.map(|id| id.to_string())
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mcp::handlers::record_doubt::handle_record_doubt;

    #[test]
    fn test_handle_resolve_doubt() {
        let db = KnoxDb::open_memory().unwrap();
        let session = crate::storage::sessions::start_session(&db, "gemini", None).unwrap();

        let doubt_params = json!({
            "session_id": session.id.as_str(),
            "actor": "gemini",
            "summary": "Should we use Mutex or RwLock for KnoxDb?"
        });
        let doubt_res = handle_record_doubt(&db, doubt_params).unwrap();
        let doubt_id = doubt_res.get("id").unwrap().as_str().unwrap();

        let resolve_params = json!({
            "doubt_id": doubt_id,
            "resolution_note": "Mutex is chosen because sqlite3 handles locking internally and writes are very fast."
        });

        let res = handle_resolve_doubt(&db, resolve_params).unwrap();
        assert_eq!(res.get("status").unwrap().as_str().unwrap(), "resolved");

        let action = crate::storage::nodes::get_action(&db, &ActionId::from(doubt_id))
            .unwrap()
            .expect("must exist");
        assert_eq!(action.is_resolved, Some(true));
        assert_eq!(
            action.resolution_note.as_deref(),
            Some(
                "Mutex is chosen because sqlite3 handles locking internally and writes are very fast."
            )
        );
    }
}
