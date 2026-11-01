//! Git staged changes scanner for human edit detection in Knox.
//! Identifies modifications made directly by developers without AI agent record.

use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::path::Path;
use std::process::Command;

use crate::core::error::{KnoxError, KnoxResult};
use crate::core::ids::ActionId;
use crate::core::types::{KnoxAction, QueryFilter, ScanReport};
use crate::ingest::summary::normalize_file_path;
use crate::storage::db::KnoxDb;
use crate::storage::nodes::insert_action;
use crate::storage::queries::query_actions;

/// Scans git staged changes, cross-references against recent AI actions,
/// and records any untracked human edits.
pub fn cmd_scan(project_path: &Path, db: &KnoxDb) -> KnoxResult<ScanReport> {
    let staged_files = fetch_staged_files(project_path)?;
    if staged_files.is_empty() {
        return Ok(ScanReport::default());
    }

    let cutoff = Utc::now() - chrono::Duration::minutes(30);
    let mut human_files = Vec::new();

    for file in &staged_files {
        let norm_path = normalize_file_path(file);
        if !is_file_tracked_by_recent_ai(db, &norm_path, cutoff)? {
            human_files.push(norm_path);
        }
    }

    let numstats = fetch_numstats(project_path);
    let diff_stat = fetch_diff_stat(project_path);
    let mut actions_recorded = 0;

    if !human_files.is_empty() {
        let summary = build_human_edit_summary(&human_files, &numstats);
        let action = KnoxAction {
            id: ActionId::new(),
            actor: "human".to_string(),
            action_type: "human_write".to_string(),
            summary,
            files: human_files.clone(),
            tags: vec!["git".to_string(), "staged".to_string()],
            confidence: None,
            timestamp: Utc::now(),
            session_id: None,
            parent_action_id: None,
            supersedes: None,
            is_current: true,
            relations: Vec::new(),
            symbols: Vec::new(),
            rationale: diff_stat.clone(),
            rejected_alternatives: Vec::new(),
            context_summary: Some("Human git staged edits detected".to_string()),
            significance: Some("minor".to_string()),
            is_resolved: Some(true),
            resolution_note: None,
        };

        insert_action(db, &action)?;
        actions_recorded = 1;
    }

    Ok(ScanReport {
        human_edits_detected: human_files.len(),
        actions_recorded,
        files_scanned: staged_files,
        diff_stat,
    })
}

/// Executes git to retrieve the list of staged modified/created/renamed files.
fn fetch_staged_files(project_path: &Path) -> KnoxResult<Vec<String>> {
    let output = Command::new("git")
        .args(["diff", "--cached", "--name-only", "--diff-filter=ACMR"])
        .current_dir(project_path)
        .output()
        .map_err(|e| KnoxError::Git(format!("Failed to execute git: {e}")))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(KnoxError::Git(format!("Git error: {stderr}")));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let files = stdout
        .lines()
        .map(|l| l.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    Ok(files)
}

/// Retrieves git staged per-file additions and deletions via numstat.
fn fetch_numstats(project_path: &Path) -> HashMap<String, (usize, usize)> {
    let mut map = HashMap::new();
    let output = Command::new("git")
        .args(["diff", "--cached", "--numstat"])
        .current_dir(project_path)
        .output()
        .ok();

    if let Some(out) = output {
        if out.status.success() {
            let stdout = String::from_utf8_lossy(&out.stdout);
            for line in stdout.lines() {
                let parts: Vec<&str> = line.split('\t').collect();
                if parts.len() >= 3 {
                    let ins = parts[0].parse::<usize>().unwrap_or(0);
                    let del = parts[1].parse::<usize>().unwrap_or(0);
                    let path = normalize_file_path(parts[2]);
                    map.insert(path, (ins, del));
                }
            }
        }
    }
    map
}

/// Retrieves git staged diff summary statistics.
fn fetch_diff_stat(project_path: &Path) -> Option<String> {
    let output = Command::new("git")
        .args(["diff", "--cached", "--stat"])
        .current_dir(project_path)
        .output()
        .ok()?;

    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        let trimmed = stdout.trim();
        if !trimmed.is_empty() {
            return Some(trimmed.to_string());
        }
    }
    None
}

/// Checks if an AI action has touched this file since the cutoff timestamp.
fn is_file_tracked_by_recent_ai(
    db: &KnoxDb,
    file_path: &str,
    cutoff: DateTime<Utc>,
) -> KnoxResult<bool> {
    let filter = QueryFilter {
        file_path: Some(file_path.to_string()),
        since: Some(cutoff),
        current_only: false,
        limit: 10,
        ..Default::default()
    };

    let query_res = query_actions(db, &filter)?;
    let tracked = query_res.results.iter().any(|act| act.actor != "human");
    Ok(tracked)
}

/// Builds a human edit action summary incorporating files and git numstat additions/deletions.
fn build_human_edit_summary(
    files: &[String],
    numstats: &HashMap<String, (usize, usize)>,
) -> String {
    let mut parts = Vec::new();
    for file in files {
        if let Some(&(ins, del)) = numstats.get(file) {
            parts.push(format!("{file} (+{ins} -{del})"));
        } else {
            parts.push(file.clone());
        }
    }
    format!("Human edit: {}", parts.join(", "))
}

/// Formats a `ScanReport` for terminal output.
pub fn format_scan_report(report: &ScanReport) -> String {
    format!(
        "Scan complete: {} staged files checked, {} untracked human edits detected ({} action recorded)",
        report.files_scanned.len(),
        report.human_edits_detected,
        report.actions_recorded
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_scan_report() {
        let report = ScanReport {
            human_edits_detected: 2,
            actions_recorded: 1,
            files_scanned: vec!["src/main.rs".to_string(), "README.md".to_string()],
            diff_stat: Some("2 files changed, 10 insertions(+)".to_string()),
        };

        let formatted = format_scan_report(&report);
        assert!(formatted.contains("2 staged files checked"));
        assert!(formatted.contains("2 untracked human edits detected"));
    }

    #[test]
    fn test_build_human_edit_summary_with_numstat() {
        let files = vec!["src/parser.rs".to_string(), "src/lib.rs".to_string()];
        let mut numstats = HashMap::new();
        numstats.insert("src/parser.rs".to_string(), (42, 15));
        numstats.insert("src/lib.rs".to_string(), (3, 1));

        let summary = build_human_edit_summary(&files, &numstats);
        assert_eq!(
            summary,
            "Human edit: src/parser.rs (+42 -15), src/lib.rs (+3 -1)"
        );
    }
}
