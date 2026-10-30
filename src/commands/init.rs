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
  - knox_start_session\n\
  - knox_end_session\n\
  - knox_record_action\n\
  - knox_record_decision\n\
  - knox_get_context\n\
  - knox_get_history\n\
  - knox_search\n\
  - knox_list_stale\n\
  - knox_scan_staged\n\
created_at: \"{created_at}\"\n\
knox_version: \"1.2.0\"\n\
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
3. **Causal Lineage**: Connect actions into directed acyclic graphs (DAGs) using `parent_action_id` (e.g. Plan → Decision → Implementation Write → Debug / Test).
4. **Explicit Edge Attribution**: Prefix every action with your agent/tool name (e.g., `gemini_write`, `cursor_plan`, `claude_decide`, `human_write`).
5. **Decisions are First-Class Citizens**: Architectural decisions (`knox_record_decision`) carry the highest weight in the graph. Always document the rationale and rejected alternatives.

---

## 2. For AI Agents — MANDATORY PROTOCOL (5-Phase Lifecycle)

Every AI agent interacting with this workspace MUST follow this 5-phase protocol:

```
┌────────────────────────────────────────────────────────┐
│ 1. Start Session      → knox_start_session             │
└───────────────────────────┬────────────────────────────┘
                            │ (returns session_id)
┌───────────────────────────▼────────────────────────────┐
│ 2. Sync Staged Edits   → knox_scan_staged              │
└───────────────────────────┬────────────────────────────┘
                            │ (captures unrecorded human edits)
┌───────────────────────────▼────────────────────────────┐
│ 3. Retrieve Context    → knox_get_context / knox_search │
└───────────────────────────┬────────────────────────────┘
                            │ (reads non-superseded history)
┌───────────────────────────▼────────────────────────────┐
│ 4. Execute & Record    → knox_record_decision / action │
└───────────────────────────┬────────────────────────────┘
                            │ (links causal parent & supersedes)
┌───────────────────────────▼────────────────────────────┐
│ 5. End Session        → knox_end_session               │
└────────────────────────────────────────────────────────┘
```

### Phase 1: Initialize Session
At the start of a conversation or continuous task block, obtain an active session ID:
- Tool: `knox_start_session`
- Required: `actor` (e.g. `"gemini"`, `"cursor"`, `"claude"`, `"copilot"`, `"opencode"`)
- Optional: `project: "knox"`, `metadata: { "task": "refactor parser" }`

### Phase 2: Check Human Modifications
Before reading or editing files, check whether manual git edits were staged since the last session:
- Tool: `knox_scan_staged`
- Argument: `{ "project": "." }`
- Purpose: Ensures human commits/staged changes are registered in the KG before you reason about state.

### Phase 3: Retrieve Context Before Modifying
Before creating or modifying any file:
- Tool: `knox_get_context`
- Filter by target file(s) and project.
- Evaluate previous decisions, architectural constraints, and open questions. Do not re-propose previously rejected patterns unless explicitly directed by the human.

### Phase 4: Record Decisions and Actions
- **When choosing an approach**: Call `knox_record_decision`. Record why an approach was selected and what alternatives were considered.
- **After code changes or test runs**: Call `knox_record_action`. Record affected files, summary with diff statistics, and causal parent links.
- **When changing prior architecture**: Provide the ID of the older action in the `supersedes` field.

### Phase 5: Conclude Session
When work is complete or the user prompt is resolved:
- Tool: `knox_end_session` with `session_id`.

---

## 3. MCP Tool Reference & Schemas

Knox exposes 9 standard tools over JSON-RPC 2.0 stdio transport (`knox serve --stdio`).

### 3.1 `knox_start_session`
Opens a new tracking session. Sessions auto-close after 30 minutes of inactivity.
```json
// Request
{
  "actor": "gemini",
  "project": "knox",
  "metadata": {
    "model": "gemini-3.8-flash",
    "goal": "Implement graph caching"
  }
}

// Response
{
  "session_id": "s-4f8e6c71-2b0a-4a88-8cf3-199b441f9d01",
  "created_at": "2026-10-30T14:00:00Z"
}
```

### 3.2 `knox_end_session`
Closes an active session explicitly.
```json
// Request
{
  "session_id": "s-4f8e6c71-2b0a-4a88-8cf3-199b441f9d01"
}

// Response
{
  "status": "closed",
  "actions_recorded": 5,
  "duration_seconds": 420
}
```

### 3.3 `knox_record_action`
Records a code write, plan, debug investigation, review, or file read.
```json
// Request
{
  "session_id": "s-4f8e6c71-2b0a-4a88-8cf3-199b441f9d01",
  "actor": "gemini",
  "action_type": "write",
  "summary": "Implemented bounded BFS in SQL recursive CTE (src/graph/loader.rs +65 -12)",
  "files": ["src/graph/loader.rs"],
  "parent_action_id": "a-91dc8f72-7489-4981-a8cf-432857e31102",
  "tags": ["graph", "bfs", "performance"],
  "confidence": 0.95
}

// Response
{
  "id": "a-2b7c6114-6ef2-48ec-9251-17983c274158",
  "status": "recorded",
  "timestamp": "2026-10-30T14:05:12Z"
}
```

### 3.4 `knox_record_decision`
Records a design or architectural choice. Always forces `action_type = "decide"`.
```json
// Request
{
  "session_id": "s-4f8e6c71-2b0a-4a88-8cf3-199b441f9d01",
  "actor": "gemini",
  "summary": "Decided on LRU cache for subgraphs over unbounded memoization to bound RAM at 75MB",
  "files": ["src/graph/cache.rs"],
  "parent_action_id": "a-91dc8f72-7489-4981-a8cf-432857e31102",
  "supersedes": "a-001a42b1-1189-4099-b1ff-9218342718aa",
  "tags": ["architecture", "cache", "memory"],
  "confidence": 0.90
}

// Response
{
  "id": "a-9c7162b4-512a-4ee1-b0e2-9f3721e78451",
  "status": "recorded",
  "timestamp": "2026-10-30T14:08:45Z"
}
```

### 3.5 `knox_get_context`
Retrieves current, active (non-superseded) context for a file, folder, or project.
```json
// Request
{
  "file_path": "src/graph/cache.rs",
  "current_only": true,
  "limit": 20
}

// Response
{
  "results": [
    {
      "id": "a-9c7162b4-512a-4ee1-b0e2-9f3721e78451",
      "actor": "gemini",
      "action_type": "decide",
      "summary": "Decided on LRU cache for subgraphs over unbounded memoization to bound RAM at 75MB",
      "files": ["src/graph/cache.rs"],
      "tags": ["architecture", "cache", "memory"],
      "confidence": 0.90,
      "is_current": true,
      "timestamp": "2026-10-30T14:08:45Z",
      "session_id": "s-4f8e6c71-2b0a-4a88-8cf3-199b441f9d01",
      "parent_action_id": "a-91dc8f72-7489-4981-a8cf-432857e31102"
    }
  ],
  "total_count": 1,
  "has_more": false
}
```

### 3.6 `knox_get_history`
Returns chronological append-only history, including superseded and replaced actions (`current_only = false`).
```json
// Request
{
  "file_path": "src/graph/cache.rs",
  "limit": 50
}
```

### 3.7 `knox_search`
Full-text search (FTS5) across action summaries and tag filters.
```json
// Request
{
  "query": "recursive CTE traversal",
  "tags": ["graph"],
  "limit": 10
}
```

### 3.8 `knox_list_stale`
Retrieves superseded actions (`is_current = false`) to inspect outdated architectural patterns.
```json
// Request
{
  "project": "knox",
  "limit": 50
}
```

### 3.9 `knox_scan_staged`
Scans git index for human modifications and records `human_write` entries.
```json
// Request
{
  "project": "."
}

// Response
{
  "human_edits_detected": 1,
  "actions_recorded": 1
}
```

---

## 4. Taxonomy & Conventions

### 4.1 Actors (`actor`)
Use the canonical lowercase identifier of the AI assistant or user:
- `gemini` — Gemini CLI, Antigravity, or Google Cloud Code Assist
- `cursor` — Cursor IDE agent
- `claude` — Claude Desktop, Claude Code, or Anthropic API
- `chatgpt` — OpenAI ChatGPT or Codex
- `copilot` — GitHub Copilot Chat
- `human` — Manual developer edits detected via git diff

### 4.2 Action Types (`action_type`)
- `plan` — High-level technical design, user requirement breakdown, or roadmap.
- `decide` — Strategic decision, pattern selection, tradeoff evaluation, or library choice.
- `write` — Code creation, file addition, or major refactoring.
- `change` — Incremental bug fix, styling tweak, or configuration adjustment.
- `debug` — Log inspection, test execution, root cause analysis, or verification.
- `read` — Context extraction, exploratory code inspection, or codebase survey.
- `review` — Code review, audit against standards, or verification sign-off.

### 4.3 Edge Naming Convention
Graph edges generated from actions follow `<actor>_<action_type>`:
- `gemini_plan`, `gemini_decide`, `gemini_write`, `gemini_debug`
- `cursor_plan`, `cursor_write`, `cursor_review`
- `claude_decide`, `claude_write`
- `human_write`

---

## 5. Direct CLI Reference

When MCP is unavailable or for local shell inspection:

| Operation | Command | Purpose |
|---|---|---|
| **Status Overview** | `knox status` | Shows total actions, open sessions, and distribution metrics |
| **Inspect File Context** | `knox query src/main.rs` | Queries non-superseded context for a specific file |
| **Keyword Search** | `knox query "parser error"` | FTS5 full-text search across summaries |
| **JSON Output** | `knox status --json` | Machine-readable JSON output for scripting |
| **Human Edit Scan** | `knox scan` | Inspects `git diff --cached` for untracked human modifications |
| **Git Hook Toggle** | `knox hook install` / `uninstall` | Installs `.git/hooks/pre-commit` automated scanner |
| **Ingest Transcript** | `knox ingest transcript.jsonl` | Imports conversation transcripts into the graph |
| **Launch MCP Server** | `knox serve --stdio` | Spawns standard JSON-RPC 2.0 stdio server |

---

## 6. Connection & Configuration

- **MCP stdio Command**: `knox serve --stdio`
- **Global Database Path**: `C:/Users/Diwa/.knox/knox.db`
- **Inactivity Session Timeout**: 1800 seconds (30 minutes)
- **Environment Variables**:
  - `KNOX_PATH`: Override database path
  - `KNOX_TIMEOUT`: Override session timeout in seconds
  - `KNOX_LOG`: Set tracing log filter (`warn`, `info`, `debug`)
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
        assert!(rendered.contains("knox_start_session"));
        assert!(rendered.contains("knox_end_session"));
        assert!(rendered.contains("knox_scan_staged"));
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
