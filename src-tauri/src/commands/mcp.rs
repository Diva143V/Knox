//! IPC command handlers for automated MCP configuration discovery,
//! schema detection, safe backup, and Knox stdio injection.

use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use tauri::Emitter;

/// Live scan log entry emitted during quick or deep scan.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct McpScanLog {
    pub level: String, // "info" | "scan" | "found" | "config" | "skip" | "error"
    pub message: String,
    pub timestamp: String,
}

/// Helper to emit live scan progress logs to the frontend via Tauri event.
fn emit_scan_log(app: Option<&tauri::AppHandle>, level: &str, message: &str) {
    if let Some(app) = app {
        let log = McpScanLog {
            level: level.to_string(),
            message: message.to_string(),
            timestamp: chrono::Local::now().format("%H:%M:%S%.3f").to_string(),
        };
        let _ = app.emit("mcp-scan-log", log);
    }
}

/// Information about a detected IDE or application's MCP configuration.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct DetectedMcpApp {
    pub id: String,
    pub name: String,
    pub category: String, // "ide" | "cli" | "extension" | "custom"
    pub file_path: String,
    pub exists: bool,
    pub is_configured: bool,
    pub format: String, // "standard" | "nested_mcp" | "zed" | "custom"
    pub last_modified: Option<String>,
    pub is_custom: bool,
}

/// Helper to expand home directory (tilde `~` or `%USERPROFILE%` / `%HOME%`).
fn expand_user_path(p: &str) -> Option<PathBuf> {
    if p.starts_with('~') {
        let home = std::env::var("USERPROFILE")
            .or_else(|_| std::env::var("HOME"))
            .ok()?;
        let rest = p.trim_start_matches('~').trim_start_matches(['/', '\\']);
        Some(PathBuf::from(home).join(rest))
    } else if p.contains("%APPDATA%") {
        let appdata = std::env::var("APPDATA").ok()?;
        let rest = p
            .replace("%APPDATA%", "")
            .trim_start_matches(['/', '\\'])
            .to_string();
        Some(PathBuf::from(appdata).join(rest))
    } else if p.contains("%LOCALAPPDATA%") {
        let localappdata = std::env::var("LOCALAPPDATA").ok()?;
        let rest = p
            .replace("%LOCALAPPDATA%", "")
            .trim_start_matches(['/', '\\'])
            .to_string();
        Some(PathBuf::from(localappdata).join(rest))
    } else {
        Some(PathBuf::from(p))
    }
}

/// Returns the resolved path of the currently running Knox executable.
fn get_knox_executable_path() -> String {
    if let Ok(exe) = std::env::current_exe() {
        if let Ok(canonical) = exe.canonicalize() {
            // Strip Windows verbatim prefix '\\?\' if present for cleaner display
            let s = canonical.display().to_string();
            return s.strip_prefix(r"\\?\").unwrap_or(&s).to_string();
        }
        return exe.display().to_string();
    }
    "knox".to_string()
}

/// Inspects a given JSON file to check if it exists, which schema it uses,
/// and whether `knox` is already configured.
fn inspect_mcp_file(path: &Path) -> (bool, bool, String, Option<String>) {
    if !path.exists() {
        return (false, false, "standard".to_string(), None);
    }

    let modified = path
        .metadata()
        .ok()
        .and_then(|m| m.modified().ok())
        .map(|time| {
            let datetime: chrono::DateTime<chrono::Utc> = time.into();
            datetime.format("%Y-%m-%d %H:%M:%S UTC").to_string()
        });

    let content = match fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => return (true, false, "standard".to_string(), modified),
    };

    let json: serde_json::Value = match serde_json::from_str(&content) {
        Ok(v) => v,
        Err(_) => return (true, false, "standard".to_string(), modified),
    };

    // Check for Zed format: { "context_servers": { "knox": { ... } } }
    if let Some(cs) = json.get("context_servers").and_then(|v| v.as_object()) {
        let is_configured = cs.contains_key("knox");
        return (true, is_configured, "zed".to_string(), modified);
    }

    // Check for nested Antigravity format: { "mcp": { "servers": { "knox": { ... } } } }
    if let Some(servers) = json
        .get("mcp")
        .and_then(|v| v.as_object())
        .and_then(|mcp| mcp.get("servers"))
        .and_then(|v| v.as_object())
    {
        let is_configured = servers.contains_key("knox");
        return (true, is_configured, "nested_mcp".to_string(), modified);
    }

    // Standard format: { "mcpServers": { "knox": { ... } } }
    if let Some(mcp_servers) = json.get("mcpServers").and_then(|v| v.as_object()) {
        let is_configured = mcp_servers.contains_key("knox");
        return (true, is_configured, "standard".to_string(), modified);
    }

    (true, false, "standard".to_string(), modified)
}

/// Standard predefined application locations to detect.
struct KnownAppDef {
    id: &'static str,
    name: &'static str,
    category: &'static str,
    path_template: &'static str,
}

const KNOWN_APPS: &[KnownAppDef] = &[
    KnownAppDef {
        id: "antigravity",
        name: "Antigravity / Gemini CLI",
        category: "ide",
        path_template: "~/.gemini/config/mcp_config.json",
    },
    KnownAppDef {
        id: "cursor",
        name: "Cursor",
        category: "ide",
        path_template: "~/.cursor/mcp.json",
    },
    KnownAppDef {
        id: "claude-desktop",
        name: "Claude Desktop",
        category: "ide",
        path_template: "%APPDATA%\\Claude\\claude_desktop_config.json",
    },
    KnownAppDef {
        id: "claude-code",
        name: "Claude Code CLI",
        category: "cli",
        path_template: "~/.claude.json",
    },
    KnownAppDef {
        id: "vscode-cline",
        name: "VS Code (Cline)",
        category: "extension",
        path_template: "%APPDATA%\\Code\\User\\globalStorage\\saoudrizwan.claude-dev\\settings\\cline_mcp_settings.json",
    },
    KnownAppDef {
        id: "vscode-roo",
        name: "VS Code (Roo Code)",
        category: "extension",
        path_template: "%APPDATA%\\Code\\User\\globalStorage\\rooveterinaryinc.roo-cline\\settings\\cline_mcp_settings.json",
    },
    KnownAppDef {
        id: "windsurf",
        name: "Windsurf",
        category: "ide",
        path_template: "~/.codeium/windsurf/mcp_config.json",
    },
    KnownAppDef {
        id: "zed",
        name: "Zed Editor",
        category: "ide",
        path_template: "%APPDATA%\\Zed\\settings.json",
    },
];

/// Scans standard predefined directories for known IDE and application MCP configs.
#[tauri::command]
pub async fn detect_mcp_configs(app: tauri::AppHandle) -> Result<Vec<DetectedMcpApp>, String> {
    detect_mcp_configs_impl(Some(&app)).await
}

/// Internal implementation of detect_mcp_configs allowing optional AppHandle for tests.
pub async fn detect_mcp_configs_impl(
    app: Option<&tauri::AppHandle>,
) -> Result<Vec<DetectedMcpApp>, String> {
    emit_scan_log(
        app,
        "info",
        "Starting quick scan of standard IDE directories...",
    );
    let mut results = Vec::new();

    for known in KNOWN_APPS {
        if let Some(expanded) = expand_user_path(known.path_template) {
            emit_scan_log(
                app,
                "scan",
                &format!("Checking {} at {}...", known.name, expanded.display()),
            );
            let (exists, is_configured, format, last_modified) = inspect_mcp_file(&expanded);
            if exists {
                if is_configured {
                    emit_scan_log(
                        app,
                        "config",
                        &format!(
                            "Found {} (Configured with Knox, schema: {})",
                            known.name, format
                        ),
                    );
                } else {
                    emit_scan_log(
                        app,
                        "found",
                        &format!("Found {} (Unconfigured, schema: {})", known.name, format),
                    );
                }
            } else {
                emit_scan_log(
                    app,
                    "skip",
                    &format!("{} config not found at expected path", known.name),
                );
            }

            results.push(DetectedMcpApp {
                id: known.id.to_string(),
                name: known.name.to_string(),
                category: known.category.to_string(),
                file_path: expanded.display().to_string(),
                exists,
                is_configured,
                format,
                last_modified,
                is_custom: false,
            });
        }
    }

    let configured_count = results.iter().filter(|r| r.is_configured).count();
    let detected_count = results.iter().filter(|r| r.exists).count();
    emit_scan_log(
        app,
        "info",
        &format!(
            "Quick scan complete: {} apps detected, {} configured with Knox.",
            detected_count, configured_count
        ),
    );

    Ok(results)
}

/// Recursively scans drives for files named `*mcp*.json`, `claude_desktop_config.json`,
/// or files containing MCP server definitions, with smart skip filters.
#[tauri::command]
pub async fn deep_scan_mcp_configs(app: tauri::AppHandle) -> Result<Vec<DetectedMcpApp>, String> {
    deep_scan_mcp_configs_impl(Some(&app)).await
}

/// Internal implementation of deep_scan_mcp_configs allowing optional AppHandle for tests.
pub async fn deep_scan_mcp_configs_impl(
    app: Option<&tauri::AppHandle>,
) -> Result<Vec<DetectedMcpApp>, String> {
    emit_scan_log(
        app,
        "info",
        "Starting deep drive scan for MCP configurations across all local drives...",
    );
    // 1. Gather all active logical drive roots on Windows or root on Unix
    let roots = get_drive_roots();
    let drives_str = roots
        .iter()
        .map(|r| r.display().to_string())
        .collect::<Vec<_>>()
        .join(", ");
    emit_scan_log(
        app,
        "info",
        &format!("Active logical drive roots detected: [{}]", drives_str),
    );

    let mut detected = Vec::new();
    let mut visited_paths = HashSet::new();

    // Include already detected standard apps to avoid duplicates
    if let Ok(standard) = detect_mcp_configs_impl(app).await {
        for app_item in standard {
            if app_item.exists {
                visited_paths.insert(app_item.file_path.clone());
                detected.push(app_item);
            }
        }
    }

    for root in roots {
        if !root.exists() {
            continue;
        }
        emit_scan_log(
            app,
            "scan",
            &format!("Scanning drive root {} (depth limit: 5)...", root.display()),
        );
        scan_directory_recursive(&root, 0, 5, &mut detected, &mut visited_paths, app);
        if detected.len() >= 60 {
            emit_scan_log(
                app,
                "info",
                "Reached candidate discovery cap (60 configs). Halting scan.",
            );
            break;
        }
    }

    emit_scan_log(
        app,
        "info",
        &format!(
            "Deep search finished: Total {} MCP configuration files cataloged.",
            detected.len()
        ),
    );

    Ok(detected)
}

/// Collects drive roots (e.g. C:\, D:\) on Windows or / on Unix.
fn get_drive_roots() -> Vec<PathBuf> {
    #[cfg(windows)]
    {
        let mut roots = Vec::new();
        // Check drive letters A to Z
        for byte in b'A'..=b'Z' {
            let drive = format!("{}:\\", byte as char);
            let path = PathBuf::from(&drive);
            if path.exists() {
                roots.push(path);
            }
        }
        roots
    }
    #[cfg(not(windows))]
    {
        vec![PathBuf::from("/")]
    }
}

/// Recursive directory walker with skip list and depth cap.
fn scan_directory_recursive(
    dir: &Path,
    current_depth: usize,
    max_depth: usize,
    results: &mut Vec<DetectedMcpApp>,
    visited: &mut HashSet<String>,
    app: Option<&tauri::AppHandle>,
) {
    if current_depth > max_depth || results.len() >= 60 {
        return;
    }

    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };

    for entry in entries.flatten() {
        let path = entry.path();
        let file_name = entry.file_name().to_string_lossy().to_string();

        if path.is_dir() {
            let lower = file_name.to_lowercase();
            // Skip heavy or irrelevant directories
            if lower == "node_modules"
                || lower == ".git"
                || lower == "$recycle.bin"
                || lower == "system volume information"
                || lower == "windows"
                || lower == "target"
                || lower == "temp"
                || lower == "tmp"
                || (current_depth == 1
                    && (lower == "program files" || lower == "program files (x86)"))
            {
                if current_depth <= 2 {
                    emit_scan_log(
                        app,
                        "skip",
                        &format!("Skipping directory: {}", path.display()),
                    );
                }
                continue;
            }

            if current_depth <= 1 {
                emit_scan_log(
                    app,
                    "scan",
                    &format!("Visiting directory: {}", path.display()),
                );
            }

            scan_directory_recursive(&path, current_depth + 1, max_depth, results, visited, app);
        } else if path.is_file() {
            let lower = file_name.to_lowercase();
            let is_candidate = lower.ends_with(".json")
                && (lower.contains("mcp")
                    || lower == "claude_desktop_config.json"
                    || lower == "cline_mcp_settings.json");

            if is_candidate {
                let canonical_str = path.display().to_string();
                if !visited.contains(&canonical_str) {
                    visited.insert(canonical_str.clone());
                    emit_scan_log(
                        app,
                        "found",
                        &format!("Discovered candidate MCP file: {}", canonical_str),
                    );
                    let (exists, is_configured, format, last_modified) = inspect_mcp_file(&path);
                    if exists {
                        if is_configured {
                            emit_scan_log(
                                app,
                                "config",
                                &format!(
                                    "Candidate {} is already configured with Knox (schema: {})",
                                    file_name, format
                                ),
                            );
                        } else {
                            emit_scan_log(
                                app,
                                "found",
                                &format!("Validated candidate {} (schema: {})", file_name, format),
                            );
                        }
                        results.push(DetectedMcpApp {
                            id: format!("deep-{}", results.len() + 1),
                            name: format!("Config: {}", file_name),
                            category: "custom".to_string(),
                            file_path: canonical_str,
                            exists: true,
                            is_configured,
                            format,
                            last_modified,
                            is_custom: true,
                        });
                    }
                }
            }
        }
    }
}

/// Inspects a custom file path provided by user.
#[tauri::command]
pub async fn inspect_custom_mcp_config(file_path: String) -> Result<DetectedMcpApp, String> {
    let path = PathBuf::from(&file_path);
    let (exists, is_configured, format, last_modified) = inspect_mcp_file(&path);

    let name = path
        .file_name()
        .map(|f| f.to_string_lossy().to_string())
        .unwrap_or_else(|| "Custom App".to_string());

    Ok(DetectedMcpApp {
        id: format!("custom-{}", chrono::Utc::now().timestamp_millis()),
        name,
        category: "custom".to_string(),
        file_path,
        exists,
        is_configured,
        format,
        last_modified,
        is_custom: true,
    })
}

/// Injects Knox MCP stdio configuration into the target JSON file.
/// If `use_system_command` is true, writes `"command": "knox"`.
/// Otherwise, resolves active absolute path (e.g. `D:\Knox\target\release\knox.exe`).
#[tauri::command]
pub async fn inject_mcp_config(
    file_path: String,
    use_system_command: Option<bool>,
) -> Result<bool, String> {
    let path = PathBuf::from(&file_path);

    // Create parent directories if missing
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("Failed to create directories: {e}"))?;
    }

    // Create a backup copy before writing
    if path.exists() {
        let backup_path = path.with_extension("bak");
        let _ = fs::copy(&path, &backup_path);
    }

    let mut json_val: serde_json::Value = if path.exists() {
        let content = fs::read_to_string(&path).map_err(|e| format!("Failed to read file: {e}"))?;
        serde_json::from_str(&content).unwrap_or_else(|_| serde_json::json!({}))
    } else {
        serde_json::json!({})
    };

    if !json_val.is_object() {
        json_val = serde_json::json!({});
    }

    let cmd = if use_system_command.unwrap_or(false) {
        "knox".to_string()
    } else {
        get_knox_executable_path()
    };

    let knox_server_entry = serde_json::json!({
        "command": cmd,
        "args": ["serve", "--stdio"]
    });

    let root_obj = json_val
        .as_object_mut()
        .ok_or_else(|| "Root JSON is not an object".to_string())?;

    // Check schema format
    if root_obj.contains_key("context_servers") {
        // Zed schema
        let cs = root_obj
            .entry("context_servers")
            .or_insert_with(|| serde_json::json!({}))
            .as_object_mut()
            .ok_or_else(|| "context_servers is not an object".to_string())?;
        cs.insert("knox".to_string(), knox_server_entry);
    } else if root_obj.contains_key("mcp") {
        // Nested mcp schema: { "mcp": { "servers": { ... } } }
        let mcp_obj = root_obj
            .entry("mcp")
            .or_insert_with(|| serde_json::json!({}))
            .as_object_mut()
            .ok_or_else(|| "mcp is not an object".to_string())?;
        let servers_obj = mcp_obj
            .entry("servers")
            .or_insert_with(|| serde_json::json!({}))
            .as_object_mut()
            .ok_or_else(|| "servers is not an object".to_string())?;
        servers_obj.insert("knox".to_string(), knox_server_entry);
    } else {
        // Standard schema: { "mcpServers": { ... } }
        let mcp_servers = root_obj
            .entry("mcpServers")
            .or_insert_with(|| serde_json::json!({}))
            .as_object_mut()
            .ok_or_else(|| "mcpServers is not an object".to_string())?;
        mcp_servers.insert("knox".to_string(), knox_server_entry);
    }

    let formatted = serde_json::to_string_pretty(&json_val)
        .map_err(|e| format!("Failed to format JSON: {e}"))?;

    fs::write(&path, formatted).map_err(|e| format!("Failed to write configuration: {e}"))?;

    Ok(true)
}

/// Removes Knox MCP stdio configuration from the target JSON file.
#[tauri::command]
pub async fn remove_mcp_config(file_path: String) -> Result<bool, String> {
    let path = PathBuf::from(&file_path);
    if !path.exists() {
        return Ok(true);
    }

    // Create a backup copy before writing
    let backup_path = path.with_extension("bak");
    let _ = fs::copy(&path, &backup_path);

    let content = fs::read_to_string(&path).map_err(|e| format!("Failed to read file: {e}"))?;
    let mut json_val: serde_json::Value =
        serde_json::from_str(&content).map_err(|e| format!("Invalid JSON: {e}"))?;

    if let Some(root_obj) = json_val.as_object_mut() {
        if let Some(mcp_servers) = root_obj
            .get_mut("mcpServers")
            .and_then(|v| v.as_object_mut())
        {
            mcp_servers.remove("knox");
        }
        if let Some(servers) = root_obj
            .get_mut("mcp")
            .and_then(|v| v.as_object_mut())
            .and_then(|mcp| mcp.get_mut("servers"))
            .and_then(|v| v.as_object_mut())
        {
            servers.remove("knox");
        }
        if let Some(cs) = root_obj
            .get_mut("context_servers")
            .and_then(|v| v.as_object_mut())
        {
            cs.remove("knox");
        }
    }

    let formatted = serde_json::to_string_pretty(&json_val)
        .map_err(|e| format!("Failed to format JSON: {e}"))?;

    fs::write(&path, formatted).map_err(|e| format!("Failed to write configuration: {e}"))?;

    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_dir(prefix: &str) -> PathBuf {
        let unique = format!(
            "{}_{}",
            prefix,
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        );
        let dir = std::env::temp_dir().join(unique);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn test_inject_and_remove_standard_mcp() {
        let dir = create_test_dir("knox_test_std");
        let config_file = dir.join("mcp.json");

        // Initial empty or existing file
        fs::write(
            &config_file,
            r#"{
  "mcpServers": {
    "other_tool": {
      "command": "python",
      "args": ["-m", "tool"]
    }
  }
}"#,
        )
        .unwrap();

        // 1. Inject using system command
        let res = tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(inject_mcp_config(
                config_file.display().to_string(),
                Some(true),
            ));
        assert!(res.is_ok());

        // Verify injection
        let (exists, is_configured, format, _) = inspect_mcp_file(&config_file);
        assert!(exists);
        assert!(is_configured);
        assert_eq!(format, "standard");

        let content = fs::read_to_string(&config_file).unwrap();
        assert!(content.contains(r#""knox""#));
        assert!(content.contains(r#""other_tool""#));
        assert!(content.contains(r#""command": "knox""#));

        // Verify backup was created
        assert!(config_file.with_extension("bak").exists());

        // 2. Remove Knox
        let rem_res = tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(remove_mcp_config(config_file.display().to_string()));
        assert!(rem_res.is_ok());

        let (_, is_configured_after, _, _) = inspect_mcp_file(&config_file);
        assert!(!is_configured_after);

        let content_after = fs::read_to_string(&config_file).unwrap();
        assert!(!content_after.contains(r#""knox""#));
        assert!(content_after.contains(r#""other_tool""#));

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_inject_nested_antigravity_schema() {
        let dir = create_test_dir("knox_test_nested");
        let config_file = dir.join("mcp_config.json");

        fs::write(
            &config_file,
            r#"{
  "mcp": {
    "servers": {
      "gemini_ext": { "command": "ext" }
    }
  }
}"#,
        )
        .unwrap();

        let res = tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(inject_mcp_config(
                config_file.display().to_string(),
                Some(false),
            ));
        assert!(res.is_ok());

        let (_, is_configured, format, _) = inspect_mcp_file(&config_file);
        assert!(is_configured);
        assert_eq!(format, "nested_mcp");

        let content = fs::read_to_string(&config_file).unwrap();
        assert!(content.contains("servers"));
        assert!(content.contains("gemini_ext"));

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_detect_mcp_configs_impl_runs_without_app() {
        let res = tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(detect_mcp_configs_impl(None));
        assert!(res.is_ok());
        let apps = res.unwrap();
        assert!(!apps.is_empty());
        // Should have scanned known apps like antigravity and cursor
        assert!(apps.iter().any(|a| a.id == "antigravity"));
        assert!(apps.iter().any(|a| a.id == "cursor"));
    }
}
