<!-- knox:config
project_id: "eb624457-04f1-41cc-98fd-08d84e9ded65"
kg_path: "C:/Users/Diwa/.knox/knox.db"
mcp_server: "knox serve --stdio"
mcp_transport: "stdio"
tools_registered:
  - knox_start_session
  - knox_end_session
  - knox_record_action
  - knox_record_decision
  - knox_get_context
  - knox_get_history
  - knox_search
  - knox_list_stale
  - knox_scan_staged
created_at: "2026-10-29T18:25:18.457451100+00:00"
knox_version: "1.2.0"
-->

# Knox — Active Knowledge Graph

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
