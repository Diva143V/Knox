//! Error types and result definitions for Knox.

/// Central error enum representing all domain and operational failures in Knox.
#[derive(Debug, thiserror::Error)]
pub enum KnoxError {
    /// Errors originating from the SQLite database.
    #[error("Database error: {0}")]
    Database(rusqlite::Error),

    /// The database was locked by another writer and the operation may be retried.
    #[error("Database is busy (another Knox process is writing); retry the operation: {0}")]
    DatabaseBusy(String),

    /// A project identifier matched more than one project row.
    #[error(
        "Ambiguous project identifier '{0}': matches multiple projects; pass the absolute path instead"
    )]
    AmbiguousProject(String),

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

    /// The same idempotency key was reused with a different payload.
    #[error("IDEM_CONFLICT: idempotency key reused with a different payload: {0}")]
    IdemConflict(String),

    /// A text field exceeded its documented cap.
    #[error("OVERSIZE_INPUT: {0}")]
    OversizeInput(String),

    /// A local reference in a batch could not be resolved.
    #[error("INVALID_REF: {0}")]
    InvalidRef(String),

    /// An edge pointed at an action in another project.
    #[error("CROSS_PROJECT: {0}")]
    CrossProject(String),

    /// A causal link would close a cycle.
    #[error("CYCLE: {0}")]
    Cycle(String),

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

impl From<rusqlite::Error> for KnoxError {
    fn from(err: rusqlite::Error) -> Self {
        match err {
            rusqlite::Error::SqliteFailure(code, message)
                if code.code == rusqlite::ErrorCode::DatabaseBusy
                    || code.code == rusqlite::ErrorCode::DatabaseLocked =>
            {
                KnoxError::DatabaseBusy(message.unwrap_or_else(|| "database is locked".to_string()))
            }
            other => KnoxError::Database(other),
        }
    }
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
