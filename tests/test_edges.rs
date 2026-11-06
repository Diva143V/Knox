//! Workstream B acceptance & consistency tests: One graph, edge semantics, and triggers.

use chrono::Utc;
use serde_json::json;

use knox::commands::record::cmd_record;
use knox::core::error::KnoxError;
use knox::core::ids::{ActionId, ProjectId};
use knox::core::types::{ActionRelation, KnoxAction, Project, Ref, RelationType};
use knox::graph::loader::load_full_graph;
use knox::graph::traversal::{causal_chain, descendants};
use knox::storage::db::KnoxDb;
use knox::storage::edges::{
    get_edge_rejections, get_edges_for_action, insert_edge, insert_ref,
};
use knox::storage::nodes::{insert_action, insert_project, resolve_doubt};

fn make_action(
    actor: &str,
    action_type: &str,
    summary: &str,
    parent: Option<ActionId>,
    supersedes: Option<ActionId>,
    commit_sha: Option<&str>,
) -> KnoxAction {
    KnoxAction {
        id: ActionId::new(),
        actor: actor.to_string(),
        action_type: action_type.to_string(),
        summary: summary.to_string(),
        files: vec!["src/lib.rs".to_string()],
        tags: vec!["core".to_string()],
        confidence: None,
        timestamp: Utc::now(),
        session_id: None,
        project_id: Some(ProjectId::from("demo")),
        parent_action_id: parent,
        supersedes,
        commit_sha: commit_sha.map(|s| s.to_string()),
        is_current: true,
        ..Default::default()
    }
}

#[test]
fn test_consistency_across_fifty_actions() {
    let db = KnoxDb::open_memory().unwrap();

    // Register project
    insert_project(
        &db,
        &Project {
            id: ProjectId::from("demo"),
            name: "demo".to_string(),
            path: "d:/Knox/demo".to_string(),
            remote: None,
            created_at: Utc::now(),
        },
    )
    .unwrap();

    // 1. Root plan
    let root_plan = make_action("human", "plan", "Design distributed graph pipeline", None, None, None);
    insert_action(&db, &root_plan).unwrap();

    let mut actions = Vec::new();
    actions.push(root_plan.clone());

    let mut prev_id = root_plan.id.clone();

    // 2. Generate 45 mixed actions
    for i in 1..=45 {
        let act_type = match i % 5 {
            0 => "decide",
            1 => "write",
            2 => "debug",
            3 => "plan",
            _ => "write",
        };
        let sha = if i % 2 == 0 {
            Some(format!("sha_{:04x}", i))
        } else {
            None
        };
        let mut act = make_action(
            if i % 2 == 0 { "gemini" } else { "cursor" },
            act_type,
            &format!("Action step #{i}"),
            Some(prev_id.clone()),
            None,
            sha.as_deref(),
        );

        if act_type == "write" {
            // Write action implements the root plan or earlier decision
            act.relations.push(ActionRelation {
                target_id: root_plan.id.clone(),
                relation_type: RelationType::Implements,
            });
        } else if act_type == "debug" {
            // Debug verifies previous action
            act.relations.push(ActionRelation {
                target_id: prev_id.clone(),
                relation_type: RelationType::Verifies,
            });
        }

        insert_action(&db, &act).unwrap();
        prev_id = act.id.clone();
        actions.push(act);
    }

    // 3. Add 2 doubts and resolve them
    let doubt1 = make_action("gemini", "doubt", "Is edge table performance adequate for 10k nodes?", Some(prev_id.clone()), None, None);
    insert_action(&db, &doubt1).unwrap();
    actions.push(doubt1.clone());

    let doubt2 = make_action("cursor", "doubt", "Should polymorphic targets support symbols?", Some(prev_id.clone()), None, None);
    insert_action(&db, &doubt2).unwrap();
    actions.push(doubt2.clone());

    let resolver = make_action("human", "decide", "Benchmark confirms sub-millisecond BFS up to 50k edges", Some(doubt1.id.clone()), None, Some("commit_abc123"));
    insert_action(&db, &resolver).unwrap();
    actions.push(resolver.clone());

    // Resolve doubt 1 with resolution note and resolver action
    resolve_doubt(
        &db,
        &doubt1.id,
        "Confirmed via Criterion benchmarks: 50k edges BFS completes in 850 microseconds",
        Some(&resolver.id),
    )
    .unwrap();

    // 4. Add 1 superseding decision (total = 50 actions)
    let superseded_target = actions[5].id.clone();
    let superseding = make_action(
        "human",
        "decide",
        "Supersede old architecture with unified typed edge graph",
        Some(actions[5].id.clone()),
        Some(superseded_target.clone()),
        Some("commit_def456"),
    );
    insert_action(&db, &superseding).unwrap();
    actions.push(superseding.clone());

    assert_eq!(actions.len(), 50);

    // 5. Verify ALL 5 sources of edges are consistent between actions and edges table:
    db.with_tx(|conn| {
        for act in &actions {
            // Source 1: parent_action_id -> caused_by edge
            if let Some(ref parent_id) = act.parent_action_id {
                let edge_id = format!("{}:caused_by:{}", act.id, parent_id);
                let edge: Option<String> = conn
                    .query_row(
                        "SELECT edge_type FROM edges WHERE id = ?1 AND from_action_id = ?2 AND to_id = ?3",
                        rusqlite::params![edge_id, act.id.as_str(), parent_id.as_str()],
                        |r| r.get(0),
                    )
                    .ok();
                assert_eq!(edge.as_deref(), Some("caused_by"), "Missing caused_by edge for {}", act.id);
            }

            // Source 2: supersedes -> supersedes edge
            if let Some(ref prior_id) = act.supersedes {
                let edge: Option<String> = conn
                    .query_row(
                        "SELECT edge_type FROM edges WHERE from_action_id = ?1 AND to_id = ?2 AND edge_type = 'supersedes'",
                        rusqlite::params![act.id.as_str(), prior_id.as_str()],
                        |r| r.get(0),
                    )
                    .ok();
                assert_eq!(edge.as_deref(), Some("supersedes"), "Missing supersedes edge for {}", act.id);
            }

            // Source 3: relations -> edge table
            for rel in &act.relations {
                let edge: Option<String> = conn
                    .query_row(
                        "SELECT edge_type FROM edges WHERE from_action_id = ?1 AND to_id = ?2 AND edge_type = ?3",
                        rusqlite::params![act.id.as_str(), rel.target_id.as_str(), rel.relation_type.to_string()],
                        |r| r.get(0),
                    )
                    .ok();
                assert_eq!(
                    edge.as_deref(),
                    Some(rel.relation_type.to_string().as_str()),
                    "Missing relation edge {} -> {} of type {}",
                    act.id,
                    rel.target_id,
                    rel.relation_type
                );
            }

            // Source 4: commit_sha -> edge provenance
            if let Some(ref sha) = act.commit_sha {
                let prov: Option<String> = conn
                    .query_row(
                        "SELECT provenance FROM edges WHERE from_action_id = ?1 LIMIT 1",
                        rusqlite::params![act.id.as_str()],
                        |r| r.get(0),
                    )
                    .ok();
                assert_eq!(prov.as_deref(), Some(sha.as_str()), "Edge provenance did not match commit_sha");
            }
        }

        // Source 5: is_resolved & resolution_note -> resolves edge note
        let resolves_note: Option<String> = conn
            .query_row(
                "SELECT note FROM edges WHERE from_action_id = ?1 AND to_id = ?2 AND edge_type = 'resolves'",
                rusqlite::params![resolver.id.as_str(), doubt1.id.as_str()],
                |r| r.get(0),
            )
            .ok();
        assert_eq!(
            resolves_note.as_deref(),
            Some("Confirmed via Criterion benchmarks: 50k edges BFS completes in 850 microseconds")
        );

        Ok(())
    })
    .unwrap();

    // 6. Test graph traversal on the 50-action component:
    let graph = load_full_graph(&db).unwrap();
    assert_eq!(graph.node_count(), 50);

    // Causal chain from the last action must reach root_plan
    let chain = causal_chain(&graph, &superseding.id);
    assert!(!chain.is_empty());
    assert_eq!(chain[0].id, superseding.id);
    assert_eq!(chain.last().unwrap().id, root_plan.id);

    // Descendants from root_plan must reach all direct and indirect children
    let desc = descendants(&graph, &root_plan.id);
    assert!(desc.len() >= 45);
}

#[test]
fn test_polymorphic_edge_to_refs_and_integrity_trigger() {
    let db = KnoxDb::open_memory().unwrap();

    insert_project(
        &db,
        &Project {
            id: ProjectId::from("demo"),
            name: "demo".to_string(),
            path: "d:/Knox/demo".to_string(),
            remote: None,
            created_at: Utc::now(),
        },
    )
    .unwrap();

    let write_act = make_action("gemini", "write", "Implement DB layer", None, None, None);
    insert_action(&db, &write_act).unwrap();

    // 1. External ref registered in refs table
    let file_ref = Ref {
        id: "file:src/db.rs".to_string(),
        kind: "file".to_string(),
        value: "src/db.rs".to_string(),
        project_id: Some(ProjectId::from("demo")),
        created_at: Utc::now(),
    };
    insert_ref(&db, &file_ref).unwrap();

    // 2. An edge pointing to the external ref succeeds
    let edge = knox::core::types::Edge {
        id: format!("{}:implements:file:src/db.rs", write_act.id),
        from_action_id: write_act.id.clone(),
        to_id: "file:src/db.rs".to_string(),
        edge_type: "implements".to_string(),
        created_at: Utc::now(),
        label: Some("database module".to_string()),
        provenance: Some("test".to_string()),
        note: None,
    };
    assert!(insert_edge(&db, &edge).is_ok());

    let fetched = get_edges_for_action(&db, &write_act.id).unwrap();
    assert!(fetched.iter().any(|e| e.to_id == "file:src/db.rs"));

    // 3. SQLite integrity trigger: an edge pointing to a non-existent action AND non-existent ref is ABORTED
    let bad_edge = knox::core::types::Edge {
        id: format!("{}:implements:ghost_target", write_act.id),
        from_action_id: write_act.id.clone(),
        to_id: "ghost_target".to_string(),
        edge_type: "implements".to_string(),
        created_at: Utc::now(),
        label: None,
        provenance: None,
        note: None,
    };
    let insert_res = insert_edge(&db, &bad_edge);
    assert!(insert_res.is_err(), "Integrity trigger must reject non-existent to_id");
}

#[test]
fn test_dropping_trigger_fails_consistency() {
    let db = KnoxDb::open_memory().unwrap();

    let parent = make_action("human", "plan", "Parent task", None, None, None);
    insert_action(&db, &parent).unwrap();

    // Drop the trigger that synchronizes parent_action_id to edges
    db.with_tx(|conn| {
        conn.execute("DROP TRIGGER IF EXISTS edges_ai_caused_by", [])?;
        Ok(())
    })
    .unwrap();

    let child = make_action("gemini", "write", "Child task without trigger", Some(parent.id.clone()), None, None);
    insert_action(&db, &child).unwrap();

    // Verify consistency failure: child has parent_action_id, but edges table has NO edge!
    let edge_count: i64 = db
        .with_tx(|conn| {
            let count: i64 = conn.query_row(
                "SELECT COUNT(*) FROM edges WHERE from_action_id = ?1 AND to_id = ?2",
                rusqlite::params![child.id.as_str(), parent.id.as_str()],
                |r| r.get(0),
            )?;
            Ok(count)
        })
        .unwrap();

    assert_eq!(edge_count, 0, "Dropping trigger caused edges table to desync from actions column as expected");
}

#[test]
fn test_direction_validation_and_rejection_telemetry() {
    let db = KnoxDb::open_memory().unwrap();

    // 1. Reversed implements is rejected with teaching error and logged in edge_rejections
    let res = cmd_record(
        &db,
        json!({
            "actor": "gemini",
            "project": "demo",
            "entries": [
                {"ref": "code", "kind": "write", "summary": "Implement storage layer"},
                {
                    "ref": "decision",
                    "kind": "decide",
                    "summary": "Use SQLite as store",
                    "relations": [{"to": "code", "relation_type": "implements"}]
                }
            ]
        }),
    );
    assert!(matches!(res, Err(KnoxError::InvalidRelation(_))));
    if let Err(KnoxError::InvalidRelation(msg)) = res {
        assert!(msg.contains("cannot implement another action"));
        assert!(msg.contains("Code actions ('write', 'debug') implement decisions ('decide')"));
    }

    // 2. Unknown relation type is rejected and logged
    let res_unknown = cmd_record(
        &db,
        json!({
            "actor": "gemini",
            "project": "demo",
            "entries": [
                {"ref": "a1", "kind": "plan", "summary": "Initial plan"},
                {
                    "ref": "a2",
                    "kind": "write",
                    "summary": "Implementation",
                    "relations": [{"to": "a1", "relation_type": "invalid_link_type"}]
                }
            ]
        }),
    );
    assert!(matches!(res_unknown, Err(KnoxError::InvalidRelation(_))));

    // 3. Verify edge_rejections logged both rejections
    let rejections = get_edge_rejections(&db, 10).unwrap();
    assert_eq!(rejections.len(), 2);
    assert_eq!(rejections[0].attempted_type, "invalid_link_type");
    assert_eq!(rejections[1].attempted_type, "implements");
}
