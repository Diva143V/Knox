//! Relevance ranking for compact engineering context.

use chrono::Utc;
use std::collections::HashSet;

use crate::core::types::{KnoxAction, RelationType};

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
    let anchor_ids = actions
        .iter()
        .filter(|action| {
            file_path.is_some_and(|file| {
                action.files.iter().any(|candidate| {
                    candidate == file || candidate.starts_with(file)
                })
            })
                || symbols.iter().any(|symbol| action.symbols.iter().any(|candidate| candidate == symbol))
                || (!query_terms.is_empty() && query_terms.iter().any(|term| {
                    action
                        .summary
                        .to_ascii_lowercase()
                        .split_whitespace()
                        .any(|word| word.contains(term))
                }))
        })
        .map(|action| action.id.clone())
        .collect::<HashSet<_>>();

    let ranking_actions = actions.clone();
    actions.sort_by(|left, right| {
        score(right, file_path, symbols, &query_terms, &anchor_ids, &ranking_actions)
            .total_cmp(&score(left, file_path, symbols, &query_terms, &anchor_ids, &ranking_actions))
            .then_with(|| left.id.cmp(&right.id))
    });
    actions
}

fn score(
    action: &KnoxAction,
    file_path: Option<&str>,
    symbols: &[String],
    query_terms: &[String],
    anchor_ids: &HashSet<crate::core::ids::ActionId>,
    actions: &[KnoxAction],
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
    score += graph_proximity(action, anchor_ids, actions);
    score + (-age_days / 180.0).exp() * 0.1
}

fn graph_proximity(
    action: &KnoxAction,
    anchor_ids: &HashSet<crate::core::ids::ActionId>,
    actions: &[KnoxAction],
) -> f64 {
    let related_ids = action
        .parent_action_id
        .iter()
        .chain(action.supersedes.iter())
        .chain(action.relations.iter().filter_map(|relation| {
            is_causal_relation(&relation.relation_type).then_some(&relation.target_id)
        }));
    if related_ids.clone().any(|id| anchor_ids.contains(id)) {
        return 0.25;
    }

    if actions.iter().any(|candidate| {
        anchor_ids.contains(&candidate.id)
            && (candidate.parent_action_id.as_ref() == Some(&action.id)
                || candidate.supersedes.as_ref() == Some(&action.id)
                || candidate.relations.iter().any(|relation| {
                    relation.target_id == action.id
                        && is_causal_relation(&relation.relation_type)
                }))
    }) {
        return 0.2;
    }
    0.0
}

fn is_causal_relation(relation_type: &RelationType) -> bool {
    !matches!(relation_type, RelationType::RelatesTo)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::ids::ActionId;
    use crate::core::types::{ActionRelation, RelationType};

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

    #[test]
    fn graph_related_action_outranks_unrelated_action() {
        let anchor = action("parser anchor", "src/parser.rs", "write");
        let mut related = action("follow-up implementation", "src/other.rs", "write");
        related.parent_action_id = Some(anchor.id.clone());
        let unrelated = action("unrelated implementation", "src/other.rs", "write");

        let ranked = rank_actions(
            vec![unrelated, related.clone(), anchor.clone()],
            Some("src/parser.rs"),
            &[],
            None,
        );

        assert_eq!(ranked[0].id, anchor.id);
        assert_eq!(ranked[1].id, related.id);
    }

    #[test]
    fn associative_edges_do_not_receive_graph_boost() {
        let anchor = action("parser anchor", "src/parser.rs", "write");
        let mut related = action("associative note", "src/other.rs", "write");
        related.relations.push(ActionRelation {
            target_id: anchor.id.clone(),
            relation_type: RelationType::RelatesTo,
        });
        let mut causal = action("causal implementation", "src/other.rs", "write");
        causal.parent_action_id = Some(anchor.id.clone());

        let ranked = rank_actions(
            vec![related.clone(), causal.clone(), anchor],
            Some("src/parser"),
            &[],
            None,
        );

        assert_eq!(ranked[1].id, causal.id);
        assert_ne!(ranked[0].id, related.id);
    }
}
