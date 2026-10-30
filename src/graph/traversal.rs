//! Traversal algorithms for navigating the Knox causal knowledge graph.

use petgraph::Direction;
use petgraph::visit::Bfs;
use std::collections::{HashMap, HashSet, VecDeque};

use super::loader::{KnoxGraph, find_node_index};
use crate::core::ids::ActionId;
use crate::core::types::KnoxAction;

/// Walks the direct causal parent links from `action_id` back to the root action.
/// Returns a sequence starting with the queried action and ending with the root ancestor:
/// `[queried_action, parent, grandparent, ..., root]`.
pub fn causal_chain(graph: &KnoxGraph, action_id: &ActionId) -> Vec<KnoxAction> {
    let Some(start_idx) = find_node_index(graph, action_id) else {
        return Vec::new();
    };

    let mut chain = Vec::new();
    let mut current_idx = Some(start_idx);
    let mut visited = HashSet::new();

    while let Some(idx) = current_idx {
        if !visited.insert(idx) {
            break; // cycle protection
        }

        chain.push(graph[idx].clone());

        // Find parent action: look for incoming edge with type "parent" or match parent_action_id
        let parent_id_opt = graph[idx].parent_action_id.clone();
        current_idx = if let Some(ref pid) = parent_id_opt {
            find_node_index(graph, pid)
        } else {
            // Alternatively check incoming edges
            graph.neighbors_directed(idx, Direction::Incoming).next()
        };
    }

    chain
}

/// Retrieves all causal descendants (actions caused directly or indirectly by `action_id`).
/// Uses breadth-first search traversing outgoing edges.
pub fn descendants(graph: &KnoxGraph, action_id: &ActionId) -> Vec<KnoxAction> {
    let Some(start_idx) = find_node_index(graph, action_id) else {
        return Vec::new();
    };

    let mut result = Vec::new();
    let mut bfs = Bfs::new(graph, start_idx);

    // Skip the root starting node
    let _ = bfs.next(graph);

    while let Some(next_idx) = bfs.next(graph) {
        result.push(graph[next_idx].clone());
    }

    result
}

/// Retrieves all causal ancestors (actions that led to `action_id`).
/// Uses breadth-first search traversing incoming edges.
pub fn ancestors(graph: &KnoxGraph, action_id: &ActionId) -> Vec<KnoxAction> {
    let Some(start_idx) = find_node_index(graph, action_id) else {
        return Vec::new();
    };

    let mut result = Vec::new();
    let mut visited = HashSet::new();
    let mut queue = VecDeque::new();

    visited.insert(start_idx);
    queue.push_back(start_idx);

    while let Some(curr) = queue.pop_front() {
        for parent_idx in graph.neighbors_directed(curr, Direction::Incoming) {
            if visited.insert(parent_idx) {
                result.push(graph[parent_idx].clone());
                queue.push_back(parent_idx);
            }
        }
    }

    result
}

/// Finds the shortest directed path between two actions in the graph.
/// Returns `Some([from, ..., to])` if connected, or `None` if unreachable.
pub fn path_between(graph: &KnoxGraph, from: &ActionId, to: &ActionId) -> Option<Vec<KnoxAction>> {
    let from_idx = find_node_index(graph, from)?;
    let to_idx = find_node_index(graph, to)?;

    if from_idx == to_idx {
        return Some(vec![graph[from_idx].clone()]);
    }

    let mut visited = HashSet::new();
    let mut queue = VecDeque::new();
    let mut prev_map: HashMap<petgraph::graph::NodeIndex, petgraph::graph::NodeIndex> =
        HashMap::new();

    visited.insert(from_idx);
    queue.push_back(from_idx);

    let mut found = false;
    while let Some(curr) = queue.pop_front() {
        if curr == to_idx {
            found = true;
            break;
        }

        for neighbor in graph.neighbors_directed(curr, Direction::Outgoing) {
            if visited.insert(neighbor) {
                prev_map.insert(neighbor, curr);
                queue.push_back(neighbor);
            }
        }
    }

    if !found {
        return None;
    }

    // Reconstruct path
    let mut path = Vec::new();
    let mut curr = to_idx;
    path.push(graph[curr].clone());

    while curr != from_idx {
        curr = *prev_map.get(&curr)?;
        path.push(graph[curr].clone());
    }

    path.reverse();
    Some(path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::types::KnoxEdge;
    use chrono::Utc;

    fn mock_action(
        actor: &str,
        action_type: &str,
        summary: &str,
        parent: Option<ActionId>,
    ) -> KnoxAction {
        KnoxAction {
            id: ActionId::new(),
            actor: actor.to_string(),
            action_type: action_type.to_string(),
            summary: summary.to_string(),
            files: vec![],
            tags: vec![],
            confidence: None,
            timestamp: Utc::now(),
            session_id: None,
            parent_action_id: parent,
            supersedes: None,
            is_current: true,
        }
    }

    fn setup_test_graph() -> (KnoxGraph, KnoxAction, KnoxAction, KnoxAction) {
        let mut graph = KnoxGraph::new();
        let plan = mock_action("human", "plan", "Plan", None);
        let write = mock_action("gemini", "write", "Write", Some(plan.id.clone()));
        let debug = mock_action("cursor", "debug", "Debug", Some(write.id.clone()));

        let n1 = graph.add_node(plan.clone());
        let n2 = graph.add_node(write.clone());
        let n3 = graph.add_node(debug.clone());

        let edge_mock = KnoxEdge {
            edge_type: "parent".to_string(),
            timestamp: Utc::now(),
            session_id: None,
            tool_source: None,
        };

        graph.add_edge(n1, n2, edge_mock.clone());
        graph.add_edge(n2, n3, edge_mock);
        (graph, plan, write, debug)
    }

    #[test]
    fn test_traversal_causal_chain_and_descendants() {
        let (graph, plan, write, debug) = setup_test_graph();

        let chain = causal_chain(&graph, &debug.id);
        assert_eq!(chain.len(), 3);
        assert_eq!(chain[0].id, debug.id);
        assert_eq!(chain[1].id, write.id);
        assert_eq!(chain[2].id, plan.id);

        let desc = descendants(&graph, &plan.id);
        assert_eq!(desc.len(), 2);
        assert_eq!(desc[0].id, write.id);
        assert_eq!(desc[1].id, debug.id);

        let path = path_between(&graph, &plan.id, &debug.id).unwrap();
        assert_eq!(path.len(), 3);
        assert_eq!(path[0].id, plan.id);
        assert_eq!(path[1].id, write.id);
        assert_eq!(path[2].id, debug.id);
    }
}
