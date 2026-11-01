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
  - knox_record_doubt\n\
  - knox_resolve_doubt\n\
  - knox_record_batch\n\
  - knox_get_context\n\
  - knox_get_history\n\
  - knox_get_lineage\n\
  - knox_get_impact\n\
  - knox_get_path\n\
  - knox_check_guardrails\n\
  - knox_search\n\
  - knox_list_stale\n\
  - knox_scan_staged\n\
resources_available:\n\
  - knox://project/overview\n\
  - knox://decisions/active\n\
  - knox://context/{{file}}\n\
prompts_available:\n\
  - architectural-audit\n\
  - session-handover\n\
created_at: \"{created_at}\"\n\
knox_version: \"1.2.1\"\n\
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
5. **Decisions are First-Class Citizens**: Architectural decisions (`knox_record_decision`) carry the highest weight in the graph. Always document rationale, symbols touched, and rejected alternatives.
6. **Continuous Cognitive Memory & Doubts**: Record user conversations, uncertainties, and questions as `doubt` actions (`knox_record_doubt`). Capture micro-decisions and implementation assumptions with `significance: "minor"`. When a doubt is verified or resolved, close the cognitive loop using `knox_resolve_doubt` linking the answering decision or commit.

---

## 2. For AI Agents — MANDATORY PROTOCOL (Streamlined 3-Phase Lifecycle)

Knox supports **Smart Implicit Sessions**: agents can immediately record actions without an explicit `knox_start_session` ceremony. Knox transparently binds actions to the agent's active session or spawns one automatically.

```
┌────────────────────────────────────────────────────────────────────────┐
│ Phase 1: Pre-flight Safety & Context   → knox_check_guardrails / ctx  │
└───────────────────────────────────┬────────────────────────────────────┘
                                    │ (checks active constraints & superseded anti-patterns)
┌───────────────────────────────────▼────────────────────────────────────┐
│ Phase 2: Execute & Cognitive Logging  → doubts / decisions / batch     │
└───────────────────────────────────┬────────────────────────────────────┘
                                    │ (logs doubts, major/minor choices, and resolves prior questions)
┌───────────────────────────────────▼────────────────────────────────────┐
│ Phase 3: Graph Navigation & Audit      → knox_get_lineage / impact     │
└────────────────────────────────────────────────────────────────────────┘
```

### Phase 1: Pre-flight Safety & Context Retrieval
Before touching files:
- Call `knox_check_guardrails` with `files: ["path/to/target.rs"]` to inspect active decisions and ensure you don't repeat superseded patterns.
- Call `knox_get_context` to retrieve token-efficient Markdown context for relevant files, symbols, or topics.

### Phase 2: Cognitive Capture, Decisions & Modifications
- **When user expresses doubt or questions architecture**: Call `knox_record_doubt` with `summary` and `context_summary`. Never leave user uncertainties floating unindexed.
- **When choosing an approach**: Call `knox_record_decision` with `summary`, `rationale`, and `rejected_alternatives`. Pass `significance: "minor"` for implementation micro-tradeoffs or `significance: "major"` for architectural system contracts.
- **When answering or resolving a previous doubt**: Call `knox_resolve_doubt` with `doubt_id`, `resolution_note`, and optional `resolved_by_action_id` to link the causal resolution.
- **After code changes or test runs**: Call `knox_record_action` with `files`, `symbols`, and `relations`.
- **For multi-step changes**: Call `knox_record_batch` with `link_to_previous: true` to record an atomic causal chain in a single turn.

### Phase 3: Graph Navigation & Verification
- Use `knox_get_lineage` to trace the causal origin of a bug or architecture ("Why is this code here?").
- Use `knox_get_impact` to assess downstream dependencies before refactoring.
- Use `knox_get_path` to find the shortest causal bridge between two decisions or actions.

---

## 3. MCP Tool Reference & Schemas

Knox exposes 16 tools, 3 resources, and 2 prompts over JSON-RPC 2.0 stdio transport (`knox serve --stdio`).

### 3.1 `knox_check_guardrails`
Pre-flight safety inspection for target files. Returns active decisions and superseded anti-patterns.
```json
// Request
{
  "files": ["src/storage/db.rs"],
  "format": "markdown"
}
```

### 3.2 `knox_record_action`
Records a code change, plan, debug investigation, review, or verification. `session_id` is optional (auto-inferred).
```json
// Request
{
  "actor": "gemini",
  "action_type": "write",
  "summary": "Implemented bounded BFS traversal for MCP lineage",
  "files": ["src/graph/loader.rs"],
  "symbols": ["load_subgraph", "build_subgraph_cte_sql"],
  "rationale": "Bound graph loading memory consumption under 50MB",
  "relations": [
    { "target_id": "a-91dc8f72", "relation_type": "implements" }
  ],
  "link_to_latest": true
}
```

### 3.3 `knox_record_decision`
Records an architectural or design decision. Forces `action_type = "decide"`. Accepts `significance: "major"` (default) or `"minor"`.
```json
// Request
{
  "actor": "gemini",
  "summary": "Adopted SQLite WAL mode for concurrent multi-agent read/write",
  "files": ["src/storage/db.rs"],
  "rationale": "High throughput read-heavy workloads without reader-writer lock contention",
  "rejected_alternatives": ["RocksDB", "DuckDB", "Flat JSON"],
  "significance": "major"
}
```

### 3.4 `knox_record_doubt`
Records an open technical doubt, uncertainty, or user question to track until answered and resolved.
```json
// Request
{
  "actor": "gemini",
  "summary": "Will SQLite WAL mode block Tauri UI if a large batch is being ingested?",
  "context_summary": "Evaluating background thread worker vs IPC thread pool",
  "files": ["src-tauri/src/commands/ingest.rs"]
}
```

### 3.5 `knox_resolve_doubt`
Marks a previously recorded doubt as resolved with an explanation note and optional answering decision/commit link.
```json
// Request
{
  "doubt_id": "a-89bc721a",
  "resolution_note": "Verified SQLite WAL allows concurrent readers on Tauri UI thread while worker thread writes batches.",
  "resolved_by_action_id": "a-91dc8f72"
}
```

### 3.6 `knox_record_batch`
Atomically records multiple actions in one call with optional predecessor chaining.
```json
// Request
{
  "actor": "claude",
  "link_to_previous": true,
  "actions": [
    { "action_type": "decide", "summary": "Design auth token schema", "files": ["src/auth.rs"] },
    { "action_type": "write", "summary": "Implement JWT verification", "files": ["src/auth.rs"] },
    { "action_type": "verify", "summary": "Unit test token expiry", "files": ["tests/test_auth.rs"] }
  ]
}
```

### 3.5 `knox_get_context`
Retrieves token-dense Markdown context (saves ~77% tokens vs raw JSON).
```json
// Request
{
  "file_path": "src/storage/db.rs",
  "symbols": ["KnoxDb::open"],
  "format": "markdown",
  "synthesis": true
}
```

### 3.6 `knox_get_lineage`
Traces causal ancestry backwards to answer "Why was this code or decision made?".
```json
// Request
{
  "action_id": "a-2b7c6114",
  "max_depth": 10,
  "format": "markdown"
}
```

### 3.7 `knox_get_impact`
Traces downstream descendants to answer "What will break or be impacted if I change this?".
```json
// Request
{
  "action_id": "a-91dc8f72",
  "max_depth": 5,
  "format": "markdown"
}
```

### 3.8 `knox_get_path`
Finds the shortest causal path between two actions in the knowledge graph.
```json
// Request
{
  "from_action_id": "a-origin-id",
  "to_action_id": "a-target-id"
}
```

### 3.9 `knox_get_history`
Returns chronological append-only history, including superseded actions.
```json
// Request
{
  "file_path": "src/graph/cache.rs",
  "limit": 50
}
```

### 3.10 `knox_search`
Full-text search (FTS5) across summaries, rationales, and context summaries.
```json
// Request
{
  "query": "SQLite WAL mode",
  "limit": 10
}
```

### 3.11 `knox_list_stale`
Lists superseded actions (`is_current = false`) to inspect outdated architectural patterns.
```json
// Request
{
  "project": "knox",
  "limit": 50
}
```

### 3.12 `knox_scan_staged`
Scans git index for untracked human modifications and records `human_write` actions.
```json
// Request
{
  "project": "."
}
```

### 3.13 `knox_start_session` & 3.14 `knox_end_session`
Explicit session boundary management (optional; sessions auto-manage if omitted).

### 3.15 Native MCP Resources (`resources/*`)
- `knox://project/overview` — High-level knowledge graph metrics, action counts, and recent activity.
- `knox://decisions/active` — Live architectural constraints, design decisions, and rationales.
- `knox://context/{file_path}` — Non-superseded context for any file in the workspace.

### 3.16 Native MCP Prompts (`prompts/*`)
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
        assert!(rendered.contains("knox_start_session"));
        assert!(rendered.contains("knox_end_session"));
        assert!(rendered.contains("knox_scan_staged"));
        assert!(rendered.contains("MANDATORY PROTOCOL"));
        assert!(rendered.contains("knox_record_decision"));
        assert!(rendered.contains("knox_record_doubt"));
        assert!(rendered.contains("knox_resolve_doubt"));
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
