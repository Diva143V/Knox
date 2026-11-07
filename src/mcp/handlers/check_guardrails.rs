//! MCP tool handler for `knox_check_guardrails`.

use serde::Deserialize;
use serde_json::{Value, json};
use std::collections::HashSet;

use crate::core::error::{KnoxError, KnoxResult};
use crate::core::ids::ActionId;
use crate::core::types::{KnoxAction, QueryFilter};
use crate::ingest::summary::normalize_file_path_with_root;
use crate::storage::db::KnoxDb;

/// Input parameters for `knox_check_guardrails`.
#[derive(Debug, Deserialize, Default)]
pub struct CheckGuardrailsParams {
    /// List of file paths to check for active architectural constraints.
    pub files: Vec<String>,
    /// Optional proposed action description to test against rejected alternatives.
    pub proposed_action: Option<String>,
    /// Whether to fail on any violation rather than only high-overlap ones.
    pub strict: Option<bool>,
    /// Optional project scope.
    #[serde(default)]
    pub project: Option<String>,
    /// Optional code symbols touched by the proposed change. Constraints
    /// recorded against these symbols are included even for new files.
    #[serde(default)]
    pub symbols: Vec<String>,
    /// Output representation format ("markdown" | "json", defaults to "markdown").
    pub format: Option<String>,
}

/// A rejected-alternative match, with the evidence needed to judge it.
#[derive(Debug, Clone, serde::Serialize)]
struct Violation {
    message: String,
    alternative: String,
    decision_id: String,
    decision_summary: String,
    overlap: f64,
    matched_span: String,
}

/// Splits text into lowercase content tokens, dropping punctuation and stopwords.
fn content_tokens(text: &str) -> Vec<String> {
    const STOPWORDS: &[&str] = &[
        "a", "an", "the", "to", "of", "in", "on", "for", "and", "or", "is", "are", "be", "with",
        "by", "as", "at", "from", "that", "this", "it", "we", "i", "you", "use", "using", "used",
    ];
    text.to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|t| !t.is_empty())
        .filter(|t| !STOPWORDS.contains(t))
        .map(|t| t.to_string())
        .collect()
}

/// Returns `(overlap, evidence)` when `proposal` meaningfully restates a
/// rejected alternative at or above the given `threshold`.
///
/// Replaces bidirectional substring containment, where an alternative such as
/// "React" matched nearly every proposal. Requires a threshold of the
/// alternative's content tokens and ignores explicitly negated proposals.
pub fn match_rejected_alternative_threshold(
    proposal: &str,
    alternative: &str,
    threshold: f64,
) -> Option<(f64, String)> {
    let alt_tokens = content_tokens(alternative);
    if alt_tokens.is_empty() {
        return None;
    }

    let proposal_tokens = content_tokens(proposal);
    if proposal_tokens.is_empty() {
        return None;
    }
    let proposal_set: HashSet<&str> = proposal_tokens.iter().map(String::as_str).collect();

    // A single short token is too ambiguous to flag on its own.
    if alt_tokens.len() == 1 && alt_tokens[0].len() < 4 {
        return None;
    }

    let matched: Vec<&String> = alt_tokens
        .iter()
        .filter(|t| proposal_set.contains(t.as_str()))
        .collect();
    if matched.is_empty() {
        return None;
    }

    let coverage = matched.len() as f64 / alt_tokens.len() as f64;
    let required = if alt_tokens.len() == 1 {
        1.0
    } else {
        threshold
    };
    if coverage < required {
        return None;
    }

    // If the proposal negates the match ("do not use X"), it is not a violation.
    let first_match = proposal_tokens
        .iter()
        .position(|t| matched.contains(&t));
    if let Some(idx) = first_match {
        const NEGATIONS: &[&str] = &[
            "not", "no", "never", "without", "avoid", "dont", "doesnt", "isnt", "wont", "nor",
        ];
        let start = idx.saturating_sub(3);
        if proposal_tokens[start..idx]
            .iter()
            .any(|t| NEGATIONS.contains(&t.as_str()))
        {
            return None;
        }
    }

    let evidence = matched
        .iter()
        .map(|t| t.as_str())
        .collect::<Vec<_>>()
        .join(" ");
    Some((coverage, evidence))
}

pub fn match_rejected_alternative(proposal: &str, alternative: &str) -> Option<(f64, String)> {
    match_rejected_alternative_threshold(proposal, alternative, 0.6)
}

fn guardrail_patterns(db: &KnoxDb, action: &KnoxAction) -> KnoxResult<Vec<String>> {
    let mut stmt = db.conn().prepare(
        "SELECT pattern FROM guardrail_rules
         WHERE action_id = ?1 AND active = 1
         ORDER BY id",
    )?;
    let patterns = stmt
        .query_map(rusqlite::params![action.id.as_str()], |row| row.get(0))?
        .collect::<Result<Vec<String>, _>>()?;
    if patterns.is_empty() {
        Ok(action.rejected_alternatives.clone())
    } else {
        Ok(patterns)
    }
}

/// Builds the scopes a guardrail lookup should cover: the file itself, each
/// ancestor directory, and any explicitly supplied symbols.
fn guardrail_scopes(norm_files: &[String]) -> Vec<(String, bool)> {
    let mut scopes: Vec<(String, bool)> = Vec::new();
    for file in norm_files {
        scopes.push((file.clone(), false));
        let mut current = file.as_str();
        while let Some(idx) = current.rfind('/') {
            let parent = &current[..idx];
            if parent.is_empty() {
                break;
            }
            scopes.push((parent.to_string(), true));
            current = parent;
        }
    }
    scopes.sort();
    scopes.dedup();
    scopes
}

/// Unions decisions/doubts across file scopes and symbols.
fn query_scoped_actions(
    db: &KnoxDb,
    scopes: &[(String, bool)],
    symbols: &[String],
    project: Option<&str>,
    action_type: &str,
    current_only: bool,
) -> KnoxResult<Vec<KnoxAction>> {
    let mut out: Vec<KnoxAction> = Vec::new();
    let mut seen: HashSet<ActionId> = HashSet::new();

    for (path, exact) in scopes {
        let filter = QueryFilter {
            file_path: Some(path.clone()),
            file_path_exact: *exact,
            action_type: Some(action_type.to_string()),
            current_only,
            project: project.map(str::to_string),
            limit: 50,
            ..Default::default()
        };
        for action in crate::storage::queries::query_actions(db, &filter)?.results {
            if seen.insert(action.id.clone()) {
                out.push(action);
            }
        }
    }

    for symbol in symbols {
        let filter = QueryFilter {
            symbols: vec![symbol.clone()],
            action_type: Some(action_type.to_string()),
            current_only,
            project: project.map(str::to_string),
            limit: 50,
            ..Default::default()
        };
        for action in crate::storage::queries::query_actions(db, &filter)?.results {
            if seen.insert(action.id.clone()) {
                out.push(action);
            }
        }
    }

    Ok(out)
}

/// Handles the `knox_check_guardrails` tool invocation.
pub fn handle_check_guardrails(db: &KnoxDb, params: Value) -> KnoxResult<Value> {
    let input: CheckGuardrailsParams = serde_json::from_value(params)
        .map_err(|e| KnoxError::InvalidInput(format!("Invalid check_guardrails params: {e}")))?;

    let query_project: Option<String> = if let Some(ref project) = input.project {
        crate::storage::sessions::resolve_project_identifier(db, project)?
            .or_else(|| Some(project.clone()))
    } else {
        crate::storage::sessions::resolve_project_identifier(db, ".")?
    };

    let project_root = if let Some(ref proj_id) = query_project {
        crate::storage::nodes::get_project(db, &crate::core::ids::ProjectId::from(proj_id.as_str()))
            .ok()
            .flatten()
            .map(|project| project.path)
    } else {
        Some(
            std::env::current_dir()
                .map_err(|e| KnoxError::InvalidInput(format!("Unable to resolve current project root: {e}")))?
                .to_string_lossy()
                .to_string(),
        )
    };
    let norm_files: Vec<String> = input
        .files
        .iter()
        .map(|file| normalize_file_path_with_root(file, project_root.as_deref()))
        .collect();
    let scopes = guardrail_scopes(&norm_files);

    // Active decisions applying to any touched file, ancestor directory or symbol.
    let active_decisions = query_scoped_actions(
        db,
        &scopes,
        &input.symbols,
        query_project.as_deref(),
        "decide",
        true,
    )?;

    // Every decision in scope, so superseded patterns can be reported without
    // letting them influence the pass/fail outcome.
    let all_decisions = query_scoped_actions(
        db,
        &scopes,
        &input.symbols,
        query_project.as_deref(),
        "decide",
        false,
    )?;
    let stale_decisions: Vec<KnoxAction> = all_decisions
        .into_iter()
        .filter(|a| !a.is_current)
        .collect();

    // Open doubts / questions in the same scope.
    let open_doubts: Vec<KnoxAction> = query_scoped_actions(
        db,
        &scopes,
        &input.symbols,
        query_project.as_deref(),
        "doubt",
        true,
    )?
    .into_iter()
    .filter(|a| a.is_resolved != Some(true))
    .collect();

    let mut violations: Vec<String> = Vec::new();
    let mut violation_details: Vec<Violation> = Vec::new();
    if let Some(ref prop) = input.proposed_action {
        for d in &active_decisions {
            for alt in guardrail_patterns(db, d)? {
                if let Some((overlap, matched_span)) =
                    match_rejected_alternative(prop, &alt)
                {
                    let message = format!(
                        "Proposed action matches rejected alternative '{}' in decision `{}` ('{}')",
                        alt, d.id, d.summary
                    );
                    violations.push(message.clone());
                    violation_details.push(Violation {
                        message,
                        alternative: alt,
                        decision_id: d.id.to_string(),
                        decision_summary: d.summary.clone(),
                        overlap,
                        matched_span,
                    });
                }
            }
        }
    }

    // Non-strict mode only fails on high-overlap matches; strict mode fails
    // on any match. Superseded decisions never affect the outcome.
    const HIGH_OVERLAP: f64 = 0.8;
    let strict = input.strict.unwrap_or(false);
    let pass = if strict {
        violations.is_empty()
    } else {
        !violation_details
            .iter()
            .any(|v| v.overlap >= HIGH_OVERLAP)
    };
    let decision_count = active_decisions.len();
    let empty_reason = if decision_count == 0 {
        let any_out_of_scope = input.files.iter().any(|f| {
            let t = f.trim();
            t.starts_with("..") || t.contains("/../") || t.contains("\\..\\")
        });
        if any_out_of_scope {
            Some("out_of_scope".to_string())
        } else {
            Some(
                crate::storage::telemetry::determine_empty_reason(
                    db,
                    query_project.as_deref(),
                    norm_files.first().map(String::as_str),
                )
                .to_string(),
            )
        }
    } else {
        None
    };

    let format_mode = input
        .format
        .as_deref()
        .unwrap_or("markdown")
        .to_ascii_lowercase();

    if format_mode == "json" {
        let mut val = json!({
            "pass": pass,
            "violations_count": violations.len(),
            "violations": violations,
            "violation_details": violation_details,
            "decision_count": decision_count,
            "open_doubts_count": open_doubts.len(),
            "open_doubts": open_doubts,
            "files": norm_files,
            "active_constraints_count": active_decisions.len(),
            "active_constraints": active_decisions,
            "superseded_patterns": stale_decisions
        });
        if let Some(ref r) = empty_reason {
            val["reason"] = json!(r);
        }
        Ok(val)
    } else {
        let md = crate::mcp::format::format_guardrails_markdown_with_reason(
            &norm_files,
            &active_decisions,
            &stale_decisions,
            &violations,
            &open_doubts,
            empty_reason.as_deref(),
        );
        let mut val = json!({
            "pass": pass,
            "violations_count": violations.len(),
            "violations": violations,
            "violation_details": violation_details,
            "decision_count": decision_count,
            "open_doubts_count": open_doubts.len(),
            "open_doubts": open_doubts,
            "files": norm_files,
            "active_constraints_count": active_decisions.len(),
            "active_constraints": active_decisions,
            "superseded_patterns": stale_decisions,
            "markdown": md
        });
        if let Some(ref r) = empty_reason {
            val["reason"] = json!(r);
        }
        Ok(val)
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
