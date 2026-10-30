# Knox — Code Production Guide

> **This document defines HOW Knox code must be written. It is the single source of truth for system architecture, module boundaries, file layout, naming conventions, error handling, and implementation order. Every AI agent and human contributor MUST read this before writing any Knox code.**

---

## System Architecture — Layer Diagram

Knox is a **layered system** with strict dependency direction: upper layers depend on lower layers, never the reverse. No circular dependencies. No layer-skipping.

```
┌─────────────────────────────────────────────────────────────────┐
│                        Layer 5: CLI                             │
│  Binary entry point. Parses args (clap), dispatches to Layer 4  │
│  Files: src/main.rs, src/cli/                                   │
└────────────────────────────┬────────────────────────────────────┘
                             │ calls
                             ▼
┌─────────────────────────────────────────────────────────────────┐
│                    Layer 4: MCP Server                          │
│  stdio JSON-RPC transport. Exposes tools. Translates MCP       │
│  requests → Layer 3 calls. Handles sessions.                   │
│  Files: src/mcp/                                                │
└────────────────────────────┬────────────────────────────────────┘
                             │ calls
                             ▼
┌─────────────────────────────────────────────────────────────────┐
│                    Layer 3: Commands                            │
│  Business logic for each operation: init, ingest, query,       │
│  status, scan. Orchestrates Layers 1+2. No I/O of its own.     │
│  Files: src/commands/                                           │
└──────────┬─────────────────────────────┬───────────────────────┘
           │ calls                       │ calls
           ▼                             ▼
┌────────────────────────┐   ┌───────────────────────────────────┐
│   Layer 2: Graph       │   │     Layer 2: Ingest               │
│   petgraph loading,    │   │     TranscriptMapper trait,       │
│   subgraph extraction, │   │     Antigravity mapper,           │
│   traversal, caching   │   │     action group clustering       │
│   Files: src/graph/    │   │     Files: src/ingest/            │
└──────────┬─────────────┘   └──────────────┬────────────────────┘
           │ calls                          │ calls
           ▼                                ▼
┌─────────────────────────────────────────────────────────────────┐
│                    Layer 1: Storage                             │
│  SQLite schema, migrations, CRUD operations for nodes/edges.   │
│  The ONLY layer that touches the database file.                │
│  Files: src/storage/                                            │
└────────────────────────────┬────────────────────────────────────┘
                             │ uses
                             ▼
┌─────────────────────────────────────────────────────────────────┐
│                    Layer 0: Core Types                          │
│  Shared data structures, error types, config. ZERO logic.      │
│  No dependencies on any other Knox layer.                      │
│  Files: src/core/                                               │
└─────────────────────────────────────────────────────────────────┘
```

### Dependency Rules

| Rule | Description |
|---|---|
| **Down only** | A layer may only call layers below it. `cli` → `mcp` → `commands` → `storage`/`graph`/`ingest` → `core`. Never upward. |
| **Core depends on nothing** | `src/core/` has zero imports from other Knox modules. Only external crates. |
| **Storage is the DB gatekeeper** | No other module opens a SQLite connection or writes raw SQL. All DB access goes through `src/storage/`. |
| **Graph never writes** | `src/graph/` is read-only. It loads data from storage into petgraph for traversal. It never mutates the DB. |
| **MCP never touches storage directly** | MCP handlers call `src/commands/` functions. Commands call storage. MCP is a transport layer, not a business logic layer. |

---

## Project File Layout

```
knox/
├── Cargo.toml                      # Workspace root / single crate
├── Cargo.lock
├── README.md
├── LICENSE                          # MIT
├── goal.md                          # Design decisions (do not delete)
├── code.md                          # This file (do not delete)
│
├── src/
│   ├── main.rs                      # Entry point: clap parse → dispatch
│   │
│   ├── core/                        # Layer 0: Shared types
│   │   ├── mod.rs                   # Re-exports everything in core
│   │   ├── types.rs                 # KnoxAction, KnoxNode, KnoxEdge, Actor, Session
│   │   ├── error.rs                 # KnoxError enum (thiserror)
│   │   ├── config.rs                # KnoxConfig (KG path, global vs per-project, timeouts)
│   │   └── ids.rs                   # ID generation (UUID v4 wrappers)
│   │
│   ├── storage/                     # Layer 1: SQLite persistence
│   │   ├── mod.rs                   # Re-exports
│   │   ├── db.rs                    # Database struct: open, close, migrate
│   │   ├── migrations.rs            # Schema versioning (CREATE TABLE, ALTER TABLE)
│   │   ├── nodes.rs                 # CRUD for nodes (Project, File, Action, Actor)
│   │   ├── edges.rs                 # CRUD for edges (contains, acted_on, targets, etc.)
│   │   ├── sessions.rs              # Session lifecycle (start, end, timeout, list)
│   │   └── queries.rs               # Complex read queries (scoped BFS, full-text search, filters)
│   │
│   ├── graph/                       # Layer 2: petgraph traversal (read-only)
│   │   ├── mod.rs                   # Re-exports
│   │   ├── loader.rs                # Load subgraph / full graph from storage → petgraph
│   │   ├── traversal.rs             # Path queries, ancestor/descendant walks, causal chains
│   │   └── cache.rs                 # LRU subgraph cache with write-through invalidation
│   │
│   ├── ingest/                      # Layer 2: Transcript ingestion
│   │   ├── mod.rs                   # Re-exports + auto-detection logic (which mapper to use)
│   │   ├── mapper.rs                # TranscriptMapper trait definition
│   │   ├── antigravity.rs           # Antigravity transcript.jsonl mapper (v1)
│   │   ├── action_group.rs          # Action group clustering state machine
│   │   └── summary.rs              # Heuristic summary generation (no LLM)
│   │
│   ├── commands/                    # Layer 3: Business logic
│   │   ├── mod.rs                   # Re-exports
│   │   ├── init.rs                  # knox init: create .knox/, generate knox.md, register project
│   │   ├── ingest.rs                # knox ingest: read transcript → mapper → storage
│   │   ├── query.rs                 # knox query: parse pattern → storage/graph → format output
│   │   ├── status.rs                # knox status: aggregate stats from storage
│   │   ├── scan.rs                  # knox scan: git staged changes → detect human edits
│   │   └── hook.rs                  # knox hook install/uninstall: git pre-commit hook management
│   │
│   ├── mcp/                         # Layer 4: MCP server
│   │   ├── mod.rs                   # Re-exports
│   │   ├── server.rs                # Tokio stdio transport, JSON-RPC dispatch loop
│   │   ├── protocol.rs              # MCP protocol types (Request, Response, Tool, etc.)
│   │   ├── tools.rs                 # Tool registration (list of all Knox MCP tools)
│   │   ├── handlers/                # One handler per MCP tool
│   │   │   ├── mod.rs
│   │   │   ├── start_session.rs     # knox_start_session handler
│   │   │   ├── end_session.rs       # knox_end_session handler
│   │   │   ├── record_action.rs     # knox_record_action handler
│   │   │   ├── record_decision.rs   # knox_record_decision handler
│   │   │   ├── get_context.rs       # knox_get_context handler
│   │   │   ├── get_history.rs       # knox_get_history handler
│   │   │   ├── search.rs            # knox_search handler
│   │   │   ├── list_stale.rs        # knox_list_stale handler
│   │   │   └── scan_staged.rs       # knox_scan_staged handler
│   │   └── session_manager.rs       # In-memory session state + 30-min auto-timeout
│   │
│   └── cli/                         # Layer 5: CLI argument parsing
│       ├── mod.rs                   # Re-exports
│       └── args.rs                  # clap derive structs: KnoxCli, InitArgs, IngestArgs, etc.
│
├── tests/                           # Integration tests
│   ├── test_storage.rs              # SQLite schema + CRUD tests
│   ├── test_graph.rs                # petgraph loading + traversal tests
│   ├── test_ingest.rs               # Transcript mapper + action grouping tests
│   ├── test_mcp.rs                  # MCP tool handler tests (mock stdio)
│   └── test_commands.rs             # End-to-end command tests
│
├── fixtures/                        # Test data
│   ├── transcript_antigravity.jsonl # Sample Antigravity transcript
│   ├── knox.db                      # Pre-seeded test database
│   └── knox_md_sample.md            # Expected knox.md output
│
└── .github/
    └── workflows/
        └── ci.yml                   # Build + test for Windows, macOS, Linux
```

### File Naming Rules

| Rule | Example |
|---|---|
| **One concern per file** | `nodes.rs` does node CRUD. `edges.rs` does edge CRUD. Never both in one file. |
| **File name = what it contains** | `antigravity.rs` contains the Antigravity mapper. Not `mapper1.rs` or `impl.rs`. |
| **`mod.rs` only re-exports** | `mod.rs` files contain `pub mod X;` and `pub use X::*;` only. No logic. |
| **Handlers match tool names** | `handlers/record_action.rs` handles `knox_record_action`. 1:1 mapping. |
| **No `utils.rs` or `helpers.rs`** | If something is shared, it belongs in `src/core/`. If it's module-specific, it stays in that module. |

---

## Module Contracts

Each module has a strict public API. Internal details are private.

### `core` — Layer 0

```rust
// src/core/types.rs — The universal data structures. Every layer uses these.

/// A single action recorded in the KG
pub struct KnoxAction {
    pub id: ActionId,
    pub actor: String,              // free string: "gemini", "cursor", "human"
    pub action_type: String,        // free string: "write", "plan", "decide", "debug"
    pub summary: String,
    pub files: Vec<String>,         // relative paths
    pub tags: Vec<String>,
    pub confidence: Option<f64>,    // 0.0–1.0, hint only
    pub timestamp: DateTime<Utc>,
    pub session_id: Option<SessionId>,
    pub parent_action_id: Option<ActionId>,
    pub supersedes: Option<ActionId>,
    pub is_current: bool,           // false if superseded by another action
}

/// Unique ID wrapper (UUID v4 string)
pub struct ActionId(pub String);
pub struct SessionId(pub String);
pub struct ProjectId(pub String);

/// A session represents one continuous work block
pub struct Session {
    pub id: SessionId,
    pub actor: String,
    pub project: Option<String>,
    pub started_at: DateTime<Utc>,
    pub ended_at: Option<DateTime<Utc>>,
    pub is_open: bool,
}

/// Query filters for knox_get_context / knox_search
pub struct QueryFilter {
    pub file_path: Option<String>,
    pub project: Option<String>,
    pub session_id: Option<SessionId>,
    pub actor: Option<String>,
    pub since: Option<DateTime<Utc>>,
    pub current_only: bool,         // default: true
    pub tags: Vec<String>,
    pub limit: usize,               // default: 20
}

/// Query results — flat list
pub struct QueryResult {
    pub results: Vec<KnoxAction>,
    pub total_count: usize,
    pub has_more: bool,
}
```

```rust
// src/core/error.rs — All errors in one enum. Never use anyhow in library code.

#[derive(Debug, thiserror::Error)]
pub enum KnoxError {
    #[error("Database error: {0}")]
    Database(#[from] rusqlite::Error),

    #[error("Session not found: {0}")]
    SessionNotFound(String),

    #[error("Session already closed: {0}")]
    SessionClosed(String),

    #[error("Action not found: {0}")]
    ActionNotFound(String),

    #[error("Invalid input: {0}")]
    InvalidInput(String),

    #[error("Transcript parse error: {0}")]
    ParseError(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Git error: {0}")]
    Git(String),
}

pub type KnoxResult<T> = Result<T, KnoxError>;
```

### `storage` — Layer 1

```rust
// src/storage/db.rs — The ONLY struct that holds a DB connection

pub struct KnoxDb {
    conn: rusqlite::Connection,     // private — nobody else touches this
}

impl KnoxDb {
    /// Open or create the database at the given path. Runs migrations.
    pub fn open(path: &Path) -> KnoxResult<Self>;

    /// Open an in-memory database (for tests)
    pub fn open_memory() -> KnoxResult<Self>;

    /// Run all pending schema migrations
    fn migrate(&self) -> KnoxResult<()>;  // private
}
```

```rust
// src/storage/nodes.rs — Node CRUD. Every fn takes &KnoxDb.

pub fn insert_action(db: &KnoxDb, action: &KnoxAction) -> KnoxResult<ActionId>;
pub fn get_action(db: &KnoxDb, id: &ActionId) -> KnoxResult<Option<KnoxAction>>;
pub fn mark_superseded(db: &KnoxDb, old_id: &ActionId, new_id: &ActionId) -> KnoxResult<()>;
pub fn query_actions(db: &KnoxDb, filter: &QueryFilter) -> KnoxResult<QueryResult>;
```

```rust
// src/storage/sessions.rs — Session lifecycle

pub fn start_session(db: &KnoxDb, actor: &str, project: Option<&str>) -> KnoxResult<Session>;
pub fn end_session(db: &KnoxDb, session_id: &SessionId) -> KnoxResult<Session>;
pub fn timeout_stale_sessions(db: &KnoxDb, max_idle: Duration) -> KnoxResult<Vec<SessionId>>;
pub fn get_session(db: &KnoxDb, session_id: &SessionId) -> KnoxResult<Option<Session>>;
```

### `graph` — Layer 2 (read-only)

```rust
// src/graph/loader.rs — Load from storage into petgraph

/// Load only the subgraph reachable from the given anchor nodes (bounded BFS)
pub fn load_subgraph(db: &KnoxDb, anchors: &[ActionId], max_depth: usize) -> KnoxResult<KnoxGraph>;

/// Load the entire graph (expensive, cached)
pub fn load_full_graph(db: &KnoxDb) -> KnoxResult<KnoxGraph>;

/// The in-memory graph type
pub type KnoxGraph = petgraph::Graph<KnoxAction, KnoxEdge, petgraph::Directed>;
```

### `ingest` — Layer 2

```rust
// src/ingest/mapper.rs — Trait that every tool's mapper implements

pub trait TranscriptMapper: Send + Sync {
    fn tool_name(&self) -> &str;
    fn can_parse(&self, raw: &[u8]) -> bool;
    fn map(&self, raw: &[u8]) -> KnoxResult<Vec<KnoxAction>>;
}
```

```rust
// src/ingest/mod.rs — Auto-detection

/// All registered mappers. Add new ones here.
pub fn all_mappers() -> Vec<Box<dyn TranscriptMapper>> {
    vec![
        Box::new(antigravity::AntigravityMapper),
        // Future: Box::new(cursor::CursorMapper),
    ]
}

/// Detect which mapper handles this file, then map it
pub fn ingest_file(path: &Path) -> KnoxResult<Vec<KnoxAction>>;
```

### `commands` — Layer 3

```rust
// src/commands/init.rs
pub fn cmd_init(project_path: &Path, config: &KnoxConfig) -> KnoxResult<()>;

// src/commands/ingest.rs
pub fn cmd_ingest(transcript_path: &Path, db: &KnoxDb) -> KnoxResult<IngestReport>;

// src/commands/query.rs
pub fn cmd_query(pattern: &str, db: &KnoxDb, graph: Option<&KnoxGraph>) -> KnoxResult<QueryResult>;

// src/commands/status.rs
pub fn cmd_status(db: &KnoxDb) -> KnoxResult<StatusReport>;

// src/commands/scan.rs
pub fn cmd_scan(project_path: &Path, db: &KnoxDb) -> KnoxResult<ScanReport>;
```

### `mcp` — Layer 4

```rust
// src/mcp/server.rs — The MCP stdio event loop

pub async fn run_mcp_server(db: KnoxDb) -> KnoxResult<()>;
// Reads JSON-RPC from stdin, dispatches to handlers, writes responses to stdout.
// Runs session timeout checker on a tokio interval.
```

```rust
// src/mcp/handlers/record_action.rs — Example handler pattern

pub fn handle_record_action(
    db: &KnoxDb,
    params: serde_json::Value,
) -> KnoxResult<serde_json::Value> {
    // 1. Deserialize params into RecordActionInput
    // 2. Validate required fields
    // 3. Call commands::insert_action(db, ...)
    // 4. Serialize result into JSON response
}
```

---

## Coding Standards

### Naming

| Element | Convention | Example |
|---|---|---|
| Crate | `snake_case` | `knox` |
| Modules | `snake_case` | `action_group`, `session_manager` |
| Types / Structs / Enums | `PascalCase` | `KnoxAction`, `QueryFilter`, `KnoxError` |
| Functions | `snake_case` | `insert_action`, `load_subgraph` |
| Constants | `SCREAMING_SNAKE_CASE` | `DEFAULT_SESSION_TIMEOUT`, `MAX_SUMMARY_LENGTH` |
| MCP tool names | `snake_case` with `knox_` prefix | `knox_get_context`, `knox_record_action` |
| Test functions | `test_<module>_<behavior>` | `test_storage_insert_action_returns_id` |

### Error Handling

| Rule | Detail |
|---|---|
| **Use `KnoxError` everywhere** | No `anyhow` in library code. `anyhow` only in `main.rs` for top-level error formatting. |
| **`?` propagation** | Use `?` to propagate errors up the call stack. Don't `unwrap()` or `expect()` in library code. |
| **Context via error variants** | Add context by matching on `KnoxError` variants, not by wrapping in strings. |
| **`unwrap()` in tests only** | Tests can `unwrap()`. Production code never. |

### Comments & Documentation

| Rule | Detail |
|---|---|
| **Every public fn gets a doc comment** | `/// Does X. Returns Y. Fails with Z.` |
| **No commented-out code** | Delete it. Git has history. |
| **`// TODO:` for known gaps** | Always include your name and date: `// TODO(diva, 2026-10-29): Add Cursor mapper` |
| **Module-level doc comment** | Every `mod.rs` starts with `//! This module handles...` |

### Function Size

| Rule | Detail |
|---|---|
| **Max 50 lines per function** | If a function exceeds 50 lines, extract a helper. |
| **Max 5 parameters** | If a function takes more than 5 args, group them into a struct. |
| **Single return type** | Every function returns `KnoxResult<T>` or a simple type. No multi-return tuples of more than 3 elements. |

### Testing

| Rule | Detail |
|---|---|
| **Unit tests in the same file** | `#[cfg(test)] mod tests { ... }` at the bottom of each `.rs` file for unit tests. |
| **Integration tests in `tests/`** | Cross-module tests go in the `tests/` directory. |
| **Use in-memory DB for tests** | `KnoxDb::open_memory()` — no file I/O in tests. Fast, isolated. |
| **One assert per test** (preferred) | Each test function tests one behavior. Name it after that behavior. |
| **Test the contract, not the implementation** | Test public API results, not internal state. |

---

## SQLite Schema

```sql
-- Version 1 schema. Managed by src/storage/migrations.rs

CREATE TABLE IF NOT EXISTS knox_meta (
    key   TEXT PRIMARY KEY,
    value TEXT NOT NULL
);
-- Stores: schema_version, created_at, knox_version

CREATE TABLE IF NOT EXISTS projects (
    id         TEXT PRIMARY KEY,        -- UUID v4
    name       TEXT NOT NULL,
    path       TEXT NOT NULL UNIQUE,
    created_at TEXT NOT NULL            -- ISO 8601
);

CREATE TABLE IF NOT EXISTS sessions (
    id         TEXT PRIMARY KEY,        -- UUID v4
    actor      TEXT NOT NULL,
    project_id TEXT,                    -- nullable (global sessions)
    started_at TEXT NOT NULL,
    ended_at   TEXT,                    -- null if still open
    is_open    INTEGER NOT NULL DEFAULT 1,
    FOREIGN KEY (project_id) REFERENCES projects(id)
);

CREATE TABLE IF NOT EXISTS actions (
    id               TEXT PRIMARY KEY,  -- UUID v4
    actor            TEXT NOT NULL,     -- free string: "gemini", "human", etc.
    action_type      TEXT NOT NULL,     -- free string: "write", "plan", etc.
    summary          TEXT NOT NULL,
    confidence       REAL,             -- nullable, 0.0–1.0
    timestamp        TEXT NOT NULL,     -- ISO 8601
    session_id       TEXT,
    parent_action_id TEXT,              -- causal link
    superseded_by    TEXT,              -- ID of the action that replaced this one
    is_current       INTEGER NOT NULL DEFAULT 1,
    FOREIGN KEY (session_id) REFERENCES sessions(id),
    FOREIGN KEY (parent_action_id) REFERENCES actions(id),
    FOREIGN KEY (superseded_by) REFERENCES actions(id)
);

CREATE TABLE IF NOT EXISTS action_files (
    action_id TEXT NOT NULL,
    file_path TEXT NOT NULL,            -- relative to project root
    PRIMARY KEY (action_id, file_path),
    FOREIGN KEY (action_id) REFERENCES actions(id)
);

CREATE TABLE IF NOT EXISTS action_tags (
    action_id TEXT NOT NULL,
    tag       TEXT NOT NULL,
    PRIMARY KEY (action_id, tag),
    FOREIGN KEY (action_id) REFERENCES actions(id)
);

-- Performance indexes (Tier 1 query acceleration)
CREATE INDEX IF NOT EXISTS idx_actions_session     ON actions(session_id);
CREATE INDEX IF NOT EXISTS idx_actions_actor        ON actions(actor);
CREATE INDEX IF NOT EXISTS idx_actions_timestamp    ON actions(timestamp);
CREATE INDEX IF NOT EXISTS idx_actions_is_current   ON actions(is_current);
CREATE INDEX IF NOT EXISTS idx_actions_parent       ON actions(parent_action_id);
CREATE INDEX IF NOT EXISTS idx_action_files_path    ON action_files(file_path);
CREATE INDEX IF NOT EXISTS idx_action_files_action  ON action_files(action_id);
CREATE INDEX IF NOT EXISTS idx_action_tags_tag      ON action_tags(tag);

-- Full-text search on summaries
CREATE VIRTUAL TABLE IF NOT EXISTS actions_fts USING fts5(
    summary,
    content='actions',
    content_rowid='rowid'
);
```

### Schema Rules

| Rule | Detail |
|---|---|
| **Append-only** | `INSERT` and `UPDATE is_current/superseded_by` only. No `DELETE`. |
| **No raw SQL outside `src/storage/`** | Every query is a function in `storage/queries.rs` or the relevant CRUD module. |
| **Migrations are versioned** | `knox_meta.schema_version` tracks the current version. New columns/tables go in new migration functions. |
| **UUIDs as text** | SQLite has no native UUID type. Store as TEXT, generate in Rust. |
| **Timestamps as ISO 8601 text** | `chrono::DateTime<Utc>.to_rfc3339()`. Sortable as strings. |

---

## Implementation Order

Build bottom-up, layer by layer. Each phase produces a testable, runnable increment.

### Phase 1: Foundation (core + storage)

```
Goal: Can create a DB, insert actions, query them back.
Test: cargo test — all storage tests pass.

Build order:
  1. src/core/ids.rs          — ActionId, SessionId, ProjectId (UUID wrappers)
  2. src/core/error.rs        — KnoxError enum
  3. src/core/types.rs        — KnoxAction, Session, QueryFilter, QueryResult
  4. src/core/config.rs       — KnoxConfig (paths, defaults)
  5. src/core/mod.rs           — re-exports
  6. src/storage/migrations.rs — CREATE TABLE statements
  7. src/storage/db.rs         — KnoxDb::open, KnoxDb::open_memory, migrate()
  8. src/storage/nodes.rs      — insert_action, get_action, mark_superseded
  9. src/storage/edges.rs      — link actions (parent, supersede, follows)
 10. src/storage/sessions.rs   — start_session, end_session, timeout_stale
 11. src/storage/queries.rs    — query_actions with QueryFilter, full-text search
 12. src/storage/mod.rs        — re-exports
 13. tests/test_storage.rs     — full test suite for storage layer
```

### Phase 2: Graph (petgraph layer)

```
Goal: Can load a subgraph from SQLite into petgraph, traverse causal chains.
Test: cargo test — graph tests pass with seeded DB.

Build order:
  1. src/graph/loader.rs      — load_subgraph (bounded BFS), load_full_graph
  2. src/graph/traversal.rs   — causal_chain, ancestors, descendants, path_between
  3. src/graph/cache.rs       — LRU cache (project+file → subgraph)
  4. src/graph/mod.rs          — re-exports
  5. tests/test_graph.rs       — traversal + cache tests
```

### Phase 3: Ingest (transcript mapping)

```
Goal: Can parse an Antigravity transcript.jsonl → Vec<KnoxAction>, store in DB.
Test: cargo test — ingest test with fixture transcript.

Build order:
  1. src/ingest/mapper.rs         — TranscriptMapper trait
  2. src/ingest/action_group.rs   — ActionGrouper state machine (IDLE→COLLECTING→EMIT)
  3. src/ingest/summary.rs        — heuristic summary generators
  4. src/ingest/antigravity.rs     — AntigravityMapper implementation
  5. src/ingest/mod.rs             — all_mappers(), ingest_file()
  6. fixtures/transcript_antigravity.jsonl — sample test data
  7. tests/test_ingest.rs          — mapper + grouping tests
```

### Phase 4: CLI Commands

```
Goal: `knox init`, `knox ingest`, `knox query`, `knox status` work end-to-end.
Test: cargo run -- init . / cargo run -- status

Build order:
  1. src/cli/args.rs          — clap derive structs
  2. src/commands/init.rs     — create .knox/, generate knox.md, register project
  3. src/commands/ingest.rs   — orchestrate: detect mapper → parse → store
  4. src/commands/query.rs    — parse pattern → query storage/graph → format output
  5. src/commands/status.rs   — aggregate stats
  6. src/commands/scan.rs     — git diff --cached → detect human edits
  7. src/commands/hook.rs     — install/uninstall git pre-commit hook
  8. src/commands/mod.rs      — re-exports
  9. src/cli/mod.rs           — re-exports
 10. src/main.rs              — match on clap command → dispatch to commands
 11. tests/test_commands.rs   — end-to-end CLI tests
```

### Phase 5: MCP Server

```
Goal: `knox serve` starts, IDE agents connect and use all tools.
Test: echo JSON-RPC to stdin, verify stdout responses.

Build order:
  1. src/mcp/protocol.rs          — MCP JSON-RPC types (Request, Response, Error)
  2. src/mcp/tools.rs             — tool registration (name, description, schema)
  3. src/mcp/session_manager.rs   — in-memory session state + timeout task
  4. src/mcp/handlers/start_session.rs
  5. src/mcp/handlers/end_session.rs
  6. src/mcp/handlers/record_action.rs
  7. src/mcp/handlers/record_decision.rs
  8. src/mcp/handlers/get_context.rs
  9. src/mcp/handlers/get_history.rs
 10. src/mcp/handlers/search.rs
 11. src/mcp/handlers/list_stale.rs
 12. src/mcp/handlers/scan_staged.rs
 13. src/mcp/handlers/mod.rs
 14. src/mcp/server.rs             — tokio stdin/stdout loop + dispatch
 15. src/mcp/mod.rs
 16. tests/test_mcp.rs             — mock stdio tool tests
```

### Phase 6: Integration & Polish

```
Goal: Everything works together. Ready for release.

Tasks:
  1. End-to-end test: init → ingest real transcript → serve → query via MCP
  2. knox.md generation: verify the template renders correctly
  3. Error messages: human-readable, actionable
  4. Logging: tracing subscriber for debug output
  5. CI: GitHub Actions workflow for Windows, macOS, Linux
  6. README.md: installation, quickstart, examples
  7. Cross-compile test: cargo build on all three platforms
```

---

## Anti-Patterns — DO NOT

| Anti-Pattern | Why It's Bad | Do This Instead |
|---|---|---|
| **God file** — putting multiple concerns in one file | Hard to find things, merge conflicts, cognitive overload | One file per concern. Split early. |
| **Layer skipping** — MCP handler directly calling `rusqlite` | Breaks the architecture. Untestable. | MCP → commands → storage. Always. |
| **Stringly-typed internals** — passing raw JSON between layers | Loses type safety. Runtime crashes instead of compile errors. | Deserialize at the boundary (MCP). Use typed structs internally. |
| **`pub` everything** | No encapsulation. Any layer can reach into any other. | Default to private. Only `pub` what the layer above needs. |
| **Monolithic handler** — one 300-line function for an MCP tool | Untestable, unreadable. | Deserialize → validate → call command → serialize. 4 steps, each small. |
| **Tests that need the filesystem** | Slow, flaky, platform-dependent. | `KnoxDb::open_memory()`. Mock everything else. |
| **Catching errors silently** — `let _ = something()` | Hides bugs. | Always handle or propagate with `?`. Log if intentionally ignoring. |
| **`clone()` without reason** | Performance waste in Rust. | Borrow where possible. Clone only when ownership is required. |

---

## Cargo.toml

```toml
[package]
name = "knox"
version = "0.1.0"
edition = "2024"
authors = ["Diva"]
description = "Knowledge graph for AI-assisted development"
license = "MIT"
repository = "https://github.com/Diva143V/Knox"
rust-version = "1.85"

[dependencies]
# Layer 0: Core
chrono = { version = "0.4", features = ["serde"] }
uuid = { version = "1", features = ["v4", "serde"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
thiserror = "1"

# Layer 1: Storage
rusqlite = { version = "0.31", features = ["bundled-full"] }

# Layer 2: Graph
petgraph = "0.6"

# Layer 3: Commands (no extra deps)

# Layer 4: MCP Server
tokio = { version = "1", features = ["full"] }

# Layer 5: CLI
clap = { version = "4", features = ["derive"] }

# Cross-cutting
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
anyhow = "1"                        # main.rs ONLY — not in library code
notify = "6"                        # file watching (optional, future)

[dev-dependencies]
tempfile = "3"                      # temp dirs for integration tests
assert_cmd = "2"                    # CLI integration tests
predicates = "3"                    # assertion helpers

[profile.release]
lto = true                          # link-time optimization
strip = true                        # strip debug symbols
codegen-units = 1                   # maximize optimization
```

---

*This document is the law. If the code disagrees with this document, the code is wrong.*

