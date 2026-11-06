//! Workstream H Integration Tests: Model Layer Foundation (H1).
//!
//! Validates:
//! - Five model traits (EmbeddingModel, RerankModel, AnsweringModel, ExtractionModel, ContradictionModel).
//! - Clean-machine promise: models off by default, zero GPU/API requirement, deterministic fallback.
//! - Migration v10: SQLite `embeddings` table creation, vector blob serialization, cosine similarity search.
//! - Mock model backend: deterministic unit vectors, citation-grounded answering.

use chrono::Utc;
use tempfile::tempdir;

use knox::core::config::KnoxConfig;
use knox::core::ids::ActionId;
use knox::core::types::KnoxAction;
use knox::models::mock::MockModelBackend;
use knox::models::profile::ModelProfileKind;
use knox::models::router::ModelRouter;
use knox::models::traits::{
    AnsweringModel, AskRequest, ContradictionModel, EmbeddingModel, ExtractionModel, RerankModel,
};
use knox::storage::db::KnoxDb;
use knox::storage::embeddings::{
    blob_to_vector, cosine_similarity, find_similar_actions, get_embedding, insert_embedding,
    vector_to_blob,
};
use knox::storage::nodes::insert_action;

#[test]
fn test_h1_clean_machine_promise_off_by_default() {
    let config = KnoxConfig::default();
    assert_eq!(config.model_profile, "off");

    let router = ModelRouter::from_config(&config);
    assert!(!router.is_enabled());
    assert_eq!(router.profile.kind, ModelProfileKind::Off);

    // Operations on Off profile gracefully fallback without crashing
    let candidate = KnoxAction {
        id: ActionId::new(),
        actor: "antigravity".to_string(),
        action_type: "decide".to_string(),
        summary: "Standardize on SQLite WAL mode".to_string(),
        timestamp: Utc::now(),
        is_current: true,
        ..Default::default()
    };

    let reranked = router.rerank("WAL mode", std::slice::from_ref(&candidate)).unwrap();
    assert_eq!(reranked.len(), 1);
    assert_eq!(reranked[0].action.id, candidate.id);

    let ask_req = AskRequest {
        question: "Why WAL mode?".to_string(),
        file_path: None,
        project: None,
        strict_citations: true,
    };
    let answered = router.answer(&ask_req, std::slice::from_ref(&candidate)).unwrap();
    assert!(answered.answer.contains("Knox project memory"));
    assert_eq!(answered.citations.len(), 1);
    assert_eq!(answered.citations[0].action_id, candidate.id);

    let proposals = router.extract_proposals("We decided to migrate schema.").unwrap();
    assert!(proposals.is_empty(), "Off profile returns empty proposals without model call");

    let contradiction = router.check_contradiction("Switch to Postgres", &candidate).unwrap();
    assert!(!contradiction.contradicts, "Off profile safely skips contradiction");
}

#[test]
fn test_h1_five_model_traits_mock_implementation() {
    let mock = MockModelBackend::new();

    // 1. EmbeddingModel
    let vec1 = mock.embed("SQLite WAL mode for concurrent readers").unwrap();
    let vec2 = mock.embed("SQLite WAL mode for concurrent readers").unwrap();
    let vec3 = mock.embed("Completely different topic about CSS styling").unwrap();

    assert_eq!(vec1.len(), 64);
    assert_eq!(vec2.len(), 64);
    assert_eq!(vec1, vec2, "Identical inputs must yield identical mock vectors");

    let sim_same = cosine_similarity(&vec1, &vec2);
    let sim_diff = cosine_similarity(&vec1, &vec3);
    assert!((sim_same - 1.0).abs() < 1e-4, "Identical vectors must have cosine similarity ~1.0");
    assert!(sim_diff < sim_same, "Different topics must have lower similarity");

    // 2. RerankModel
    let action_sqlite = KnoxAction {
        id: ActionId::new(),
        actor: "developer".to_string(),
        action_type: "decide".to_string(),
        summary: "Configure SQLite WAL journal mode".to_string(),
        files: vec!["src/db.rs".to_string()],
        timestamp: Utc::now(),
        is_current: true,
        ..Default::default()
    };
    let action_ui = KnoxAction {
        id: ActionId::new(),
        actor: "developer".to_string(),
        action_type: "write".to_string(),
        summary: "Add dark mode CSS palette".to_string(),
        files: vec!["ui/theme.css".to_string()],
        timestamp: Utc::now(),
        is_current: true,
        ..Default::default()
    };

    let reranked = mock
        .rerank("sqlite journal mode", &[action_ui.clone(), action_sqlite.clone()])
        .unwrap();
    assert_eq!(reranked.len(), 2);
    assert_eq!(reranked[0].action.id, action_sqlite.id, "SQLite action must be ranked #1");
    assert!(reranked[0].score > reranked[1].score);

    // 3. AnsweringModel
    let ask_req = AskRequest {
        question: "What database mode is configured?".to_string(),
        file_path: None,
        project: None,
        strict_citations: true,
    };
    let resp = mock.answer(&ask_req, std::slice::from_ref(&action_sqlite)).unwrap();
    assert!(resp.grounded);
    assert_eq!(resp.citations.len(), 1);
    assert_eq!(resp.citations[0].action_id, action_sqlite.id);
    assert!(resp.answer.contains(&format!("[ref:{}]", action_sqlite.id)));

    // 4. ExtractionModel
    let transcript = "The team decided to use Rust 2024 edition for all new crates.\nImplemented parser in src/parser.rs";
    let extracted = mock.extract_proposals(transcript).unwrap();
    assert_eq!(extracted.len(), 2);
    assert_eq!(extracted[0].action_type, "decide");
    assert_eq!(extracted[1].action_type, "write");

    // 5. ContradictionModel
    let contradiction = mock
        .check_contradiction(
            "Do not use SQLite, instead replace SQLite with MongoDB database",
            &action_sqlite,
        )
        .unwrap();
    assert!(contradiction.contradicts, "Must flag explicit negation of established decision");
    assert_eq!(contradiction.established_id, action_sqlite.id);

    let non_contradiction = mock
        .check_contradiction("Optimize SQLite index performance", &action_sqlite)
        .unwrap();
    assert!(!non_contradiction.contradicts);
}

#[test]
fn test_h1_migration_v10_and_vector_persistence() {
    let temp = tempdir().unwrap();
    let db_path = temp.path().join("embeddings_test.db");
    let db = KnoxDb::open(&db_path).unwrap();

    let action = KnoxAction {
        id: ActionId::new(),
        actor: "agent".to_string(),
        action_type: "decide".to_string(),
        summary: "Architecture decision: Event-driven actor bus".to_string(),
        timestamp: Utc::now(),
        is_current: true,
        ..Default::default()
    };
    insert_action(&db, &action).unwrap();

    // Verify vector blob serialization
    let original_vector = vec![0.1f32, -0.4f32, 0.9f32, 0.0f32];
    let blob = vector_to_blob(&original_vector);
    let recovered = blob_to_vector(&blob);
    assert_eq!(original_vector, recovered);

    // Insert embedding into DB (Migration v10 table)
    insert_embedding(&db, &action.id, &original_vector, "mock-embed").unwrap();

    let fetched = get_embedding(&db, &action.id).unwrap().expect("embedding must exist");
    assert_eq!(fetched, original_vector);

    // Find similar actions
    let query_vector = vec![0.12f32, -0.38f32, 0.88f32, 0.05f32];
    let matches = find_similar_actions(&db, &query_vector, 5, 0.8).unwrap();
    assert_eq!(matches.len(), 1);
    assert_eq!(matches[0].0, action.id);
    assert!(matches[0].1 > 0.95, "Vector should have high cosine similarity");
}

#[test]
fn test_h1_model_cli_commands() {
    let config = KnoxConfig {
        model_profile: "mock".to_string(),
        model_endpoint: Some("mock://test".to_string()),
        embedding_model: Some("mock-embed".to_string()),
        chat_model: Some("mock-chat".to_string()),
        ..Default::default()
    };

    // Verify cmd_model_status executes cleanly
    knox::commands::cmd_model_status(&config).unwrap();

    // Verify cmd_model_test executes cleanly
    knox::commands::cmd_model_test(&config, "Explain Knox architecture").unwrap();
}

#[test]
fn test_h2_hybrid_ranking_models_off_identical_to_baseline() {
    let act1 = KnoxAction {
        id: ActionId::new(),
        actor: "dev".to_string(),
        action_type: "decide".to_string(),
        summary: "Use rusqlite for relational storage".to_string(),
        files: vec!["src/db.rs".to_string()],
        timestamp: Utc::now(),
        is_current: true,
        ..Default::default()
    };
    let act2 = KnoxAction {
        id: ActionId::new(),
        actor: "dev".to_string(),
        action_type: "write".to_string(),
        summary: "Add web interface styles".to_string(),
        files: vec!["ui/style.css".to_string()],
        timestamp: Utc::now(),
        is_current: true,
        ..Default::default()
    };

    let actions = vec![act2.clone(), act1.clone()];
    let baseline = knox::retrieval::rank::rank_actions(
        actions.clone(),
        Some("src/db.rs"),
        &[],
        Some("relational storage"),
    );

    let config_off = KnoxConfig {
        model_profile: "off".to_string(),
        ..Default::default()
    };
    let router_off = ModelRouter::from_config(&config_off);

    let hybrid = knox::retrieval::rank::rank_actions_hybrid(
        actions,
        Some("src/db.rs"),
        &[],
        Some("relational storage"),
        Some(&router_off),
        None,
    );

    assert_eq!(
        baseline.len(),
        hybrid.len(),
        "Model off must return same number of actions"
    );
    for (b, h) in baseline.iter().zip(hybrid.iter()) {
        assert_eq!(b.id, h.id, "Model off must produce byte-identical ranking order");
    }
}

#[test]
fn test_h2_hybrid_ranking_semantic_boost() {
    let temp = tempdir().unwrap();
    let db = KnoxDb::open(&temp.path().join("h2_rank.db")).unwrap();

    let act_sqlite = KnoxAction {
        id: ActionId::new(),
        actor: "dev".to_string(),
        action_type: "decide".to_string(),
        summary: "SQLite database configuration for write-ahead logging".to_string(),
        timestamp: Utc::now(),
        is_current: true,
        ..Default::default()
    };
    let act_css = KnoxAction {
        id: ActionId::new(),
        actor: "dev".to_string(),
        action_type: "decide".to_string(),
        summary: "CSS style system with dark mode theme".to_string(),
        timestamp: Utc::now(),
        is_current: true,
        ..Default::default()
    };

    insert_action(&db, &act_sqlite).unwrap();
    insert_action(&db, &act_css).unwrap();

    let config_mock = KnoxConfig {
        model_profile: "mock".to_string(),
        ..Default::default()
    };
    let router_mock = ModelRouter::from_config(&config_mock);

    // Query for "database logging", which semantically matches the SQLite action
    let actions = vec![act_css.clone(), act_sqlite.clone()];
    let ranked = knox::retrieval::rank::rank_actions_hybrid(
        actions,
        None,
        &[],
        Some("database logging"),
        Some(&router_mock),
        Some(&db),
    );

    assert_eq!(ranked[0].id, act_sqlite.id, "SQLite action must receive semantic boost and rank first");
}

#[test]
fn test_h2_semantic_deduplication_in_record() {
    let temp = tempdir().unwrap();
    let db = KnoxDb::open(&temp.path().join("h2_dedupe.db")).unwrap();

    unsafe {
        std::env::set_var("KNOX_MODEL_PROFILE", "mock");
    }

    let payload1 = serde_json::json!({
        "actor": "dev",
        "project": "proj",
        "entries": [
            {
                "kind": "decide",
                "summary": "Standardize SQLite database engine",
                "why": "Reliable embedded storage"
            }
        ]
    });

    let resp1 = knox::commands::cmd_record(&db, payload1).unwrap();
    let ids1 = resp1.get("ids").and_then(|v| v.as_array()).unwrap();
    assert_eq!(ids1.len(), 1);
    let id1 = ids1[0].as_str().unwrap().to_string();
    let deduped1 = resp1.get("deduped").and_then(|v| v.as_array()).unwrap();
    assert_eq!(deduped1.len(), 0, "First record must not be deduped");

    // Verify embedding was created and stored for the new action
    let emb = get_embedding(&db, &ActionId::from(id1.clone())).unwrap();
    assert!(emb.is_some(), "Recorded action must have embedding stored in embeddings table");

    // Second record with identical semantic content
    let payload2 = serde_json::json!({
        "actor": "dev",
        "project": "proj",
        "entries": [
            {
                "kind": "decide",
                "summary": "Standardize SQLite database engine",
                "why": "Reliable embedded storage"
            }
        ]
    });

    let resp2 = knox::commands::cmd_record(&db, payload2).unwrap();
    let ids2 = resp2.get("ids").and_then(|v| v.as_array()).unwrap();
    assert_eq!(ids2.len(), 1);
    let id2 = ids2[0].as_str().unwrap();
    assert_eq!(id1, id2, "Near-identical entry must dedupe to existing action ID");
    let deduped2 = resp2.get("deduped").and_then(|v| v.as_array()).unwrap();
    assert_eq!(deduped2.len(), 1, "Second record must be counted in deduped");
    assert_eq!(deduped2[0].as_str().unwrap(), id1);

    unsafe {
        std::env::remove_var("KNOX_MODEL_PROFILE");
    }
}

#[test]
fn test_h2_related_section_in_grouped_context() {
    let parent = KnoxAction {
        id: ActionId::new(),
        actor: "dev".to_string(),
        action_type: "decide".to_string(),
        summary: "Architecture plan".to_string(),
        timestamp: Utc::now(),
        is_current: true,
        ..Default::default()
    };
    let mut child = KnoxAction {
        id: ActionId::new(),
        actor: "dev".to_string(),
        action_type: "write".to_string(),
        summary: "Implemented parser".to_string(),
        timestamp: Utc::now(),
        parent_action_id: Some(parent.id.clone()),
        is_current: true,
        ..Default::default()
    };
    child.relations.push(knox::core::types::ActionRelation {
        target_id: parent.id.clone(),
        relation_type: knox::core::types::RelationType::Implements,
    });

    let digest = knox::mcp::format::format_markdown_digest("Project", &[parent, child], true);
    assert!(
        digest.contains("#### 4. Related Actions & Graph Lineage"),
        "Grouped digest with relations must render Related Actions section:\n{digest}"
    );
    assert!(digest.contains("implements"));
    assert!(digest.contains("caused_by"));
}

#[test]
fn test_c2_cloud_egress_guard_and_project_egress() {
    use knox::core::config::KnoxConfig;
    use knox::models::ModelRouter;
    use knox::storage::{is_project_egress_allowed, set_project_egress, KnoxDb};

    // 1. Cloud profile with allow_cloud_egress = false refuses to attach adapters
    let config = KnoxConfig {
        model_profile: "cloud".to_string(),
        model_endpoint: Some("https://api.openai.com/v1".to_string()),
        model_api_key: Some("sk-test".to_string()),
        allow_cloud_egress: false,
        ..Default::default()
    };
    let router = ModelRouter::from_config(&config);
    assert!(router.embedding().is_none(), "Cloud profile without egress opt-in must not load embedding adapter");
    assert!(!router.is_enabled(), "Cloud profile without egress opt-in must not be enabled");

    // 2. Cloud profile with allow_cloud_egress = true attaches adapters
    let config_allowed = KnoxConfig {
        model_profile: "cloud".to_string(),
        model_endpoint: Some("https://api.openai.com/v1".to_string()),
        model_api_key: Some("sk-test".to_string()),
        allow_cloud_egress: true,
        ..Default::default()
    };
    let router_allowed = ModelRouter::from_config(&config_allowed);
    assert!(router_allowed.embedding().is_some(), "Cloud profile with egress opt-in must load embedding adapter");
    assert!(router_allowed.is_enabled());

    // 3. Project-level egress check (v11 schema migration)
    let db = KnoxDb::open_memory().unwrap();
    let proj = knox::core::types::Project {
        id: knox::core::ids::ProjectId::new(),
        name: "LocalProject".to_string(),
        path: "D:\\LocalOnly".to_string(),
        remote: None,
        created_at: chrono::Utc::now(),
    };
    knox::storage::insert_project(&db, &proj).unwrap();

    // Invariant: default egress is false (data never leaves the machine)
    assert!(!is_project_egress_allowed(&db, proj.id.as_str()).unwrap());

    // Explicit opt-in grants egress
    set_project_egress(&db, proj.id.as_str(), true).unwrap();
    assert!(is_project_egress_allowed(&db, proj.id.as_str()).unwrap());

    // Revocation turns egress off
    set_project_egress(&db, proj.id.as_str(), false).unwrap();
    assert!(!is_project_egress_allowed(&db, proj.id.as_str()).unwrap());

    // 4. ModelRouter::for_project fails closed on LocalProject when egress = 0
    let router_proj_blocked = ModelRouter::for_project(&config, Some(&db), Some(proj.id.as_str()));
    assert!(!router_proj_blocked.is_enabled());
    assert!(router_proj_blocked.check_egress().is_err());
    let err_msg = router_proj_blocked.embed("test query").unwrap_err().to_string();
    assert!(err_msg.contains("Cloud model requests blocked"));

    // 5. ModelRouter::for_project enables cloud adapters when project grants egress
    set_project_egress(&db, proj.id.as_str(), true).unwrap();
    let router_proj_allowed = ModelRouter::for_project(&config, Some(&db), Some(proj.id.as_str()));
    assert!(router_proj_allowed.is_enabled());
    assert!(router_proj_allowed.check_egress().is_ok());
    assert!(router_proj_allowed.embedding().is_some());
}

