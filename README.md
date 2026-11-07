# Knox 🛡️

[![CI](https://github.com/Diva143V/Knox/actions/workflows/ci.yml/badge.svg)](https://github.com/Diva143V/Knox/actions/workflows/ci.yml)
[![Release](https://github.com/Diva143V/Knox/actions/workflows/release.yml/badge.svg)](https://github.com/Diva143V/Knox/releases)
[![Crates.io](https://img.shields.io/crates/v/knox.svg)](https://crates.io/crates/knox)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](https://opensource.org/licenses/MIT)
[![Rust 1.85+](https://img.shields.io/badge/rust-1.85%2B-orange.svg)](https://www.rust-lang.org)

**Knox** is a persistent, cross-platform knowledge graph for AI-assisted software development. Built in Rust, it captures architectural decisions, implementation actions, and human edits across disparate coding sessions and AI coding agents (Antigravity, Cursor, Claude Code, Gemini CLI, etc.).

Knox acts as an external long-term memory for AI agents, providing contextual continuity across agent crashes, conversation restarts, and tool switches.

---

## Key Features

- **Unified Hybrid Binary (`knox.exe`):** Single binary running under the Windows GUI subsystem (`windows_subsystem = "windows"`). Opens the desktop application with zero console flash when double-clicked, launched from shortcuts, or run without arguments; dynamically attaches to the parent console (`AttachConsole`) for CLI subcommands (`knox query`, `knox status`, `knox scan`); and preserves clean stdio pipes (`FILE_TYPE_PIPE`) when spawned by AI agents (`knox serve --stdio`).
- **Native Desktop GUI (Tauri v2 + React 19):** Modern, dark-first desktop experience featuring an interactive Knowledge Graph canvas, telemetry dashboard, FTS5 query console, live transcript ingestion studio, and Git human-edit scanner.
- **Cosmos Orb Mode & Dynamic D3 Physics Simulation:** Dual-mode graph canvas allowing developers to switch between structured rectangular cards and luminous Cosmos circular nodes (`CircleNode.tsx`) with role-based glowing radial auras. Integrated continuous live D3 force physics simulation (`useLivePhysics.ts`) provides interactive node grabbing, spring-link constraints, and fluid reheat motion.
- **Interactive Getting Started Tour with Peek Window Mode:** 9-step guided walkthrough modal (`OnboardingTourModal.tsx`) with live background tab synchronization, keyboard shortcuts, and instant "Peek Window" toggle (<kbd>P</kbd> or Eye icon) that docks into a floating bottom pill so developers can view the entire live application screen in motion without obstruction.
- **Focused Model Context Protocol (MCP) Server (7 Tools):** Standard JSON-RPC 2.0 stdio server (`knox serve --stdio`) exposing seven high-level workflow tools — begin, context, check, trace, record, handoff, and ask. Legacy tool names are no longer routed.
- **Token-Compact Markdown Digest Engine:** Automatically formats query and context outputs into dense, high-signal Markdown digests, reducing context window token consumption compared to raw JSON envelopes (measured via knox metrics).
- **Token-Efficiency Measurements:** The public `knox::metrics::measure_token_efficiency` helper reports estimated tool-definition and context-format tokens, plus compact/Markdown savings against serialized action JSON.
- **Native MCP Resources & Prompts:** Exposes live architectural resources (`knox://project/overview`, `knox://decisions/active`, `knox://context/{file}`) and guided handover prompts (`/architectural-audit`, `/session-handover`).
- **V2 Schema & Semantic Relations:** First-class directional relationship graph (`depends_on`, `implements`, `verifies`, `relates_to`, `reverts`), code symbol indexing, and Architectural Decision Records (ADRs) with rationale and prohibited rejected alternatives.
- **Windows NSIS Setup Wizard & Dedicated Uninstaller:** Consumer-grade installer package (`Knox_2.1.0_x64-setup.exe`, 5.46 MB with LZMA compression). Offers dual install scope (per-user `%LOCALAPPDATA%\Programs\Knox` default requiring no elevation, or machine-wide `Program Files\Knox`), creates Start Menu and Desktop shortcuts, and registers `uninstall.exe` in Windows Settings > Installed apps.
- **In-App Onboarding Wizard:** First-run interactive setup (`OnboardingWizard.tsx`) verifying SQLite database health, guiding keyboard shortcuts, and providing 1-click MCP configuration for Antigravity, Cursor, Claude Code, and Gemini CLI. Re-accessible anytime via the "Setup Guide" button in the desktop header.
- **Synchronous Shell Wrapper (`knox.cmd`):** Ships with a native Windows CMD/PowerShell launcher to ensure prompt synchronization and clean return codes during terminal operations.
- **Transcript Ingestion:** Automatically transforms IDE conversation logs (e.g. Antigravity's `transcript.jsonl`) into high-signal knowledge graph entries without LLM dependencies.
- **Human Edit Detection:** Detects developer edits made outside of AI sessions via Git staged changes (`git diff --cached`) and registers them in the knowledge graph.
- **Automated Git Hook:** One-command pre-commit hook integration (`knox hook install`) ensuring the knowledge graph stays in sync on every commit.

---

## Architecture

Knox v2.1.0 is organized into three clean layers:

```
┌────────────────────────────────────────────────────────┐
│  Presentation: React 19 + Vite + Tailwind v4 + shadcn  │  ui/ (SPA, Zustand, React Flow)
└───────────────────────────┬────────────────────────────┘
                            │ Tauri IPC (JSON-RPC)
┌───────────────────────────▼────────────────────────────┐
│  Desktop Shell & Unified Binary (knox.exe / knox.cmd)  │  src-tauri/src/main.rs, lib.rs
│  - Zero console flash GUI launch (#![windows_subsystem])│
│  - Win32 Console Attach & Stdio Pipe Detection         │
└───────────────────────────┬────────────────────────────┘
                            │ Rust API calls
┌───────────────────────────▼────────────────────────────┐
│  Knox Domain Engine (Pure Rust Core Library)           │  src/
│  - Layer 4: MCP stdio server (src/mcp/)                │
│  - Layer 3: Commands orchestration (src/commands/)     │
│  - Layer 2: Graph traversal & Ingest (graph/, ingest/) │
│  - Layer 1: SQLite WAL & FTS5 (src/storage/)           │
│  - Layer 0: Core types & config (src/core/)            │
└────────────────────────────────────────────────────────┘
```

---

## Installation & Setup

### 1. Windows Setup Wizard (Recommended for End Users)

Download `Knox_2.1.0_x64-setup.exe` (~5.46 MB) from [GitHub Releases](https://github.com/Diva143V/Knox/releases).
- **Interactive Setup**: Guides through license agreement, installation directory, and shortcut creation.
- **Dual Install Scope**: Installs by default to `%LOCALAPPDATA%\Programs\Knox` without requiring administrative privileges, with an optional machine-wide installation to `C:\Program Files\Knox`.
- **Integrated Uninstaller**: Packages `uninstall.exe` and registers Knox in Windows *Settings > Installed apps* for clean removal.

### 2. Standalone Binary & Terminal Wrapper

Download the standalone `knox.exe` (17.68 MB) and `knox.cmd` wrapper from GitHub Releases and place them in your `PATH`.
- Run `knox` or `knox.exe` without arguments to launch the desktop application.
- Run `knox <subcommand>` (e.g. `knox status`, `knox query`) or use `knox.cmd` for synchronous terminal command execution in Command Prompt and PowerShell.

### 3. Building from Source

Prerequisites: Rust 1.85+ (Edition 2024), Node.js 20+, and `pnpm`.

```bash
git clone https://github.com/Diva143V/Knox.git
cd Knox

# Install frontend dependencies
pnpm install

# Run desktop app in development mode (hot-reloading UI)
pnpm tauri dev

# Build the release NSIS installer wizard (target/release/bundle/nsis/Knox_2.1.0_x64-setup.exe)
pnpm build

# Build portable standalone binary (target/release/knox.exe)
pnpm build:portable

# Run comprehensive workspace test suite
pnpm test
```

---

## IDE & Agent MCP Configuration

Connect Knox to any MCP-compatible IDE or coding agent. When spawned by an AI agent (`knox serve --stdio`), Knox detects inherited pipes (`FILE_TYPE_PIPE`) and bypasses console attachment, keeping standard I/O pristine for JSON-RPC 2.0 communication.

> 💡 **Quick Tip:** All snippets below can be copied with 1 click directly inside the Knox Desktop application via the **In-App Onboarding Wizard** or the **Setup Guide** button in the header.

### Cursor (`~/.cursor/mcp.json` or `.cursor/mcp.json`)
```json
{
  "mcpServers": {
    "knox": {
      "command": "knox",
      "args": ["serve", "--stdio"]
    }
  }
}
```

### Claude Desktop (`claude_desktop_config.json`)
```json
{
  "mcpServers": {
    "knox": {
      "command": "knox",
      "args": ["serve", "--stdio"]
    }
  }
}
```

### Claude Code
Add via Claude Code CLI:
```bash
claude mcp add knox -- knox serve --stdio
```
Or in your Claude Code config:
```json
{
  "mcpServers": {
    "knox": {
      "command": "knox",
      "args": ["serve", "--stdio"]
    }
  }
}
```

### Antigravity / Gemini CLI
Add to your workspace `.gemini/settings.json` or configure via `knox.md`:
```json
{
  "mcpServers": {
    "knox": {
      "command": "knox",
      "args": ["serve", "--stdio"]
    }
  }
}
```

---

## Quickstart

### 1. Initialize a Project

Initialize Knox in your project directory:

```bash
knox init
```

This creates `knox.md` in the project root containing machine-readable configuration and mandatory protocol instructions for AI agents, and creates the knowledge graph database at `~/.knox/knox.db`.

### 2. Ingest Existing Transcripts

Bootstrap the knowledge graph from an existing conversation log:

```bash
knox ingest /path/to/transcript.jsonl
```

### 3. Start the MCP Server

Launch Knox as a stdio MCP server for your IDE (Cursor, Claude Desktop, Antigravity, Claude Code):

```bash
knox serve --stdio
```

### 4. Query & Inspect

Check the status or query recorded context from the terminal:

```bash
# View summary metrics
knox status

# Query decisions and actions affecting a file
knox query src/parser.rs

# Search by full-text keyword
knox query parser
```

### 5. Install Git Pre-Commit Hook

Automatically record human edits whenever you run `git commit`:

```bash
knox hook install
```

### 6. Launch Desktop GUI & In-App Onboarding Wizard

Launch the desktop application by double-clicking `knox.exe`, clicking the Windows Start Menu shortcut, or running from the terminal:

```bash
knox gui
```

*(Launching `knox` without arguments also opens the GUI with zero console window flash).*

On first launch, Knox automatically presents the **In-App Onboarding Wizard** (`OnboardingWizard.tsx`):
1. **Welcome & Architecture:** Core principles of persistent local graph memory and zero context loss.
2. **Workspace & DB Validation:** Live status check for `~/.knox/knox.db` and active project `knox.md`.
3. **1-Click AI Configuration:** Ready-to-copy JSON configuration snippets for Antigravity, Cursor, Claude Code, and Gemini CLI.
4. **Keyboard Navigation:** Cheat sheet for `Ctrl+K` (Command Palette), `1-8` (View Switching), and `F5` (Refresh).

*(The Setup Guide can be reopened at any time by clicking the **Setup Guide** button in the desktop title bar).*

The desktop cockpit provides 8 specialized views:
1. **Dashboard:** High-level metrics, recent actions, database diagnostics, and quick actions.
2. **Knowledge Graph Canvas:** Interactive 2D graph with pan/zoom, node dragging, minimap, layout algorithms (Force, Hierarchical ELK, Dagre), and action detail inspector.
3. **Query Console:** Full-text search (SQLite FTS5) with filters for actor, action type, date range, and current/superseded actions.
4. **Deep Statistics:** Action totals, session counts, actor distributions, and 7-day activity trend charts.
5. **Transcript Ingestion Studio:** Native file picker and drag-and-drop ingestion with live preview.
6. **Human Edit Scanner:** Instant scan of Git staged changes (`git diff --cached`) with diff stats and one-click graph sync.
7. **MCP Server & IDE Configurator:** Live tool directory and 1-click copyable configuration snippets.
8. **Workspace & Hook Wizard:** Directory picker, `knox.md` viewer, and Git pre-commit hook toggle.

---

## CLI Reference

Knox ships with a unified binary `knox.exe`. For Windows Command Prompt and PowerShell, use the included `knox.cmd` wrapper to ensure synchronous execution and clean prompt return.

| Command | Description | Example |
|---|---|---|
| `knox gui` | Launches interactive desktop GUI with zero console window flash. | `knox gui` |
| `knox init [PATH]` | Initializes workspace, creates `knox.md`, and sets up database. | `knox init .` |
| `knox ingest <PATH>` | Imports a conversation transcript into the knowledge graph. | `knox ingest transcript.jsonl` |
| `knox query <PATTERN>` | Queries context by file, keyword, tag, or actor prefix. | `knox query "src/lib.rs"` |
| `knox status` | Displays aggregate status report and actor/action breakdowns. | `knox status --json` |
| `knox doctor [PROJECT]` | Evaluates database integrity, schema version, zombie sessions, and edge promotion candidates. | `knox doctor` |
| `knox metrics [PROJECT]` | Computes empirical retrieval hit rate, empty rate, tokens per retrieval, and latency percentiles. | `knox metrics` |
| `knox scan [PROJECT]` | Scans Git staged changes for unrecorded human modifications. | `knox scan` |
| `knox hook <ACTION>` | Installs or uninstalls the Git pre-commit hook (`install` \| `uninstall`). | `knox hook install` |
| `knox ask <QUESTION>` | Grounded technical question answering across project memory. | `knox ask "Why SQLite?"` |
| `knox check <FILES...>` | Pre-flight guardrail check against active architectural constraints. | `knox check src/db.rs --intent "Use Redis"` |
| `knox serve` | Launches the MCP stdio JSON-RPC server (`--stdio`). | `knox serve --stdio` |
| `knox --version` | Displays current version (`knox 2.1.0`). | `knox --version` |
| `knox --help` | Displays command-line help and usage documentation. | `knox --help` |

---

## Empirical Metrics & Verification

Knox provides built-in empirical evaluation tools to measure retrieval quality, token efficiency, operational latency, and guardrail precision. Every metric published by Knox is defined with an explicit mathematical formula and denominator, and can be reproduced with a single command.

### Reproducibility Commands

```bash
# 1. Inspect real-time operational retrieval telemetry and database health
knox metrics

# 2. Run operational diagnostics, integrity check, and edge promotion analysis
knox doctor

# 3. Reproduce the benchmark recall, latency, and token efficiency suite
cargo test --test test_bench -- --nocapture

# 4. Reproduce the 50-pair token-overlap calibration report
cargo test --test test_calibration -- --nocapture
```

### Metrics Definitions & Denominators

| Metric | Formula | Denominator | Purpose |
|---|---|---|---|
| **Hit Rate** | `hits / total_queries` | Total read queries in `query_log` | Measures proportion of retrieval requests returning at least one relevant record (`rows > 0`). |
| **Empty Rate** | `empty_queries / total_queries` | Total read queries in `query_log` | Measures proportion of retrieval requests returning zero records or an explicit empty reason. |
| **Empty Classification** | `count(reason) / total_queries` | Total read queries in `query_log` | Categorizes empty results into `no_records` (database empty), `no_match` (query didn't match), or `out_of_scope` (path outside workspace). |
| **Tokens / Retrieval** | `successful_tokens / hits` | Total successful retrieval queries (`hits`) | Measures average token payload emitted per successful context query. |
| **Latency (p50 / p95)** | Percentiles of query `latency_ms` | Total queries recorded in `query_log` | Quantifies median and tail response latency in milliseconds under real workloads. |
| **Duplicate Rate** | `duplicate_actions / total_actions` | Total recorded actions in project | Tracks cohesion and deduplication across multiple sessions and transcript ingests. |
| **Tool Definition Footprint** | `estimate_tokens(all_tools())` | Registered MCP tools (`TOOL_COUNT = 7`) | Enforces protocol lightness (budgeted target: <= 850 tokens). |
| **Guardrail Overlap Score** | `matched_tokens / alt_tokens` | Total content tokens in rejected alternative | Evaluates semantic overlap against prohibited ADR alternatives. Calibrated across 50 labeled pairs to empirically justify the 0.60 threshold. |

---

## Knowledge Graph Architecture: The 4 Grounded Questions

Knox operates an in-process, acyclic causal knowledge graph backed by SQLite WAL and `petgraph`. Rather than treating a graph as an associative keyword cloud, Knox earns the word "graph" by answering four concrete engineering questions with fail-closed precision:

### 1. Why does this exist? (Causal Ancestry)
- **Question:** Given a line of code, file, or action, what original intent, plan, or architectural decision prompted it?
- **Live Path:** `knox_trace` (`direction: "ancestors"`), `knox trace --from <ACTION_ID>`, or `knox trace --file <PATH>`.
- **Mechanism:** Starting from the anchor action, walks incoming causal edges (`caused_by`, `implements`, `supersedes`, `depends_on`, `verifies`, `reverts`, `resolves`) back to the root plan or decision.
- **Guarantee:** Cycle-safe bounded breadth-first search. Associative relations (`relates_to`) and conflict markers (`contradicts`) are strictly excluded so ancestry reflects actual causal lineage rather than topical association.

### 2. What breaks if I change this? (Downstream Impact)
- **Question:** If an architectural decision or core file is modified, what downstream actions, implementations, and files are affected?
- **Live Path:** `knox_trace` (`direction: "impact"`), `knox trace --from <ID> --direction impact`, or `knox trace --file <PATH> --direction impact`.
- **Mechanism:** File-first anchor resolution identifies all current actions covering the file, then expands the bounded directional subgraph forwards along outgoing causal edges.
- **Guarantee:** Strict causal edge filtering in both the recursive SQLite CTE and in-memory PetGraph. Blast radius is summarized (`Affects X actions across Y files`), grouped by impacted file, with an explicit warning callout if traversal hits `max_nodes`. Non-causal edges never pollute the impact report.

### 3. What contradicts what? (Architectural Conflict Tracking)
- **Question:** Does a proposed code change or new decision violate established architectural constraints or prior ADRs?
- **Live Path:** `knox_record`, `knox_check`, `knox_context`, and `knox_handoff`.
- **Mechanism:** When recording actions, active decisions covering touched files are evaluated by the local/cloud model (`router.check_contradiction`). When conflicts are detected, an explicit `contradicts` edge with `provenance: "model"` is persisted in the knowledge graph.
- **Guarantee:** Surfaced prominently in `knox_context` under the `#### ⚠️ Architectural Contradictions & Conflicts` section with rationale callouts, and reported as an audit count in `knox_handoff`.

### 4. What decisions touch this file? (File Endpoints in the Graph)
- **Question:** What architectural decisions, past implementations, or constraints govern a specific source file?
- **Live Path:** `knox_context` (`file_path: "..."`), `knox query <FILE>`, and `knox_check`.
- **Mechanism:** Files, symbols, and documents are registered as first-class endpoints in the unified `refs` and `edges` graph (e.g. `file:src/db.rs`). Actions connect via typed edges (`relates_to`, `implements`).
- **Guarantee:** File-scoped queries resolve through graph joins on `edges` (`e.to_id = 'file:' || ?`) alongside `action_files`, allowing arbitrary external references (`{"target": "file:src/db.rs", "type": "relates_to"}`) to participate in knowledge graph queries.

---

## MCP Tools Reference (7 Tools)

When running `knox serve`, Knox exposes 7 tools (`TOOL_COUNT = 7`) over the Model Context Protocol. Legacy tool names are no longer routed.

### 1. `knox_begin`
Resumes or starts a scoped turn session, returning active task context and handoff digests from previous sessions.
- **Inputs:** `workspace` (string, required), `actor` (string, required), `task` (string, optional), `budget` (integer, optional).
- **Output:** `{ "scope": { "session_id": "...", "project": "...", "actor": "..." }, "context": { ... } }`.

### 2. `knox_context`
Returns compact, token-budgeted engineering context with grouped markdown digests, active decisions, and conflict warnings.
- **Inputs:** `project` (string, required), `file_path` (string, optional), `symbols` (array, optional), `synthesis` (bool, optional), `budget` (integer, optional, default 1200).
- **Output:** Grouped markdown digest (`mode: "grouped"`) or compact one-line entries with `budget_used`, `truncated`, and `has_more` metadata.

### 3. `knox_check`
Pre-flight guardrail check against active architectural constraints and prohibited patterns before editing code.
- **Inputs:** `files` (array of strings, required), `intent` (string, optional), `symbols` (array, optional), `project` (string, optional).
- **Output:** `{ "pass": true/false, "violations_count": N, "violation_details": [...], "markdown": "..." }`.

### 4. `knox_trace`
Traces causal ancestry or downstream impact using the bounded knowledge graph.
- **Inputs:** `from` (action ID, optional), `file_path` (string, optional), `direction` ("ancestors" | "descendants" | "impact" | "both", default "ancestors"), `max_depth` (default 10), `max_nodes` (default 200), `budget` (default 800).
- **Output:** Ordered DAG lineage, downstream impact grouped by file, or bidirectional blast radius with token-bounded markdown.

### 5. `knox_record`
Atomic structured batch entry recorder for actions, decisions, and doubts with deduplication and contradiction checking.
- **Inputs:** `actor` (string, required), `project` (string, optional), `entries` (array of action objects, required), `session_id` (string, optional), `dry_run` (bool, optional), `idem` (string, optional).
- **Output:** `{ "status": "recorded", "session_id": "...", "ids": [...], "deduped": [...], "warnings": [...] }`.

### 6. `knox_handoff`
Finalizes an active work session, produces a concise handoff digest, and audits active contradictions for the next turn.
- **Inputs:** `session_id` (string, required), `budget` (integer, optional, default 800).
- **Output:** `{ "status": "closed", "session_id": "...", "ended_at": "...", "contradictions": N, "digest": { ... } }`.

### 7. `knox_ask`
Grounded technical question answering across project memory with verified citation enforcement.
- **Inputs:** `question` (string, required), `project` (string, required), `budget` (integer, optional, default 1200), `strict_citations` (bool, optional, default true).
- **Output:** `{ "answer": "...", "citations": [...], "grounded": true, "confidence": 0.95 }`.

### Database Migration Operations

Knox provides explicit migration safety commands for existing databases:

```text
knox migrate check
knox migrate apply
knox migrate restore
knox migrate split --output <directory>
```

`apply` creates an adjacent timestamped backup before applying pending migrations
and validates SQLite integrity plus project/action/relationship orphan counts.
`restore` restores the newest adjacent backup. `split` creates one validated
project-local database per registered project and leaves the source database
untouched.

---

## Native MCP Resources & Prompts

### Resources (`resources/*`)
- `knox://project/overview` — High-level knowledge graph telemetry, action counts, and recent activity.
- `knox://decisions/active` — Live architectural constraints, design decisions, and ADR rationales.
- `knox://context/{file_path}` — Non-superseded context digest for any file in the workspace.

### Prompts (`prompts/*`)
- `architectural-audit` — Guided audit of active decisions, stale patterns, and constraint compliance.
- `session-handover` — Synthesized context digest for handing over work smoothly to the next agent.

---

## Configuration & Environment Variables

| Variable | Default | Description |
|---|---|---|
| `KNOX_PATH` | `~/.knox/knox.db` | Filesystem path to the SQLite knowledge graph database. |
| `KNOX_TIMEOUT` | `1800` | Inactivity timeout in seconds before closing idle sessions (30 minutes). |
| `KNOX_LOG` | `warn` | Logging level filter (`trace`, `debug`, `info`, `warn`, `error`). |

---

## Release Notes

### v2.1.0
- **Seventh MCP Workflow Tool (`knox_ask`):** Added grounded technical question answering synthesizing answers across project memory with strict citation verification and prompt injection defenses.
- **Full CLI & MCP Parity (`knox ask` & `knox check`):** Added native CLI subcommands for human verification matching agent MCP workflow capabilities.
- **Response Field Normalization:** Standardized guardrail violation reporting from `confidence` to empirical `overlap` score.
- **Automated Schema Migrations (v9 & v10):** Added Git remote URL normalization backfill (Schema v9) and vector embedding cosine similarity tables (Schema v10).
- **Safe Upgrade Lifecycle:** Existing Knox databases automatically and safely migrate on first open.

---

## Contributing

1. Clone the repository: `git clone https://github.com/Diva143V/Knox.git`
2. Install dependencies: `pnpm install`
3. Run workspace test suite: `pnpm test` (or `cargo test --workspace`)
4. Typecheck frontend: `pnpm typecheck`
5. Run Rust linter: `pnpm lint` (or `cargo clippy --workspace --all-targets`)
6. Build release packages: `pnpm build` (NSIS wizard) or `pnpm build:portable` (standalone `knox.exe`)

---

## License

This project is licensed under the [MIT License](LICENSE).
