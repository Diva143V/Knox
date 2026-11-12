//! Configuration management and default settings for Knox.

use serde::{Deserialize, Serialize};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use crate::core::error::KnoxResult;

/// Default session idle timeout before automatic closure (30 minutes).
pub const DEFAULT_SESSION_TIMEOUT: Duration = Duration::from_secs(30 * 60);

/// Default directory name used for Knox data.
pub const DEFAULT_KNOX_DIR: &str = ".knox";

/// Default SQLite database filename.
pub const DEFAULT_DB_FILENAME: &str = "knox.db";

/// Default configuration filename.
pub const DEFAULT_CONFIG_FILENAME: &str = "config.json";

/// Central configuration for Knox operations and storage.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct KnoxConfig {
    /// Filesystem path to the SQLite knowledge graph database file.
    pub kg_path: PathBuf,
    /// Duration of inactivity before an open session is considered stale.
    pub session_timeout: Duration,
    /// If true, uses a global shared database across projects by default.
    pub global_mode: bool,
    /// Optional default transcript path for auto-capture.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub transcript_path: Option<PathBuf>,
    /// Active model profile: "off" (default), "mock", "local", "cloud".
    #[serde(default = "default_model_profile")]
    pub model_profile: String,
    /// Base URL endpoint for local or cloud model provider (e.g. http://localhost:11434).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model_endpoint: Option<String>,
    /// Optional API key for cloud model provider.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model_api_key: Option<String>,
    /// Model identifier for embeddings (e.g. "nomic-embed-text", "text-embedding-3-small").
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub embedding_model: Option<String>,
    /// Model identifier for text chat and synthesis (e.g. "llama3.2", "gpt-4o-mini").
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub chat_model: Option<String>,
    /// Explicit opt-in permission flag for outbound cloud model requests.
    #[serde(default)]
    pub allow_cloud_egress: bool,
    /// Persistent flag indicating if the stray database consolidation prompt has been presented.
    #[serde(default)]
    pub stray_db_prompted: bool,
}

fn default_model_profile() -> String {
    "off".to_string()
}

impl KnoxConfig {
    /// Creates a configuration by inspecting `config.json` first, then `KNOX_*` environment variables,
    /// falling back to standard defaults for any unset values.
    pub fn from_env() -> Self {
        let config_path = env::var("KNOX_CONFIG_PATH")
            .map(expand_tilde)
            .unwrap_or_else(|_| default_config_file_path());

        let mut config = Self::load_from_file(&config_path).unwrap_or_default();

        if let Ok(path_str) = env::var("KNOX_PATH")
            .or_else(|_| env::var("KNOX_KG_PATH"))
            .or_else(|_| env::var("KNOX_DB_PATH"))
        {
            if !path_str.trim().is_empty() {
                config.kg_path = expand_tilde(path_str.trim());
            }
        }

        if let Ok(timeout_str) = env::var("KNOX_SESSION_TIMEOUT") {
            if let Ok(secs) = timeout_str.trim().parse::<u64>() {
                config.session_timeout = Duration::from_secs(secs);
            }
        }

        if let Ok(global_str) = env::var("KNOX_GLOBAL_MODE") {
            let lower = global_str.trim().to_lowercase();
            config.global_mode = matches!(lower.as_str(), "1" | "true" | "yes");
            if !config.global_mode {
                tracing::warn!("KNOX_GLOBAL_MODE is deprecated and will be removed in future releases. Forcing global mode.");
            }
        }

        if let Ok(transcript_str) = env::var("KNOX_TRANSCRIPT_PATH") {
            if !transcript_str.trim().is_empty() {
                config.transcript_path = Some(expand_tilde(transcript_str.trim()));
            }
        }

        if let Ok(profile_str) = env::var("KNOX_MODEL_PROFILE") {
            let trimmed = profile_str.trim();
            if !trimmed.is_empty() {
                config.model_profile = trimmed.to_lowercase();
            }
        }

        if let Ok(endpoint) = env::var("KNOX_MODEL_ENDPOINT") {
            let trimmed = endpoint.trim();
            if !trimmed.is_empty() {
                config.model_endpoint = Some(trimmed.to_string());
            }
        }

        if let Ok(key) = env::var("KNOX_MODEL_API_KEY") {
            let trimmed = key.trim();
            if !trimmed.is_empty() {
                config.model_api_key = Some(trimmed.to_string());
            }
        }

        if let Ok(m) = env::var("KNOX_EMBEDDING_MODEL") {
            let trimmed = m.trim();
            if !trimmed.is_empty() {
                config.embedding_model = Some(trimmed.to_string());
            }
        }

        if let Ok(m) = env::var("KNOX_CHAT_MODEL") {
            let trimmed = m.trim();
            if !trimmed.is_empty() {
                config.chat_model = Some(trimmed.to_string());
            }
        }

        if let Ok(egress) = env::var("KNOX_ALLOW_CLOUD_EGRESS") {
            let trimmed = egress.trim().to_lowercase();
            config.allow_cloud_egress = trimmed == "1" || trimmed == "true" || trimmed == "yes";
        }

        config
    }

    /// Loads configuration from a JSON file on disk, if present.
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> KnoxResult<Self> {
        let p = path.as_ref();
        let content = fs::read_to_string(p)?;
        let config: Self = serde_json::from_str(&content)?;
        Ok(config)
    }

    /// Saves current configuration to the specified JSON file path, creating directories as needed.
    pub fn save_to_file<P: AsRef<Path>>(&self, path: P) -> KnoxResult<()> {
        let p = path.as_ref();
        if let Some(parent) = p.parent() {
            if !parent.as_os_str().is_empty() {
                fs::create_dir_all(parent)?;
            }
        }
        let json = serde_json::to_string_pretty(self)?;
        fs::write(p, json)?;
        Ok(())
    }

    /// Saves current configuration to the default config path (`~/.knox/config.json`).
    pub fn save_default(&self) -> KnoxResult<()> {
        self.save_to_file(default_config_file_path())
    }

    /// Resolves the database used by runtime commands.
    ///
    /// Following the unified storage model, Knox always routes to the shared
    /// global database (`~/.knox/knox.db`), unless an explicit `kg_path` override was configured.
    pub fn resolve_db_path(&self) -> PathBuf {
        if !self.global_mode {
            tracing::warn!("KNOX_GLOBAL_MODE is deprecated and will be removed in future releases. Forcing global mode.");
        }
        self.kg_path.clone()
    }

    /// Resolves the database path for a specific project (used by `knox init`).
    /// Aligns with `resolve_db_path` to guarantee consistent global database routing.
    pub fn resolve_db_path_for(&self, _project_path: &Path) -> PathBuf {
        self.resolve_db_path()
    }

    /// True when `kg_path` was set explicitly rather than left at its default.
    pub fn has_explicit_kg_path(&self) -> bool {
        self.kg_path != default_global_db_path()
    }
}

/// Scans upwards from `start` to detect an existing legacy project-local database file.
pub fn detect_project_db(start: &Path) -> Option<PathBuf> {
    let mut current = Some(start);
    while let Some(dir) = current {
        let candidate = dir.join(DEFAULT_KNOX_DIR).join(DEFAULT_DB_FILENAME);
        if candidate.exists() {
            return Some(candidate);
        }
        current = dir.parent();
    }
    None
}

/// Walks up from `start` to find an existing project-local database, falling
/// back to `<start>/.knox/knox.db`.
pub fn find_project_db_path(start: &Path) -> PathBuf {
    detect_project_db(start).unwrap_or_else(|| start.join(DEFAULT_KNOX_DIR).join(DEFAULT_DB_FILENAME))
}

impl Default for KnoxConfig {
    fn default() -> Self {
        Self {
            kg_path: default_global_db_path(),
            session_timeout: DEFAULT_SESSION_TIMEOUT,
            global_mode: true,
            transcript_path: None,
            model_profile: "off".to_string(),
            model_endpoint: None,
            model_api_key: None,
            embedding_model: None,
            chat_model: None,
            allow_cloud_egress: false,
            stray_db_prompted: false,
        }
    }
}

/// Computes the default global SQLite database path (`~/.knox/knox.db`).
pub fn default_global_db_path() -> PathBuf {
    if let Some(home) = home_dir() {
        home.join(DEFAULT_KNOX_DIR).join(DEFAULT_DB_FILENAME)
    } else {
        PathBuf::from(DEFAULT_KNOX_DIR).join(DEFAULT_DB_FILENAME)
    }
}

/// Computes the default configuration file path (`~/.knox/config.json`).
pub fn default_config_file_path() -> PathBuf {
    if let Some(home) = home_dir() {
        home.join(DEFAULT_KNOX_DIR).join(DEFAULT_CONFIG_FILENAME)
    } else {
        PathBuf::from(DEFAULT_KNOX_DIR).join(DEFAULT_CONFIG_FILENAME)
    }
}

/// Expands a leading `~` or `~/` in a path string to the user's home directory.
pub fn expand_tilde<P: AsRef<Path>>(path: P) -> PathBuf {
    let p = path.as_ref();
    if let Ok(stripped) = p.strip_prefix("~") {
        if let Some(home) = home_dir() {
            let relative = stripped
                .strip_prefix("/")
                .or_else(|_| stripped.strip_prefix("\\"))
                .unwrap_or(stripped);
            return home.join(relative);
        }
    }
    p.to_path_buf()
}

/// Returns the user's home directory across Windows, macOS, and Linux.
pub fn home_dir() -> Option<PathBuf> {
    env::var("USERPROFILE")
        .or_else(|_| env::var("HOME"))
        .or_else(|_| {
            let drive = env::var("HOMEDRIVE");
            let path = env::var("HOMEPATH");
            match (drive, path) {
                (Ok(d), Ok(p)) => Ok(format!("{}{}", d, p)),
                _ => Err(env::VarError::NotPresent),
            }
        })
        .ok()
        .map(PathBuf::from)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = KnoxConfig::default();
        assert_eq!(config.session_timeout, DEFAULT_SESSION_TIMEOUT);
        assert!(config.global_mode);
        assert!(!config.stray_db_prompted);
        assert!(
            config
                .kg_path
                .ends_with(Path::new(DEFAULT_KNOX_DIR).join(DEFAULT_DB_FILENAME))
        );
    }

    #[test]
    fn test_expand_tilde() {
        let home = home_dir().unwrap_or_else(|| PathBuf::from("/home/user"));
        let expanded = expand_tilde("~/.knox/knox.db");
        assert_eq!(expanded, home.join(".knox").join("knox.db"));

        let normal = expand_tilde("/tmp/knox.db");
        assert_eq!(normal, PathBuf::from("/tmp/knox.db"));
    }

    #[test]
    fn test_config_serialization() {
        let config = KnoxConfig::default();
        let json = serde_json::to_string(&config).unwrap();
        let deserialized: KnoxConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(config, deserialized);
    }

    #[test]
    fn test_config_file_save_and_load() {
        let temp_dir = tempfile::tempdir().unwrap();
        let file_path = temp_dir.path().join("sub").join("test_config.json");

        let config = KnoxConfig {
            kg_path: PathBuf::from("/custom/knox.db"),
            session_timeout: Duration::from_secs(7200),
            global_mode: false,
            transcript_path: None,
            model_profile: "mock".to_string(),
            model_endpoint: Some("http://localhost:11434".to_string()),
            model_api_key: None,
            embedding_model: Some("nomic-embed-text".to_string()),
            chat_model: Some("llama3.2".to_string()),
            allow_cloud_egress: false,
            stray_db_prompted: false,
        };

        config.save_to_file(&file_path).unwrap();
        assert!(file_path.exists());

        let loaded = KnoxConfig::load_from_file(&file_path).unwrap();
        assert_eq!(config, loaded);
    }
}
