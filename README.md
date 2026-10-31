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
- **Native Desktop GUI (Tauri v2 + React 19):** Modern, dark-first desktop experience featuring an interactive 2D Knowledge Graph visualizer with force-directed and hierarchical layouts, metrics dashboard, FTS5 query console, live transcript ingestion studio, and Git human-edit scanner.
- **Windows NSIS Setup Wizard & Dedicated Uninstaller:** Consumer-grade installer package (`Knox_1.2.1_x64-setup.exe`, 5.46 MB with LZMA compression). Offers dual install scope (per-user `%LOCALAPPDATA%\Programs\Knox` default requiring no elevation, or machine-wide `Program Files\Knox`), creates Start Menu and Desktop shortcuts, and registers `uninstall.exe` in Windows Settings > Installed apps.
- **In-App Onboarding Wizard:** First-run interactive setup (`OnboardingWizard.tsx`) verifying SQLite database health, guiding keyboard shortcuts, and providing 1-click MCP configuration for Antigravity, Cursor, Claude Code, and Gemini CLI. Re-accessible anytime via the "Setup Guide" button in the desktop header.
- **Synchronous Shell Wrapper (`knox.cmd`):** Ships with a native Windows CMD/PowerShell launcher to ensure prompt synchronization and clean return codes during terminal operations.
- **Model Context Protocol (MCP) Server:** Standard JSON-RPC 2.0 stdio server (`knox serve --stdio`) exposing 9 rich tools for any MCP-compatible IDE or assistant.
- **Transcript Ingestion:** Automatically transforms IDE conversation logs (e.g. Antigravity's `transcript.jsonl`) into high-signal knowledge graph entries without LLM dependencies.
- **Human Edit Detection:** Detects developer edits made outside of AI sessions via Git staged changes (`git diff --cached`) and registers them in the knowledge graph.
- **Automated Git Hook:** One-command pre-commit hook integration (`knox hook install`) ensuring the knowledge graph stays in sync on every commit.
- **Causal Linking & Supersession:** Connects actions into directed acyclic graphs of intent (`parent_action_id`) and tracks when decisions supersede older ones (`supersedes`).

---

## Architecture

Knox v1.2.1 is organized into three clean layers:

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

Download `Knox_1.2.1_x64-setup.exe` (~5.46 MB) from [GitHub Releases](https://github.com/Diva143V/Knox/releases).
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

# Build the release NSIS installer wizard (target/release/bundle/nsis/Knox_1.2.1_x64-setup.exe)
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
| `knox scan [PROJECT]` | Scans Git staged changes for unrecorded human modifications. | `knox scan` |
| `knox hook <ACTION>` | Installs or uninstalls the Git pre-commit hook (`install` \| `uninstall`). | `knox hook install` |
| `knox serve` | Launches the MCP stdio JSON-RPC server (`--stdio`). | `knox serve --stdio` |
| `knox --version` | Displays current version (`knox 1.2.1`). | `knox --version` |
| `knox --help` | Displays command-line help and usage documentation. | `knox --help` |

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
2. Install dependencies: `pnpm install`
3. Run workspace test suite: `pnpm test` (or `cargo test --workspace`)
4. Typecheck frontend: `pnpm typecheck`
5. Run Rust linter: `pnpm lint` (or `cargo clippy --workspace --all-targets`)
6. Build release packages: `pnpm build` (NSIS wizard) or `pnpm build:portable` (standalone `knox.exe`)

---

## License

This project is licensed under the [MIT License](LICENSE).
