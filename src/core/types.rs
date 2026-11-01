//! Domain types and shared data structures for Knox.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::ids::{ActionId, ProjectId, SessionId};

/// Semantic nature of a relationship between two actions in the graph.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RelationType {
    /// This action depends on the outcome or design of the target.
    DependsOn,
    /// This implementation writes code satisfying the target plan or decision.
    Implements,
    /// This test, audit, or verification validates the target action.
    Verifies,
    /// This action supersedes, undoes, or fixes the target.
    Reverts,
    /// General thematic or associative connection.
    RelatesTo,
    /// This decision, action, or commit resolves or answers a prior doubt.
    Resolves,
}

impl std::fmt::Display for RelationType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DependsOn => write!(f, "depends_on"),
            Self::Implements => write!(f, "implements"),
            Self::Verifies => write!(f, "verifies"),
            Self::Reverts => write!(f, "reverts"),
            Self::RelatesTo => write!(f, "relates_to"),
            Self::Resolves => write!(f, "resolves"),
        }
    }
}

impl std::str::FromStr for RelationType {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "depends_on" | "dependson" | "depends" => Ok(Self::DependsOn),
            "implements" | "implement" => Ok(Self::Implements),
            "verifies" | "verify" | "tests" | "test" => Ok(Self::Verifies),
            "reverts" | "revert" | "fixes" | "fix" => Ok(Self::Reverts),
            "relates_to" | "relatesto" | "relates" => Ok(Self::RelatesTo),
            "resolves" | "resolve" | "answers" | "answer" => Ok(Self::Resolves),
            other => Err(format!("Unknown relation type: {other}")),
        }
    }
}

/// A directed semantic relationship linking this action to another action.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActionRelation {
    /// Target action identifier being referenced.
    pub target_id: ActionId,
    /// Semantic type of the relation.
    pub relation_type: RelationType,
}

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
    /// Semantic relations pointing from this action to other actions.
    #[serde(default)]
    pub relations: Vec<ActionRelation>,
    /// Source code symbols touched (structs, functions, traits, modules).
    #[serde(default)]
    pub symbols: Vec<String>,
    /// Detailed architectural rationale or design justification.
    #[serde(default)]
    pub rationale: Option<String>,
    /// Rejected alternatives and reasons why they were discarded.
    #[serde(default)]
    pub rejected_alternatives: Vec<String>,
    /// High-level context summary of the active objective/state.
    #[serde(default)]
    pub context_summary: Option<String>,
    /// Significance level: "major" (e.g. ADRs, major milestones) vs "minor" (micro-decisions, tweaks).
    #[serde(default)]
    pub significance: Option<String>,
    /// Resolution status (primarily for doubts/questions): true if resolved, false if open.
    #[serde(default)]
    pub is_resolved: Option<bool>,
    /// Resolution explanation or note when a doubt is resolved.
    #[serde(default)]
    pub resolution_note: Option<String>,
}

impl Default for KnoxAction {
    fn default() -> Self {
        Self {
            id: ActionId::new(),
            actor: String::new(),
            action_type: String::new(),
            summary: String::new(),
            files: Vec::new(),
            tags: Vec::new(),
            confidence: None,
            timestamp: Utc::now(),
            session_id: None,
            parent_action_id: None,
            supersedes: None,
            is_current: true,
            relations: Vec::new(),
            symbols: Vec::new(),
            rationale: None,
            rejected_alternatives: Vec::new(),
            context_summary: None,
            significance: Some("major".to_string()),
            is_resolved: Some(true),
            resolution_note: None,
        }
    }
}

/// Parameters for updating an existing `KnoxAction` in the knowledge graph.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct UpdateActionParams {
    /// Unique identifier of the action node to update.
    pub id: ActionId,
    /// Updated concise summary.
    #[serde(default)]
    pub summary: Option<String>,
    /// Updated categorical action type.
    #[serde(default)]
    pub action_type: Option<String>,
    /// Updated actor attribution.
    #[serde(default)]
    pub actor: Option<String>,
    /// Updated relative files touched or referenced.
    #[serde(default)]
    pub files: Option<Vec<String>>,
    /// Updated semantic tags.
    #[serde(default)]
    pub tags: Option<Vec<String>>,
    /// Updated code symbols.
    #[serde(default)]
    pub symbols: Option<Vec<String>>,
    /// Updated architectural rationale.
    #[serde(default)]
    pub rationale: Option<String>,
    /// Updated context summary.
    #[serde(default)]
    pub context_summary: Option<String>,
    /// Updated rejected alternatives.
    #[serde(default)]
    pub rejected_alternatives: Option<Vec<String>>,
    /// Updated significance level ("major" | "minor").
    #[serde(default)]
    pub significance: Option<String>,
    /// Updated resolution status for doubts.
    #[serde(default)]
    pub is_resolved: Option<bool>,
    /// Updated resolution note.
    #[serde(default)]
    pub resolution_note: Option<String>,
    /// Updated direct parent action id (pass Some(None) to clear parent).
    #[serde(default)]
    pub parent_action_id: Option<Option<ActionId>>,
    /// Replaces all semantic relations if provided.
    #[serde(default)]
    pub relations: Option<Vec<ActionRelation>>,
    /// Adds specific semantic relations without replacing existing ones.
    #[serde(default)]
    pub add_relations: Option<Vec<ActionRelation>>,
    /// Removes specific target action relations if provided.
    #[serde(default)]
    pub remove_relations: Option<Vec<String>>,
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
    /// Filter by categorical action type (e.g. "decide", "write", "plan").
    #[serde(default)]
    pub action_type: Option<String>,
    /// Filter by touched code symbols (must contain any of these symbols).
    #[serde(default)]
    pub symbols: Vec<String>,
    /// Optional keyword search query across summaries, rationales, and context.
    #[serde(default)]
    pub query: Option<String>,
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
            action_type: None,
            symbols: Vec::new(),
            query: None,
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
    /// Number of duplicate actions skipped during ingestion.
    #[serde(default)]
    pub duplicates_skipped: usize,
    /// Name of the detected tool/agent format that was ingested.
    #[serde(default)]
    pub tool_detected: String,
}

/// Dry-run preview summary of a transcript before committing to the database.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct IngestPreview {
    /// Identifier or name of the detected tool (e.g. "antigravity", "claude", "cursor", "git").
    pub tool_detected: String,
    /// Total number of actions parsed from the transcript.
    pub total_actions: usize,
    /// Total number of plan actions.
    pub plans_count: usize,
    /// Total number of file write modifications.
    pub writes_count: usize,
    /// Total number of debug/verification executions.
    pub debugs_count: usize,
    /// Total number of architectural decisions.
    pub decisions_count: usize,
    /// List of unique files referenced across all parsed actions.
    pub files_touched: Vec<String>,
    /// Number of actions that already exist in the knowledge graph.
    pub duplicates_count: usize,
    /// Sample preview actions (first N actions for visual staging).
    pub sample_actions: Vec<KnoxAction>,
}

/// Information about a detected local agent transcript file.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DetectedTranscript {
    /// File path to the detected transcript.
    pub path: String,
    /// Source or tool type (e.g. "Antigravity", "Claude Code", "Cursor", "Git History").
    pub source: String,
    /// File size in bytes (or 0 for synthetic sources like Git history).
    pub size_bytes: u64,
    /// Last modified ISO timestamp.
    pub last_modified: String,
    /// Estimated number of actions or commits available.
    pub estimated_actions: usize,
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
            ..Default::default()
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
