//! Transcript ingestion command for Knox.
//! Parses conversational transcripts and persists them into the knowledge graph.

use std::collections::HashSet;
use std::path::Path;

use rusqlite::params;

use crate::core::error::{KnoxError, KnoxResult};
use crate::core::types::{IngestPreview, IngestReport, KnoxAction};
use crate::ingest::{ingest_bytes_with_tool, ingest_file_with_tool};
use crate::storage::db::KnoxDb;
use crate::storage::edges::link_follows;
use crate::storage::nodes::insert_action;
use crate::storage::sessions::{end_session, start_session};

/// Imports a conversation transcript into the Knox knowledge graph.
///
/// 1. Reads and maps the transcript into `KnoxAction` records via `ingest_file_with_tool`.
/// 2. Records an ingestion session in the database (associating with inferred project).
/// 3. Checks for duplicates and inserts each unique action into the database.
/// 4. Links consecutive actions within the session.
/// 5. Closes the session and returns an `IngestReport`.
pub fn cmd_ingest(transcript_path: &Path, db: &KnoxDb) -> KnoxResult<IngestReport> {
    let (tool_detected, actions) = ingest_file_with_tool(transcript_path)?;
    if actions.is_empty() {
        return Ok(IngestReport {
            tool_detected,
            ..Default::default()
        });
    }

    let existing_projects = crate::storage::list_projects(db).unwrap_or_default();
    let inferred_project = infer_project_path(transcript_path, &actions, &existing_projects);

    persist_actions_with_session(actions, tool_detected, inferred_project.as_deref(), db)
}

/// Imports raw transcript content directly into the Knox knowledge graph.
pub fn cmd_ingest_raw(
    content: &str,
    db: &KnoxDb,
    actor_override: Option<&str>,
) -> KnoxResult<IngestReport> {
    let (mut tool_detected, mut actions) = ingest_bytes_with_tool(content.as_bytes())?;
    if actions.is_empty() {
        return Ok(IngestReport {
            tool_detected,
            ..Default::default()
        });
    }

    if let Some(actor) = actor_override {
        let trimmed = actor.trim();
        if !trimmed.is_empty() && !trimmed.eq_ignore_ascii_case("auto") {
            tool_detected = trimmed.to_string();
            for a in &mut actions {
                a.actor = trimmed.to_string();
            }
        }
    }

    persist_actions_with_session(actions, tool_detected, None, db)
}

/// Ingests commit history from a Git repository into the Knox knowledge graph.
pub fn cmd_ingest_git(
    project_path: &Path,
    max_commits: usize,
    db: &KnoxDb,
) -> KnoxResult<IngestReport> {
    let actions = crate::ingest::git_log::run_git_log(project_path, max_commits)?;
    if actions.is_empty() {
        return Ok(IngestReport {
            tool_detected: "git".to_string(),
            ..Default::default()
        });
    }

    let project_str = project_path.to_string_lossy().to_string();
    persist_actions_with_session(actions, "git".to_string(), Some(&project_str), db)
}

/// Generates a dry-run preview of a transcript before committing to the database.
pub fn cmd_preview(
    path: Option<&Path>,
    content: Option<&str>,
    db: &KnoxDb,
) -> KnoxResult<IngestPreview> {
    let (tool_detected, actions) = if let Some(p) = path {
        ingest_file_with_tool(p)?
    } else if let Some(c) = content {
        ingest_bytes_with_tool(c.as_bytes())?
    } else {
        return Err(KnoxError::ParseError(
            "Neither path nor content provided for preview".to_string(),
        ));
    };

    let conn = db.conn();
    let mut plans_count = 0;
    let mut writes_count = 0;
    let mut debugs_count = 0;
    let mut decisions_count = 0;
    let mut files_set = HashSet::new();
    let mut duplicates_count = 0;

    for a in &actions {
        match a.action_type.as_str() {
            "plan" => plans_count += 1,
            "write" => writes_count += 1,
            "debug" => debugs_count += 1,
            "decide" => decisions_count += 1,
            _ => {}
        }

        for f in &a.files {
            files_set.insert(f.clone());
        }

        let is_dup = conn
            .query_row(
                "SELECT 1 FROM actions WHERE actor = ?1 AND action_type = ?2 AND summary = ?3 LIMIT 1",
                params![a.actor.as_str(), a.action_type.as_str(), a.summary.as_str()],
                |_| Ok(true),
            )
            .unwrap_or(false);

        if is_dup {
            duplicates_count += 1;
        }
    }

    let sample_actions = actions.iter().take(6).cloned().collect();
    let mut files_touched: Vec<String> = files_set.into_iter().collect();
    files_touched.sort();

    Ok(IngestPreview {
        tool_detected,
        total_actions: actions.len(),
        plans_count,
        writes_count,
        debugs_count,
        decisions_count,
        files_touched,
        duplicates_count,
        sample_actions,
    })
}

/// Common helper to persist actions, link follows edges, and record session metrics.
fn persist_actions_with_session(
    mut actions: Vec<KnoxAction>,
    tool_detected: String,
    project_scope: Option<&str>,
    db: &KnoxDb,
) -> KnoxResult<IngestReport> {
    let session = start_session(db, &tool_detected, project_scope)?;
    let mut files_touched = HashSet::new();
    let mut previous_action_id = None;
    let mut actions_imported = 0;
    let mut duplicates_skipped = 0;

    let conn = db.conn();

    for action in &mut actions {
        // Smart deduplication check
        let is_duplicate = conn
            .query_row(
                "SELECT 1 FROM actions WHERE actor = ?1 AND action_type = ?2 AND summary = ?3 LIMIT 1",
                params![action.actor.as_str(), action.action_type.as_str(), action.summary.as_str()],
                |_| Ok(true),
            )
            .unwrap_or(false);

        if is_duplicate {
            duplicates_skipped += 1;
            continue;
        }

        if action.session_id.is_none() {
            action.session_id = Some(session.id.clone());
        }

        for file in &action.files {
            files_touched.insert(file.clone());
        }

        let current_id = insert_action(db, action)?;
        actions_imported += 1;

        if let Some(ref prev_id) = previous_action_id {
            let _ = link_follows(db, prev_id, &current_id);
        }
        previous_action_id = Some(current_id);
    }

    let _ = end_session(db, &session.id);

    Ok(IngestReport {
        actions_imported,
        files_touched: files_touched.len(),
        sessions_created: if actions_imported > 0 { 1 } else { 0 },
        duplicates_skipped,
        tool_detected,
    })
}

/// Attempts to infer the project root directory from transcript path or action file paths,
/// matching against existing tracked projects if possible.
fn infer_project_path(
    transcript_path: &Path,
    actions: &[crate::core::types::KnoxAction],
    existing_projects: &[crate::core::types::Project],
) -> Option<String> {
    let trans_str = transcript_path.to_string_lossy().to_string();

    // 1. Check if transcript path or any action files explicitly match an existing project
    for proj in existing_projects {
        let norm_proj_path = proj.path.replace('\\', "/").to_lowercase();
        let norm_trans = trans_str.replace('\\', "/").to_lowercase();
        if !norm_proj_path.is_empty() && norm_trans.contains(&norm_proj_path) {
            return Some(proj.path.clone());
        }
        for action in actions {
            for file in &action.files {
                let norm_f = file.replace('\\', "/").to_lowercase();
                if !norm_proj_path.is_empty() && norm_f.contains(&norm_proj_path) {
                    return Some(proj.path.clone());
                }
            }
        }
    }

    // 2. If exactly one project already exists, attach to it by default
    if existing_projects.len() == 1 {
        return Some(existing_projects[0].path.clone());
    }

    // 3. Try to find common absolute path from action files
    for action in actions {
        for file in &action.files {
            let p = Path::new(file);
            if p.is_absolute() {
                if let Some(parent) = p.parent() {
                    let parent_str = parent.to_string_lossy().to_string();
                    let clean = parent_str
                        .strip_prefix(r"\\?\")
                        .unwrap_or(&parent_str)
                        .to_string();
                    if !clean.is_empty() {
                        return Some(clean);
                    }
                }
            }
        }
    }

    // 4. Derive from transcript_path parent, skipping internal subdirectories
    let mut curr = transcript_path.parent();
    while let Some(dir) = curr {
        let name = dir.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if !name.is_empty()
            && !name.starts_with('.')
            && name != "logs"
            && name != "fixtures"
            && name != "brain"
            && name != "target"
        {
            let path_str = dir.to_string_lossy().to_string();
            let clean = path_str
                .strip_prefix(r"\\?\")
                .unwrap_or(&path_str)
                .to_string();
            if !clean.is_empty() {
                return Some(clean);
            }
        }
        curr = dir.parent();
    }

    // Fallback: direct parent of transcript_path
    transcript_path.parent().map(|p| {
        let s = p.to_string_lossy().to_string();
        s.strip_prefix(r"\\?\").unwrap_or(&s).to_string()
    })
}

/// Formats an `IngestReport` for friendly CLI output.
pub fn format_ingest_report(report: &IngestReport) -> String {
    let mut msg = format!(
        "Ingested {} actions across {} touched files ({} session created, format: {})",
        report.actions_imported,
        report.files_touched,
        report.sessions_created,
        report.tool_detected
    );
    if report.duplicates_skipped > 0 {
        msg.push_str(&format!(
            ", skipped {} duplicate(s)",
            report.duplicates_skipped
        ));
    }
    msg
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_cmd_ingest_with_antigravity_fixture() {
        let db = KnoxDb::open_memory().unwrap();
        let fixture_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("fixtures")
            .join("transcript_antigravity.jsonl");

        let report = cmd_ingest(&fixture_path, &db).unwrap();
        assert_eq!(report.actions_imported, 6);
        assert_eq!(report.files_touched, 2);
        assert_eq!(report.sessions_created, 1);
        assert_eq!(report.tool_detected, "antigravity");

        // Re-ingest the same fixture -> duplicates should be skipped!
        let second_report = cmd_ingest(&fixture_path, &db).unwrap();
        assert_eq!(second_report.actions_imported, 0);
        assert_eq!(second_report.duplicates_skipped, 6);
    }

    #[test]
    fn test_cmd_preview_and_ingest_raw() {
        let db = KnoxDb::open_memory().unwrap();
        let raw_md = "## User\nBuild universal ingest\n\n## Assistant\nCreated universal mappers in src/ingest/";

        let preview = cmd_preview(None, Some(raw_md), &db).unwrap();
        assert_eq!(preview.total_actions, 2);
        assert_eq!(preview.duplicates_count, 0);

        let report = cmd_ingest_raw(raw_md, &db, Some("custom-agent")).unwrap();
        assert_eq!(report.actions_imported, 2);
        assert_eq!(report.tool_detected, "custom-agent");
    }
}
