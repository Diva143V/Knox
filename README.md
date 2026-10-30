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

- **Persistent Knowledge Graph:** Backed by SQLite (WAL mode, foreign keys, FTS5 full-text search) with petgraph in-memory traversals.
- **Native Desktop GUI & Installer Wizard:** Hardware-accelerated, pure Rust desktop application (`knox gui`) featuring a classic installer setup wizard, interactive 2D Knowledge Graph visualizer with force-directed physics, metrics dashboard, query console, and live transcript studio.
- **Model Context Protocol (MCP) Server:** Standard JSON-RPC 2.0 stdio server (`knox serve --stdio`) exposing 9 rich tools for any MCP-compatible IDE or assistant.
- **Transcript Ingestion:** Automatically transforms IDE conversation logs (e.g. Antigravity's `transcript.jsonl`) into high-signal knowledge graph entries without LLM dependencies.
- **Human Edit Detection:** Detects developer edits made outside of AI sessions via Git staged changes (`git diff --cached`) and registers them in the knowledge graph.
- **Automated Git Hook:** One-command pre-commit hook integration (`knox hook install`) ensuring the knowledge graph stays in sync on every commit.
- **Causal Linking & Supersession:** Connects actions into directed acyclic graphs of intent (`parent_action_id`) and tracks when decisions supersede older ones (`supersedes`).

---

## Architecture

Knox is engineered with strict downward layer dependencies:

```
┌────────────────────────────────────────────────────────┐
│                 Layer 5: CLI & GUI                     │  src/main.rs, src/cli/, src/gui/
└───────────────────────────┬────────────────────────────┘
                            │
┌───────────────────────────▼────────────────────────────┐
│                  Layer 4: MCP Server                   │  src/mcp/
└───────────────────────────┬────────────────────────────┘
                            │
┌───────────────────────────▼────────────────────────────┐
│                  Layer 3: Commands                     │  src/commands/
└─────────────┬────────────────────────────┬─────────────┘
              │                            │
┌─────────────▼───────────┐    ┌───────────▼─────────────┐
│     Layer 2: Graph      │    │     Layer 2: Ingest     │  src/graph/, src/ingest/
└─────────────┬───────────┘    └───────────┬─────────────┘
              │                            │
┌─────────────▼────────────────────────────▼─────────────┐
│                  Layer 1: Storage                      │  src/storage/
└───────────────────────────┬────────────────────────────┘
                            │
┌───────────────────────────▼────────────────────────────┐
│                  Layer 0: Core Types                   │  src/core/
└────────────────────────────────────────────────────────┘
```

---

## Installation

### Pre-compiled Release Binaries

Download pre-built standalone binaries for Linux, macOS, and Windows from [GitHub Releases](https://github.com/Diva143V/Knox/releases). Each release includes SHA256 checksums and has the complete SQLite engine bundled with zero external runtime dependencies.

### From Crates.io

```bash
cargo install knox
```

### From Source (Cargo)

Prerequisites: Rust 1.85+ (Edition 2024).

```bash
git clone https://github.com/Diva143V/Knox.git
cd Knox
cargo install --path .
```

---

## IDE & Agent MCP Configuration

Connect Knox to any MCP-compatible IDE or coding agent by adding it to your MCP configuration:

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

### Antigravity / Gemini Code Assist
Add to your project's `.gemini/settings.json` or configure via `knox.md`:
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

Launch Knox as a stdio MCP server for your IDE (Cursor, Claude Desktop, Antigravity):

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

### 6. Launch Desktop GUI & Setup Wizard

Launch the hardware-accelerated pure Rust desktop GUI cockpit:

```bash
knox gui
```

The GUI includes:
1. **Welcome & Diagnostics:** System health check, SQLite database status, and setup guide.
2. **Workspace & Hook Wizard:** Directory picker, `knox.md` viewer, and one-click Git hook toggle.
3. **Interactive Knowledge Graph Canvas:** 2D force-directed physics layout with pan, zoom, node drag, hover tooltips, and detailed metadata inspector drawer.
4. **Metrics & Deep Statistics:** Action totals, session counts, actor distributions, and action type breakdowns.
5. **Live Query Console:** Full-text search and filtering with interactive tabular action list.
6. **Transcript Ingestion Studio:** Native file picker and live progress reporting for transcript imports.
7. **Human Edit Scanner:** Instant scan of Git staged changes with diff stats and one-click graph sync.
8. **MCP Server & IDE Configurator:** Live tool directory and ready-to-copy JSON snippets for Cursor, Claude Desktop, and Antigravity.

---

## CLI Reference

| Command | Description | Example |
|---|---|---|
| `knox init [PATH]` | Initializes workspace, creates `knox.md`, and sets up database. | `knox init .` |
| `knox ingest <PATH>` | Imports a conversation transcript into the knowledge graph. | `knox ingest transcript.jsonl` |
| `knox query <PATTERN>` | Queries context by file, keyword, tag, or actor prefix. | `knox query "src/lib.rs"` |
| `knox status` | Displays aggregate status report and actor/action breakdowns. | `knox status --json` |
| `knox scan [PROJECT]` | Scans Git staged changes for unrecorded human modifications. | `knox scan` |
| `knox hook <ACTION>` | Installs or uninstalls the Git pre-commit hook. | `knox hook install` |
| `knox gui [PATH]` | Launches interactive desktop GUI and installer wizard. | `knox gui` |
| `knox serve` | Launches the MCP stdio JSON-RPC server. | `knox serve --stdio` |

---

## MCP Tools Reference

When running `knox serve`, Knox exposes 9 tools over the Model Context Protocol:

### 1. `knox_start_session`
Starts a new work session and returns a `session_id`.
- **Inputs:** `actor` (string, required), `project` (string, optional), `metadata` (object, optional).
- **Output:** `{ "session_id": "...", "created_at": "..." }`.

### 2. `knox_end_session`
Closes an active session.
- **Inputs:** `session_id` (string, required).
- **Output:** `{ "status": "closed", "actions_recorded": N, "duration_seconds": N }`.

### 3. `knox_record_action`
Records an action (write, debug, read, review, plan, change) linked to a session.
- **Inputs:** `session_id`, `actor`, `action_type`, `summary` (required); `files`, `parent_action_id`, `tags`, `confidence`, `supersedes` (optional).
- **Output:** `{ "id": "...", "status": "recorded", "timestamp": "..." }`.

### 4. `knox_record_decision`
Records a design decision (forces `action_type = "decide"`).
- **Inputs:** `session_id`, `actor`, `summary` (required); `files`, `parent_action_id`, `tags`, `confidence`, `supersedes` (optional).
- **Output:** `{ "id": "...", "status": "recorded", "timestamp": "..." }`.

### 5. `knox_get_context`
Retrieves current context for a file or project (excludes superseded actions by default).
- **Inputs:** `file_path`, `project`, `session_id`, `actor`, `since`, `current_only` (default `true`), `tags`, `limit` (default 20).
- **Output:** `{ "results": [...], "total_count": N, "has_more": false }`.

### 6. `knox_get_history`
Full append-only history including superseded actions (`current_only` defaults to `false`).
- **Inputs:** Same as `knox_get_context`.
- **Output:** Same flat list format as `knox_get_context`.

### 7. `knox_search`
Full-text keyword search across action summaries with optional metadata filters.
- **Inputs:** `query` (string, required); `project`, `actor`, `tags`, `since`, `limit` (optional).
- **Output:** `{ "results": [...], "total_count": N, "has_more": false }`.

### 8. `knox_list_stale`
Lists superseded actions for cleanup and historical review.
- **Inputs:** `project`, `file_path`, `limit` (default 50).
- **Output:** Flat list of actions where `is_current: false`.

### 9. `knox_scan_staged`
Scans git staged modifications and records any untracked human edits.
- **Inputs:** `project` (string, optional, defaults to `.`).
- **Output:** `{ "human_edits_detected": N, "actions_recorded": N }`.

---

## Configuration & Environment Variables

| Variable | Default | Description |
|---|---|---|
| `KNOX_PATH` | `~/.knox/knox.db` | Filesystem path to the SQLite knowledge graph database. |
| `KNOX_TIMEOUT` | `1800` | Inactivity timeout in seconds before closing idle sessions (30 minutes). |
| `KNOX_LOG` | `warn` | Logging level filter (`trace`, `debug`, `info`, `warn`, `error`). |

---

## Contributing

1. Clone the repository: `git clone https://github.com/Diva143V/Knox.git`
2. Run test suite: `cargo test`
3. Run linter: `cargo clippy --all-targets`
4. Format check: `cargo fmt --check`

---

## License

This project is licensed under the [MIT License](LICENSE).
