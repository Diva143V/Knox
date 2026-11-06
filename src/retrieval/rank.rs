//! Relevance ranking for compact engineering context.

use chrono::Utc;
use std::collections::{HashMap, HashSet};

use crate::core::ids::ActionId;
use crate::core::types::{KnoxAction, RelationType};
use crate::models::router::ModelRouter;
use crate::storage::db::KnoxDb;
use crate::storage::embeddings::{cosine_similarity, get_embeddings_batch};

/// Hybrid ranking combining lexical/graph signals with semantic embedding similarity (Workstream H2).
///
/// When `router` is None or models are Off, guarantees 100% byte-identical output to `rank_actions`.
pub fn rank_actions_hybrid(
    mut actions: Vec<KnoxAction>,
    file_path: Option<&str>,
    symbols: &[String],
    query: Option<&str>,
    router: Option<&ModelRouter>,
    db: Option<&KnoxDb>,
) -> Vec<KnoxAction> {
    let query_str = match query {
        Some(q) if !q.trim().is_empty() => q.trim(),
        _ => return rank_actions(actions, file_path, symbols, query),
    };

    let router = match router {
        Some(r) if r.is_enabled() => r,
        _ => return rank_actions(actions, file_path, symbols, Some(query_str)),
    };

    let query_vector = match router.embed(query_str) {
        Ok(v) => v,
        Err(_) => return rank_actions(actions, file_path, symbols, Some(query_str)),
    };

    let query_terms = query_str
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

    let action_ids: Vec<_> = actions.iter().map(|a| a.id.clone()).collect();
    let mut embeddings: HashMap<crate::core::ids::ActionId, Vec<f32>> = HashMap::new();

    if let Some(db) = db {
        if let Ok(batch) = get_embeddings_batch(db, &action_ids) {
            embeddings = batch;
        }
    }

    // Embed missing actions on the fly and cache if db is available
    let mut missing_embeddings: Vec<(ActionId, Vec<f32>)> = Vec::new();
    for action in &actions {
        if !embeddings.contains_key(&action.id) {
            if let Ok(vec) = router.embed(&action.summary) {
                embeddings.insert(action.id.clone(), vec.clone());
                missing_embeddings.push((action.id.clone(), vec));
            }
        }
    }

    if let Some(db) = db {
        if !missing_embeddings.is_empty() {
            let _ = db.with_tx(|conn| {
                for (id, vec) in &missing_embeddings {
                    crate::storage::embeddings::insert_embedding_conn(
                        conn,
                        id,
                        vec,
                        &router.profile.embedding_model,
                    )?;
                }
                Ok(())
            });
        }
    }

    let ranking_actions = actions.clone();
    actions.sort_by(|left, right| {
        let left_base = score(left, file_path, symbols, &query_terms, &anchor_ids, &ranking_actions);
        let right_base = score(right, file_path, symbols, &query_terms, &anchor_ids, &ranking_actions);

        let left_sim = embeddings
            .get(&left.id)
            .map(|ev| cosine_similarity(&query_vector, ev) as f64)
            .unwrap_or(0.0);

        let right_sim = embeddings
            .get(&right.id)
            .map(|ev| cosine_similarity(&query_vector, ev) as f64)
            .unwrap_or(0.0);

        let left_total = left_base + (left_sim * 0.4);
        let right_total = right_base + (right_sim * 0.4);

        right_total
            .total_cmp(&left_total)
            .then_with(|| right.timestamp.cmp(&left.timestamp))
            .then_with(|| right.id.cmp(&left.id))
    });

    actions
}

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
            .then_with(|| right.timestamp.cmp(&left.timestamp))
            .then_with(|| right.id.cmp(&left.id))
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
