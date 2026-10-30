//! Integration tests for Knox core types and configuration.

use chrono::Utc;
use knox::core::*;
use std::time::Duration;

#[test]
fn test_integration_ids_and_actions() {
    let action_id = ActionId::new();
    let session_id = SessionId::new();

    let action = KnoxAction {
        id: action_id.clone(),
        actor: "antigravity".to_string(),
        action_type: "plan".to_string(),
        summary: "Initialize Phase 1A".to_string(),
        files: vec!["src/core/mod.rs".to_string()],
        tags: vec!["phase1a".to_string(), "core".to_string()],
        confidence: Some(0.95),
        timestamp: Utc::now(),
        session_id: Some(session_id.clone()),
        parent_action_id: None,
        supersedes: None,
        is_current: true,
    };

    assert_eq!(action.id, action_id);
    assert!(action.is_current);
    assert_eq!(action.files.len(), 1);
}

#[test]
fn test_integration_query_filter_defaults() {
    let filter = QueryFilter::default();
    assert!(filter.current_only);
    assert_eq!(filter.limit, 20);
    assert!(filter.since.is_none());
}

#[test]
fn test_integration_config_defaults() {
    let config = KnoxConfig::default();
    assert_eq!(config.session_timeout, Duration::from_secs(1800));
    assert!(config.global_mode);
}
