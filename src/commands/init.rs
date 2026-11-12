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

    let project = ensure_project_record(&db, &project_name, &canonical_str, &resolved_path)?;
    let knox_md_path = resolved_path.join("knox.md");
    write_knox_md(&knox_md_path, &project, &db_path)?;
    ensure_gitignore(&resolved_path)?;

    Ok(())
}

/// Adds `.knox/` to the project `.gitignore` when one already exists or the
/// directory is a git repository, so local databases are never committed.
fn ensure_gitignore(project_path: &Path) -> KnoxResult<()> {
    let gitignore = project_path.join(".gitignore");
    if !gitignore.exists() && !project_path.join(".git").exists() {
        return Ok(());
    }
    let existing = fs::read_to_string(&gitignore).unwrap_or_default();
    if existing
        .lines()
        .any(|line| line.trim() == ".knox/" || line.trim() == ".knox")
    {
        return Ok(());
    }
    let mut updated = existing;
    if !updated.is_empty() && !updated.ends_with('\n') {
        updated.push('\n');
    }
    updated.push_str(".knox/\n");
    fs::write(&gitignore, updated)?;
    Ok(())
}

/// Registers the initialized workspace in the cross-project registry.
///
/// Kept separate from `cmd_init` so unit tests that initialize temporary
/// workspaces never touch the developer's real `~/.knox/registry.db`.
pub fn register_workspace(project_path: &Path) -> KnoxResult<()> {
    let resolved_path = resolve_project_path(project_path);
    let name = extract_project_name(&resolved_path);
    let root = resolved_path.to_string_lossy().to_string();
    let remote = crate::git::detect_remote(&resolved_path);
    crate::storage::registry::register_project(&root, &name, remote.as_deref())
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
    let db_path = config.resolve_db_path_for(project_path);
    if let Some(parent) = db_path.parent() {
        fs::create_dir_all(parent)?;
    }
    Ok(db_path)
}

/// Fetches existing project record or creates and inserts a new one.
fn ensure_project_record(
    db: &KnoxDb,
    project_name: &str,
    canonical_path: &str,
    project_path: &Path,
) -> KnoxResult<Project> {
    let remote = crate::git::detect_remote(project_path);

    // 1. If project has a remote, check if another clone of this repository already exists
    if let Some(ref rem) = remote {
        if let Some(existing) = crate::storage::nodes::get_project_by_remote(db, rem)? {
            return Ok(existing);
        }
    }

    // 2. Check if a project record already exists for this exact path
    if let Some(mut existing) = get_project_by_path(db, canonical_path)? {
        if existing.remote.is_none() && remote.is_some() {
            existing.remote = remote;
            insert_project(db, &existing)?;
        }
        return Ok(existing);
    }

    // 3. Create a new project record with normalized remote
    let project = Project {
        id: ProjectId::new(),
        name: project_name.to_string(),
        path: canonical_path.to_string(),
        remote,
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
    let tools_yaml = crate::mcp::tools::LISTED_TOOL_NAMES
        .iter()
        .map(|name| format!("  - {name}\n"))
        .collect::<String>();
    let version = env!("CARGO_PKG_VERSION");
    format!(
        "<!-- knox:config\n\
project_id: \"{project_id}\"\n\
kg_path: \"{kg_path}\"\n\
mcp_server: \"knox serve --stdio\"\n\
mcp_transport: \"stdio\"\n\
tools_registered:\n\
{tools_yaml}\
resources_available:\n\
  - knox://project/overview\n\
  - knox://decisions/active\n\
  - knox://context/{{file}}\n\
prompts_available:\n\
  - architectural-audit\n\
  - session-handover\n\
created_at: \"{created_at}\"\n\
knox_version: \"{version}\"\n\
-->\n\n"
    )
}

/// Renders the mandatory instructions and protocol documentation body for `knox.md`.
fn render_knox_protocol_body() -> &'static str {
    r#"# Knox — Active Knowledge Graph

> **External long-term memory for AI coding agents.**  
> Knox persists plans, architectural decisions, and code modifications in a queryable knowledge graph. Instead of wasting thousands of tokens rediscovering project context, AI agents query Knox to instantly understand what was tried, what was decided, and why.

---

## 1. Core Operating Principles

1. **Read Before Acting (Zero Token Waste)**: Never re-derive codebase architecture, previous attempts, or design decisions from scratch. Query Knox first to retrieve prior context.
2. **Append-Only Immutability**: Historical records are never deleted. When an approach or architecture changes, supersede the previous decision (`supersedes: "<old_action_id>"`).
3. **Causal Lineage & Semantic Graph**: Connect actions into directed graphs using `parent_action_id` and first-class `relations` (`depends_on`, `implements`, `verifies`, `relates_to`, `reverts`).
4. **Explicit Edge Attribution**: Identify your agent in `actor` (e.g., `gemini`, `cursor`, `claude`, `codex`, `human`).
5. **Decisions are First-Class Citizens**: Architectural decisions (recorded via `knox_record` with kind `decide`) carry the highest weight in the graph. Always document rationale, symbols touched, and rejected alternatives.
6. **Continuous Cognitive Memory & Doubts**: Record user conversations, uncertainties, and questions as `doubt` entries via `knox_record`. Capture micro-decisions and implementation assumptions with `significance: "minor"`. When a doubt is verified or resolved, close the cognitive loop using `knox_record` with resolving links.

---

## 2. For AI Agents — MANDATORY PROTOCOL (Streamlined 3-Phase Lifecycle)

Knox supports **Smart Implicit Sessions**: agents can immediately record actions without an explicit session ceremony. Knox transparently binds actions to the agent's active session or spawns one automatically.

```
┌────────────────────────────────────────────────────────────────────────┐
│ Phase 1: Pre-flight Safety & Context   → knox_check / ctx              │
└───────────────────────────────────┬────────────────────────────────────┘
                                    │ (checks active constraints & superseded anti-patterns)
┌───────────────────────────────────▼────────────────────────────────────┐
│ Phase 2: Execute & Cognitive Logging  → knox_record                    │
└───────────────────────────────────┬────────────────────────────────────┘
                                    │ (logs decisions, doubts, and writes)
┌───────────────────────────────────▼────────────────────────────────────┐
│ Phase 3: Graph Navigation & Audit      → knox_trace                    │
└────────────────────────────────────────────────────────────────────────┘
```

### Phase 1: Pre-flight Safety & Context Retrieval
Before touching files:
- Call `knox_check` with `files: ["path/to/target.rs"]` to inspect active decisions and ensure you don't repeat superseded patterns.
- Call `knox_context` to retrieve token-efficient Markdown context for relevant files, symbols, or topics.

### Phase 2: Execute & Cognitive Logging
- **Decisions**: Call `knox_record` with a `decide` entry including `summary`, `rationale`, and `rejected_alternatives`.
- **Doubts**: Call `knox_record` with a `doubt` entry with `summary` and `context_summary`.
- **Code changes**: Call `knox_record` with a `write` entry including `files`, `symbols`, and `relations`.
- **Multi-step changes**: Use `knox_record` with multiple entries and local `ref` links in one atomic call.

### Phase 3: Graph Navigation & Verification
- Use `knox_trace` to trace the causal origin of a bug or architecture ("Why is this code here?").
- Use `knox_context` to assess relevant context before refactoring.
- Use `knox_begin` to resume a session and receive the handoff digest from the previous turn.
- Use `knox_handoff` to close a session and produce the digest for the next session.

---

## 3. MCP Tool Reference & Schemas

Knox exposes 8 tools, 3 resources, and 2 prompts over JSON-RPC 2.0 stdio transport (`knox serve --stdio`).

### 3.1 `knox_begin`
Resumes or creates a scoped session and returns compact task context. Call this at the start of every turn.

### 3.2 `knox_context`
Returns compact, token-budgeted engineering context for a file, symbol, query, or project.

### 3.3 `knox_check`
Pre-flight guardrail check. Reads active decisions, open doubts, and superseded anti-patterns for target files before editing.

### 3.4 `knox_trace`
Traces causal ancestry backwards from an action or file to answer "Why does this code exist?".

### 3.5 `knox_record`
Atomically records structured engineering entries (decisions, doubts, code changes) in one transaction. Includes local refs, dedupe, idempotency, and validation.

### 3.6 `knox_handoff`
Closes a session and returns a compact digest (open doubts, constraints touched, last outcomes, unresolved plans) for the next session to resume from.

### 3.7 `knox_ask`
Synthesizes engineering knowledge, decisions, and architectural context using model intelligence or hybrid retrieval.

### 3.8 `knox_update`
Ingests new lines from agent conversation transcripts into project memory using deterministic incremental offsets. Call periodically during long workflows or when sync is needed.

### 3.9 Native MCP Resources (`resources/*`)
- `knox://project/overview` — High-level knowledge graph metrics, action counts, and recent activity.
- `knox://decisions/active` — Live architectural constraints, design decisions, and rationales.
- `knox://context/{file_path}` — Non-superseded context for any file in the workspace.

### 3.10 Native MCP Prompts (`prompts/*`)
- `architectural-audit` — Guided audit of active decisions, stale patterns, and constraint compliance.
- `session-handover` — Synthesized context digest for handing over work to the next agent.

---

## 4. Taxonomy & Conventions

### 4.1 Semantic Relations (`relation_type`)
- `depends_on` — Target action is a technical prerequisite.
- `implements` — Action puts a design decision or plan into code.
- `verifies` — Test or benchmark confirms an implementation.
- `relates_to` — General conceptual association.
- `reverts` — Explicitly rolls back or undoes target action.
- `resolves` — Action or decision answers and resolves a prior open doubt.

Example relation in `knox_record`:
```json
{
  "entries": [
    {
      "type": "write",
      "summary": "Implemented auth handler",
      "relations": [
        { "target": "@ref:1", "type": "implements" }
      ]
    }
  ]
}
```

### 4.2 Action Types (`action_type`)
- `plan` — Technical roadmap or requirement breakdown.
- `decide` — Strategic architectural choice or tradeoff evaluation.
- `doubt` — Open uncertainty, question, or hypothesis being verified.
- `dialogue` — User prompt conversation or design discussion record.
- `write` — Code creation, refactoring, or file additions.
- `change` — Incremental bug fix or configuration edit.
- `debug` — Root-cause analysis, log inspection, or diagnosis.
- `read` — Context extraction or codebase survey.
- `review` — Verification audit or security check.
"#
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
        assert!(rendered.contains(&format!("knox_version: \"{}\"", env!("CARGO_PKG_VERSION"))));

        // All advertised tools must be present
        for tool_name in crate::mcp::tools::LISTED_TOOL_NAMES {
            assert!(
                rendered.contains(tool_name),
                "knox.md missing listed tool: {tool_name}"
            );
        }

        // Retired tools must not be registered
        assert!(!rendered.contains("knox_start_session"));
        assert!(!rendered.contains("knox_end_session"));
        assert!(!rendered.contains("knox_scan_staged"));
        assert!(!rendered.contains("knox_check_guardrails"));
        assert!(!rendered.contains("knox_diff_inspect"));
        assert!(!rendered.contains("knox_verify_drift"));
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
            transcript_path: None,
            ..Default::default()
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
