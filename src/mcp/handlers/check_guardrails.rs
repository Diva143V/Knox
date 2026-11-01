//! MCP tool handler for `knox_check_guardrails`.

use serde::Deserialize;
use serde_json::{Value, json};

use crate::core::error::{KnoxError, KnoxResult};
use crate::core::types::{KnoxAction, QueryFilter};
use crate::ingest::summary::normalize_file_path;
use crate::storage::db::KnoxDb;

/// Input parameters for `knox_check_guardrails`.
#[derive(Debug, Deserialize, Default)]
pub struct CheckGuardrailsParams {
    /// List of file paths to check for active architectural constraints.
    pub files: Vec<String>,
    /// Optional proposed action description to test against rejected alternatives.
    pub proposed_action: Option<String>,
    /// Whether to enforce strict checking (flagging any related superseded patterns as failure).
    pub strict: Option<bool>,
    /// Optional project scope.
    #[serde(default)]
    pub project: Option<String>,
    /// Output representation format ("markdown" | "json", defaults to "markdown").
    pub format: Option<String>,
}

/// Handles the `knox_check_guardrails` tool invocation.
pub fn handle_check_guardrails(db: &KnoxDb, params: Value) -> KnoxResult<Value> {
    let input: CheckGuardrailsParams = serde_json::from_value(params)
        .map_err(|e| KnoxError::InvalidInput(format!("Invalid check_guardrails params: {e}")))?;

    let mut active_decisions: Vec<KnoxAction> = Vec::new();
    let mut stale_decisions: Vec<KnoxAction> = Vec::new();
    let mut open_doubts: Vec<KnoxAction> = Vec::new();

    let norm_files: Vec<String> = input.files.iter().map(|f| normalize_file_path(f)).collect();

    for file in &norm_files {
        // Query active decisions
        let active_filter = QueryFilter {
            file_path: Some(file.clone()),
            action_type: Some("decide".to_string()),
            current_only: true,
            project: input.project.clone(),
            limit: 20,
            ..Default::default()
        };
        let active_res = crate::storage::queries::query_actions(db, &active_filter)?;
        for a in active_res.results {
            if !active_decisions.iter().any(|d| d.id == a.id) {
                active_decisions.push(a);
            }
        }

        // Query superseded/stale decisions
        let stale_filter = QueryFilter {
            file_path: Some(file.clone()),
            action_type: Some("decide".to_string()),
            current_only: false,
            project: input.project.clone(),
            limit: 20,
            ..Default::default()
        };
        let stale_res = crate::storage::queries::query_actions(db, &stale_filter)?;
        for a in stale_res.results {
            if !a.is_current && !stale_decisions.iter().any(|d| d.id == a.id) {
                stale_decisions.push(a);
            }
        }

        // Query open doubts / questions on target files
        let doubt_filter = QueryFilter {
            file_path: Some(file.clone()),
            action_type: Some("doubt".to_string()),
            current_only: true,
            project: input.project.clone(),
            limit: 20,
            ..Default::default()
        };
        let doubt_res = crate::storage::queries::query_actions(db, &doubt_filter)?;
        for a in doubt_res.results {
            if a.is_resolved != Some(true) && !open_doubts.iter().any(|d| d.id == a.id) {
                open_doubts.push(a);
            }
        }
    }

    let mut violations: Vec<String> = Vec::new();
    if let Some(ref prop) = input.proposed_action {
        let prop_lower = prop.to_lowercase();
        for d in &active_decisions {
            for alt in &d.rejected_alternatives {
                let alt_lower = alt.trim().to_lowercase();
                if !alt_lower.is_empty()
                    && (prop_lower.contains(&alt_lower) || alt_lower.contains(&prop_lower))
                {
                    violations.push(format!(
                        "Proposed action matches rejected alternative '{}' in decision `{}` ('{}')",
                        alt, d.id, d.summary
                    ));
                }
            }
        }
    }

    let strict = input.strict.unwrap_or(false);
    let pass = violations.is_empty() && (!strict || stale_decisions.is_empty());
    let decision_count = active_decisions.len();

    let format_mode = input
        .format
        .as_deref()
        .unwrap_or("markdown")
        .to_ascii_lowercase();

    if format_mode == "json" {
        Ok(json!({
            "pass": pass,
            "violations_count": violations.len(),
            "violations": violations,
            "decision_count": decision_count,
            "open_doubts_count": open_doubts.len(),
            "open_doubts": open_doubts,
            "files": norm_files,
            "active_constraints_count": active_decisions.len(),
            "active_constraints": active_decisions,
            "superseded_patterns": stale_decisions
        }))
    } else {
        let md = crate::mcp::format::format_guardrails_markdown(
            &norm_files,
            &active_decisions,
            &stale_decisions,
            &violations,
            &open_doubts,
        );
        Ok(json!({
            "pass": pass,
            "violations_count": violations.len(),
            "violations": violations,
            "decision_count": decision_count,
            "open_doubts_count": open_doubts.len(),
            "open_doubts": open_doubts,
            "files": norm_files,
            "active_constraints_count": active_decisions.len(),
            "active_constraints": active_decisions,
            "superseded_patterns": stale_decisions,
            "markdown": md
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::ids::ActionId;
    use chrono::Utc;

    #[test]
    fn test_handle_check_guardrails() {
        let db = KnoxDb::open_memory().unwrap();
        let dec = KnoxAction {
            id: ActionId::new(),
            actor: "gemini".to_string(),
            action_type: "decide".to_string(),
            summary: "Use rusqlite bundled feature".to_string(),
            rationale: Some("Ensures zero runtime dependency".to_string()),
            files: vec!["Cargo.toml".to_string()],
            timestamp: Utc::now(),
            is_current: true,
            ..Default::default()
        };
        crate::storage::nodes::insert_action(&db, &dec).unwrap();

        let res = handle_check_guardrails(
            &db,
            json!({
                "files": ["Cargo.toml"]
            }),
        )
        .unwrap();

        assert_eq!(res.get("active_constraints_count").unwrap(), 1);
        assert!(
            res.get("markdown")
                .unwrap()
                .as_str()
                .unwrap()
                .contains("rusqlite")
        );
    }

    #[test]
    fn test_handle_check_guardrails_violations() {
        let db = KnoxDb::open_memory().unwrap();
        let dec = KnoxAction {
            id: ActionId::new(),
            actor: "gemini".to_string(),
            action_type: "decide".to_string(),
            summary: "Use rusqlite bundled feature".to_string(),
            rationale: Some("Ensures zero runtime dependency".to_string()),
            rejected_alternatives: vec!["libsqlite3-sys dynamic linking".to_string()],
            files: vec!["Cargo.toml".to_string()],
            timestamp: Utc::now(),
            is_current: true,
            ..Default::default()
        };
        crate::storage::nodes::insert_action(&db, &dec).unwrap();

        // Check proposed action that violates the decision
        let res = handle_check_guardrails(
            &db,
            json!({
                "files": ["Cargo.toml"],
                "proposed_action": "Switch to libsqlite3-sys dynamic linking"
            }),
        )
        .unwrap();

        assert_eq!(res.get("pass").unwrap(), false);
        assert_eq!(res.get("violations_count").unwrap(), 1);
        let violations = res.get("violations").unwrap().as_array().unwrap();
        assert!(
            violations[0]
                .as_str()
                .unwrap()
                .contains("libsqlite3-sys dynamic linking")
        );
    }

    #[test]
    fn test_handle_check_guardrails_surfaces_open_doubts() {
        let db = KnoxDb::open_memory().unwrap();
        let doubt = KnoxAction {
            id: ActionId::new(),
            actor: "human".to_string(),
            action_type: "doubt".to_string(),
            summary: "Is dynamic linking required on musl targets?".to_string(),
            context_summary: Some("Need to verify static compilation for musl".to_string()),
            files: vec!["Cargo.toml".to_string()],
            timestamp: Utc::now(),
            is_current: true,
            is_resolved: Some(false),
            ..Default::default()
        };
        crate::storage::nodes::insert_action(&db, &doubt).unwrap();

        let res = handle_check_guardrails(
            &db,
            json!({
                "files": ["Cargo.toml"]
            }),
        )
        .unwrap();

        assert_eq!(res.get("open_doubts_count").unwrap(), 1);
        let md = res.get("markdown").unwrap().as_str().unwrap();
        assert!(md.contains("Open Doubts & Technical Inquiries"));
        assert!(md.contains("Is dynamic linking required on musl targets?"));
    }
}
