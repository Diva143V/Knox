//! Core shared types, error definitions, identifiers, and configuration for Knox.

pub mod config;
pub mod budget;
pub mod error;
pub mod guardrails;
pub mod ids;
pub mod types;

pub use config::{
    DEFAULT_DB_FILENAME, DEFAULT_KNOX_DIR, DEFAULT_SESSION_TIMEOUT, KnoxConfig, expand_tilde,
};
pub use error::{KnoxError, KnoxResult};
pub use ids::{ActionId, ProjectId, SessionId};
pub use types::{
    ActionDelta, ActionDeltaInput, DeltaChangeType, DeltaSummary, Edge, EdgeRejection, EdgeType,
    IngestReport, KnoxAction, KnoxEdge, Project, QueryFilter, QueryLogEntry, QueryResult, Ref,
    ScanReport, Session, StatusReport,
};
