//! Identifier wrapper types for Knox.

use serde::{Deserialize, Serialize};
use std::fmt;

/// Unique identifier for an action in the knowledge graph.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct ActionId(pub String);

impl ActionId {
    /// Generates a new random `ActionId` using UUID v4.
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4().to_string())
    }

    /// Returns a string slice representing the ID.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Default for ActionId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for ActionId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<String> for ActionId {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl From<&str> for ActionId {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

impl AsRef<str> for ActionId {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

/// Unique identifier for a continuous work session.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct SessionId(pub String);

impl SessionId {
    /// Generates a new random `SessionId` using UUID v4.
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4().to_string())
    }

    /// Returns a string slice representing the ID.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Default for SessionId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for SessionId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<String> for SessionId {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl From<&str> for SessionId {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

impl AsRef<str> for SessionId {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

/// Unique identifier for a project tracked by Knox.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct ProjectId(pub String);

impl ProjectId {
    /// Generates a new random `ProjectId` using UUID v4.
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4().to_string())
    }

    /// Returns a string slice representing the ID.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Default for ProjectId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for ProjectId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<String> for ProjectId {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl From<&str> for ProjectId {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

impl AsRef<str> for ProjectId {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_action_id_generation_and_display() {
        let id = ActionId::new();
        assert!(!id.as_str().is_empty());
        assert_eq!(id.to_string(), id.as_str());
    }

    #[test]
    fn test_session_id_from_and_as_ref() {
        let raw = "test-session-123";
        let session_id = SessionId::from(raw);
        assert_eq!(session_id.as_ref(), raw);
        assert_eq!(session_id.as_str(), raw);
    }

    #[test]
    fn test_project_id_serialization() {
        let id = ProjectId::from("proj-abc");
        let json = serde_json::to_string(&id).unwrap();
        let deserialized: ProjectId = serde_json::from_str(&json).unwrap();
        assert_eq!(id, deserialized);
    }
}
