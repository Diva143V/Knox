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
/// 2. Records an ingestion session in the database.
/// 3. Inserts each action into the database and tracks affected files.
/// 4. Links consecutive actions within the session.
/// 5. Closes the session and returns an `IngestReport`.
pub fn cmd_ingest(transcript_path: &Path, db: &KnoxDb) -> KnoxResult<IngestReport> {
    let mut actions = ingest_file(transcript_path)?;
    if actions.is_empty() {
        return Ok(IngestReport::default());
    }

    let session = start_session(db, "ingest", None)?;
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
