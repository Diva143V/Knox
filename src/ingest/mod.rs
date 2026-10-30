//! Transcript ingestion and mapping pipelines for Knox.
//!
//! Provides the `TranscriptMapper` trait and tool-specific mappers (starting with Antigravity)
//! to ingest conversational transcripts into persistent knowledge graph actions.

pub mod action_group;
pub mod antigravity;
pub mod mapper;
pub mod summary;

use std::path::Path;

use crate::core::error::{KnoxError, KnoxResult};
use crate::core::types::KnoxAction;

pub use action_group::*;
pub use antigravity::*;
pub use mapper::*;
pub use summary::*;

/// Returns an ordered list of all registered transcript mappers.
pub fn all_mappers() -> Vec<Box<dyn TranscriptMapper>> {
    vec![Box::new(antigravity::AntigravityMapper)]
}

/// Automatically detects the appropriate mapper for a transcript file and maps it into actions.
pub fn ingest_file(path: &Path) -> KnoxResult<Vec<KnoxAction>> {
    let bytes = std::fs::read(path)?;

    for mapper in all_mappers() {
        if mapper.can_parse(&bytes) {
            return mapper.map(&bytes);
        }
    }

    Err(KnoxError::ParseError(format!(
        "No suitable transcript mapper found for file: {}",
        path.display()
    )))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_all_mappers_contains_antigravity() {
        let mappers = all_mappers();
        assert!(!mappers.is_empty());
        assert_eq!(mappers[0].tool_name(), "antigravity");
    }

    #[test]
    fn test_ingest_file_auto_detection() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(
            file,
            r#"{{"step_index":0,"source":"USER_EXPLICIT","type":"USER_INPUT","content":"Build feature"}}"#
        )
        .unwrap();
        writeln!(
            file,
            r#"{{"step_index":1,"source":"MODEL","type":"PLANNER_RESPONSE","content":"Feature built"}}"#
        )
        .unwrap();

        let actions = ingest_file(file.path()).unwrap();
        assert_eq!(actions.len(), 2);
        assert_eq!(actions[0].summary, "Build feature");
        assert_eq!(actions[1].summary, "Feature built");
    }

    #[test]
    fn test_ingest_file_unsupported_format() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "unsupported content").unwrap();

        let res = ingest_file(file.path());
        assert!(res.is_err());
    }
}
