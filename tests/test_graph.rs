//! Integration tests for the Knox graph layer (petgraph loading, traversal, and LRU cache).

use chrono::Utc;
use knox::core::*;
use knox::graph::*;
use knox::storage::*;

fn make_action(
    actor: &str,
    action_type: &str,
    summary: &str,
    file: &str,
    tag: &str,
    parent: Option<ActionId>,
) -> KnoxAction {
    KnoxAction {
        id: ActionId::new(),
        actor: actor.to_string(),
        action_type: action_type.to_string(),
        summary: summary.to_string(),
        files: vec![file.to_string()],
        tags: vec![tag.to_string()],
        confidence: None,
        timestamp: Utc::now(),
        session_id: None,
        parent_action_id: parent,
        supersedes: None,
        is_current: true,
    }
}

fn seed_test_chain(db: &KnoxDb) -> (KnoxAction, KnoxAction, KnoxAction) {
    let plan = make_action(
        "human",
        "plan",
        "Architecting system",
        "plan.md",
        "architecture",
        None,
    );
    insert_action(db, &plan).unwrap();

    let write = make_action(
        "antigravity",
        "write",
        "Implemented graph",
        "src/graph/loader.rs",
        "graph",
        Some(plan.id.clone()),
    );
    insert_action(db, &write).unwrap();

    let debug = make_action(
        "cursor",
        "debug",
        "Verified BFS",
        "src/graph/traversal.rs",
        "debug",
        Some(write.id.clone()),
    );
    insert_action(db, &debug).unwrap();

    (plan, write, debug)
}

#[test]
fn test_graph_seed_chain_and_subgraph_traversal() {
    let db = KnoxDb::open_memory().unwrap();
    let (plan, write, debug) = seed_test_chain(&db);

    let graph = load_subgraph(&db, std::slice::from_ref(&write.id), 1).unwrap();
    assert_eq!(graph.node_count(), 3);
    assert_eq!(graph.edge_count(), 2);

    assert!(find_node_index(&graph, &plan.id).is_some());
    assert!(find_node_index(&graph, &write.id).is_some());
    assert!(find_node_index(&graph, &debug.id).is_some());

    let chain = causal_chain(&graph, &debug.id);
    assert_eq!(chain.len(), 3);
    assert_eq!(chain[0].id, debug.id);

    let desc = descendants(&graph, &plan.id);
    assert_eq!(desc.len(), 2);
    assert_eq!(desc[0].id, write.id);

    let path = path_between(&graph, &plan.id, &debug.id).expect("path should exist");
    assert_eq!(path.len(), 3);
    assert_eq!(path[0].id, plan.id);
}

#[test]
fn test_graph_cache_hit_miss_and_invalidation() {
    let db = KnoxDb::open_memory().unwrap();

    let action = KnoxAction {
        id: ActionId::new(),
        actor: "gemini".to_string(),
        action_type: "write".to_string(),
        summary: "Created cached file".to_string(),
        files: vec!["src/cached.rs".to_string()],
        tags: vec![],
        confidence: None,
        timestamp: Utc::now(),
        session_id: None,
        parent_action_id: None,
        supersedes: None,
        is_current: true,
    };
    insert_action(&db, &action).unwrap();

    let mut cache = GraphCache::new(4);
    let key = CacheKey::new(None, Some("src/cached.rs".to_string()));

    // Cache miss on first call
    assert!(cache.get(&key).is_none());
    let loaded = cache.get_or_load(&db, &key).unwrap();
    assert_eq!(loaded.node_count(), 1);
    assert_eq!(cache.len(), 1);

    // Cache hit on second call
    let cached = cache.get(&key).expect("should be a cache hit");
    assert_eq!(cached.node_count(), 1);

    // Test invalidation
    cache.invalidate(&key);
    assert!(cache.get(&key).is_none());
    assert_eq!(cache.len(), 0);

    // Invalidate all
    let _ = cache.get_or_load(&db, &key).unwrap();
    assert_eq!(cache.len(), 1);
    cache.invalidate_all();
    assert_eq!(cache.len(), 0);
}
