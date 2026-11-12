<!-- knox:config
project_id: "eb624457-04f1-41cc-98fd-08d84e9ded65"
kg_path: "C:/Users/Diwa/.knox/knox.db"
mcp_server: "knox serve --stdio"
mcp_transport: "stdio"
tools_registered:
  - knox_begin
  - knox_context
  - knox_check
  - knox_trace
  - knox_record
  - knox_handoff
  - knox_ask
  - knox_update
resources_available:
- knox://project/overview
- knox://decisions/active
- knox://context/{file}
prompts_available:
- architectural-audit
- session-handover
created_at: "2026-10-29T18:25:18.457451100+00:00"
knox_version: "2.1.0"
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
