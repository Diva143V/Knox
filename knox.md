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
  - knox_record_doubt
  - knox_resolve_doubt
  - knox_record_batch
  - knox_update_action
  - knox_get_context
  - knox_get_history
  - knox_get_lineage
  - knox_get_impact
  - knox_get_path
  - knox_check_guardrails
  - knox_search
  - knox_list_stale
  - knox_scan_staged
  - knox_ingest
  - knox_diff_inspect
  - knox_blame_causal
  - knox_verify_drift
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

Knox exposes 6 tools, 3 resources, and 2 prompts over JSON-RPC 2.0 stdio transport (`knox serve --stdio`).

### 3.1 `knox_begin`
Resumes or creates a scoped session and returns compact task context. Call this at the start of every turn.
```json
// Request
{
  "workspace": "/path/to/project",
  "actor": "gemini",
  "task": "Implement parser streaming",
  "budget": 1200
}
// Response includes scope.session_id, scope.resumed, scope.parent_session_id, and budgeted context
```

### 3.2 `knox_context`
Returns compact, token-budgeted engineering context for a file, symbol, query, or project.
```json
// Request
{
  "view": "file",
  "files": ["src/storage/db.rs"],
  "symbols": ["KnoxDb::open"],
  "budget": 1200
}
```

### 3.3 `knox_check`
Pre-flight guardrail check. Reads active decisions, open doubts, and superseded anti-patterns for target files before editing.
```json
// Request
{
  "files": ["src/storage/db.rs"],
  "symbols": ["KnoxDb::open"],
  "intent": "Refactor connection pooling"
}
```

### 3.4 `knox_trace`
Traces causal ancestry backwards from an action or file to answer "Why does this code exist?".
```json
// Request
{
  "from": "a-91dc8f72",
  "max_depth": 10
}
```

### 3.5 `knox_record`
Atomically records structured engineering entries (decisions, doubts, code changes) in one transaction. Includes local refs, dedupe, idempotency, and validation.
```json
// Request
{
  "actor": "gemini",
  "session_id": "s-abc123",
  "entries": [
    {
      "ref": "plan1",
      "kind": "plan",
      "summary": "Implement WAL mode",
      "files": ["src/storage/db.rs"]
    },
    {
      "kind": "write",
      "summary": "Apply WAL pragma",
      "files": ["src/storage/db.rs"],
      "caused_by": "plan1",
      "relations": [{ "to": "plan1", "relation_type": "implements" }]
    }
  ]
}
```

### 3.6 `knox_handoff`
Closes a session and returns a compact digest (open doubts, constraints touched, last outcomes, unresolved plans) for the next session to resume from.
```json
// Request
{
  "session_id": "s-abc123",
  "budget": 800
}
```

### 3.22 Native MCP Resources (`resources/*`)
- `knox://project/overview` — High-level knowledge graph metrics, action counts, and recent activity.
- `knox://decisions/active` — Live architectural constraints, design decisions, and rationales.
- `knox://context/{file_path}` — Non-superseded context for any file in the workspace.

### 3.23 Native MCP Prompts (`prompts/*`)
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
