//! Domain types and shared data structures for Knox.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::ids::{ActionId, ProjectId, SessionId};

/// A single action recorded in the knowledge graph.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct KnoxAction {
    /// Unique identifier for this action.
    pub id: ActionId,
    /// Software or human actor that performed this action (e.g., "gemini", "cursor", "human").
    pub actor: String,
    /// Categorical type of action (e.g., "write", "plan", "decide", "debug").
    pub action_type: String,
    /// Human-readable concise summary of the action and intent.
    pub summary: String,
    /// Relative file paths touched or referenced by this action.
    pub files: Vec<String>,
    /// Semantic tags associated with this action.
    pub tags: Vec<String>,
    /// Confidence score between 0.0 and 1.0 (optional hint).
    pub confidence: Option<f64>,
    /// Timestamp when this action occurred.
    pub timestamp: DateTime<Utc>,
    /// Session during which this action took place.
    pub session_id: Option<SessionId>,
    /// Direct causal predecessor action.
    pub parent_action_id: Option<ActionId>,
    /// ID of a prior decision or action that this action supersedes.
    pub supersedes: Option<ActionId>,
    /// Indicates whether this action represents current state (false if superseded).
    pub is_current: bool,
}

/// Directed edge connecting nodes in the in-memory graph.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct KnoxEdge {
    /// Relationship type (e.g., "parent", "follows", "superseded_by", "targets").
    pub edge_type: String,
    /// Timestamp of when the edge was established.
    pub timestamp: DateTime<Utc>,
    /// Session in which this edge was established.
    pub session_id: Option<SessionId>,
    /// Tool or software that created the edge.
    pub tool_source: Option<String>,
}

/// Project tracked by Knox.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Project {
    /// Unique identifier for the project.
    pub id: ProjectId,
    /// Human-friendly name of the project.
    pub name: String,
    /// Absolute or canonical filesystem path to project root.
    pub path: String,
    /// Timestamp when project tracking was initialized.
    pub created_at: DateTime<Utc>,
}

/// A session representing a continuous block of work.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Session {
    /// Unique identifier for this session.
    pub id: SessionId,
    /// Primary actor initiating the session.
    pub actor: String,
    /// Associated project identifier or name, if scoped.
    pub project: Option<String>,
    /// Timestamp when the session started.
    pub started_at: DateTime<Utc>,
    /// Timestamp when the session was closed, or None if still active.
    pub ended_at: Option<DateTime<Utc>>,
    /// Active state flag.
    pub is_open: bool,
}

/// Query filter criteria for retrieving actions from the knowledge graph.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct QueryFilter {
    /// Filter by exact or prefix file path.
    pub file_path: Option<String>,
    /// Filter by project identifier or name.
    pub project: Option<String>,
    /// Filter by specific session ID.
    pub session_id: Option<SessionId>,
    /// Filter by specific actor string.
    pub actor: Option<String>,
    /// Filter for actions recorded after this timestamp.
    pub since: Option<DateTime<Utc>>,
    /// Restrict results to active, non-superseded actions (default: true).
    pub current_only: bool,
    /// Filter by tags (must contain all matching tags).
    pub tags: Vec<String>,
    /// Maximum number of records to return (default: 20).
    pub limit: usize,
}

impl Default for QueryFilter {
    fn default() -> Self {
        Self {
            file_path: None,
            project: None,
            session_id: None,
            actor: None,
            since: None,
            current_only: true,
            tags: Vec::new(),
            limit: 20,
        }
    }
}

/// Result set returned from knowledge graph queries.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct QueryResult {
    /// Retrieved actions matching the filter criteria.
    pub results: Vec<KnoxAction>,
    /// Total count of matching actions in the store.
    pub total_count: usize,
    /// Indicates whether additional matching actions exist beyond the limit.
    pub has_more: bool,
}

/// Summary report returned after importing a transcript.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct IngestReport {
    /// Total count of imported actions.
    pub actions_imported: usize,
    /// Total count of unique files referenced.
    pub files_touched: usize,
    /// Number of sessions created or identified.
    pub sessions_created: usize,
}

/// Aggregate status report for the Knox knowledge graph.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct StatusReport {
    /// Total number of actions stored in the database.
    pub total_actions: usize,
    /// Total number of recorded sessions.
    pub total_sessions: usize,
    /// Total number of distinct projects tracked.
    pub projects_tracked: usize,
    /// Timestamp of the most recent action, if any.
    pub last_action_timestamp: Option<DateTime<Utc>>,
    /// Breakdown of actions by actor name.
    pub actions_by_actor: HashMap<String, usize>,
    /// Breakdown of actions by action type.
    pub actions_by_type: HashMap<String, usize>,
}

/// Report produced by scanning staged git modifications.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScanReport {
    /// Number of human edits detected without recent AI attribution.
    pub human_edits_detected: usize,
    /// Total actions newly inserted into the knowledge graph.
    pub actions_recorded: usize,
    /// File paths that were scanned.
    pub files_scanned: Vec<String>,
    /// Short diff statistics summary string.
    pub diff_stat: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_query_filter_default() {
        let filter = QueryFilter::default();
        assert!(filter.current_only);
        assert_eq!(filter.limit, 20);
        assert!(filter.tags.is_empty());
        assert!(filter.file_path.is_none());
    }

    #[test]
    fn test_action_serialization_round_trip() {
        let action = KnoxAction {
            id: ActionId::new(),
            actor: "antigravity".to_string(),
            action_type: "write".to_string(),
            summary: "Created core types".to_string(),
            files: vec!["src/core/types.rs".to_string()],
            tags: vec!["core".to_string()],
            confidence: Some(1.0),
            timestamp: Utc::now(),
            session_id: Some(SessionId::new()),
            parent_action_id: None,
            supersedes: None,
            is_current: true,
        };

        let json = serde_json::to_string(&action).unwrap();
        let deserialized: KnoxAction = serde_json::from_str(&json).unwrap();
        assert_eq!(action, deserialized);
    }

    #[test]
    fn test_session_serialization_round_trip() {
        let session = Session {
            id: SessionId::new(),
            actor: "developer".to_string(),
            project: Some("knox".to_string()),
            started_at: Utc::now(),
            ended_at: None,
            is_open: true,
        };

        let json = serde_json::to_string(&session).unwrap();
        let deserialized: Session = serde_json::from_str(&json).unwrap();
        assert_eq!(session, deserialized);
    }
}
