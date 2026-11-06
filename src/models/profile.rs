//! Model profile definitions and configuration.
//!
//! Knox enforces the **clean-machine promise**:
//! - Default profile is `Off`: requires zero GPU, zero local daemons, and zero API keys.
//! - When `Off`, the system outputs are 100% byte-identical to v2.1.
//! - `Mock` profile enables deterministic, fast, network-free evaluation in CI.
//! - `Local` profile connects to Ollama, LocalAI, or vLLM over local HTTP.
//! - `Cloud` profile connects to standard OpenAI-compatible cloud endpoints.

use serde::{Deserialize, Serialize};

use crate::core::config::KnoxConfig;

/// Supported model deployment profiles.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ModelProfileKind {
    /// Models disabled (Default). Zero external dependencies, pure heuristic fallbacks.
    #[default]
    Off,
    /// In-memory deterministic mock models for unit tests and CI.
    Mock,
    /// Local HTTP provider (e.g. Ollama at http://localhost:11434).
    Local,
    /// Remote OpenAI-compatible cloud provider.
    Cloud,
}

impl ModelProfileKind {
    pub fn from_str_loose(s: &str) -> Self {
        match s.trim().to_ascii_lowercase().as_str() {
            "mock" | "test" => Self::Mock,
            "local" | "ollama" => Self::Local,
            "cloud" | "remote" | "openai" => Self::Cloud,
            _ => Self::Off,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Off => "off",
            Self::Mock => "mock",
            Self::Local => "local",
            Self::Cloud => "cloud",
        }
    }

    pub fn is_enabled(&self) -> bool {
        !matches!(self, Self::Off)
    }
}

/// Resolved configuration for model router and adapters.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelProfile {
    pub kind: ModelProfileKind,
    pub endpoint: String,
    pub api_key: Option<String>,
    pub embedding_model: String,
    pub chat_model: String,
}

impl ModelProfile {
    /// Resolves model profile from KnoxConfig.
    pub fn from_config(config: &KnoxConfig) -> Self {
        let kind = ModelProfileKind::from_str_loose(&config.model_profile);
        let endpoint = config
            .model_endpoint
            .clone()
            .unwrap_or_else(|| match kind {
                ModelProfileKind::Local => "http://localhost:11434".to_string(),
                ModelProfileKind::Cloud => "https://api.openai.com/v1".to_string(),
                _ => String::new(),
            });

        let embedding_model = config
            .embedding_model
            .clone()
            .unwrap_or_else(|| match kind {
                ModelProfileKind::Local => "nomic-embed-text".to_string(),
                ModelProfileKind::Cloud => "text-embedding-3-small".to_string(),
                ModelProfileKind::Mock => "mock-embed-64d".to_string(),
                ModelProfileKind::Off => "none".to_string(),
            });

        let chat_model = config
            .chat_model
            .clone()
            .unwrap_or_else(|| match kind {
                ModelProfileKind::Local => "llama3.2".to_string(),
                ModelProfileKind::Cloud => "gpt-4o-mini".to_string(),
                ModelProfileKind::Mock => "mock-grounded-v1".to_string(),
                ModelProfileKind::Off => "none".to_string(),
            });

        Self {
            kind,
            endpoint,
            api_key: config.model_api_key.clone(),
            embedding_model,
            chat_model,
        }
    }

    /// Creates an explicit Mock profile for testing.
    pub fn mock() -> Self {
        Self {
            kind: ModelProfileKind::Mock,
            endpoint: "mock://in-memory".to_string(),
            api_key: None,
            embedding_model: "mock-embed-64d".to_string(),
            chat_model: "mock-grounded-v1".to_string(),
        }
    }

    /// Creates an explicit Off profile.
    pub fn off() -> Self {
        Self {
            kind: ModelProfileKind::Off,
            endpoint: String::new(),
            api_key: None,
            embedding_model: "none".to_string(),
            chat_model: "none".to_string(),
        }
    }
}

