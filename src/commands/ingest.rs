//! Transcript ingestion command for Knox.
//! Parses conversational transcripts and persists them into the knowledge graph.

use std::collections::HashSet;
use std::path::Path;

use crate::core::error::KnoxResult;
use crate::core::types::IngestReport;
use crate::ingest::ingest_file;
use crate::storage::db::KnoxDb;
use crate::storage::edges::link_follows;
use crate::storage::nodes::insert_action;
use crate::storage::sessions::{end_session, start_session};

/// Imports a conversation transcript into the Knox knowledge graph.
///
/// 1. Reads and maps the transcript into `KnoxAction` records via `ingest_file`.
/// 2. Records an ingestion session in the database (associating with inferred project).
/// 3. Inserts each action into the database and tracks affected files.
/// 4. Links consecutive actions within the session.
/// 5. Closes the session and returns an `IngestReport`.
pub fn cmd_ingest(transcript_path: &Path, db: &KnoxDb) -> KnoxResult<IngestReport> {
    let mut actions = ingest_file(transcript_path)?;
    if actions.is_empty() {
        return Ok(IngestReport::default());
    }

    let existing_projects = crate::storage::list_projects(db).unwrap_or_default();
    let inferred_project = infer_project_path(transcript_path, &actions, &existing_projects);
    let session = start_session(db, "ingest", inferred_project.as_deref())?;
    let mut files_touched = HashSet::new();
    let mut previous_action_id = None;

    for action in &mut actions {
        if action.session_id.is_none() {
            action.session_id = Some(session.id.clone());
        }

        for file in &action.files {
            files_touched.insert(file.clone());
        }

        let current_id = insert_action(db, action)?;

        if let Some(ref prev_id) = previous_action_id {
            let _ = link_follows(db, prev_id, &current_id);
        }
        previous_action_id = Some(current_id);
    }

    let _ = end_session(db, &session.id);

    Ok(IngestReport {
        actions_imported: actions.len(),
        files_touched: files_touched.len(),
        sessions_created: 1,
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
    format!(
        "Ingested {} actions across {} touched files ({} session created)",
        report.actions_imported, report.files_touched, report.sessions_created
    )
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

        let output = format_ingest_report(&report);
        assert!(output.contains("6 actions"));
    }
}
