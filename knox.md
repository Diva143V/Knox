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
  - knox_record_batch
  - knox_get_context
  - knox_get_history
  - knox_get_lineage
  - knox_get_impact
  - knox_get_path
  - knox_check_guardrails
  - knox_search
  - knox_list_stale
  - knox_scan_staged
resources_available:
  - knox://project/overview
  - knox://decisions/active
  - knox://context/{file}
prompts_available:
  - architectural-audit
  - session-handover
created_at: "2026-10-29T18:25:18.457451100+00:00"
knox_version: "1.2.1"
-->

# Knox — Active Knowledge Graph

> **External long-term memory for AI coding agents.**  
> Knox persists plans, architectural decisions, and code modifications in a queryable knowledge graph. Instead of wasting thousands of tokens rediscovering project context, AI agents query Knox to instantly understand what was tried, what was decided, and why.

---

## 1. Core Operating Principles

1. **Read Before Acting (Zero Token Waste)**: Never re-derive codebase architecture, previous attempts, or design decisions from scratch. Query Knox first to retrieve prior context.
2. **Append-Only Immutability**: Historical records are never deleted. When an approach or architecture changes, supersede the previous decision (`supersedes: "<old_action_id>"`).
3. **Causal Lineage & Semantic Graph**: Connect actions into directed graphs using `parent_action_id` and first-class `relations` (`depends_on`, `implements`, `verifies`, `relates_to`, `reverts`).
4. **Explicit Edge Attribution**: Identify your agent in `actor` (e.g., `gemini`, `cursor`, `claude`, `codex`, `human`).
5. **Decisions are First-Class Citizens**: Architectural decisions (`knox_record_decision`) carry the highest weight in the graph. Always document rationale, symbols touched, and rejected alternatives.

---

## 2. For AI Agents — MANDATORY PROTOCOL (Streamlined 3-Phase Lifecycle)

Knox supports **Smart Implicit Sessions**: agents can immediately record actions without an explicit `knox_start_session` ceremony. Knox transparently binds actions to the agent's active session or spawns one automatically.

```
┌────────────────────────────────────────────────────────────────────────┐
│ Phase 1: Pre-flight Safety & Context   → knox_check_guardrails / ctx  │
└───────────────────────────────────┬────────────────────────────────────┘
                                    │ (checks active constraints & superseded anti-patterns)
┌───────────────────────────────────▼────────────────────────────────────┐
│ Phase 2: Execute & Record Changes     → knox_record_decision / batch   │
└───────────────────────────────────┬────────────────────────────────────┘
                                    │ (logs rationale, symbols, and semantic relations)
┌───────────────────────────────────▼────────────────────────────────────┐
│ Phase 3: Graph Navigation & Audit      → knox_get_lineage / impact     │
└────────────────────────────────────────────────────────────────────────┘
```

### Phase 1: Pre-flight Safety & Context Retrieval
Before touching files:
- Call `knox_check_guardrails` with `files: ["path/to/target.rs"]` to inspect active decisions and ensure you don't repeat superseded patterns.
- Call `knox_get_context` to retrieve token-efficient Markdown context for relevant files, symbols, or topics.

### Phase 2: Record Decisions & Modifications
- **When choosing an approach**: Call `knox_record_decision` with `summary`, `rationale`, and `rejected_alternatives`.
- **After code changes or test runs**: Call `knox_record_action` with `files`, `symbols`, and `relations`.
- **For multi-step changes**: Call `knox_record_batch` with `link_to_previous: true` to record an atomic causal chain in a single turn.

### Phase 3: Graph Navigation & Verification
- Use `knox_get_lineage` to trace the causal origin of a bug or architecture ("Why is this code here?").
- Use `knox_get_impact` to assess downstream dependencies before refactoring.
- Use `knox_get_path` to find the shortest causal bridge between two decisions or actions.

---

## 3. MCP Tool Reference & Schemas

Knox exposes 14 tools, 3 resources, and 2 prompts over JSON-RPC 2.0 stdio transport (`knox serve --stdio`).

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
Records an architectural or design decision. Forces `action_type = "decide"`.
```json
// Request
{
  "actor": "gemini",
  "summary": "Adopted SQLite WAL mode for concurrent multi-agent read/write",
  "files": ["src/storage/db.rs"],
  "rationale": "High throughput read-heavy workloads without reader-writer lock contention",
  "rejected_alternatives": ["RocksDB", "DuckDB", "Flat JSON"]
}
```

### 3.4 `knox_record_batch`
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

### 4.2 Action Types (`action_type`)
- `plan` — Technical roadmap or requirement breakdown.
- `decide` — Strategic architectural choice or tradeoff evaluation.
- `write` — Code creation, refactoring, or file additions.
- `change` — Incremental bug fix or configuration edit.
- `debug` — Root-cause analysis, log inspection, or diagnosis.
- `read` — Context extraction or codebase survey.
- `review` — Verification audit or security check.
- `verify` — Automated or manual test verification.

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
