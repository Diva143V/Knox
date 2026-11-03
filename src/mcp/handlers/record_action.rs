//! MCP tool handler for `knox_record_action`.

use chrono::Utc;
use serde::Deserialize;
use serde_json::{Value, json};

use crate::core::error::{KnoxError, KnoxResult};
use crate::core::ids::{ActionId, SessionId};
use crate::core::types::{
    ActionDelta, ActionDeltaInput, ActionRelation, DeltaChangeType, DeltaSummary, KnoxAction,
    QueryFilter, RelationType,
};
use crate::ingest::summary::normalize_file_path_with_root;
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
    /// Optional significance level ("major" vs "minor").
    #[serde(default)]
    pub significance: Option<String>,
    /// Optional resolution status (false for open doubts, true for resolved/standard).
    #[serde(default)]
    pub is_resolved: Option<bool>,
    /// Optional resolution note.
    #[serde(default)]
    pub resolution_note: Option<String>,
    /// Optional Git commit SHA if this action is committed or associated with a commit.
    #[serde(default)]
    pub commit_sha: Option<String>,
    /// Explicit code deltas or hunks to associate with this action.
    #[serde(default)]
    pub deltas: Option<Vec<ActionDeltaInput>>,
    /// Automatically capture working tree git diff for touched files (defaults to true).
    #[serde(default)]
    pub auto_git_diff: Option<bool>,
    /// Architectural or technical rationale for deleted code in this action.
    #[serde(default)]
    pub deletion_reason: Option<String>,
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

    // Resolve the caller's project once, so reuse compares identity rather than
    // a display string. A bare name only resolves when it is unambiguous.
    let project_id = match project {
        Some(p) => crate::storage::sessions::resolve_project_identifier(db, p)?,
        None => None,
    };

    // Try finding the most recent open session for this actor *and* project.
    let recent = crate::storage::sessions::list_recent_sessions(db, 20)?;
    for s in recent {
        if s.is_open && s.actor == actor {
            let same_project = match &project_id {
                Some(pid) => s.project_id.as_ref().map(|value| value.as_str()) == Some(pid.as_str()),
                None => s.project_id.is_none(),
            };
            if same_project {
                return Ok((s.id, true)); // implicit reused session
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

    let session = crate::storage::sessions::get_session(db, &session_id)?
        .ok_or_else(|| KnoxError::SessionNotFound(session_id.to_string()))?;
    let project_id = session.project_id.clone();
    let project_root = project_id
        .as_ref()
        .and_then(|pid| crate::storage::nodes::get_project(db, pid).ok().flatten())
        .map(|p| p.path);

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

    let norm_files: Vec<String> = input
        .files
        .iter()
        .map(|f| normalize_file_path_with_root(f, project_root.as_deref()))
        .collect();
    let action_id = ActionId::new();
    let now = Utc::now();
    let normalized_action_type = normalize_action_type(&input.action_type);

    let is_doubt = normalized_action_type == "doubt" || normalized_action_type == "question";
    let is_resolved = input.is_resolved.or(Some(!is_doubt));
    let significance = input
        .significance
        .or_else(|| Some(default_significance(&normalized_action_type).to_string()));

    // Collect code deltas (explicit deltas + automatic git diff capture)
    let mut collected_deltas: Vec<ActionDelta> = Vec::new();

    // 1. Explicit deltas from input
    if let Some(ref input_deltas) = input.deltas {
        for d in input_deltas {
            let change_type = d
                .change_type
                .parse::<DeltaChangeType>()
                .unwrap_or(DeltaChangeType::Modified);
            collected_deltas.push(ActionDelta {
                id: uuid::Uuid::new_v4().to_string(),
                action_id: action_id.clone(),
                file_path: normalize_file_path_with_root(&d.file_path, project_root.as_deref()),
                change_type,
                old_path: d
                    .old_path
                    .clone()
                    .map(|p| normalize_file_path_with_root(&p, project_root.as_deref())),
                lines_added: d.lines_added.unwrap_or(0),
                lines_deleted: d.lines_deleted.unwrap_or(0),
                diff_hunk: d.diff_hunk.clone(),
                deleted_code: d.deleted_code.clone(),
                deletion_reason: d.deletion_reason.clone().or_else(|| input.deletion_reason.clone()),
                created_at: now,
            });
        }
    }

    // 2. Auto git diff capture if enabled
    if input.auto_git_diff.unwrap_or(true) {
        let repo_root = if let Some(root) = project_root.as_ref() {
            std::path::PathBuf::from(root)
        } else {
            // Never fall back to an arbitrary project: capture from the current
            // working directory instead of guessing which repository was meant.
            std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."))
        };

        if let Ok(git_deltas) =
            crate::git::diff::capture_working_tree_diff(&action_id, &repo_root, &norm_files)
        {
            for gd in git_deltas {
                if !collected_deltas.iter().any(|d| d.file_path == gd.file_path) {
                    collected_deltas.push(gd);
                }
            }
        }
    }

    // 3. Sanitize and bound all collected deltas
    let mut files_set = std::collections::HashSet::new();
    let mut total_added = 0u32;
    let mut total_deleted = 0u32;

    for delta in &mut collected_deltas {
        if delta.deletion_reason.is_none() && input.deletion_reason.is_some() {
            delta.deletion_reason = input.deletion_reason.clone();
        }
        crate::core::guardrails::sanitize_delta(delta);
        files_set.insert(delta.file_path.clone());
        total_added += delta.lines_added;
        total_deleted += delta.lines_deleted;
    }

    let delta_summary = if !collected_deltas.is_empty() {
        Some(DeltaSummary {
            total_files: files_set.len(),
            lines_added: total_added,
            lines_deleted: total_deleted,
        })
    } else {
        None
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
        project_id,
        parent_action_id,
        supersedes: input.supersedes.map(ActionId::from),
        relations,
        symbols: input.symbols,
        rationale: input.rationale,
        rejected_alternatives: input.rejected_alternatives,
        context_summary: input.context_summary,
        significance,
        is_resolved,
        resolution_note: input.resolution_note,
        commit_sha: input.commit_sha.clone(),
        delta_summary: delta_summary.clone(),
    };

    crate::storage::nodes::insert_action_with_deltas(db, &action, &collected_deltas)?;

    Ok(json!({
        "id": action_id.to_string(),
        "action_id": action_id.to_string(),
        "status": "recorded",
        "session_id": session_id.to_string(),
        "session_implicit": is_implicit,
        "timestamp": now.to_rfc3339(),
        "commit_sha": input.commit_sha,
        "delta_summary": delta_summary,
    }))
}

/// Canonicalises the action-type vocabulary used by storage and guardrails.
pub fn normalize_action_type(raw: &str) -> String {
    match raw.trim().to_lowercase().as_str() {
        "decision" => "decide".to_string(),
        "question" => "doubt".to_string(),
        other => other.to_string(),
    }
}

/// Picks a default significance instead of labelling every action "major".
pub fn default_significance(action_type: &str) -> &'static str {
    match action_type {
        "decide" | "plan" | "doubt" => "major",
        _ => "minor",
    }
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
