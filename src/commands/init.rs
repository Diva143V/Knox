//! Project initialization command for Knox.
//! Creates project records, ensures database readiness, and generates `knox.md`.

use chrono::Utc;
use std::fs;
use std::path::{Path, PathBuf};

use crate::core::config::KnoxConfig;
use crate::core::error::KnoxResult;
use crate::core::ids::ProjectId;
use crate::core::types::Project;
use crate::storage::db::KnoxDb;
use crate::storage::nodes::{get_project_by_path, insert_project};

/// Initializes Knox for the project at the given path.
///
/// 1. Resolves canonical project directory and name.
/// 2. Opens or initializes the knowledge graph database.
/// 3. Registers the project in the database if not already present.
/// 4. Generates or updates `knox.md` idempotently with configuration and agent instructions.
pub fn cmd_init(project_path: &Path, config: &KnoxConfig) -> KnoxResult<()> {
    let resolved_path = resolve_project_path(project_path);
    let project_name = extract_project_name(&resolved_path);
    let canonical_str = resolved_path.to_string_lossy().to_string();

    let db_path = determine_db_path(&resolved_path, config)?;
    let db = KnoxDb::open(&db_path)?;

    let project = ensure_project_record(&db, &project_name, &canonical_str)?;
    let knox_md_path = resolved_path.join("knox.md");
    write_knox_md(&knox_md_path, &project, &db_path)?;

    Ok(())
}

/// Resolves path to canonical form if it exists, or normalized absolute path.
fn resolve_project_path(path: &Path) -> PathBuf {
    fs::canonicalize(path).unwrap_or_else(|_| {
        if path.is_absolute() {
            path.to_path_buf()
        } else {
            std::env::current_dir()
                .map(|cur| cur.join(path))
                .unwrap_or_else(|_| path.to_path_buf())
        }
    })
}

/// Extracts a friendly project name from the path's terminal folder.
fn extract_project_name(path: &Path) -> String {
    path.file_name()
        .and_then(|n| n.to_str())
        .filter(|s| !s.is_empty())
        .unwrap_or("knox-project")
        .to_string()
}

/// Computes the SQLite database location based on global vs per-project configuration.
fn determine_db_path(project_path: &Path, config: &KnoxConfig) -> KnoxResult<PathBuf> {
    if config.global_mode {
        Ok(config.kg_path.clone())
    } else {
        let dot_knox = project_path.join(".knox");
        fs::create_dir_all(&dot_knox)?;
        Ok(dot_knox.join("knox.db"))
    }
}

/// Fetches existing project record or creates and inserts a new one.
fn ensure_project_record(
    db: &KnoxDb,
    project_name: &str,
    canonical_path: &str,
) -> KnoxResult<Project> {
    if let Some(existing) = get_project_by_path(db, canonical_path)? {
        return Ok(existing);
    }

    let project = Project {
        id: ProjectId::new(),
        name: project_name.to_string(),
        path: canonical_path.to_string(),
        created_at: Utc::now(),
    };

    insert_project(db, &project)?;
    Ok(project)
}

/// Generates the contents of `knox.md` according to the canonical Knox specification.
pub fn render_knox_md(project_id: &str, kg_path: &str, created_at: &str) -> String {
    let mut doc = render_knox_config_header(project_id, kg_path, created_at);
    doc.push_str(render_knox_protocol_body());
    doc
}

/// Renders the YAML configuration frontmatter block for `knox.md`.
fn render_knox_config_header(project_id: &str, kg_path: &str, created_at: &str) -> String {
    format!(
        "<!-- knox:config\n\
project_id: \"{project_id}\"\n\
kg_path: \"{kg_path}\"\n\
mcp_server: \"knox serve --stdio\"\n\
mcp_transport: \"stdio\"\n\
tools_registered:\n\
  - knox_get_context\n\
  - knox_get_history\n\
  - knox_record_action\n\
  - knox_record_decision\n\
  - knox_search\n\
  - knox_list_stale\n\
created_at: \"{created_at}\"\n\
knox_version: \"0.1.0\"\n\
-->\n\n"
    )
}

/// Renders the mandatory instructions and protocol documentation body for `knox.md`.
fn render_knox_protocol_body() -> &'static str {
    "# Knox — Active Knowledge Graph\n\n\
This project is tracked by **Knox**, a persistent knowledge graph that stores\n\
decisions, plans, and actions from every AI-assisted coding session.\n\n\
## For AI Agents — MANDATORY PROTOCOL\n\n\
1. **BEFORE acting on any task**: Call `knox_get_context` with the target file\n\
   path(s) to retrieve current decisions, prior approaches, and recent actions.\n\
   Do NOT re-derive context that already exists in the KG.\n\n\
2. **AFTER completing significant work**: Call `knox_record_action` with:\n\
   - `actor`: Your software name (e.g., `gemini`, `cursor`, `chatgpt`)\n\
   - `action_type`: One of `plan`, `write`, `change`, `decide`, `debug`, `read`, `review`\n\
   - `files`: List of affected file paths (relative to project root)\n\
   - `summary`: Concise description of what was done and why\n\
   - `supersedes` (optional): ID of a prior decision this replaces\n\n\
3. **When making a design decision or choosing an approach**: Call\n\
   `knox_record_decision` — these are the highest-value entries in the KG.\n\
   Always include the rationale and alternatives considered.\n\n\
4. **Edge naming convention**: Prefix every action with your software name:\n\
   `gemini_write`, `cursor_plan`, `chatgpt_decide`, `copilot_debug`, etc.\n\n\
5. **Never skip the read step**. Even if you think you know the context,\n\
   the KG may contain decisions from other tools or sessions you're unaware of.\n\n\
## For Humans\n\n\
Run `knox status` to see what's tracked. Run `knox query <file>` to inspect\n\
the decision history for any file. The KG is append-only — nothing is ever\n\
deleted, only superseded.\n\n\
## Connection\n\n\
- **MCP Server**: `knox serve --stdio`\n\
- **Direct CLI**: `knox query`, `knox status`\n\
- **KG Location**: See config block above\n"
}

/// Writes `knox.md` idempotently, preserving existing IDs when available.
fn write_knox_md(target_file: &Path, project: &Project, db_path: &Path) -> KnoxResult<()> {
    let (project_id, created_at) = if target_file.exists() {
        let existing_content = fs::read_to_string(target_file).unwrap_or_default();
        parse_existing_knox_md_metadata(&existing_content)
            .unwrap_or_else(|| (project.id.to_string(), project.created_at.to_rfc3339()))
    } else {
        (project.id.to_string(), project.created_at.to_rfc3339())
    };

    let normalized_db_path = db_path.to_string_lossy().replace('\\', "/");
    let content = render_knox_md(&project_id, &normalized_db_path, &created_at);
    fs::write(target_file, content)?;
    Ok(())
}

/// Parses existing project_id and created_at from an existing knox.md file.
fn parse_existing_knox_md_metadata(content: &str) -> Option<(String, String)> {
    let mut proj_id = None;
    let mut created = None;

    for line in content.lines() {
        let trimmed = line.trim();
        if let Some(val) = extract_yaml_value(trimmed, "project_id:") {
            proj_id = Some(val);
        } else if let Some(val) = extract_yaml_value(trimmed, "created_at:") {
            created = Some(val);
        }
    }

    match (proj_id, created) {
        (Some(id), Some(dt)) => Some((id, dt)),
        _ => None,
    }
}

/// Helper to extract unquoted string values from YAML lines.
fn extract_yaml_value(line: &str, key: &str) -> Option<String> {
    if let Some(stripped) = line.strip_prefix(key) {
        let raw = stripped.trim();
        let unquoted = raw.trim_matches('"').trim_matches('\'');
        Some(unquoted.to_string())
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_render_knox_md_contains_mandatory_sections() {
        let rendered = render_knox_md("test-proj-id", "~/.knox/knox.db", "2026-10-29T18:00:00Z");
        assert!(rendered.contains("project_id: \"test-proj-id\""));
        assert!(rendered.contains("kg_path: \"~/.knox/knox.db\""));
        assert!(rendered.contains("MANDATORY PROTOCOL"));
        assert!(rendered.contains("knox_record_decision"));
    }

    #[test]
    fn test_cmd_init_flow_in_temp_directory() {
        let dir = tempdir().unwrap();
        let project_dir = dir.path().join("my-cool-project");
        fs::create_dir_all(&project_dir).unwrap();

        let db_file = dir.path().join("knox_test.db");
        let config = KnoxConfig {
            kg_path: db_file,
            session_timeout: std::time::Duration::from_secs(1800),
            global_mode: true,
        };

        cmd_init(&project_dir, &config).unwrap();

        let knox_md = project_dir.join("knox.md");
        assert!(knox_md.exists());
        let content = fs::read_to_string(&knox_md).unwrap();
        assert!(content.contains("MANDATORY PROTOCOL"));

        // Idempotency: re-running does not fail and preserves metadata
        cmd_init(&project_dir, &config).unwrap();
        let content2 = fs::read_to_string(&knox_md).unwrap();
        assert_eq!(content, content2);
    }
}
