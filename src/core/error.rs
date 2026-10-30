//! Error types and result definitions for Knox.

/// Central error enum representing all domain and operational failures in Knox.
#[derive(Debug, thiserror::Error)]
pub enum KnoxError {
    /// Errors originating from the SQLite database.
    #[error("Database error: {0}")]
    Database(#[from] rusqlite::Error),

    /// Requested session could not be located.
    #[error("Session not found: {0}")]
    SessionNotFound(String),

    /// Action performed on a session that has already concluded.
    #[error("Session already closed: {0}")]
    SessionClosed(String),

    /// Action node could not be found by its identifier.
    #[error("Action not found: {0}")]
    ActionNotFound(String),

    /// Arguments or parameters provided were invalid.
    #[error("Invalid input: {0}")]
    InvalidInput(String),

    /// Failed to parse a transcript during ingestion.
    #[error("Transcript parse error: {0}")]
    ParseError(String),

    /// Standard I/O failure.
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Serialization or deserialization failure.
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    /// Git command or repository inspection failure.
    #[error("Git error: {0}")]
    Git(String),
}

/// Convenience result alias for operations returning a `KnoxError`.
pub type KnoxResult<T> = Result<T, KnoxError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display_formatting() {
        let err = KnoxError::SessionNotFound("session-404".to_string());
        assert_eq!(err.to_string(), "Session not found: session-404");

        let err = KnoxError::ActionNotFound("action-123".to_string());
        assert_eq!(err.to_string(), "Action not found: action-123");

        let err = KnoxError::InvalidInput("missing field".to_string());
        assert_eq!(err.to_string(), "Invalid input: missing field");

        let err = KnoxError::Git("detached head".to_string());
        assert_eq!(err.to_string(), "Git error: detached head");
    }

    #[test]
    fn test_error_from_conversions() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file missing");
        let knox_err: KnoxError = io_err.into();
        assert!(matches!(knox_err, KnoxError::Io(_)));

        let json_err: serde_json::Error =
            serde_json::from_str::<String>("invalid json").unwrap_err();
        let knox_err: KnoxError = json_err.into();
        assert!(matches!(knox_err, KnoxError::Json(_)));
    }
}
