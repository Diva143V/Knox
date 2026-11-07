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
    /// This action directly contradicts an established architectural decision.
    Contradicts,
    /// Direct causal origin for this action.
    CausedBy,
    /// Replaces or supersedes an earlier decision or action.
    Supersedes,
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
            Self::Contradicts => write!(f, "contradicts"),
            Self::CausedBy => write!(f, "caused_by"),
            Self::Supersedes => write!(f, "supersedes"),
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
            "contradicts" | "contradict" => Ok(Self::Contradicts),
            "caused_by" | "causedby" => Ok(Self::CausedBy),
            "supersedes" | "supersede" => Ok(Self::Supersedes),
            other => Err(format!("Unknown relation type: {other}")),
        }
    }
}

/// Target size limits for code deltas.
pub const MAX_DIFF_LINES: usize = 250;
pub const MAX_HUNK_BYTES: usize = 16_384; // 16 KiB limit
pub const MAX_DELETED_SNIPPET_BYTES: usize = 4_096; // 4 KiB limit
pub const MAX_DELTA_FILE_BYTES: usize = 512 * 1024; // 512 KiB limit

/// Type of filesystem or git modification recorded in an action delta.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeltaChangeType {
    /// New file created or added.
    Added,
    /// Existing file modified.
    Modified,
    /// File removed or deleted.
    Deleted,
    /// File renamed or moved.
    Renamed,
}

impl std::fmt::Display for DeltaChangeType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Added => write!(f, "added"),
            Self::Modified => write!(f, "modified"),
            Self::Deleted => write!(f, "deleted"),
            Self::Renamed => write!(f, "renamed"),
        }
    }
}

impl std::str::FromStr for DeltaChangeType {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "added" | "add" => Ok(Self::Added),
            "modified" | "modify" | "mod" => Ok(Self::Modified),
            "deleted" | "delete" | "del" => Ok(Self::Deleted),
            "renamed" | "rename" => Ok(Self::Renamed),
            other => Err(format!("Unknown change type: {other}")),
        }
    }
}

/// A granular code-level diff hunk or deleted code snippet associated with an action.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ActionDelta {
    /// Unique identifier for this delta record.
    pub id: String,
    /// Action foreign key this delta belongs to.
    pub action_id: ActionId,
    /// Relative path of the affected file.
    pub file_path: String,
    /// Nature of modification (added, modified, deleted, renamed).
    pub change_type: DeltaChangeType,
    /// Previous path if file was renamed.
    pub old_path: Option<String>,
    /// Number of lines added.
    pub lines_added: u32,
    /// Number of lines deleted.
    pub lines_deleted: u32,
    /// Bounded unified diff hunk patch.
    pub diff_hunk: Option<String>,
    /// Snippet of deleted code if logic was removed.
    pub deleted_code: Option<String>,
    /// Architectural rationale explaining why code was deleted.
    pub deletion_reason: Option<String>,
    /// Creation timestamp.
    pub created_at: DateTime<Utc>,
}

/// Input parameters for recording an action delta via MCP or tool call.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ActionDeltaInput {
    /// Relative path of the affected file.
    pub file_path: String,
    /// Change type (e.g. "added", "modified", "deleted", "renamed").
    pub change_type: String,
    /// Previous path if renamed.
    #[serde(default)]
    pub old_path: Option<String>,
    /// Explicit line count added (optional; derived if omitted).
    #[serde(default)]
    pub lines_added: Option<u32>,
    /// Explicit line count deleted (optional; derived if omitted).
    #[serde(default)]
    pub lines_deleted: Option<u32>,
    /// Unified diff hunk patch.
    #[serde(default)]
    pub diff_hunk: Option<String>,
    /// Deleted code snippet.
    #[serde(default)]
    pub deleted_code: Option<String>,
    /// Architectural reason for code removal.
    #[serde(default)]
    pub deletion_reason: Option<String>,
}

/// Summary of code delta statistics for an action.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeltaSummary {
    /// Total number of distinct files touched in this action.
    pub total_files: usize,
    /// Aggregated lines added across all files.
    pub lines_added: u32,
    /// Aggregated lines deleted across all files.
    pub lines_deleted: u32,
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
    /// Project scope resolved at write time. Populated even for session-less
    /// writes so project-filtered queries can see hook and ingest actions.
    #[serde(default)]
    pub project_id: Option<ProjectId>,
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
    /// Optional Git commit SHA if recorded or tied to a commit.
    #[serde(default)]
    pub commit_sha: Option<String>,
    /// Aggregated code change statistics for this action.
    #[serde(default)]
    pub delta_summary: Option<DeltaSummary>,
    /// Stable key used to merge exact duplicate records within the dedupe window.
    #[serde(default)]
    pub dedupe_key: Option<String>,
    /// Number of times an equivalent record has been observed.
    #[serde(default = "default_seen_count")]
    pub seen_count: u32,
}

/// Default observation count for actions created before dedupe existed.
fn default_seen_count() -> u32 {
    1
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
            project_id: None,
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
            commit_sha: None,
            delta_summary: None,
            dedupe_key: None,
            seen_count: 1,
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

/// Canonical edge types recognized by Knox.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EdgeType {
    CausedBy,
    Contradicts,
    Implements,
    DependsOn,
    Verifies,
    RelatesTo,
    Reverts,
    Supersedes,
    Resolves,
}

impl std::fmt::Display for EdgeType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::CausedBy => write!(f, "caused_by"),
            Self::Contradicts => write!(f, "contradicts"),
            Self::Implements => write!(f, "implements"),
            Self::DependsOn => write!(f, "depends_on"),
            Self::Verifies => write!(f, "verifies"),
            Self::RelatesTo => write!(f, "relates_to"),
            Self::Reverts => write!(f, "reverts"),
            Self::Supersedes => write!(f, "supersedes"),
            Self::Resolves => write!(f, "resolves"),
        }
    }
}

impl std::str::FromStr for EdgeType {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "caused_by" | "causedby" => Ok(Self::CausedBy),
            "contradicts" | "contradict" => Ok(Self::Contradicts),
            "implements" | "implement" => Ok(Self::Implements),
            "depends_on" | "dependson" | "depends" => Ok(Self::DependsOn),
            "verifies" | "verify" | "tests" | "test" => Ok(Self::Verifies),
            "relates_to" | "relatesto" | "relates" => Ok(Self::RelatesTo),
            "reverts" | "revert" | "fixes" | "fix" => Ok(Self::Reverts),
            "supersedes" | "supersede" => Ok(Self::Supersedes),
            "resolves" | "resolve" | "answers" | "answer" => Ok(Self::Resolves),
            other => Err(format!("Unknown edge type: {other}")),
        }
    }
}

/// A persisted edge record linking an action to an action or a reference target.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Edge {
    /// Unique identifier for this edge.
    pub id: String,
    /// Originating action identifier.
    pub from_action_id: ActionId,
    /// Target identifier (either an action ID or a ref ID).
    pub to_id: String,
    /// Canonical edge relation type.
    pub edge_type: String,
    /// Timestamp when this edge was established.
    pub created_at: DateTime<Utc>,
    /// Optional semantic label or context tag.
    pub label: Option<String>,
    /// Optional provenance tracking (e.g. git commit sha, transcript source).
    pub provenance: Option<String>,
    /// Optional annotation or resolution note.
    pub note: Option<String>,
}

/// A reference entity representing an external or non-action target (e.g. file, symbol).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Ref {
    /// Unique identifier for this ref (e.g., "file:src/db.rs" or "symbol:KnoxDb::open").
    pub id: String,
    /// Kind of entity ("file", "symbol", "doc", "url").
    pub kind: String,
    /// Exact target value.
    pub value: String,
    /// Scoped project identifier.
    #[serde(default)]
    pub project_id: Option<ProjectId>,
    /// Timestamp when this ref was registered.
    pub created_at: DateTime<Utc>,
}

/// An edge rejection entry recorded when an agent attempts an invalid relation.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EdgeRejection {
    /// Unique rejection identifier.
    pub id: String,
    /// ISO 8601 timestamp string.
    pub ts: String,
    /// Project identifier if scoped.
    pub project_id: Option<String>,
    /// Attempted source identifier.
    pub from_id: Option<String>,
    /// Attempted target identifier.
    pub to_id: Option<String>,
    /// Attempted relation or edge type.
    pub attempted_type: String,
    /// Explanatory reason or teaching error message.
    pub reason: String,
}

/// Directed edge connecting nodes in the in-memory graph.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct KnoxEdge {
    /// Relationship type (e.g., "caused_by", "parent", "supersedes", "targets").
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
    /// Optional normalized Git remote URL.
    #[serde(default)]
    pub remote: Option<String>,
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
    /// Resolved project identifier, if the session is project-scoped.
    #[serde(default)]
    pub project_id: Option<ProjectId>,
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
    /// When true, `file_path` matches the exact path only (no directory prefix
    /// expansion). Used by guardrails to query a directory scope deliberately.
    #[serde(default)]
    pub file_path_exact: bool,
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
    /// Optional pagination cursor encoding `(timestamp_ms, id)` in base64.
    #[serde(default)]
    pub cursor: Option<String>,
}

impl Default for QueryFilter {
    fn default() -> Self {
        Self {
            file_path: None,
            file_path_exact: false,
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
            cursor: None,
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
    /// Optional cursor to retrieve the next page of results.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub next: Option<String>,
    /// Specific reason if the result set is empty ("no_records", "no_match", "out_of_scope").
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

/// An entry in the operational query telemetry log.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct QueryLogEntry {
    /// Unique query log identifier.
    pub id: String,
    /// Timestamp in milliseconds since epoch.
    pub ts_ms: i64,
    /// Name of the tool or command executed.
    pub tool: String,
    /// Project identifier if scoped.
    pub project_id: Option<String>,
    /// Number of action rows returned.
    pub rows: usize,
    /// Estimated tokens consumed.
    pub tokens: usize,
    /// Execution latency in milliseconds.
    pub latency_ms: u64,
    /// Specific reason if result was empty ("no_records", "no_match", "out_of_scope").
    pub reason: Option<String>,
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
            project_id: Some(ProjectId::new()),
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
