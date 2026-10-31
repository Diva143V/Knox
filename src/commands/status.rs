//! Status reporting command for Knox.
//! Aggregates and displays knowledge graph statistics.

use crate::core::error::KnoxResult;
use crate::core::types::StatusReport;
use crate::storage::db::KnoxDb;
use crate::storage::queries::get_status_report;

/// Computes aggregate knowledge graph statistics.
pub fn cmd_status(db: &KnoxDb) -> KnoxResult<StatusReport> {
    get_status_report(db)
}

/// Formats a `StatusReport` into human-readable text for the CLI.
pub fn format_status_report(report: &StatusReport) -> String {
    let mut lines = Vec::new();
    lines.push("Knox Knowledge Graph Status".to_string());
    lines.push("===========================".to_string());
    lines.push(format!("Total Actions:     {}", report.total_actions));
    lines.push(format!("Total Sessions:    {}", report.total_sessions));
    lines.push(format!("Projects Tracked:  {}", report.projects_tracked));

    let last_time = match report.last_action_timestamp {
        Some(ts) => ts.format("%Y-%m-%d %H:%M:%S UTC").to_string(),
        None => "None".to_string(),
    };
    lines.push(format!("Last Action:       {}", last_time));

    lines.push("\nActions by Actor:".to_string());
    if report.actions_by_actor.is_empty() {
        lines.push("  (none)".to_string());
    } else {
        let mut sorted_actors: Vec<_> = report.actions_by_actor.iter().collect();
        sorted_actors.sort_by_key(|(k, _)| (*k).clone());
        for (actor, count) in sorted_actors {
            lines.push(format!("  - {:<12}: {}", actor, count));
        }
    }

    lines.push("\nActions by Type:".to_string());
    if report.actions_by_type.is_empty() {
        lines.push("  (none)".to_string());
    } else {
        let mut sorted_types: Vec<_> = report.actions_by_type.iter().collect();
        sorted_types.sort_by_key(|(k, _)| (*k).clone());
        for (atype, count) in sorted_types {
            lines.push(format!("  - {:<12}: {}", atype, count));
        }
    }

    lines.join("\n")
}

/// Formats a `StatusReport` into pretty JSON.
pub fn format_status_json(report: &StatusReport) -> KnoxResult<String> {
    serde_json::to_string_pretty(report).map_err(crate::core::error::KnoxError::Json)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::ids::ActionId;
    use crate::core::types::KnoxAction;
    use crate::storage::nodes::insert_action;
    use chrono::Utc;

    #[test]
    fn test_cmd_status_reporting() {
        let db = KnoxDb::open_memory().unwrap();

        let action = KnoxAction {
            id: ActionId::new(),
            actor: "developer".to_string(),
            action_type: "write".to_string(),
            summary: "Added feature".to_string(),
            files: vec![],
            tags: vec![],
            confidence: None,
            timestamp: Utc::now(),
            session_id: None,
            parent_action_id: None,
            supersedes: None,
            is_current: true,
            ..Default::default()
        };
        insert_action(&db, &action).unwrap();

        let report = cmd_status(&db).unwrap();
        assert_eq!(report.total_actions, 1);
        assert_eq!(*report.actions_by_actor.get("developer").unwrap(), 1);
        assert_eq!(*report.actions_by_type.get("write").unwrap(), 1);

        let formatted = format_status_report(&report);
        assert!(formatted.contains("Total Actions:     1"));
        assert!(formatted.contains("developer"));
    }
}
