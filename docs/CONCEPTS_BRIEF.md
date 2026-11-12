# Knox Causal Code Delta & High-Impact MCP Tooling Expansion

* **Author:** Knox Architecture & Agent Working Group
* **Date:** 2026-11-02

## Overview

Knox currently acts as a causal knowledge graph that logs high-level action metadata (`actor`, `action_type`, `summary`, `files`, `symbols`, and relations) in SQLite via an MCP stdio server. However, it operates as a metadata-only ledger: it does not record the actual code diffs, deleted implementation snippets, or structural syntax changes associated with an agent's decisions. When an agent deletes logic, refactors an interface, or asks why a block was removed three sessions ago, Knox cannot reconstruct the code state or explain the removal.

This feature explores expanding the Knox MCP server and SQLite storage model to capture granular code changes—including unified diffs, deleted code fragments, and structural semantic deltas—paired with new high-impact investigative tools (`knox_blame_causal`, `knox_diff_inspect`, and `knox_verify_drift`) while strictly controlling payload size, token consumption, and secret leakage.

## Analysis

### 1. Storage & Payload Economics (The SQLite & Token Bloat Risk)
* **Problem:** Storing raw code diffs or full file snapshots directly in SQLite rows will inflate `knox.db` rapidly, especially in long-running projects with automated agents generating thousands of lines per session.
* **Risk:** When agents query `knox_get_context` or `knox_get_history`, returning full code diffs will exhaust the LLM's context window and increase latency and API token costs.
* **Mitigation:** Code deltas must be stored in a separate table (`action_deltas`), compressed (e.g., zstd or SQLite blob), truncated to line-bounded hunks, or content-addressed. Query endpoints must default to returning diff summaries (e.g., lines added/deleted, touched functions) and only return full patch hunks when explicitly requested via a dedicated tool like `knox_diff_inspect`.

### 2. Capture Ergonomics & Ingestion Reliability
* **Problem:** If capturing code changes requires agents to manually construct unified diffs or paste code strings into tool payloads, agents will either hallucinate the diff, truncate it, or skip calling the tool due to cognitive overhead.
* **Risk:** Out-of-band edits by developers (human edits) or other agents bypass the MCP tool entirely, leading to graph-code drift.
* **Mitigation:** The MCP server can leverage Git directly (`git diff` between HEAD/index or unstaged changes) or inspect file state on project paths when an action is recorded, relieving the LLM from formatting raw diff strings.

### 3. Security, Privacy, and Secret Exposure
* **Problem:** Agents frequently edit configuration files, environment variables, or connection strings.
* **Risk:** Recording raw deleted or replaced code risks persisting credentials, API keys, or private tokens permanently in SQLite.
* **Mitigation:** Knox must apply guardrail checks (pattern matching for common secrets, high-entropy tokens, `.env` file suppression) before persisting diff payloads into the graph.

## Goals vs. Non-Goals

### In-Scope Goals
* **Granular Causal Code Delta Schema:** Add an `action_deltas` table linked to `actions(id)` storing file path, change kind (`added`, `modified`, `deleted`), line numbers, deleted snippet preview, and compact unified diff hunk.
* **Deleted Code Preservation:** Explicitly track why code was deleted, capturing the deleted block and its architectural replacement rationale.
* **New Investigative MCP Tools:**
  * `knox_diff_inspect`: Retrieve exact code changes and deleted code associated with an action ID or causal branch.
  * `knox_blame_causal`: Map a file path and line number directly back to the causal agent decision and session that wrote or touched it.
  * `knox_verify_drift`: Scan the working tree against the latest recorded actions to detect unrecorded or out-of-band file modifications.
* **Automatic Git-Assisted Delta Capture:** Allow `knox_record_action` to optionally auto-resolve diffs from the local git working tree rather than requiring the LLM to write raw patch strings.

### Out-of-Scope Non-Goals
* **VCS Replacement:** Knox is not a version control system. It does not replace Git or maintain a full commit graph for the whole repository.
* **AST Recompilation Engine:** Knox will not build an in-memory language server or perform deep full-project AST parsing across all languages.
* **Unbounded Blob Storage:** Full-file before/after copies will not be stored in SQLite. Only bounded patch hunks and deleted snippet previews will be retained.

## Alternatives Weighed

1. **Rely entirely on standard `git log` and `git diff`:**
   * *Rejected:* Git commits do not record agent session context, causal links (`parent_action_id`, `supersedes`), architectural rationale, or unresolved doubts. Commits are often squashed, aggregated, or delayed until the end of a long multi-turn session.
2. **Store full file snapshots on every recorded action:**
   * *Rejected:* Unacceptable storage bloat and SQLite write amplification. Storing full files for every micro-decision causes disk footprint to grow linearly with file size $\times$ action count.
3. **Require LLM to generate unified diffs in tool arguments:**
   * *Rejected:* LLMs frequently make whitespace errors, hallucinate line offsets, and consume enormous output tokens outputting diffs that the host machine's git environment can generate locally with 100% precision in 5 milliseconds.

## Key Decisions & Staging for Next Steps

* **Decision 1:** Diffs must be generated and validated on the host (via git or local file comparison) whenever possible, using the MCP server as an executor rather than forcing the LLM to format patches.
* **Decision 2:** Query endpoints (`knox_get_context`, `knox_get_history`) must remain concise. Full code changes and deleted snippets will be on-demand queries via `knox_diff_inspect`.

---

## Detailed Implementation Plan

### 1. Target Scope & File Status Matrix

| File Path | Status | Purpose & Description |
| :--- | :--- | :--- |
| `Cargo.toml` | `[Modify]` | Add `regex = "1.10"` to `[dependencies]` |
| `src/lib.rs` | `[Modify]` | Register `pub mod git;` |
| `src/core/mod.rs` | `[Modify]` | Register `pub mod guardrails;` and re-export sanitization functions |
| `src/core/types.rs` | `[Modify]` | Add `DeltaChangeType`, `ActionDelta`, `ActionDeltaInput`, `DeltaSummary`, and size constants |
| `src/core/guardrails.rs` | `[Create]` | Implement secret scrubbing and line/byte truncation logic |
| `src/storage/mod.rs` | `[Modify]` | Re-export delta persistence functions |
| `src/storage/migrations.rs` | `[Modify]` | Implement migration v4 (`V4_SCHEMA_SQL`), update `run_migrations`, update unit test |
| `src/storage/nodes.rs` | `[Modify]` | Implement `insert_deltas`, `get_deltas_by_action`, `get_deltas_by_file`, and commit/line lookup |
| `src/storage/queries.rs` | `[Modify]` | Join `action_deltas` in `query_actions` to populate `KnoxAction.delta_summary` |
| `src/git/mod.rs` | `[Create]` | Register `pub mod diff;` |
| `src/git/diff.rs` | `[Create]` | Implement `capture_working_tree_diff` and unified diff parser |
| `src/mcp/tools.rs` | `[Modify]` | Update `TOOL_NAMES` to 21, add tool definitions for 3 new tools, update `record_action` schema |
| `src/mcp/format.rs` | `[Modify]` | Format concise `[+X/-Y lines]` delta summaries in `knox_get_context` and `knox_get_history` |
| `src/mcp/handlers/mod.rs` | `[Modify]` | Register new handlers and add match arms in `execute_tool` |
| `src/mcp/handlers/record_action.rs` | `[Modify]` | Handle `auto_git_diff`, `deltas`, `commit_sha`, sanitize deltas, and persist |
| `src/mcp/handlers/diff_inspect.rs` | `[Create]` | Implement handler for `knox_diff_inspect` |
| `src/mcp/handlers/blame_causal.rs` | `[Create]` | Implement handler for `knox_blame_causal` |
| `src/mcp/handlers/verify_drift.rs` | `[Create]` | Implement handler for `knox_verify_drift` reusing `src/commands/scan.rs` |
| `tests/test_mcp_deltas.rs` | `[Create]` | Integration tests for delta capture, secret scrubbing, inspection, blame, and drift |

---

### 2. Data Model & Storage Schema (SQLite Migration v4)

#### File: `src/storage/migrations.rs`
Increment schema version to `4` and execute `V4_SCHEMA_SQL`:
```sql
BEGIN TRANSACTION;

-- Add optional commit SHA tracking to actions
ALTER TABLE actions ADD COLUMN commit_sha TEXT;

-- Table storing granular patch hunks and deleted code blocks
CREATE TABLE IF NOT EXISTS action_deltas (
    id                TEXT PRIMARY KEY,
    action_id         TEXT NOT NULL,
    file_path         TEXT NOT NULL,
    change_type       TEXT NOT NULL CHECK(change_type IN ('added', 'modified', 'deleted', 'renamed')),
    old_path          TEXT,
    lines_added       INTEGER NOT NULL DEFAULT 0,
    lines_deleted     INTEGER NOT NULL DEFAULT 0,
    diff_hunk         TEXT,
    deleted_code      TEXT,
    deletion_reason   TEXT,
    created_at        TEXT NOT NULL,
    FOREIGN KEY (action_id) REFERENCES actions(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_actions_commit_sha     ON actions(commit_sha);
CREATE INDEX IF NOT EXISTS idx_action_deltas_action   ON action_deltas(action_id);
CREATE INDEX IF NOT EXISTS idx_action_deltas_file     ON action_deltas(file_path);
CREATE INDEX IF NOT EXISTS idx_action_deltas_type     ON action_deltas(change_type);

UPDATE knox_meta SET value = '4' WHERE key = 'schema_version';

COMMIT;
```

Update `run_migrations`:
```rust
if current_version < 4 {
    apply_v4(conn)?;
}
```
Update test `test_migrations_apply_cleanly` assertion from `3` to `4`.

#### File: `src/core/types.rs`
Define Rust data types and constants:
```rust
use std::fmt;
use std::str::FromStr;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use crate::core::ids::ActionId;

pub const MAX_DIFF_LINES: usize = 250;
pub const MAX_HUNK_BYTES: usize = 16_384;            // 16 KiB limit
pub const MAX_DELETED_SNIPPET_BYTES: usize = 4_096;  // 4 KiB limit
pub const MAX_DELTA_FILE_BYTES: usize = 512 * 1024;  // 512 KiB limit

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DeltaChangeType {
    Added,
    Modified,
    Deleted,
    Renamed,
}

impl fmt::Display for DeltaChangeType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Added => write!(f, "added"),
            Self::Modified => write!(f, "modified"),
            Self::Deleted => write!(f, "deleted"),
            Self::Renamed => write!(f, "renamed"),
        }
    }
}

impl FromStr for DeltaChangeType {
    type Err = crate::core::error::KnoxError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "added" => Ok(Self::Added),
            "modified" => Ok(Self::Modified),
            "deleted" => Ok(Self::Deleted),
            "renamed" => Ok(Self::Renamed),
            _ => Err(crate::core::error::KnoxError::InvalidInput(format!("Unknown change type: {s}"))),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionDelta {
    pub id: String,
    pub action_id: ActionId,
    pub file_path: String,
    pub change_type: DeltaChangeType,
    pub old_path: Option<String>,
    pub lines_added: u32,
    pub lines_deleted: u32,
    pub diff_hunk: Option<String>,
    pub deleted_code: Option<String>,
    pub deletion_reason: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionDeltaInput {
    pub file_path: String,
    pub change_type: String,
    pub old_path: Option<String>,
    pub lines_added: Option<u32>,
    pub lines_deleted: Option<u32>,
    pub diff_hunk: Option<String>,
    pub deleted_code: Option<String>,
    pub deletion_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DeltaSummary {
    pub total_files: usize,
    pub lines_added: u32,
    pub lines_deleted: u32,
}
```

Add fields to `KnoxAction`:
```rust
pub commit_sha: Option<String>,
pub delta_summary: Option<DeltaSummary>,
```

#### File: `src/storage/nodes.rs`
Implement free functions following Knox storage conventions:
```rust
pub fn insert_deltas(db: &KnoxDb, deltas: &[ActionDelta]) -> KnoxResult<()>;
pub fn get_deltas_by_action(db: &KnoxDb, action_id: &ActionId) -> KnoxResult<Vec<ActionDelta>>;
pub fn get_deltas_by_file(db: &KnoxDb, file_path: &str, limit: usize) -> KnoxResult<Vec<ActionDelta>>;
pub fn prune_action_deltas(db: &KnoxDb, max_age_days: i64) -> KnoxResult<usize>;
```

---

### 3. Guardrails & Secret Sanitization

#### File: `src/core/guardrails.rs`
1. **File Exclusion**:
   ```rust
   pub fn is_file_excluded(file_path: &str) -> bool
   ```
   Excludes `.env`, `.env.*`, `*.pem`, `*.key`, lockfiles (`Cargo.lock`, `package-lock.json`, `pnpm-lock.yaml`, `yarn.lock`), and binary extensions (`.png`, `.exe`, `.dll`, `.wasm`, `.db`).
2. **Secret Scrubbing**:
   ```rust
   pub fn scrub_secrets(input: &str) -> String
   ```
   Uses regex patterns for `ghp_[A-Za-z0-9]{36}`, `sk-[A-Za-z0-9_-]{20,}`, `AIza[0-9A-Za-z-_]{35}`, and `Bearer\s+[A-Za-z0-9._~+/-]{20,}`, replacing matches with `[REDACTED_SECRET]`.
3. **Delta Sanitization**:
   ```rust
   pub fn sanitize_delta(delta: &mut ActionDelta)
   ```
   Scrubs secrets in `diff_hunk` and `deleted_code`. Truncates `diff_hunk` at `MAX_DIFF_LINES` or `MAX_HUNK_BYTES` appending `\n... [diff truncated: exceeded line/byte limit]`. Truncates `deleted_code` at `MAX_DELETED_SNIPPET_BYTES`.

---

### 4. Git Diff Capture Module

#### File: `src/git/diff.rs`
Execute Git CLI line-by-line streaming parser:
```rust
pub fn capture_working_tree_diff(
    action_id: &ActionId,
    repo_root: &Path,
    files: &[String],
) -> KnoxResult<Vec<ActionDelta>>
```
Execution logic:
1. Verify HEAD exists: run `git rev-parse --verify HEAD`.
   * If HEAD exists: execute `git diff -U3 HEAD -- <files>`.
   * If HEAD does not exist (orphan repository): execute `git diff -U3 --cached -- <files>` or diff against `/dev/null`.
2. Inspect untracked files via `git status --porcelain -- <files>`. For untracked files, treat as `DeltaChangeType::Added`.
3. Parse unified diff hunks:
   * Parse `--- a/<path>` and `+++ b/<path>`.
   * Detect line changes: count lines starting with `+` (excluding `+++`) and `-` (excluding `---`).
   * Extract deleted code lines into `deleted_code`.
4. Construct and return `Vec<ActionDelta>` with foreign key `action_id`.

---

### 5. New MCP Tool Declarations & Wire Schemas

#### File: `src/mcp/tools.rs`
1. Update `pub const TOOL_NAMES: [&str; 21] = [ ... ];`
2. Update `tool_record_action()` to accept optional parameters:
   * `deltas`: JSON array of `ActionDeltaInput`.
   * `auto_git_diff`: boolean (defaults to `true` at runtime if repository is valid and files are provided).
   * `deletion_reason`: string.
   * `commit_sha`: string.
3. Declare `tool_diff_inspect()`, `tool_blame_causal()`, `tool_verify_drift()`.

#### Response Data Structures:
```rust
#[derive(Debug, Serialize, Deserialize)]
pub struct DiffInspectResponse {
    pub action_id: ActionId,
    pub summary: String,
    pub deltas: Vec<ActionDelta>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CausalBlameResponse {
    pub file_path: String,
    pub line_number: u32,
    pub commit_sha: Option<String>,
    pub action: Option<KnoxAction>,
    pub delta: Option<ActionDelta>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct VerifyDriftResponse {
    pub is_clean: bool,
    pub untracked_files: Vec<String>,
    pub modified_files: Vec<String>,
    pub total_drift_files: usize,
    pub drift_summary: String,
}
```

---

### 6. MCP Handlers & Registration

#### File: `src/mcp/handlers/mod.rs`
1. Declare modules:
   ```rust
   pub mod blame_causal;
   pub mod diff_inspect;
   pub mod verify_drift;
   ```
2. In `execute_tool`, add match arms:
   * `"knox_diff_inspect" => diff_inspect::handle_diff_inspect(db, params).await,`
   * `"knox_blame_causal" => blame_causal::handle_blame_causal(db, params).await,`
   * `"knox_verify_drift" => verify_drift::handle_verify_drift(db, params).await,`

#### File: `src/mcp/handlers/record_action.rs`
1. Resolve `repo_root`: Check active project path from SQLite `projects.path`, falling back to `std::env::current_dir()`.
2. If `auto_git_diff` is `true` (or unset with `files` present), call `capture_working_tree_diff`.
3. Merge any explicit `deltas` provided in tool arguments.
4. Call `guardrails::sanitize_delta` on each delta.
5. Persist deltas via `storage::nodes::insert_deltas`.

---

### 7. Formatting & Digest Integration

#### File: `src/mcp/format.rs`
In `format_markdown_digest`, when `action.delta_summary` is populated, format a compact line:
```text
  Δ deltas: 3 files (+45 / -12 lines)
```
Ensure full diff hunks are never inlined into `knox_get_context` or `knox_get_history` to prevent LLM token exhaustion.

---

### 8. Verification Commands

1. `cargo test storage::migrations::tests`: Verify schema migration v1 through v4 applies idempotently.
2. `cargo test --test test_mcp_deltas`: Run dedicated integration tests for delta capture, secret scrubbing, inspection, blame, and drift.
3. `cargo test`: Ensure all existing 157 tests pass with zero regressions.
4. `cargo check --all-targets`: Verify zero compiler errors or warnings.
