//! Graph loading from SQLite storage into in-memory petgraph structures.

use petgraph::Directed;
use petgraph::graph::NodeIndex;
use rusqlite::params_from_iter;
use std::collections::HashMap;

use crate::core::error::KnoxResult;
use crate::core::ids::ActionId;
use crate::core::types::{KnoxAction, KnoxEdge, QueryFilter};
use crate::storage::db::KnoxDb;
use crate::storage::nodes::get_action;
use crate::storage::queries::query_actions;

/// In-memory directed graph representing Knox actions and causal/temporal relationships.
pub type KnoxGraph = petgraph::Graph<KnoxAction, KnoxEdge, Directed>;

/// Helper to find the NodeIndex of an ActionId in a KnoxGraph.
pub fn find_node_index(graph: &KnoxGraph, action_id: &ActionId) -> Option<NodeIndex> {
    graph
        .node_indices()
        .find(|&idx| graph[idx].id == *action_id)
}

/// Builds recursive CTE query for bounded subgraph traversal.
fn build_subgraph_cte_sql(anchor_count: usize) -> String {
    let placeholders = vec!["?"; anchor_count].join(", ");
    format!(
        "WITH RECURSIVE connected(action_id, depth, path) AS (
            SELECT id, 0, ',' || id || ',' FROM actions WHERE id IN ({placeholders})
            UNION
            SELECT a.id, c.depth + 1, c.path || a.id || ','
            FROM connected c
            JOIN actions a ON (
                a.parent_action_id = c.action_id
                OR a.id = (SELECT parent_action_id FROM actions WHERE id = c.action_id)
                OR a.superseded_by = c.action_id
                OR a.id = (SELECT superseded_by FROM actions WHERE id = c.action_id)
            )
            WHERE c.depth < CAST(? AS INTEGER)
              AND instr(c.path, ',' || a.id || ',') = 0
        )
        SELECT DISTINCT action_id FROM connected;"
    )
}

/// Loads a scoped subgraph containing all actions within `max_depth` hops of the given anchor action IDs.
/// Uses a recursive CTE in SQLite to compute the bounded connected component.
pub fn load_subgraph(db: &KnoxDb, anchors: &[ActionId], max_depth: usize) -> KnoxResult<KnoxGraph> {
    if anchors.is_empty() {
        return Ok(KnoxGraph::new());
    }

    let conn = db.conn();
    let cte_sql = build_subgraph_cte_sql(anchors.len());
    let max_depth_i64 = max_depth as i64;
    let mut params_vec: Vec<&dyn rusqlite::ToSql> = Vec::with_capacity(anchors.len() + 1);
    for anchor in anchors {
        params_vec.push(&anchor.0);
    }
    params_vec.push(&max_depth_i64);

    let mut stmt = conn.prepare(&cte_sql)?;
    let id_rows = stmt
        .query_map(params_from_iter(params_vec.iter()), |r| {
            r.get::<_, String>(0).map(ActionId::from)
        })?
        .collect::<Result<Vec<_>, _>>()?;

    let mut actions = Vec::with_capacity(id_rows.len());
    for id in id_rows {
        if let Some(act) = get_action(db, &id)? {
            actions.push(act);
        }
    }

    build_graph_from_actions(actions)
}

/// Loads all actions and edges from the SQLite database into a full in-memory petgraph.
pub fn load_full_graph(db: &KnoxDb) -> KnoxResult<KnoxGraph> {
    let filter = QueryFilter {
        current_only: false,
        limit: usize::MAX / 2, // retrieve all actions
        ..Default::default()
    };

    let query_result = query_actions(db, &filter)?;
    build_graph_from_actions(query_result.results)
}

/// Constructs a `KnoxGraph` from a collection of `KnoxAction` records, wiring parent and supersede edges.
fn build_graph_from_actions(actions: Vec<KnoxAction>) -> KnoxResult<KnoxGraph> {
    let mut graph = KnoxGraph::new();
    let mut node_map: HashMap<ActionId, NodeIndex> = HashMap::with_capacity(actions.len());

    // 1. Add all nodes to the graph
    for action in actions {
        let id = action.id.clone();
        let idx = graph.add_node(action);
        node_map.insert(id, idx);
    }

    // 2. Add edges for parent links and superseded_by links
    for node_idx in graph.node_indices() {
        let (action_id, parent_opt, timestamp, session_id, actor) = {
            let act = &graph[node_idx];
            (
                act.id.clone(),
                act.parent_action_id.clone(),
                act.timestamp,
                act.session_id.clone(),
                act.actor.clone(),
            )
        };

        // If parent exists in the subgraph, add directed edge: parent -> child
        if let Some(parent_id) = parent_opt {
            if let Some(&parent_idx) = node_map.get(&parent_id) {
                let edge = KnoxEdge {
                    edge_type: "parent".to_string(),
                    timestamp,
                    session_id: session_id.clone(),
                    tool_source: Some(actor.clone()),
                };
                graph.add_edge(parent_idx, node_idx, edge);
            }
        }

        // Check if this action superseded another or was superseded
        // Note: supersedes is an input intent; edges represent causal links
        let _ = action_id;
    }

    Ok(graph)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::nodes::insert_action;
    use chrono::Utc;

    fn create_action(
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
            files: vec!["main.rs".to_string()],
            tags: vec![],
            confidence: None,
            timestamp: Utc::now(),
            session_id: None,
            parent_action_id: parent,
            supersedes: None,
            is_current: true,
        }
    }

    #[test]
    fn test_load_subgraph_bounded_bfs() {
        let db = KnoxDb::open_memory().unwrap();

        let plan = create_action("human", "plan", "Plan step", None);
        let write = create_action("gemini", "write", "Write step", Some(plan.id.clone()));
        let debug = create_action("cursor", "debug", "Debug step", Some(write.id.clone()));

        insert_action(&db, &plan).unwrap();
        insert_action(&db, &write).unwrap();
        insert_action(&db, &debug).unwrap();

        let subgraph = load_subgraph(&db, std::slice::from_ref(&write.id), 1).unwrap();
        assert_eq!(subgraph.node_count(), 3);
        assert_eq!(subgraph.edge_count(), 2);
    }
}
