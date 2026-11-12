//! Incremental transcript update command for Knox.
//!
//! Business logic for `knox_update`: resolves transcript files (explicit or auto-discovered),
//! enforces file size caps and project scoping, invokes `cmd_ingest_incremental`,
//! and returns structured `UpdateReport`.

use std::path::{Path, PathBuf};
use crate::core::error::{KnoxError, KnoxResult};
use crate::core::types::UpdateReport;
use crate::ingest::MAX_INGEST_FILE_BYTES;
use crate::storage::db::KnoxDb;

/// Runs the transcript update workflow for `knox_update` and CLI `knox update`.
pub fn cmd_update(
    db: &KnoxDb,
    transcript_path: Option<&Path>,
    project_scope: Option<&str>,
    discover: bool,
) -> KnoxResult<UpdateReport> {
    // 1. Resolve project scope: explicit param -> workspace root "." -> fail closed with NoScope
    let project_id_str = if let Some(p) = project_scope {
        crate::storage::sessions::resolve_project_identifier(db, p)?
            .or_else(|| Some(p.to_string()))
    } else {
        crate::storage::sessions::resolve_project_identifier(db, ".")?
    };

    let Some(project_id) = project_id_str else {
        return Err(KnoxError::NoScope("No project scope resolved. Run within a project directory or pass --project.".to_string()));
    };

    // 2. Resolve transcript path
    let target_path: Option<PathBuf> = if let Some(path) = transcript_path {
        if !path.exists() {
            return Ok(UpdateReport {
                source: path.display().to_string(),
                markdown: format!("Transcript `{}` does not exist (0 actions imported).", path.display()),
                ..Default::default()
            });
        }
        if !path.is_file() {
            return Err(KnoxError::InvalidInput(format!(
                "Transcript path `{}` is not a regular file.",
                path.display()
            )));
        }
        if let Ok(meta) = std::fs::metadata(path) {
            if meta.len() > MAX_INGEST_FILE_BYTES {
                return Err(KnoxError::InvalidInput(format!(
                    "Transcript file exceeds maximum allowed size ({} MB limit): {}",
                    MAX_INGEST_FILE_BYTES / (1024 * 1024),
                    path.display()
                )));
            }
        }
        Some(path.to_path_buf())
    } else if discover {
        // Resolve project root directory for discovery
        let root = if let Ok(Some(proj)) = crate::storage::nodes::get_project(
            db,
            &crate::core::ids::ProjectId::from(project_id.as_str()),
        ) {
            PathBuf::from(proj.path)
        } else {
            std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
        };

        let raw_candidates = crate::ingest::detect_local_transcripts(Some(&root));
        // Filter out directories and synthetic targets (e.g. "Git History")
        let file_candidates: Vec<_> = raw_candidates
            .into_iter()
            .filter(|c| c.source != "Git History" && Path::new(&c.path).is_file())
            .collect();

        let canonical_root = root.canonicalize().unwrap_or_else(|_| root.clone());
        let project_candidates: Vec<_> = file_candidates
            .iter()
            .filter(|c| {
                let p = Path::new(&c.path);
                let cp = p.canonicalize().unwrap_or_else(|_| p.to_path_buf());
                cp.starts_with(&canonical_root)
            })
            .cloned()
            .collect();

        let active_candidates = if !project_candidates.is_empty() {
            project_candidates
        } else {
            file_candidates
        };

        if active_candidates.is_empty() {
            return Ok(UpdateReport {
                markdown: "No local transcript candidates discovered.".to_string(),
                ..Default::default()
            });
        } else if active_candidates.len() > 1 {
            let paths: Vec<String> = active_candidates.iter().map(|c| c.path.clone()).collect();
            return Err(KnoxError::InvalidInput(format!(
                "Ambiguous transcripts found: {:?}. Please specify an explicit transcript_path.",
                paths
            )));
        } else {
            Some(PathBuf::from(&active_candidates[0].path))
        }
    } else {
        return Err(KnoxError::InvalidInput(
            "No transcript path provided and discovery is disabled.".to_string(),
        ));
    };

    let Some(target) = target_path else {
        return Ok(UpdateReport {
            markdown: "No transcript target identified.".to_string(),
            ..Default::default()
        });
    };

    // 3. Resolve active session if available
    let active_session = crate::storage::sessions::list_recent_sessions(db, 10)?
        .into_iter()
        .find(|s| s.is_open && s.project_id.as_ref().map(|p| p.as_str()) == Some(project_id.as_str()));
    let active_session_id = active_session.as_ref().map(|s| &s.id);

    // 4. Perform incremental ingest
    let ingest_report = crate::commands::ingest::cmd_ingest_incremental(
        &target,
        db,
        active_session_id,
        Some(&project_id),
    )?;

    // 5. Query recorded offset
    let path_key = target
        .canonicalize()
        .unwrap_or_else(|_| target.clone())
        .to_string_lossy()
        .to_string();
    let (last_line, _) = crate::commands::ingest::get_ingest_offset(db, &path_key)?
        .unwrap_or((0, 0));

    let mut report = UpdateReport {
        actions_imported: ingest_report.actions_imported,
        duplicates_skipped: ingest_report.duplicates_skipped,
        files_touched: ingest_report.files_touched,
        tool_detected: ingest_report.tool_detected,
        source: target.display().to_string(),
        offset: last_line,
        markdown: String::new(),
    };

    report.markdown = format_update_report_markdown(&report);
    Ok(report)
}

/// Formats `UpdateReport` into a Markdown summary for MCP responses.
pub fn format_update_report_markdown(report: &UpdateReport) -> String {
    format!(
        "### Ingestion Update\n- **Source**: `{}`\n- **Tool Detected**: {}\n- **Actions Imported**: {}\n- **Duplicates Skipped**: {}\n- **Files Touched**: {}\n- **Line Offset**: {}\n",
        report.source,
        if report.tool_detected.is_empty() { "unknown" } else { &report.tool_detected },
        report.actions_imported,
        report.duplicates_skipped,
        report.files_touched,
        report.offset
    )
}

/// Formats `UpdateReport` into human-readable text for CLI terminal stdout.
pub fn format_update_report_table(report: &UpdateReport) -> String {
    format!(
        "Ingestion Update:\n  Source:             {}\n  Tool Detected:      {}\n  Actions Imported:   {}\n  Duplicates Skipped: {}\n  Files Touched:      {}\n  Line Offset:        {}",
        report.source,
        if report.tool_detected.is_empty() { "unknown" } else { &report.tool_detected },
        report.actions_imported,
        report.duplicates_skipped,
        report.files_touched,
        report.offset
    )
}

