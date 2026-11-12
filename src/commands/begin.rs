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

fn default_workspace() -> String {
    ".".to_string()
}

/// Input parameters for `knox_begin`.
#[derive(Debug, Deserialize)]
pub struct BeginParams {
    /// Project scope filter (defaults to current directory ".").
    #[serde(default = "default_workspace")]
    pub workspace: String,
    pub actor: String,
    pub task: Option<String>,
    pub budget: Option<usize>,
    /// Optional transcript path to auto-capture memory from since last offset.
    pub transcript_path: Option<String>,
}

/// Runs the session begin workflow for `knox_begin`.
///
/// Finds the most recent open session for the actor/workspace, or creates a
/// new one linked to the previous session for the same actor/project.
/// Returns `scope` (session metadata) and `context` (budgeted task context).
pub fn cmd_begin(db: &KnoxDb, params: Value) -> KnoxResult<Value> {
    let start_instant = std::time::Instant::now();
    let input: BeginParams = serde_json::from_value(params)
        .map_err(|e| KnoxError::InvalidInput(format!("Invalid begin params: {e}")))?;
    let actor = input.actor.trim();
    if actor.is_empty() {
        return Err(KnoxError::InvalidInput(
            "Actor must not be empty".to_string(),
        ));
    }

    let workspace_str = if input.workspace.trim().is_empty() {
        ".".to_string()
    } else {
        input.workspace.trim().to_string()
    };

    let existing =
        crate::storage::sessions::find_open_session(db, actor, Some(&workspace_str))?;
    let (session, resumed, parent_session) = match existing {
        Some(session) => (session, true, None),
        None => {
            let session =
                crate::storage::sessions::start_session(db, actor, Some(&workspace_str))?;
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

    // Auto-capture: ingest transcript lines since the last offset on begin,
    // when a path is configured via params, environment, or KnoxConfig.
    let configured_transcript = input
        .transcript_path
        .as_deref()
        .map(std::path::PathBuf::from)
        .or_else(|| {
            std::env::var("KNOX_TRANSCRIPT_PATH")
                .ok()
                .map(|p| crate::core::config::expand_tilde(p.trim()))
        })
        .or_else(|| {
            let cfg = crate::core::config::KnoxConfig::from_env();
            cfg.transcript_path
        })
        .map(|p| {
            if p.is_relative() {
                let ws_path = std::path::Path::new(&workspace_str);
                let combined = ws_path.join(&p);
                if combined.exists() {
                    combined
                } else {
                    p
                }
            } else {
                p
            }
        });

    let mut actions_captured = 0;
    if let Some(ref path) = configured_transcript {
        if path.exists() {
            if let Ok(report) = crate::commands::ingest::cmd_ingest_incremental(
                path,
                db,
                Some(&session.id),
                Some(&input.workspace),
            ) {
                actions_captured = report.actions_imported;
            }
        }
    } else {
        // Discovery fallback: if no transcript is configured, check for exactly one local candidate
        let root = std::path::Path::new(&input.workspace);
        let root_dir = if root.is_dir() {
            Some(root.to_path_buf())
        } else if let Ok(Some(proj)) = crate::storage::nodes::get_project(
            db,
            &crate::core::ids::ProjectId::from(input.workspace.as_str()),
        ) {
            Some(std::path::PathBuf::from(proj.path))
        } else {
            std::env::current_dir().ok()
        };

        if let Some(r) = root_dir.as_deref() {
            let canonical_root = r.canonicalize().unwrap_or_else(|_| r.to_path_buf());
            let candidates: Vec<_> = crate::ingest::detect_local_transcripts(Some(r))
                .into_iter()
                .filter(|c| c.source != "Git History" && std::path::Path::new(&c.path).is_file())
                .collect();

            let project_candidates: Vec<_> = candidates
                .iter()
                .filter(|c| {
                    let p = std::path::Path::new(&c.path);
                    let cp = p.canonicalize().unwrap_or_else(|_| p.to_path_buf());
                    cp.starts_with(&canonical_root)
                })
                .cloned()
                .collect();

            let active_candidates = if !project_candidates.is_empty() {
                project_candidates
            } else {
                candidates
            };

            if active_candidates.len() == 1 {
                let disc_path = std::path::Path::new(&active_candidates[0].path);
                if let Ok(report) = crate::commands::ingest::cmd_ingest_incremental(
                    disc_path,
                    db,
                    Some(&session.id),
                    Some(&input.workspace),
                ) {
                    actions_captured = report.actions_imported;
                }
            }
        }
    }

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
    let reason = context.get("reason").and_then(Value::as_str);

    let latency_ms = start_instant.elapsed().as_millis() as u64;
    let budget_used = context
        .get("budget_used")
        .and_then(Value::as_u64)
        .unwrap_or(0) as usize;
    let rows = context
        .get("results")
        .and_then(Value::as_array)
        .map(|a| a.len())
        .unwrap_or(0);

    crate::storage::record_query_log_best_effort(
        db,
        &crate::core::types::QueryLogEntry {
            id: format!("ql_{}", uuid::Uuid::new_v4().simple()),
            ts_ms: chrono::Utc::now().timestamp_millis(),
            tool: "knox_begin".to_string(),
            project_id: Some(input.workspace.clone()),
            rows,
            tokens: budget_used,
            latency_ms,
            reason: reason.map(String::from),
        },
    );

    let mut response = json!({
        "scope": {
            "session_id": session.id.to_string(),
            "project": session.project,
            "actor": actor,
            "resumed": resumed,
            "parent_session_id": parent_session.map(|id| id.to_string()),
            "actions_captured": actions_captured
        },
        "context": context
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
