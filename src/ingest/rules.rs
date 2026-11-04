//! Ingestion rule engine implementing the R01–R10 rule table from the plan.
//! Each transcript step is classified against an ordered rule set; the first
//! matching rule determines the action type, confidence, and whether the
//! entry is excluded from ingestion.

use crate::ingest::summary::HumanInputKind;

/// A single ingestion rule from the R01–R10 table.
#[derive(Clone, Debug)]
pub struct IngestRule {
    /// Rule identifier (e.g. "R01").
    pub id: &'static str,
    /// Human-readable name.
    pub name: &'static str,
    /// Condition description from the plan.
    pub condition: &'static str,
    /// Outcome action type produced when matched.
    pub outcome: &'static str,
    /// Confidence score (0.0–1.0).
    pub confidence: f64,
    /// When true, matching entries are excluded from ingestion entirely.
    pub excluded: bool,
}

/// The result of classifying a transcript step against the rule table.
#[derive(Clone, Debug)]
pub struct RuleMatch {
    /// The matched rule identifier.
    pub rule_id: String,
    /// The rule's confidence score.
    pub confidence: f64,
    /// Human-readable reason for the classification.
    pub reason: String,
    /// The action type to produce, if any.
    pub action_type: Option<String>,
    /// Whether the entry should be excluded from ingestion.
    pub excluded: bool,
}

/// The complete R01–R10 rule table, evaluated in order.
fn rule_table() -> &'static [IngestRule] {
    &[
        IngestRule {
            id: "R01",
            name: "tool_write_or_edit",
            condition: "Tool call writes or edits a file",
            outcome: "action",
            confidence: 0.90,
            excluded: false,
        },
        IngestRule {
            id: "R02",
            name: "test_build_pass_after_edit",
            condition: "Test or build fails, then passes after an edit",
            outcome: "outcome",
            confidence: 0.80,
            excluded: false,
        },
        IngestRule {
            id: "R03",
            name: "read_only_tool_call",
            condition: "Shell or tool call only reads or lists",
            outcome: "excluded",
            confidence: 0.0,
            excluded: true,
        },
        IngestRule {
            id: "R04",
            name: "user_imperative_plan",
            condition: "User imperative with a task verb and an object",
            outcome: "plan",
            confidence: 0.60,
            excluded: false,
        },
        IngestRule {
            id: "R05",
            name: "explicit_decision",
            condition: "Explicit decision phrasing",
            outcome: "decision",
            confidence: 0.50,
            excluded: false,
        },
        IngestRule {
            id: "R06",
            name: "unanswered_question",
            condition: "Question unanswered within 6 turns or at segment end",
            outcome: "doubt",
            confidence: 0.70,
            excluded: false,
        },
        IngestRule {
            id: "R07",
            name: "chatter",
            condition: "Acknowledgement, greeting, <15 chars, no refs",
            outcome: "excluded",
            confidence: 0.0,
            excluded: true,
        },
        IngestRule {
            id: "R08",
            name: "coalesce_same_file_edits",
            condition: "Same-file edits in consecutive turns",
            outcome: "action (merged)",
            confidence: 0.85,
            excluded: false,
        },
        IngestRule {
            id: "R09",
            name: "restated_entry",
            condition: "Assistant restates an earlier entry",
            outcome: "merged by dedupe",
            confidence: 1.0,
            excluded: false,
        },
        IngestRule {
            id: "R10",
            name: "error_with_exit_code",
            condition: "Error output with exit code or stack trace",
            outcome: "debug action",
            confidence: 0.60,
            excluded: false,
        },
    ]
}

/// Returns all registered ingestion rules.
pub fn all_rules() -> Vec<IngestRule> {
    rule_table().to_vec()
}

/// Returns a rule by its identifier.
pub fn find_rule(rule_id: &str) -> Option<IngestRule> {
    rule_table().iter().find(|r| r.id == rule_id).cloned()
}

/// Classifies a transcript step against the ordered rule table.
///
/// Returns the first matching rule, or None if no rule applies.
pub fn classify_step(
    step_type: &str,
    content: &str,
    tool_calls: &[(&str, &serde_json::Value)],
    human_kind: HumanInputKind,
    has_referenced_files: bool,
    is_error_output: bool,
    consecutive_same_file_edit: bool,
) -> Option<RuleMatch> {
    // R03: read-only tool calls are excluded.
    if step_type == "PLANNER_RESPONSE" {
        for (name, _args) in tool_calls {
            let lower = name.to_ascii_lowercase();
            if lower.contains("read")
                || lower.contains("list")
                || lower == "glob"
                || lower == "grep"
                || lower == "ls"
            {
                return Some(RuleMatch {
                    rule_id: "R03".to_string(),
                    confidence: 0.0,
                    reason: "read-only tool call; excluded from ingestion".to_string(),
                    action_type: None,
                    excluded: true,
                });
            }
        }
    }

    // R07: chatter — short acknowledgements/greetings with no refs.
    if step_type == "USER_INPUT" {
        let trimmed = content.trim();
        if trimmed.len() < 15 && !has_referenced_files {
            let lower = trimmed.to_ascii_lowercase();
            let is_chatter = matches!(
                lower.as_str(),
                "ok" | "okay" | "thanks" | "thank you"
                    | "great" | "good" | "nice" | "cool"
                    | "yes" | "no" | "sure" | "ack"
                    | "hello" | "hi" | "hey"
            ) || human_kind == HumanInputKind::Dialogue;
            if is_chatter {
                return Some(RuleMatch {
                    rule_id: "R07".to_string(),
                    confidence: 0.0,
                    reason: "acknowledgement or greeting; excluded as chatter".to_string(),
                    action_type: None,
                    excluded: true,
                });
            }
        }
    }

    // R10: error output with exit code or stack → debug action.
    if is_error_output {
        return Some(RuleMatch {
            rule_id: "R10".to_string(),
            confidence: 0.60,
            reason: "error output with exit code or stack trace".to_string(),
            action_type: Some("debug".to_string()),
            excluded: false,
        });
    }

    // R01: tool call writes or edits a file → action.
    if step_type == "PLANNER_RESPONSE" && !tool_calls.is_empty() {
        let has_write = tool_calls.iter().any(|(name, _)| {
            let lower = name.to_ascii_lowercase();
            lower.contains("write")
                || lower.contains("edit")
                || lower == "create_file"
                || lower == "modify_file"
        });
        if has_write {
            return Some(RuleMatch {
                rule_id: "R01".to_string(),
                confidence: 0.90,
                reason: "tool call writes or edits a file".to_string(),
                action_type: Some("write".to_string()),
                excluded: false,
            });
        }
    }

    // R04: user imperative with task verb → plan.
    if step_type == "USER_INPUT" && human_kind == HumanInputKind::Plan {
        return Some(RuleMatch {
            rule_id: "R04".to_string(),
            confidence: 0.60,
            reason: "user imperative with task verb and object".to_string(),
            action_type: Some("plan".to_string()),
            excluded: false,
        });
    }

    // R05: explicit decision phrasing → decision.
    if step_type == "PLANNER_RESPONSE" && tool_calls.is_empty() && !content.is_empty() {
        let lower = content.to_ascii_lowercase();
        let is_decision = lower.contains("decide")
            || lower.contains("decision")
            || lower.contains("choose")
            || lower.contains("will use")
            || lower.contains("going with")
            || lower.contains("prefer");
        if is_decision {
            let confidence = if has_referenced_files { 0.80 } else { 0.50 };
            return Some(RuleMatch {
                rule_id: "R05".to_string(),
                confidence,
                reason: format!(
                    "explicit decision phrasing (confidence boosted by file refs: {})",
                    has_referenced_files
                ),
                action_type: Some("decide".to_string()),
                excluded: false,
            });
        }
    }

    // R06: question → doubt only if unanswered within 6 turns or at segment end.
    if step_type == "USER_INPUT" && human_kind == HumanInputKind::Doubt {
        return Some(RuleMatch {
            rule_id: "R06".to_string(),
            confidence: 0.70,
            reason: "question classified as doubt".to_string(),
            action_type: Some("doubt".to_string()),
            excluded: false,
        });
    }

    // R08: same-file edits in consecutive turns → coalesced.
    if consecutive_same_file_edit {
        return Some(RuleMatch {
            rule_id: "R08".to_string(),
            confidence: 0.85,
            reason: "same-file edits coalesced into one action".to_string(),
            action_type: Some("write".to_string()),
            excluded: false,
        });
    }

    None
}

/// Returns a human-readable summary of the full rule table for --explain output.
pub fn rule_table_summary() -> String {
    let mut lines: Vec<String> = rule_table()
        .iter()
        .map(|r| {
            format!(
                "  {} {:<28} confidence={:.2}  {}",
                r.id, r.name, r.confidence, r.condition
            )
        })
        .collect();
    lines.insert(0, "Ingestion rules (R01–R10):".to_string());
    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_rule_table_has_ten_rules() {
        assert_eq!(all_rules().len(), 10);
        assert_eq!(all_rules()[0].id, "R01");
        assert_eq!(all_rules()[9].id, "R10");
    }

    #[test]
    fn test_find_rule_by_id() {
        assert!(find_rule("R01").is_some());
        assert_eq!(find_rule("R01").unwrap().confidence, 0.90);
        assert!(find_rule("R03").unwrap().excluded);
        assert!(find_rule("R07").unwrap().excluded);
        assert!(find_rule("R99").is_none());
    }

    #[test]
    fn test_classify_read_only_tool_excluded() {
        let result = classify_step(
            "PLANNER_RESPONSE",
            "",
            &[("read_file", &json!({"path": "src/lib.rs"}))],
            HumanInputKind::Plan,
            false,
            false,
            false,
        );
        assert!(result.is_some());
        let m = result.unwrap();
        assert_eq!(m.rule_id, "R03");
        assert!(m.excluded);
    }

    #[test]
    fn test_classify_chatter_excluded() {
        let result = classify_step(
            "USER_INPUT",
            "ok",
            &[],
            HumanInputKind::Dialogue,
            false,
            false,
            false,
        );
        assert!(result.is_some());
        let m = result.unwrap();
        assert_eq!(m.rule_id, "R07");
        assert!(m.excluded);
    }

    #[test]
    fn test_classify_write_tool_r01() {
        let result = classify_step(
            "PLANNER_RESPONSE",
            "",
            &[("write_file", &json!({"path": "src/main.rs"}))],
            HumanInputKind::Plan,
            true,
            false,
            false,
        );
        assert!(result.is_some());
        let m = result.unwrap();
        assert_eq!(m.rule_id, "R01");
        assert_eq!(m.confidence, 0.90);
        assert_eq!(m.action_type, Some("write".to_string()));
        assert!(!m.excluded);
    }

    #[test]
    fn test_classify_imperative_plan_r04() {
        let result = classify_step(
            "USER_INPUT",
            "implement the parser",
            &[],
            HumanInputKind::Plan,
            false,
            false,
            false,
        );
        assert!(result.is_some());
        let m = result.unwrap();
        assert_eq!(m.rule_id, "R04");
        assert_eq!(m.confidence, 0.60);
        assert_eq!(m.action_type, Some("plan".to_string()));
    }

    #[test]
    fn test_classify_decision_r05_boosted_with_files() {
        let result = classify_step(
            "PLANNER_RESPONSE",
            "We will use a layered architecture.",
            &[],
            HumanInputKind::Plan,
            true,
            false,
            false,
        );
        assert!(result.is_some());
        let m = result.unwrap();
        assert_eq!(m.rule_id, "R05");
        assert_eq!(m.confidence, 0.80);
        assert_eq!(m.action_type, Some("decide".to_string()));
    }

    #[test]
    fn test_classify_decision_r05_not_boosted_without_files() {
        let result = classify_step(
            "PLANNER_RESPONSE",
            "We will use a layered architecture.",
            &[],
            HumanInputKind::Plan,
            false,
            false,
            false,
        );
        assert!(result.is_some());
        let m = result.unwrap();
        assert_eq!(m.confidence, 0.50);
    }

    #[test]
    fn test_classify_error_output_r10() {
        let result = classify_step(
            "GENERIC",
            "Command exited with code 1.\nError: stack overflow",
            &[],
            HumanInputKind::Plan,
            false,
            true,
            false,
        );
        assert!(result.is_some());
        let m = result.unwrap();
        assert_eq!(m.rule_id, "R10");
        assert_eq!(m.confidence, 0.60);
        assert_eq!(m.action_type, Some("debug".to_string()));
    }

    #[test]
    fn test_classify_question_r06() {
        let result = classify_step(
            "USER_INPUT",
            "why does the parser fail?",
            &[],
            HumanInputKind::Doubt,
            false,
            false,
            false,
        );
        assert!(result.is_some());
        let m = result.unwrap();
        assert_eq!(m.rule_id, "R06");
        assert_eq!(m.action_type, Some("doubt".to_string()));
    }

    #[test]
    fn test_classify_no_match() {
        let result = classify_step(
            "PLANNER_RESPONSE",
            "Some generic reasoning text.",
            &[("think", &json!({}))],
            HumanInputKind::Plan,
            false,
            false,
            false,
        );
        assert!(result.is_none());
    }
}
