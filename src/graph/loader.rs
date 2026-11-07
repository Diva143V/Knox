//! Graph loading from SQLite storage into in-memory petgraph structures.

use petgraph::Directed;
use petgraph::graph::NodeIndex;
use rusqlite::params_from_iter;
use std::collections::HashMap;

use crate::core::error::KnoxResult;
use crate::core::ids::ActionId;
use crate::core::types::{KnoxAction, KnoxEdge, QueryFilter};
use crate::storage::db::KnoxDb;
use crate::storage::queries::query_actions;

/// In-memory directed graph representing Knox actions and causal/temporal relationships.
pub type KnoxGraph = petgraph::Graph<KnoxAction, KnoxEdge, Directed>;

/// Default maximum number of component nodes to explore in a bounded subgraph traversal.
pub const DEFAULT_MAX_NODES: usize = 200;

/// Bounded subgraph traversal result with cutoff metadata.
#[derive(Clone, Debug)]
pub struct BoundedSubgraph {
    /// In-memory directed graph of the component up to `max_nodes`.
    pub graph: KnoxGraph,
    /// Whether the connected component traversal was truncated at `max_nodes`.
    pub truncated: bool,
    /// Boundary action IDs at the cutoff that were not expanded further.
    pub frontier_ids: Vec<ActionId>,
}

/// Direction in which a subgraph should be expanded from its anchors.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SubgraphDirection {
    /// Follow causes backwards: parents, superseded predecessors and incoming
    /// semantic relations.
    Ancestors,
    /// Follow effects forwards: children, successors and outgoing semantic
    /// relations.
    Descendants,
    /// Follow both directions (bounded connected component).
    Both,
}

/// Helper to find the NodeIndex of an ActionId in a KnoxGraph.
///
/// Prefer [`build_node_index`] when several lookups are needed; this performs
/// a linear scan.
pub fn find_node_index(graph: &KnoxGraph, action_id: &ActionId) -> Option<NodeIndex> {
    graph
        .node_indices()
        .find(|&idx| graph[idx].id == *action_id)
}

/// Builds an `ActionId -> NodeIndex` map in a single pass so traversals do not
/// rescan every node for each hop.
pub fn build_node_index(graph: &KnoxGraph) -> HashMap<ActionId, NodeIndex> {
    graph
        .node_indices()
        .map(|idx| (graph[idx].id.clone(), idx))
        .collect()
}

/// SQL filter restricting recursive subgraph expansion strictly to causal edges.
/// Associative links (`relates_to`) and conflict markers (`contradicts`) are non-causal
/// and must never pollute the bounded node budget during traversal.
pub const CAUSAL_EDGE_SQL_FILTER: &str =
    "e.edge_type IN ('caused_by', 'parent', 'supersedes', 'implements', 'depends_on', 'verifies', 'reverts', 'resolves')";

/// Builds the recursive CTE for a bounded, *directional* subgraph traversal.
fn build_subgraph_cte_sql(anchor_count: usize, direction: SubgraphDirection) -> String {
    let placeholders = vec!["?"; anchor_count].join(", ");

    let recursive_select = match direction {
        SubgraphDirection::Ancestors => {
            format!(
                "SELECT a.id, c.depth + 1, c.path || a.id || ','
            FROM connected c
            JOIN edges e ON e.from_action_id = c.action_id
            JOIN actions a ON a.id = e.to_id
            WHERE c.depth < CAST(? AS INTEGER)
              AND instr(c.path, ',' || a.id || ',') = 0
              AND {CAUSAL_EDGE_SQL_FILTER}"
            )
        }
        SubgraphDirection::Descendants => {
            format!(
                "SELECT a.id, c.depth + 1, c.path || a.id || ','
            FROM connected c
            JOIN edges e ON e.to_id = c.action_id
            JOIN actions a ON a.id = e.from_action_id
            WHERE c.depth < CAST(? AS INTEGER)
              AND instr(c.path, ',' || a.id || ',') = 0
              AND {CAUSAL_EDGE_SQL_FILTER}"
            )
        }
        SubgraphDirection::Both => {
            format!(
                "SELECT a.id, c.depth + 1, c.path || a.id || ','
            FROM connected c
            JOIN edges e ON (e.from_action_id = c.action_id OR e.to_id = c.action_id)
            JOIN actions a ON a.id = (CASE WHEN e.from_action_id = c.action_id THEN e.to_id ELSE e.from_action_id END)
            WHERE c.depth < CAST(? AS INTEGER)
              AND instr(c.path, ',' || a.id || ',') = 0
              AND {CAUSAL_EDGE_SQL_FILTER}"
            )
        }
    };

    format!(
        "WITH RECURSIVE connected(action_id, depth, path) AS (
            SELECT id, 0, ',' || id || ',' FROM actions WHERE id IN ({placeholders})
            UNION
            {recursive_select}
        )
        SELECT action_id, MIN(depth) as min_depth FROM connected
        GROUP BY action_id
        ORDER BY min_depth ASC
        LIMIT ?;"
    )
}

/// Loads a scoped subgraph containing all actions within `max_depth` hops of the given anchor action IDs.
/// Uses a recursive CTE in SQLite to compute the bounded connected component up to `DEFAULT_MAX_NODES`.
pub fn load_subgraph(db: &KnoxDb, anchors: &[ActionId], max_depth: usize) -> KnoxResult<KnoxGraph> {
    load_subgraph_directional(db, anchors, max_depth, SubgraphDirection::Both)
}

/// Loads a scoped subgraph expanded only in the requested direction up to `DEFAULT_MAX_NODES`.
pub fn load_subgraph_directional(
    db: &KnoxDb,
    anchors: &[ActionId],
    max_depth: usize,
    direction: SubgraphDirection,
) -> KnoxResult<KnoxGraph> {
    load_subgraph_bounded(db, anchors, max_depth, DEFAULT_MAX_NODES, direction)
        .map(|bounded| bounded.graph)
}

/// Loads a scoped subgraph bounded by both `max_depth` and `max_nodes`.
pub fn load_subgraph_bounded(
    db: &KnoxDb,
    anchors: &[ActionId],
    max_depth: usize,
    max_nodes: usize,
    direction: SubgraphDirection,
) -> KnoxResult<BoundedSubgraph> {
    if anchors.is_empty() {
        return Ok(BoundedSubgraph {
            graph: KnoxGraph::new(),
            truncated: false,
            frontier_ids: Vec::new(),
        });
    }

    let conn = db.conn();
    let cte_sql = build_subgraph_cte_sql(anchors.len(), direction);
    let max_depth_i64 = max_depth as i64;
    let limit_i64 = (max_nodes + 1) as i64;
    let mut params_vec: Vec<&dyn rusqlite::ToSql> = Vec::with_capacity(anchors.len() + 2);
    for anchor in anchors {
        params_vec.push(&anchor.0);
    }
    params_vec.push(&max_depth_i64);
    params_vec.push(&limit_i64);

    let mut stmt = conn.prepare(&cte_sql)?;
    let id_rows = stmt
        .query_map(params_from_iter(params_vec.iter()), |r| {
            r.get::<_, String>(0).map(ActionId::from)
        })?
        .collect::<Result<Vec<_>, _>>()?;

    let (selected_ids, frontier_ids, truncated) = if id_rows.len() > max_nodes {
        let (selected, frontier) = id_rows.split_at(max_nodes);
        (selected.to_vec(), frontier.to_vec(), true)
    } else {
        (id_rows, Vec::new(), false)
    };

    let actions = crate::storage::queries::get_actions_by_ids(db, &selected_ids)?;
    let graph = build_graph_from_actions(db, actions)?;

    Ok(BoundedSubgraph {
        graph,
        truncated,
        frontier_ids,
    })
}

/// Loads all actions and edges from the SQLite database into a full in-memory petgraph.
pub fn load_full_graph(db: &KnoxDb) -> KnoxResult<KnoxGraph> {
    let filter = QueryFilter {
        current_only: false,
        limit: usize::MAX / 2, // retrieve all actions
        ..Default::default()
    };

    let query_result = query_actions(db, &filter)?;
    build_graph_from_actions(db, query_result.results)
}

/// Constructs a `KnoxGraph` from a collection of `KnoxAction` records, reading edges from the `edges` table.
fn build_graph_from_actions(db: &KnoxDb, actions: Vec<KnoxAction>) -> KnoxResult<KnoxGraph> {
    let mut graph = KnoxGraph::new();
    let mut node_map: HashMap<ActionId, NodeIndex> = HashMap::with_capacity(actions.len());

    // 1. Add all nodes to the graph
    for action in actions {
        let id = action.id.clone();
        let idx = graph.add_node(action);
        node_map.insert(id, idx);
    }

    if node_map.is_empty() {
        return Ok(graph);
    }

    // 2. Add edges directly from the `edges` table
    let action_ids: Vec<ActionId> = node_map.keys().cloned().collect();
    let edges = crate::storage::edges::get_all_edges_for_actions(db, &action_ids)?;

    for edge in edges {
        let target_action_id = ActionId::from(edge.to_id);
        if let (Some(&cause_idx), Some(&effect_idx)) =
            (node_map.get(&target_action_id), node_map.get(&edge.from_action_id))
        {
            let knox_edge = KnoxEdge {
                edge_type: edge.edge_type,
                timestamp: edge.created_at,
                session_id: graph[effect_idx].session_id.clone(),
                tool_source: edge
                    .provenance
                    .or_else(|| Some(graph[effect_idx].actor.clone())),
            };
            graph.add_edge(cause_idx, effect_idx, knox_edge);
        }
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
            ..Default::default()
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

    #[test]
    fn test_loader_petgraph_edge_direction_and_traversal_symmetry() {
        let db = KnoxDb::open_memory().unwrap();

        let plan = create_action("human", "plan", "Plan step", None);
        let write = create_action("gemini", "write", "Write step", Some(plan.id.clone()));
        insert_action(&db, &plan).unwrap();
        insert_action(&db, &write).unwrap();

        let subgraph = load_subgraph(&db, &[plan.id.clone(), write.id.clone()], 2).unwrap();
        let node_map = build_node_index(&subgraph);
        let plan_idx = *node_map.get(&plan.id).unwrap();
        let write_idx = *node_map.get(&write.id).unwrap();

        // 1. Explicitly assert the petgraph edge direction: cause (plan) -> effect (write)
        assert!(
            subgraph.find_edge(plan_idx, write_idx).is_some(),
            "Petgraph must direct edges from cause (plan) to effect (write)"
        );
        assert!(
            subgraph.find_edge(write_idx, plan_idx).is_none(),
            "Petgraph must NOT direct edges backwards from effect to cause"
        );

        // 2. Assert causal_chain traverses incoming edges from write back to plan
        let chain = crate::graph::traversal::causal_chain(&subgraph, &write.id);
        assert_eq!(chain.len(), 2);
        assert_eq!(chain[0].id, write.id);
        assert_eq!(chain[1].id, plan.id);

        // 3. Assert descendants traverses outgoing edges from plan forward to write
        let desc = crate::graph::traversal::descendants(&subgraph, &plan.id);
        assert_eq!(desc.len(), 1);
        assert_eq!(desc[0].id, write.id);

        // 4. Assert ancestors traverses incoming edges from write back to plan
        let anc = crate::graph::traversal::ancestors(&subgraph, &write.id);
        assert_eq!(anc.len(), 1);
        assert_eq!(anc[0].id, plan.id);

        // 5. Assert leaf has no descendants and root has no ancestors
        assert!(crate::graph::traversal::descendants(&subgraph, &write.id).is_empty());
        assert!(crate::graph::traversal::ancestors(&subgraph, &plan.id).is_empty());
    }

    #[test]
    fn test_subgraph_cte_excludes_relates_to_and_contradicts() {
        let db = KnoxDb::open_memory().unwrap();

        let plan = create_action("human", "plan", "Design API architecture", None);
        let write = create_action("gemini", "write", "Implement endpoints", Some(plan.id.clone()));
        let note = create_action("human", "read", "Associative research note", None);
        let conflict = create_action("cursor", "decide", "Conflicting design proposal", None);

        insert_action(&db, &plan).unwrap();
        insert_action(&db, &write).unwrap();
        insert_action(&db, &note).unwrap();
        insert_action(&db, &conflict).unwrap();

        // Insert non-causal edges directly: note relates_to plan, conflict contradicts plan
        let now = Utc::now();
        crate::storage::edges::insert_edge(
            &db,
            &crate::core::types::Edge {
                id: format!("{}:{}:relates_to", note.id, plan.id),
                from_action_id: note.id.clone(),
                to_id: plan.id.to_string(),
                edge_type: "relates_to".to_string(),
                created_at: now,
                label: None,
                provenance: None,
                note: None,
            },
        )
        .unwrap();

        crate::storage::edges::insert_edge(
            &db,
            &crate::core::types::Edge {
                id: format!("{}:{}:contradicts", conflict.id, plan.id),
                from_action_id: conflict.id.clone(),
                to_id: plan.id.to_string(),
                edge_type: "contradicts".to_string(),
                created_at: now,
                label: None,
                provenance: None,
                note: None,
            },
        )
        .unwrap();

        // 1. Descendant expansion from plan: must ONLY include plan and write, NEVER note or conflict
        let desc_subgraph = load_subgraph_directional(&db, std::slice::from_ref(&plan.id), 3, SubgraphDirection::Descendants).unwrap();
        assert_eq!(desc_subgraph.node_count(), 2);
        let desc_map = build_node_index(&desc_subgraph);
        assert!(desc_map.contains_key(&plan.id));
        assert!(desc_map.contains_key(&write.id));
        assert!(!desc_map.contains_key(&note.id), "relates_to must NOT be expanded in descendant CTE");
        assert!(!desc_map.contains_key(&conflict.id), "contradicts must NOT be expanded in descendant CTE");

        // 2. Ancestor expansion from write: must ONLY include write and plan
        let anc_subgraph = load_subgraph_directional(&db, std::slice::from_ref(&write.id), 3, SubgraphDirection::Ancestors).unwrap();
        assert_eq!(anc_subgraph.node_count(), 2);
        let anc_map = build_node_index(&anc_subgraph);
        assert!(anc_map.contains_key(&write.id));
        assert!(anc_map.contains_key(&plan.id));
        assert!(!anc_map.contains_key(&note.id));
        assert!(!anc_map.contains_key(&conflict.id));

        // 3. Both directions expansion from plan: must NOT expand across non-causal edges
        let both_subgraph = load_subgraph_directional(&db, std::slice::from_ref(&plan.id), 3, SubgraphDirection::Both).unwrap();
        assert_eq!(both_subgraph.node_count(), 2);
        let both_map = build_node_index(&both_subgraph);
        assert!(both_map.contains_key(&plan.id));
        assert!(both_map.contains_key(&write.id));
        assert!(!both_map.contains_key(&note.id));
        assert!(!both_map.contains_key(&conflict.id));
    }
}
