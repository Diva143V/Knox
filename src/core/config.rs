//! Configuration management and default settings for Knox.

use serde::{Deserialize, Serialize};
use std::env;
use std::path::{Path, PathBuf};
use std::time::Duration;

/// Default session idle timeout before automatic closure (30 minutes).
pub const DEFAULT_SESSION_TIMEOUT: Duration = Duration::from_secs(30 * 60);

/// Default directory name used for Knox data.
pub const DEFAULT_KNOX_DIR: &str = ".knox";

/// Default SQLite database filename.
pub const DEFAULT_DB_FILENAME: &str = "knox.db";

/// Central configuration for Knox operations and storage.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct KnoxConfig {
    /// Filesystem path to the SQLite knowledge graph database file.
    pub kg_path: PathBuf,
    /// Duration of inactivity before an open session is considered stale.
    pub session_timeout: Duration,
    /// If true, uses a global shared database across projects by default.
    pub global_mode: bool,
}

impl KnoxConfig {
    /// Creates a configuration by inspecting `KNOX_*` environment variables,
    /// falling back to standard defaults for any unset values.
    pub fn from_env() -> Self {
        let mut config = Self::default();

        if let Ok(path_str) = env::var("KNOX_KG_PATH").or_else(|_| env::var("KNOX_DB_PATH")) {
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
        }

        config
    }
}

impl Default for KnoxConfig {
    fn default() -> Self {
        Self {
            kg_path: default_global_db_path(),
            session_timeout: DEFAULT_SESSION_TIMEOUT,
            global_mode: true,
        }
    }
}

/// Computes the default global SQLite database path (`~/.knox/knox.db`).
fn default_global_db_path() -> PathBuf {
    if let Some(home) = home_dir() {
        home.join(DEFAULT_KNOX_DIR).join(DEFAULT_DB_FILENAME)
    } else {
        PathBuf::from(DEFAULT_KNOX_DIR).join(DEFAULT_DB_FILENAME)
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
}
