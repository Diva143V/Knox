//! Budget-aware context packing with explicit constraint priority.

use crate::core::budget::estimate_tokens;
use crate::core::types::KnoxAction;
use crate::mcp::format::format_compact_digest;

/// Selects the highest-value actions that fit in a compact context budget.
///
/// Decisions and unresolved doubts are reserved before ordinary actions. The
/// returned actions retain their ranking order so the response remains stable.
pub fn pack_actions(actions: &[KnoxAction], budget: usize) -> (Vec<KnoxAction>, bool) {
    if actions.is_empty() {
        return (Vec::new(), false);
    }

    let mut selected = Vec::new();
    let mut selected_indices = Vec::new();
    let mut used = 0;

    for (index, action) in actions.iter().enumerate() {
        if !is_constraint(action) {
            continue;
        }
        add_if_fits(
            action,
            index,
            budget,
            &mut used,
            &mut selected,
            &mut selected_indices,
        );
    }
    for (index, action) in actions.iter().enumerate() {
        if selected_indices.contains(&index) {
            continue;
        }
        add_if_fits(
            action,
            index,
            budget,
            &mut used,
            &mut selected,
            &mut selected_indices,
        );
    }

    selected.sort_by_key(|action| {
        actions
            .iter()
            .position(|candidate| candidate.id == action.id)
            .unwrap_or(usize::MAX)
    });
    let truncated = selected.len() < actions.len();
    (selected, truncated)
}

fn is_constraint(action: &KnoxAction) -> bool {
    action.action_type == "decide"
        || action.action_type == "decision"
        || ((action.action_type == "doubt" || action.action_type == "question")
            && action.is_resolved != Some(true))
}

fn add_if_fits(
    action: &KnoxAction,
    index: usize,
    budget: usize,
    used: &mut usize,
    selected: &mut Vec<KnoxAction>,
    selected_indices: &mut Vec<usize>,
) {
    let line = format_compact_digest(std::slice::from_ref(action));
    let separator = usize::from(!selected.is_empty());
    let candidate = *used + separator + estimate_tokens(&line);
    if candidate <= budget {
        *used = candidate;
        selected.push(action.clone());
        selected_indices.push(index);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::ids::ActionId;

    fn action(action_type: &str, summary: &str) -> KnoxAction {
        KnoxAction {
            id: ActionId::new(),
            action_type: action_type.to_string(),
            summary: summary.to_string(),
            actor: "agent".to_string(),
            ..Default::default()
        }
    }

    #[test]
    fn reserves_decisions_before_ordinary_actions() {
        let actions = vec![
            action("write", "ordinary implementation detail"),
            action("decide", "critical architecture constraint"),
        ];
        let budget = estimate_tokens(&format_compact_digest(&actions[1..2]));
        let (packed, truncated) = pack_actions(&actions, budget);

        assert!(truncated);
        assert_eq!(packed.len(), 1);
        assert_eq!(packed[0].action_type, "decide");
    }

    #[test]
    fn reserves_legacy_decision_actions_before_ordinary_actions() {
        let actions = vec![
            action("write", "ordinary implementation detail"),
            action("decision", "legacy architecture constraint"),
        ];
        let budget = estimate_tokens(&format_compact_digest(&actions[1..2]));
        let (packed, truncated) = pack_actions(&actions, budget);

        assert!(truncated);
        assert_eq!(packed.len(), 1);
        assert_eq!(packed[0].action_type, "decision");
    }

    #[test]
    fn unresolved_doubts_are_reserved_but_resolved_doubts_are_not() {
        let actions = vec![
            action(
                "write",
                "ordinary implementation detail that should not displace constraints",
            ),
            action("doubt", "open constraint"),
            KnoxAction {
                is_resolved: Some(true),
                ..action("doubt", "resolved history")
            },
        ];
        let budget = estimate_tokens(&format_compact_digest(&actions[1..2]));
        let (packed, _) = pack_actions(&actions, budget);

        assert_eq!(packed.len(), 1);
        assert_eq!(packed[0].summary, "open constraint");
    }
}
