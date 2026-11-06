//! Query telemetry logging and operational health metrics.

use rusqlite::params;

use super::db::KnoxDb;
use crate::core::error::KnoxResult;
use crate::core::types::QueryLogEntry;

/// Records an operational query log entry into the `query_log` table.
///
/// Ensures telemetry privacy: strictly records aggregate statistics (latency, tokens, rows, tool, project),
/// and NEVER logs query strings, keywords, or file paths.
pub fn record_query_log(db: &KnoxDb, entry: &QueryLogEntry) -> KnoxResult<()> {
    db.conn().execute(
        "INSERT INTO query_log (id, ts_ms, tool, project_id, rows, tokens, latency_ms, reason)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![
            entry.id,
            entry.ts_ms,
            entry.tool,
            entry.project_id.as_deref(),
            entry.rows as i64,
            entry.tokens as i64,
            entry.latency_ms as i64,
            entry.reason.as_deref(),
        ],
    )?;
    Ok(())
}

/// Records a query log entry with best-effort error handling, logging any SQLite issue without failing the calling tool.
pub fn record_query_log_best_effort(db: &KnoxDb, entry: &QueryLogEntry) {
    if let Err(e) = record_query_log(db, entry) {
        tracing::debug!(error = %e, "Query telemetry log recorded with best-effort, ignored error");
    }
}

/// Prunes query log entries older than `max_age_days` days to prevent unbounded database growth.
pub fn prune_query_log(db: &KnoxDb, max_age_days: i64) -> KnoxResult<usize> {
    let cutoff_ms = chrono::Utc::now().timestamp_millis() - (max_age_days * 86_400_000);
    let deleted = db.conn().execute(
        "DELETE FROM query_log WHERE ts_ms < ?1",
        params![cutoff_ms],
    )?;
    Ok(deleted)
}

/// Aggregated query telemetry statistics.
#[derive(Debug, Clone, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct QueryTelemetryStats {
    pub total_queries: usize,
    pub empty_queries: usize,
    pub empty_rate: f64,
    pub no_records_count: usize,
    pub no_match_count: usize,
    pub out_of_scope_count: usize,
}

/// Retrieves aggregated query log metrics, optionally scoped to a project.
pub fn get_query_log_stats(db: &KnoxDb, project_id: Option<&str>) -> KnoxResult<QueryTelemetryStats> {
    let conn = db.conn();
    let resolved_id = project_id.and_then(|p| {
        crate::storage::sessions::resolve_project_identifier(db, p)
            .ok()
            .flatten()
    });
    let target_id = resolved_id.as_deref().unwrap_or_else(|| project_id.unwrap_or(""));

    let (total, empty, no_records, no_match, out_of_scope): (i64, i64, i64, i64, i64) = conn
        .query_row(
            "SELECT
                COUNT(*),
                COUNT(CASE WHEN reason IS NOT NULL OR rows = 0 THEN 1 END),
                COUNT(CASE WHEN reason = 'no_records' THEN 1 END),
                COUNT(CASE WHEN reason = 'no_match' THEN 1 END),
                COUNT(CASE WHEN reason = 'out_of_scope' THEN 1 END)
             FROM query_log
             WHERE (?1 IS NULL OR project_id = ?1 OR project_id = ?2)",
            params![project_id, target_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?)),
        )?;

    let total_queries = total as usize;
    let empty_queries = empty as usize;
    let empty_rate = if total_queries > 0 {
        empty_queries as f64 / total_queries as f64
    } else {
        0.0
    };

    Ok(QueryTelemetryStats {
        total_queries,
        empty_queries,
        empty_rate,
        no_records_count: no_records as usize,
        no_match_count: no_match as usize,
        out_of_scope_count: out_of_scope as usize,
    })
}

/// Retrieves all recorded query latencies in milliseconds, sorted in ascending order.
pub fn get_query_latencies(db: &KnoxDb, project_id: Option<&str>) -> KnoxResult<Vec<i64>> {
    let conn = db.conn();
    let resolved_id = project_id.and_then(|p| {
        crate::storage::sessions::resolve_project_identifier(db, p)
            .ok()
            .flatten()
    });
    let target_id = resolved_id.as_deref().unwrap_or_else(|| project_id.unwrap_or(""));

    let mut stmt = conn.prepare(
        "SELECT latency_ms FROM query_log
         WHERE (?1 IS NULL OR project_id = ?1 OR project_id = ?2)
         ORDER BY latency_ms ASC",
    )?;
    let rows = stmt.query_map(params![project_id, target_id], |r| r.get(0))?;
    let mut latencies = Vec::new();
    for r in rows {
        latencies.push(r?);
    }
    Ok(latencies)
}

/// Returns `(successful_queries_count, total_tokens_successful)`.
pub fn get_successful_query_token_stats(
    db: &KnoxDb,
    project_id: Option<&str>,
) -> KnoxResult<(usize, usize)> {
    let conn = db.conn();
    let resolved_id = project_id.and_then(|p| {
        crate::storage::sessions::resolve_project_identifier(db, p)
            .ok()
            .flatten()
    });
    let target_id = resolved_id.as_deref().unwrap_or_else(|| project_id.unwrap_or(""));

    let (hits, tokens): (i64, i64) = conn.query_row(
        "SELECT COUNT(*), COALESCE(SUM(tokens), 0)
         FROM query_log
         WHERE (?1 IS NULL OR project_id = ?1 OR project_id = ?2)
           AND rows > 0 AND reason IS NULL",
        params![project_id, target_id],
        |r| Ok((r.get(0)?, r.get(1)?)),
    )?;
    Ok((hits as usize, tokens as usize))
}

/// Returns `(duplicate_actions_count, total_actions_count)`.
pub fn get_action_duplicate_stats(
    db: &KnoxDb,
    project_id: Option<&str>,
) -> KnoxResult<(usize, usize)> {
    let conn = db.conn();
    let resolved_id = project_id.and_then(|p| {
        crate::storage::sessions::resolve_project_identifier(db, p)
            .ok()
            .flatten()
    });
    let target_id = resolved_id.as_deref().unwrap_or_else(|| project_id.unwrap_or(""));

    let total: i64 = conn.query_row(
        "SELECT COUNT(*) FROM actions
         WHERE (?1 IS NULL OR project_id = ?1 OR project_id = ?2
                OR session_id IN (SELECT id FROM sessions WHERE project_id = ?1 OR project_id = ?2))",
        params![project_id, target_id],
        |r| r.get(0),
    )?;

    // Duplicate count: sum of (occurrences - 1) for actions sharing action_type and normalized summary
    let duplicates: i64 = conn.query_row(
        "SELECT COALESCE(SUM(cnt - 1), 0) FROM (
             SELECT COUNT(*) as cnt FROM actions
             WHERE (?1 IS NULL OR project_id = ?1 OR project_id = ?2
                    OR session_id IN (SELECT id FROM sessions WHERE project_id = ?1 OR project_id = ?2))
             GROUP BY action_type, LOWER(TRIM(summary))
             HAVING cnt > 1
         )",
        params![project_id, target_id],
        |r| r.get(0),
    )?;

    Ok((duplicates as usize, total as usize))
}

/// Counts zombie sessions (sessions that remain open with no activity for > 24 hours).
pub fn get_zombie_session_count(db: &KnoxDb) -> KnoxResult<usize> {
    let conn = db.conn();
    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM sessions s
         WHERE s.is_open = 1
           AND (strftime('%s', 'now') - strftime('%s', COALESCE((SELECT MAX(a.timestamp) FROM actions a WHERE a.session_id = s.id), s.started_at))) > 86400",
        [],
        |r| r.get(0),
    )?;
    Ok(count as usize)
}

/// Retrieves the top rejected edge types and their occurrence counts.
pub fn get_top_rejected_edge_types(db: &KnoxDb, limit: usize) -> KnoxResult<Vec<(String, usize)>> {
    let conn = db.conn();
    let mut stmt = conn.prepare(
        "SELECT attempted_type, COUNT(*) as cnt
         FROM edge_rejections
         GROUP BY attempted_type
         ORDER BY cnt DESC
         LIMIT ?1",
    )?;
    let rows = stmt.query_map(params![limit as i64], |r| {
        Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)? as usize))
    })?;
    let mut results = Vec::new();
    for row in rows {
        results.push(row?);
    }
    Ok(results)
}

/// Determines the specific reason when a read query produces zero rows.
pub fn determine_empty_reason(
    db: &KnoxDb,
    project_id: Option<&str>,
    file_path: Option<&str>,
) -> &'static str {
    // 1. File path outside project or invalid scope
    if let Some(fp) = file_path {
        let clean_fp = fp.trim();
        if clean_fp.starts_with("..") || clean_fp.contains("/../") || clean_fp.contains("\\..\\") {
            return "out_of_scope";
        }
    }

    if let Some(proj) = project_id {
        let clean_proj = proj.trim();
        if clean_proj.is_empty()
            || clean_proj.starts_with("..")
            || clean_proj.contains("/../")
            || clean_proj.contains("\\..\\")
        {
            return "out_of_scope";
        }

        // Check if project is recognized (exists in `projects` table or has any action)
        let resolved_id = crate::storage::sessions::resolve_project_identifier(db, clean_proj)
            .ok()
            .flatten();

        let is_registered: bool = resolved_id.is_some() || db.conn().query_row(
            "SELECT 1 FROM projects WHERE id = ?1 OR name = ?1 OR path = ?1",
            params![clean_proj],
            |_| Ok(()),
        ).is_ok();

        let target_id = resolved_id.as_deref().unwrap_or(clean_proj);

        let has_actions_for_project: bool = db.conn().query_row(
            "SELECT 1 FROM actions WHERE project_id = ?1 OR project_id = ?2 OR session_id IN (SELECT id FROM sessions WHERE project_id = ?1 OR project_id = ?2) LIMIT 1",
            params![target_id, clean_proj],
            |_| Ok(()),
        ).is_ok();

        if !is_registered && !has_actions_for_project && clean_proj != "demo" {
            return "out_of_scope";
        }

        if !has_actions_for_project {
            return "no_records";
        }

        "no_match"
    } else {
        let total_actions: i64 = db.conn().query_row(
            "SELECT COUNT(*) FROM actions",
            [],
            |r| r.get(0),
        ).unwrap_or(0);

        if total_actions == 0 {
            "no_records"
        } else {
            "no_match"
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::ids::ActionId;
    use crate::core::types::KnoxAction;
    use crate::storage::nodes::insert_action;
    use chrono::Utc;

    #[test]
    fn test_record_and_retrieve_query_log() {
        let db = KnoxDb::open_memory().unwrap();

        let entry1 = QueryLogEntry {
            id: "ql_1".to_string(),
            ts_ms: Utc::now().timestamp_millis(),
            tool: "knox_context".to_string(),
            project_id: Some("demo".to_string()),
            rows: 5,
            tokens: 450,
            latency_ms: 12,
            reason: None,
        };
        record_query_log(&db, &entry1).unwrap();

        let entry2 = QueryLogEntry {
            id: "ql_2".to_string(),
            ts_ms: Utc::now().timestamp_millis(),
            tool: "knox_context".to_string(),
            project_id: Some("demo".to_string()),
            rows: 0,
            tokens: 50,
            latency_ms: 8,
            reason: Some("no_match".to_string()),
        };
        record_query_log(&db, &entry2).unwrap();

        let stats = get_query_log_stats(&db, Some("demo")).unwrap();
        assert_eq!(stats.total_queries, 2);
        assert_eq!(stats.empty_queries, 1);
        assert!((stats.empty_rate - 0.5).abs() < f64::EPSILON);
        assert_eq!(stats.no_match_count, 1);
        assert_eq!(stats.no_records_count, 0);
    }

    #[test]
    fn test_determine_empty_reason_phases() {
        let db = KnoxDb::open_memory().unwrap();

        // 1. Unregistered project -> out_of_scope
        assert_eq!(
            determine_empty_reason(&db, Some("unknown_workspace_xyz"), None),
            "out_of_scope"
        );

        // Path traversing upwards -> out_of_scope
        assert_eq!(
            determine_empty_reason(&db, Some("demo"), Some("../../etc/passwd")),
            "out_of_scope"
        );

        // 2. Demo project with 0 records -> no_records
        assert_eq!(
            determine_empty_reason(&db, Some("demo"), None),
            "no_records"
        );

        // 3. Insert an action for demo project -> now should be no_match
        let action = KnoxAction {
            id: ActionId::new(),
            actor: "human".to_string(),
            action_type: "plan".to_string(),
            summary: "Initial setup".to_string(),
            files: vec!["src/main.rs".to_string()],
            tags: vec![],
            confidence: None,
            timestamp: Utc::now(),
            session_id: None,
            parent_action_id: None,
            supersedes: None,
            project_id: Some(crate::core::ids::ProjectId::from("demo")),
            is_current: true,
            ..Default::default()
        };
        insert_action(&db, &action).unwrap();

        assert_eq!(
            determine_empty_reason(&db, Some("demo"), Some("src/missing.rs")),
            "no_match"
        );
    }

    #[test]
    fn test_prune_query_log_retention_sweep() {
        let db = KnoxDb::open_memory().unwrap();

        // 1. Old entry (100 days old)
        let old_ts = Utc::now().timestamp_millis() - (100 * 86_400_000);
        let old_entry = QueryLogEntry {
            id: "ql_old".to_string(),
            ts_ms: old_ts,
            tool: "knox_context".to_string(),
            project_id: Some("demo".to_string()),
            rows: 2,
            tokens: 100,
            latency_ms: 5,
            reason: None,
        };
        record_query_log_best_effort(&db, &old_entry);

        // 2. Fresh entry (1 day old)
        let fresh_ts = Utc::now().timestamp_millis() - (1 * 86_400_000);
        let fresh_entry = QueryLogEntry {
            id: "ql_fresh".to_string(),
            ts_ms: fresh_ts,
            tool: "knox_context".to_string(),
            project_id: Some("demo".to_string()),
            rows: 5,
            tokens: 300,
            latency_ms: 10,
            reason: None,
        };
        record_query_log_best_effort(&db, &fresh_entry);

        let initial_stats = get_query_log_stats(&db, None).unwrap();
        assert_eq!(initial_stats.total_queries, 2);

        // Run 90-day retention sweep
        let pruned = prune_query_log(&db, 90).unwrap();
        assert_eq!(pruned, 1, "Must prune exactly the 100-day-old entry");

        let after_stats = get_query_log_stats(&db, None).unwrap();
        assert_eq!(after_stats.total_queries, 1);
    }
}

