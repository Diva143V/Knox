//! Relevance ranking for compact engineering context.

use chrono::Utc;

use crate::core::types::KnoxAction;

/// Ranks actions for a context request using stable, explainable signals.
pub fn rank_actions(
    mut actions: Vec<KnoxAction>,
    file_path: Option<&str>,
    symbols: &[String],
    query: Option<&str>,
) -> Vec<KnoxAction> {
    let query_terms = query
        .unwrap_or_default()
        .to_ascii_lowercase()
        .split_whitespace()
        .map(str::to_string)
        .collect::<Vec<_>>();

    actions.sort_by(|left, right| {
        score(right, file_path, symbols, &query_terms)
            .total_cmp(&score(left, file_path, symbols, &query_terms))
            .then_with(|| left.id.cmp(&right.id))
    });
    actions
}

fn score(
    action: &KnoxAction,
    file_path: Option<&str>,
    symbols: &[String],
    query_terms: &[String],
) -> f64 {
    let mut score = 0.0;
    if let Some(file) = file_path {
        if action.files.iter().any(|candidate| candidate == file) {
            score += 1.0;
        } else if action
            .files
            .iter()
            .any(|candidate| candidate.starts_with(file))
        {
            score += 0.4;
        }
    }
    if symbols
        .iter()
        .any(|symbol| action.symbols.iter().any(|candidate| candidate == symbol))
    {
        score += 0.8;
    }
    let summary = action.summary.to_ascii_lowercase();
    score += query_terms
        .iter()
        .filter(|term| summary.split_whitespace().any(|word| word.contains(term.as_str())))
        .count() as f64
        * 0.15;
    score += match action.action_type.as_str() {
        "decide" => 0.2,
        "doubt" | "question" if action.is_resolved != Some(true) => 0.3,
        _ => 0.0,
    };
    score += action.confidence.unwrap_or(0.0) * 0.1;
    let age_days = (Utc::now() - action.timestamp).num_days().max(0) as f64;
    score + (-age_days / 180.0).exp() * 0.1
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::ids::ActionId;

    fn action(summary: &str, file: &str, action_type: &str) -> KnoxAction {
        KnoxAction {
            id: ActionId::new(),
            actor: "agent".to_string(),
            action_type: action_type.to_string(),
            summary: summary.to_string(),
            files: vec![file.to_string()],
            ..Default::default()
        }
    }

    #[test]
    fn exact_scope_and_decisions_rank_first() {
        let ranked = rank_actions(
            vec![
                action("write parser", "src/other.rs", "write"),
                action("decide parser strategy", "src/parser.rs", "decide"),
            ],
            Some("src/parser.rs"),
            &[],
            Some("parser"),
        );
        assert_eq!(ranked[0].action_type, "decide");
    }
}
