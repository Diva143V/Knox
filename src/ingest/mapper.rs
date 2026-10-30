//! Transcript mapper trait for converting coding agent conversation transcripts into Knox actions.

use crate::core::error::KnoxResult;
use crate::core::types::KnoxAction;

/// Trait implemented by tool-specific transcript parsers (e.g., Antigravity, Cursor, Copilot).
pub trait TranscriptMapper: Send + Sync {
    /// Returns the unique identifier of the tool or IDE that generated the transcript.
    fn tool_name(&self) -> &str;

    /// Checks if this mapper can parse the provided raw transcript data.
    fn can_parse(&self, raw: &[u8]) -> bool;

    /// Maps raw transcript data into a sequence of Knox actions.
    fn map(&self, raw: &[u8]) -> KnoxResult<Vec<KnoxAction>>;
}
