//! Operational health diagnostics and verification command.
//!
//! Evaluates query log metrics, zombie sessions, rejected edge types,
//! database storage statistics, schema integrity, and model bindings.

use serde::{Deserialize, Serialize};

use crate::core::error::KnoxResult;
use crate::storage::db::KnoxDb;

/// Promotion threshold: a rejected edge type attempted at least 10 times is flagged.
pub const EDGE_PROMOTION_THRESHOLD: usize = 10;

/// Recommended promotion candidate identified from edge rejections.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PromotionCandidate {
    pub attempted_type: String,
    pub rejection_count: usize,
    pub threshold: usize,
    pub promotion_path: String,
}

/// Breakdown of empty read queries by failure classification.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct EmptyBreakdown {
    pub no_records: usize,
    pub no_match: usize,
    pub out_of_scope: usize,
}

/// Staleness and tool census diagnostics for workspace `knox.md`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KnoxMdStatus {
    /// Whether `knox.md` was discovered in the project or workspace directory.
    pub exists: bool,
    /// Absolute or relative path to the inspected `knox.md` file.
    pub path: Option<String>,
    /// Tool count parsed from the header or body (or census of known tools).
    pub tool_count: usize,
    /// Number of expected tools registered in the runtime (`TOOL_COUNT = 7`).
    pub expected_tool_count: usize,
    /// Tools listed in `LISTED_TOOL_NAMES` that are missing from `knox.md`.
    pub missing_tools: Vec<String>,
    /// Whether the file is missing tools or out of date.
    pub is_stale: bool,
    /// Human-readable diagnostic status message.
    pub message: String,
}

/// Comprehensive health and operational report for Knox.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DoctorReport {
    /// Overall proportion of read queries that returned empty result sets (0.0 - 1.0).
    pub empty_rate: f64,
    /// Detailed classification of empty query causes.
    pub empty_breakdown: EmptyBreakdown,
    /// Number of open sessions inactive for > 24 hours.
    pub zombie_sessions: usize,
    /// Most frequent rejected edge types and their occurrence counts.
    pub top_rejected_edge_types: Vec<(String, usize)>,
    /// Edge types exceeding the promotion threshold and their documented promotion path.
    pub promotion_candidates: Vec<PromotionCandidate>,
    /// SQLite database file size in bytes.
    pub db_size_bytes: u64,
    /// SQLite Write-Ahead Log (WAL) file size in bytes.
    pub wal_size_bytes: u64,
    /// Total database pages.
    pub page_count: i64,
    /// Page size in bytes.
    pub page_size: i64,
    /// Result of SQLite `PRAGMA integrity_check` ("ok" or error string).
    pub integrity_check: String,
    /// Active SQLite schema version (e.g. 9).
    pub schema_version: i32,
    /// Known or active agent/model bindings in the database or project.
    pub bound_models: Vec<String>,
    /// Status and staleness check of workspace `knox.md` protocol instructions.
    #[serde(default)]
    pub knox_md_status: Option<KnoxMdStatus>,
}

/// Checks the workspace `knox.md` file for tool census and staleness against current crate definitions.
pub fn check_knox_md(project: Option<&str>) -> KnoxMdStatus {
    let candidate_paths = {
        let mut paths = Vec::new();
        if let Some(p) = project {
            let pb = std::path::PathBuf::from(p);
            paths.push(pb.join("knox.md"));
            paths.push(pb.clone());
        }
        if let Ok(cur) = std::env::current_dir() {
            paths.push(cur.join("knox.md"));
        }
        paths.push(std::path::PathBuf::from("knox.md"));
        paths
    };

    let target = candidate_paths.into_iter().find(|p| p.is_file());

    let Some(path) = target else {
        return KnoxMdStatus {
            exists: false,
            path: None,
            tool_count: 0,
            expected_tool_count: crate::mcp::tools::TOOL_COUNT,
            missing_tools: crate::mcp::tools::LISTED_TOOL_NAMES
                .iter()
                .map(|s| s.to_string())
                .collect(),
            is_stale: false,
            message: "No knox.md protocol reference found in workspace.".to_string(),
        };
    };

    let content = std::fs::read_to_string(&path).unwrap_or_default();
    let mut missing_tools = Vec::new();
    let mut found_tools = Vec::new();

    for &tool in &crate::mcp::tools::LISTED_TOOL_NAMES {
        if content.contains(tool) {
            found_tools.push(tool.to_string());
        } else {
            missing_tools.push(tool.to_string());
        }
    }

    let is_stale = !missing_tools.is_empty() || content.contains("6 tools");
    let message = if is_stale {
        format!(
            "knox.md is stale: missing {} tool(s) {:?}. Run 'knox init' to refresh.",
            missing_tools.len(),
            missing_tools
        )
    } else {
        "knox.md protocol reference is up to date (7 tools).".to_string()
    };

    KnoxMdStatus {
        exists: true,
        path: Some(path.display().to_string()),
        tool_count: found_tools.len(),
        expected_tool_count: crate::mcp::tools::TOOL_COUNT,
        missing_tools,
        is_stale,
        message,
    }
}

/// Runs operational diagnostics and computes a `DoctorReport`.
pub fn cmd_doctor(db: &KnoxDb, project: Option<&str>) -> KnoxResult<DoctorReport> {
    // Retention sweeps: best-effort pruning across append-only tables
    if let Err(e) = crate::storage::prune_query_log(db, 90) {
        tracing::warn!(error = %e, "Retention sweep: query_log prune failed");
    }
    if let Err(e) = crate::storage::prune_idempotency_keys(db, 14) {
        tracing::warn!(error = %e, "Retention sweep: idempotency_keys prune failed");
    }
    if let Err(e) = crate::storage::prune_edge_rejections(db, 30) {
        tracing::warn!(error = %e, "Retention sweep: edge_rejections prune failed");
    }
    if let Err(e) = crate::storage::prune_action_deltas(db, 90) {
        tracing::warn!(error = %e, "Retention sweep: action_deltas prune failed");
    }
    if let Err(e) = crate::storage::prune_sessions(db, 90) {
        tracing::warn!(error = %e, "Retention sweep: sessions prune failed");
    }
    if let Err(e) = crate::commands::ingest::prune_ingest_log(db, 30) {
        tracing::warn!(error = %e, "Retention sweep: ingest_log prune failed");
    }

    let stats = crate::storage::get_query_log_stats(db, project)?;
    let zombie_sessions = crate::storage::get_zombie_session_count(db)?;
    let top_rejected_edge_types = crate::storage::get_top_rejected_edge_types(db, 10)?;
    let storage = db.get_storage_stats(None)?;
    let integrity_check = db.integrity_check()?;

    let bound_models = {
        let conn = db.conn();
        let resolved_id = project.and_then(|p| {
            crate::storage::sessions::resolve_project_identifier(db, p)
                .ok()
                .flatten()
        });
        let target_id = resolved_id.as_deref().unwrap_or_else(|| project.unwrap_or(""));
        let mut stmt = conn.prepare(
            "SELECT DISTINCT actor FROM actions
             WHERE (?1 IS NULL OR project_id = ?1 OR project_id = ?2
                    OR session_id IN (SELECT id FROM sessions WHERE project_id = ?1 OR project_id = ?2))
               AND actor IS NOT NULL AND actor != ''
             ORDER BY actor",
        )?;
        let rows = stmt.query_map(rusqlite::params![project, target_id], |r| r.get::<_, String>(0))?;
        let mut models = Vec::new();
        for r in rows {
            models.push(r?);
        }
        if models.is_empty() {
            vec![
                "claude".to_string(),
                "cursor".to_string(),
                "gemini".to_string(),
            ]
        } else {
            models
        }
    };

    let mut promotion_candidates = Vec::new();
    for (edge_type, count) in &top_rejected_edge_types {
        if *count >= EDGE_PROMOTION_THRESHOLD {
            let variant = edge_type
                .split('_')
                .map(|part| {
                    let mut chars = part.chars();
                    match chars.next() {
                        Some(c) => c.to_uppercase().collect::<String>() + chars.as_str(),
                        None => String::new(),
                    }
                })
                .collect::<String>();

            let promotion_path = format!(
                "Documented promotion path for '{edge_type}':\n  \
                 1. Add `{variant}` variant to `EdgeType` enum in `src/core/types.rs`\n  \
                 2. Add `\"{edge_type}\" => Ok(Self::{variant})` in `EdgeType::from_str` and Display\n  \
                 3. Add directional validation in `src/commands/record.rs` (`validate_edge_direction`)\n  \
                 4. Update graph traversal in `src/graph/traversal.rs` if traversal applies\n  \
                 5. Document semantics and add unit test in `tests/test_edges.rs`"
            );

            promotion_candidates.push(PromotionCandidate {
                attempted_type: edge_type.clone(),
                rejection_count: *count,
                threshold: EDGE_PROMOTION_THRESHOLD,
                promotion_path,
            });
        }
    }

    Ok(DoctorReport {
        empty_rate: stats.empty_rate,
        empty_breakdown: EmptyBreakdown {
            no_records: stats.no_records_count,
            no_match: stats.no_match_count,
            out_of_scope: stats.out_of_scope_count,
        },
        zombie_sessions,
        top_rejected_edge_types,
        promotion_candidates,
        db_size_bytes: storage.db_size_bytes,
        wal_size_bytes: storage.wal_size_bytes,
        page_count: storage.page_count,
        page_size: storage.page_size,
        integrity_check,
        schema_version: storage.schema_version,
        bound_models,
        knox_md_status: Some(check_knox_md(project)),
    })
}

/// Formats a `DoctorReport` into a human-readable text summary for terminal display.
pub fn format_doctor_report(report: &DoctorReport) -> String {
    let mut out = String::with_capacity(1024);
    out.push_str("=== Knox Health & Diagnostics Report ===\n\n");
    out.push_str(&format!(
        "SQLite Integrity Check : {}\n",
        report.integrity_check
    ));
    out.push_str(&format!(
        "Schema Version         : v{}\n",
        report.schema_version
    ));
    out.push_str(&format!(
        "Database Size          : {} bytes ({} pages @ {} bytes/page)\n",
        report.db_size_bytes, report.page_count, report.page_size
    ));
    if report.wal_size_bytes > 0 {
        out.push_str(&format!(
            "WAL Size               : {} bytes\n",
            report.wal_size_bytes
        ));
    }
    out.push_str(&format!(
        "Zombie Sessions (>24h) : {}\n",
        report.zombie_sessions
    ));
    out.push_str(&format!(
        "Query Empty Rate       : {:.2}%\n",
        report.empty_rate * 100.0
    ));
    out.push_str(&format!(
        "  - no_records         : {}\n",
        report.empty_breakdown.no_records
    ));
    out.push_str(&format!(
        "  - no_match           : {}\n",
        report.empty_breakdown.no_match
    ));
    out.push_str(&format!(
        "  - out_of_scope       : {}\n",
        report.empty_breakdown.out_of_scope
    ));

    out.push_str("\nTop Rejected Edge Types:\n");
    if report.top_rejected_edge_types.is_empty() {
        out.push_str("  (none recorded)\n");
    } else {
        for (edge_type, count) in &report.top_rejected_edge_types {
            out.push_str(&format!("  - {:<18}: {} rejection(s)\n", edge_type, count));
        }
    }

    if !report.promotion_candidates.is_empty() {
        out.push_str("\n--- Edge Type Promotion Candidates ---\n");
        for candidate in &report.promotion_candidates {
            out.push_str(&format!(
                "WARNING: Rejected edge type '{}' reached {} rejection(s) (threshold: {})\n",
                candidate.attempted_type, candidate.rejection_count, candidate.threshold
            ));
            out.push_str(&format!("  {}\n\n", candidate.promotion_path));
        }
    }

    out.push_str("\nBound / Active Models:\n");
    for m in &report.bound_models {
        out.push_str(&format!("  - {}\n", m));
    }

    if let Some(ref md_status) = report.knox_md_status {
        out.push_str("\nProtocol Reference (knox.md):\n");
        out.push_str(&format!(
            "  Status: {}\n",
            if md_status.is_stale {
                "STALE"
            } else if md_status.exists {
                "UP TO DATE"
            } else {
                "NOT FOUND"
            }
        ));
        out.push_str(&format!("  {}\n", md_status.message));
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cmd_doctor_runs_on_empty_db() {
        let db = KnoxDb::open_memory().unwrap();
        let report = cmd_doctor(&db, None).unwrap();
        assert_eq!(report.integrity_check, "ok");
        assert_eq!(report.schema_version, 11);
        assert_eq!(report.zombie_sessions, 0);
        assert_eq!(report.empty_rate, 0.0);
        assert!(!report.bound_models.is_empty());

        let text = format_doctor_report(&report);
        assert!(text.contains("=== Knox Health & Diagnostics Report ==="));
        assert!(text.contains("SQLite Integrity Check : ok"));
    }

    #[test]
    fn test_cmd_doctor_flags_edge_promotion_candidates() {
        let db = KnoxDb::open_memory().unwrap();

        // Record 10 rejections for "fixes" and 2 for "blocks"
        for i in 0..10 {
            let from = format!("from_{i}");
            let to = format!("to_{i}");
            crate::storage::edges::record_edge_rejection(
                &db,
                Some("demo"),
                Some(&from),
                Some(&to),
                "fixes",
                "unknown edge type",
            )
            .unwrap();
        }
        for i in 0..2 {
            let from = format!("from_{i}");
            let to = format!("to_{i}");
            crate::storage::edges::record_edge_rejection(
                &db,
                Some("demo"),
                Some(&from),
                Some(&to),
                "blocks",
                "unknown edge type",
            )
            .unwrap();
        }

        let report = cmd_doctor(&db, Some("demo")).unwrap();
        assert_eq!(report.promotion_candidates.len(), 1);
        let cand = &report.promotion_candidates[0];
        assert_eq!(cand.attempted_type, "fixes");
        assert_eq!(cand.rejection_count, 10);
        assert_eq!(cand.threshold, EDGE_PROMOTION_THRESHOLD);
        assert!(cand.promotion_path.contains("Documented promotion path for 'fixes'"));
        assert!(cand.promotion_path.contains("Add `Fixes` variant to `EdgeType` enum"));

        let text = format_doctor_report(&report);
        assert!(text.contains("Edge Type Promotion Candidates"));
        assert!(text.contains("WARNING: Rejected edge type 'fixes' reached 10 rejection(s)"));
        assert!(text.contains("Documented promotion path for 'fixes'"));
    }

    #[test]
    fn test_knox_md_staleness_and_tool_census() {
        let temp = tempfile::tempdir().unwrap();
        let knox_md_path = temp.path().join("knox.md");

        // 1. Stale knox.md with only 6 tools (missing knox_ask)
        let stale_content = "# Protocol\nKnox exposes 6 tools:\n- knox_begin\n- knox_context\n- knox_check\n- knox_trace\n- knox_record\n- knox_handoff\n";
        std::fs::write(&knox_md_path, stale_content).unwrap();

        let status = check_knox_md(Some(temp.path().to_str().unwrap()));
        assert!(status.exists);
        assert!(status.is_stale);
        assert_eq!(status.tool_count, 6);
        assert!(status.missing_tools.contains(&"knox_ask".to_string()));
        assert!(status.message.contains("missing 1 tool(s)"));

        // 2. Fresh knox.md with all 7 tools
        let fresh_content = "# Protocol\nKnox exposes 7 tools:\n- knox_begin\n- knox_context\n- knox_check\n- knox_trace\n- knox_record\n- knox_handoff\n- knox_ask\n";
        std::fs::write(&knox_md_path, fresh_content).unwrap();

        let status_fresh = check_knox_md(Some(temp.path().to_str().unwrap()));
        assert!(status_fresh.exists);
        assert!(!status_fresh.is_stale);
        assert_eq!(status_fresh.tool_count, 7);
        assert!(status_fresh.missing_tools.is_empty());
    }
}

