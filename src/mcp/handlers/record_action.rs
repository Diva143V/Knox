//! MCP tool handler for `knox_record_action`.

use chrono::Utc;
use serde::Deserialize;
use serde_json::{Value, json};

use crate::core::error::{KnoxError, KnoxResult};
use crate::core::ids::{ActionId, SessionId};
use crate::core::types::{ActionRelation, KnoxAction, QueryFilter, RelationType};
use crate::ingest::summary::normalize_file_path;
use crate::storage::db::KnoxDb;

/// Input for a semantic relation to another action.
#[derive(Debug, Deserialize, Clone)]
pub struct ActionRelationInput {
    /// Referenced target action ID.
    pub target_id: String,
    /// Semantic nature (depends_on, implements, verifies, reverts, relates_to).
    pub relation_type: Option<String>,
}

/// Input parameters for `knox_record_action`.
#[derive(Debug, Deserialize)]
pub struct RecordActionParams {
    /// Active session identifier (optional; auto-resolved if omitted).
    pub session_id: Option<String>,
    /// Software or agent name performing the action.
    pub actor: String,
    /// Semantic action type (write, debug, read, review, plan, change, decide).
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
    /// Optional explicit semantic relationships.
    #[serde(default)]
    pub relations: Vec<ActionRelationInput>,
    /// Optional code symbols touched (e.g. structs, functions).
    #[serde(default)]
    pub symbols: Vec<String>,
    /// Optional architectural rationale or technical justification.
    #[serde(default)]
    pub rationale: Option<String>,
    /// Optional list of rejected alternatives.
    #[serde(default)]
    pub rejected_alternatives: Vec<String>,
    /// Optional high-level context summary.
    #[serde(default)]
    pub context_summary: Option<String>,
    /// Optional project scope identifier.
    #[serde(default)]
    pub project: Option<String>,
    /// Convenience helper: automatically link to immediate predecessor in session.
    #[serde(default)]
    pub link_to_latest: Option<bool>,
    /// Convenience helper: automatically link `implements` to latest decision in session.
    #[serde(default)]
    pub implements_latest_decision: Option<bool>,
}

/// Resolves an existing open session for this actor or spawns a new one transparently.
pub fn resolve_session(
    db: &KnoxDb,
    actor: &str,
    project: Option<&str>,
    provided_id: Option<&str>,
) -> KnoxResult<(SessionId, bool)> {
    if let Some(sid_str) = provided_id {
        let trimmed = sid_str.trim();
        if !trimmed.is_empty() {
            let sid = SessionId::from(trimmed);
            let session = crate::storage::sessions::get_session(db, &sid)?
                .ok_or_else(|| KnoxError::SessionNotFound(sid.to_string()))?;
            if !session.is_open {
                return Err(KnoxError::SessionClosed(sid.to_string()));
            }
            return Ok((sid, false)); // explicit session
        }
    }

    // Try finding the most recent open session for this actor
    let recent = crate::storage::sessions::list_recent_sessions(db, 20)?;
    for s in recent {
        if s.is_open && s.actor == actor {
            if let Some(p) = project {
                if s.project.as_deref() == Some(p) {
                    return Ok((s.id, true)); // implicit reused session
                }
            } else {
                return Ok((s.id, true));
            }
        }
    }

    // Transparently spawn an active session
    let new_session = crate::storage::sessions::start_session(db, actor, project)?;
    Ok((new_session.id, true)) // implicit newly created session
}

/// Handles the `knox_record_action` tool invocation.
pub fn handle_record_action(db: &KnoxDb, params: Value) -> KnoxResult<Value> {
    let input: RecordActionParams = serde_json::from_value(params)
        .map_err(|e| KnoxError::InvalidInput(format!("Invalid record_action params: {e}")))?;

    validate_basic_params(&input)?;
    let (session_id, is_implicit) = resolve_session(
        db,
        input.actor.trim(),
        input.project.as_deref(),
        input.session_id.as_deref(),
    )?;

    let mut parent_action_id = input.parent_action_id.clone().map(ActionId::from);

    // Convenience shortcut: link to immediate predecessor in this session
    if input.link_to_latest.unwrap_or(false) && parent_action_id.is_none() {
        let filter = QueryFilter {
            session_id: Some(session_id.clone()),
            current_only: false,
            limit: 1,
            ..Default::default()
        };
        let res = crate::storage::queries::query_actions(db, &filter)?;
        if let Some(prev) = res.results.first() {
            parent_action_id = Some(prev.id.clone());
        }
    }

    // Parse relations
    let mut relations: Vec<ActionRelation> = input
        .relations
        .iter()
        .map(|r| {
            let rel_type = r
                .relation_type
                .as_deref()
                .unwrap_or("relates_to")
                .parse::<RelationType>()
                .unwrap_or(RelationType::RelatesTo);
            ActionRelation {
                target_id: ActionId::from(r.target_id.trim()),
                relation_type: rel_type,
            }
        })
        .collect();

    // Convenience shortcut: implements_latest_decision
    if input.implements_latest_decision.unwrap_or(false) {
        let filter = QueryFilter {
            action_type: Some("decide".to_string()),
            current_only: true,
            limit: 1,
            ..Default::default()
        };
        let res = crate::storage::queries::query_actions(db, &filter)?;
        if let Some(dec) = res.results.first() {
            if !relations.iter().any(|r| r.target_id == dec.id) {
                relations.push(ActionRelation {
                    target_id: dec.id.clone(),
                    relation_type: RelationType::Implements,
                });
            }
        }
    }

    let norm_files = input.files.iter().map(|f| normalize_file_path(f)).collect();
    let action_id = ActionId::new();
    let now = Utc::now();
    let normalized_action_type = match input.action_type.trim().to_lowercase().as_str() {
        "decision" => "decide".to_string(),
        other => other.to_string(),
    };

    let action = KnoxAction {
        id: action_id.clone(),
        actor: input.actor.trim().to_string(),
        action_type: normalized_action_type,
        summary: input.summary.trim().to_string(),
        files: norm_files,
        tags: input.tags,
        confidence: input.confidence,
        is_current: true,
        timestamp: now,
        session_id: Some(session_id.clone()),
        parent_action_id,
        supersedes: input.supersedes.map(ActionId::from),
        relations,
        symbols: input.symbols,
        rationale: input.rationale,
        rejected_alternatives: input.rejected_alternatives,
        context_summary: input.context_summary,
    };

    crate::storage::nodes::insert_action(db, &action)?;

    Ok(json!({
        "id": action_id.to_string(),
        "action_id": action_id.to_string(),
        "status": "recorded",
        "session_id": session_id.to_string(),
        "session_implicit": is_implicit,
        "timestamp": now.to_rfc3339()
    }))
}

/// Validates required basic parameters and confidence range.
fn validate_basic_params(input: &RecordActionParams) -> KnoxResult<()> {
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

    Ok(())
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
    fn test_handle_record_action_implicit_session_creation() {
        let db = KnoxDb::open_memory().unwrap();

        // Calling record_action without a session_id should transparently create one
        let params = json!({
            "actor": "gemini",
            "action_type": "write",
            "summary": "Auto-session write"
        });

        let res = handle_record_action(&db, params).unwrap();
        assert_eq!(res.get("status").unwrap(), "recorded");
        assert_eq!(res.get("session_implicit").unwrap(), true);
        assert!(res.get("session_id").is_some());
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
