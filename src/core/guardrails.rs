//! Secret sanitization, exclusion filters, and payload boundary enforcement for Knox deltas.

use regex::Regex;
use std::sync::LazyLock;

use super::types::{ActionDelta, MAX_DELETED_SNIPPET_BYTES, MAX_DIFF_LINES, MAX_HUNK_BYTES};

/// Common token patterns for secret detection and scrubbing.
static SECRET_PATTERNS: LazyLock<Vec<Regex>> = LazyLock::new(|| {
    vec![
        // GitHub Personal Access Tokens and OAuth tokens
        Regex::new(r"(?:ghp|gho|ghu|ghs)_[A-Za-z0-9]{36}").unwrap(),
        // OpenAI and similar standard API keys
        Regex::new(r"sk-[A-Za-z0-9_-]{20,}").unwrap(),
        // Google Cloud / Firebase API keys
        Regex::new(r"AIza[0-9A-Za-z-_]{35}").unwrap(),
        // AWS Access Key IDs
        Regex::new(r"AKIA[0-9A-Z]{16}").unwrap(),
        // Bearer tokens with high entropy
        Regex::new(r"(Bearer\s+)[A-Za-z0-9._~+/-]{20,}").unwrap(),
    ]
});

/// Determines if a file path represents a credential, binary, or dependency lockfile
/// that should be excluded from causal code delta capture.
pub fn is_file_excluded(file_path: &str) -> bool {
    let normalized = file_path.replace('\\', "/").to_ascii_lowercase();
    let filename = normalized.rsplit('/').next().unwrap_or(&normalized);

    // Environment and credential files
    if filename == ".env" || filename.starts_with(".env.") || filename.ends_with(".env") {
        return true;
    }
    if filename.ends_with(".pem")
        || filename.ends_with(".key")
        || filename.ends_with(".cert")
        || filename.ends_with(".crt")
        || filename.ends_with(".pfx")
        || filename.ends_with(".p12")
    {
        return true;
    }

    // Common package and dependency lockfiles
    if matches!(
        filename,
        "cargo.lock"
            | "package-lock.json"
            | "pnpm-lock.yaml"
            | "yarn.lock"
            | "poetry.lock"
            | "composer.lock"
            | "gemfile.lock"
    ) {
        return true;
    }

    // Binary and compiled asset extensions
    let binary_extensions = [
        ".exe", ".dll", ".so", ".dylib", ".wasm", ".bin", ".db", ".sqlite", ".sqlite3", ".png",
        ".jpg", ".jpeg", ".gif", ".ico", ".pdf", ".zip", ".tar", ".gz", ".7z", ".mp4", ".woff",
        ".woff2", ".ttf", ".eot",
    ];

    binary_extensions.iter().any(|ext| filename.ends_with(ext))
}

/// Redacts detected API keys and secrets from arbitrary text.
pub fn scrub_secrets(input: &str) -> String {
    let mut sanitized = input.to_string();

    for pattern in SECRET_PATTERNS.iter() {
        if pattern.as_str().starts_with("(Bearer") {
            sanitized = pattern.replace_all(&sanitized, "$1[REDACTED_SECRET]").into_owned();
        } else {
            sanitized = pattern.replace_all(&sanitized, "[REDACTED_SECRET]").into_owned();
        }
    }

    sanitized
}

/// Sanitizes and bounds an `ActionDelta` in-place before database persistence.
pub fn sanitize_delta(delta: &mut ActionDelta) {
    if let Some(hunk) = &mut delta.diff_hunk {
        let scrubbed = scrub_secrets(hunk);

        // Enforce MAX_DIFF_LINES ceiling
        let lines: Vec<&str> = scrubbed.lines().collect();
        let bounded = if lines.len() > MAX_DIFF_LINES {
            let mut truncated = lines[..MAX_DIFF_LINES].join("\n");
            truncated.push_str("\n... [diff truncated: exceeded line limit]");
            truncated
        } else {
            scrubbed
        };

        // Enforce MAX_HUNK_BYTES ceiling
        if bounded.len() > MAX_HUNK_BYTES {
            let mut byte_truncated = bounded[..MAX_HUNK_BYTES].to_string();
            byte_truncated.push_str("\n... [diff truncated: exceeded byte limit]");
            *hunk = byte_truncated;
        } else {
            *hunk = bounded;
        }
    }

    if let Some(snippet) = &mut delta.deleted_code {
        let scrubbed = scrub_secrets(snippet);

        // Enforce MAX_DELETED_SNIPPET_BYTES ceiling
        if scrubbed.len() > MAX_DELETED_SNIPPET_BYTES {
            let mut truncated = scrubbed[..MAX_DELETED_SNIPPET_BYTES].to_string();
            truncated.push_str("\n... [deleted code snippet truncated: exceeded byte limit]");
            *snippet = truncated;
        } else {
            *snippet = scrubbed;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::ids::ActionId;
    use crate::core::types::DeltaChangeType;
    use chrono::Utc;

    #[test]
    fn test_is_file_excluded() {
        assert!(is_file_excluded(".env"));
        assert!(is_file_excluded("config/.env.production"));
        assert!(is_file_excluded("certs/server.pem"));
        assert!(is_file_excluded("Cargo.lock"));
        assert!(is_file_excluded("ui/pnpm-lock.yaml"));
        assert!(is_file_excluded("target/debug/knox.exe"));
        assert!(is_file_excluded("assets/logo.png"));

        assert!(!is_file_excluded("src/lib.rs"));
        assert!(!is_file_excluded("Cargo.toml"));
        assert!(!is_file_excluded("src/mcp/handlers/record_action.rs"));
    }

    #[test]
    fn test_scrub_secrets() {
        let text = "Token ghp_123456789012345678901234567890123456 and Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.e30.t-IDcNV3 adhere.";
        let scrubbed = scrub_secrets(text);
        assert!(!scrubbed.contains("ghp_123456789012345678901234567890123456"));
        assert!(scrubbed.contains("[REDACTED_SECRET]"));
        assert!(scrubbed.contains("Bearer [REDACTED_SECRET]"));
    }

    #[test]
    fn test_sanitize_delta_line_truncation() {
        let long_hunk = (0..300)
            .map(|i| format!("+line {i} ghp_123456789012345678901234567890123456"))
            .collect::<Vec<_>>()
            .join("\n");

        let mut delta = ActionDelta {
            id: "d1".to_string(),
            action_id: ActionId::new(),
            file_path: "src/main.rs".to_string(),
            change_type: DeltaChangeType::Modified,
            old_path: None,
            lines_added: 300,
            lines_deleted: 0,
            diff_hunk: Some(long_hunk),
            deleted_code: None,
            deletion_reason: None,
            created_at: Utc::now(),
        };

        sanitize_delta(&mut delta);
        let hunk = delta.diff_hunk.unwrap();
        assert!(hunk.contains("[REDACTED_SECRET]"));
        assert!(hunk.contains("... [diff truncated: exceeded line limit]"));
        assert!(hunk.lines().count() <= MAX_DIFF_LINES + 2);
    }
}
