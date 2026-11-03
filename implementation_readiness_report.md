# Implementation Readiness Audit Report

* **Target Document:** [`d:\Knox\concepts_brief.md`](file:///d:/Knox/concepts_brief.md)
* **Target Scope:** Detailed Implementation Plan (Lines 61–220)
* **Standard Evaluated:** [Implementation Readiness Standard](file:///C:/Users/Diwa/.gemini/config/plugins/elephant-goldfish-model/resources/document_standards/implementation-readiness-standard.md)
* **Mode:** `REVIEW`
* **Overall Verdict:** 🔴 **FAIL** (5 Blockers, 8 Majors, 4 Nits) — Violates the Zero-Decision Rule and Actionability Requirements.

---

## Reviewer Status

| Reviewer Persona | Lens / Mandate | Status | Findings |
| :--- | :--- | :--- | :--- |
| `egm-implementation-readiness` | Pragmatic Tech Lead & Implementer | **Completed** | 3 Blockers, 6 Majors, 3 Nits |
| `egm-codebase-details` | Codebase Quality & Storage Architecture | **Completed** | 3 Blockers, 9 Majors, 1 Nit |
| `egm-style` | Technical Copy Editor & Precision | **Completed** | 3 Blockers, 5 Majors, 15 Nits |

---

## Prioritized Findings Matrix

### 🔴 Blocker Issues (Must Fix Before Code Can Be Written)

#### 1. Infeasible Git-to-Action Mapping in `knox_blame_causal`
* **Reviewers:** `egm-implementation-readiness`, `egm-codebase-details`, `egm-style`
* **Affected Scope:** Lines 170–174, 184 (`src/mcp/handlers/blame_causal.rs`)
* **Finding:** Line 184 instructs the implementer to *"combine git blame -L commit SHA with actions.id lookup or timestamp matching"*. Knox's SQLite schema (`actions`, `action_deltas`) stores zero Git commit hashes. Instructing the implementer to perform an `actions.id` lookup by commit SHA is impossible, and *"or timestamp matching"* leaves the matching window, tie-breaking heuristics, and uncommitted dirty line handling entirely to implementer guesswork.
* **Actionable Fix:**
  1. Add `commit_sha: Option<String>` to `V4_SCHEMA_SQL` and `RecordActionParams`, OR
  2. Specify the exact fallback heuristic: Query `action_deltas` where `file_path = ?` and line numbers overlap with the blame hunk, resolving the nearest causal action record. Specify error response when no attribution is found.

#### 2. Storage Layer Architecture Mismatch (`&self` vs Free Functions)
* **Reviewers:** `egm-codebase-details`, `egm-implementation-readiness`
* **Affected Scope:** Lines 125–130 (`src/storage/nodes.rs`)
* **Finding:** The plan specifies `pub fn insert_deltas(&self, ...)` and `&str` action IDs. Knox's storage layer in `src/storage/nodes.rs` does not use struct methods; all operations are free functions taking `db: &KnoxDb` and strongly-typed `&ActionId` (e.g. `pub fn insert_action(db: &KnoxDb, action: &KnoxAction) -> KnoxResult<ActionId>`). Implementing `&self` will fail compilation.
* **Actionable Fix:** Update function signatures to:
  ```rust
  pub fn insert_deltas(db: &KnoxDb, deltas: &[ActionDelta]) -> KnoxResult<()>
  pub fn get_deltas_by_action(db: &KnoxDb, action_id: &ActionId) -> KnoxResult<Vec<ActionDelta>>
  pub fn get_deltas_by_file(db: &KnoxDb, file_path: &str, limit: usize) -> KnoxResult<Vec<ActionDelta>>
  ```
  Ensure `src/storage/mod.rs` re-exports these functions.

#### 3. False Tool Registration Location & Missing Handler Dispatch
* **Reviewers:** `egm-codebase-details`, `egm-implementation-readiness`
* **Affected Scope:** Line 201 (Task 3: MCP Handlers & Registration)
* **Finding:** The plan instructs: *"Register new tools in `src/mcp/tools.rs`, `src/mcp/protocol.rs`, and `src/mcp/server.rs`"*. `protocol.rs` only defines JSON-RPC DTOs and `server.rs` only manages stdio streams. Tool registration in Knox belongs strictly in `src/mcp/tools.rs` (`TOOL_NAMES` array and `all_tools()`), and tool execution dispatch belongs in `src/mcp/handlers/mod.rs` (`execute_tool` match block).
* **Actionable Fix:** Update file targets to `src/mcp/tools.rs` and `src/mcp/handlers/mod.rs`, removing references to `protocol.rs` and `server.rs`.

#### 4. Undefined `ActionDeltaInput` Struct & Wire Schema
* **Reviewers:** `egm-style`, `egm-implementation-readiness`, `egm-codebase-details`
* **Affected Scope:** Line 161 (`src/mcp/tools.rs`)
* **Finding:** `RecordActionParams` is augmented with `deltas: Option<Vec<ActionDeltaInput>>`, but `ActionDeltaInput` is never defined. Field optionality, validation constraints, and JSON Schema definitions are omitted.
* **Actionable Fix:** Explicitly define `ActionDeltaInput` in `src/core/types.rs`:
  ```rust
  #[derive(Debug, Clone, Deserialize)]
  pub struct ActionDeltaInput {
      pub file_path: String,
      pub change_type: String, // added, modified, deleted, renamed
      pub old_path: Option<String>,
      pub lines_added: Option<u32>,
      pub lines_deleted: Option<u32>,
      pub diff_hunk: Option<String>,
      pub deleted_code: Option<String>,
      pub deletion_reason: Option<String>,
  }
  ```
  Define its JSON Schema properties in `src/mcp/tools.rs`.

#### 5. Hand-Waving Git Diff & Working Tree Resolution
* **Reviewers:** `egm-implementation-readiness`, `egm-style`, `egm-codebase-details`
* **Affected Scope:** Lines 143–153 (`src/git/diff.rs`)
* **Finding:** `capture_working_tree_diff(repo_root: &Path, files: &[String]) -> KnoxResult<Vec<ActionDelta>>` cannot construct `ActionDelta` because it does not accept `action_id`. Furthermore, the plan does not specify how `handle_record_action` resolves `repo_root` (via `projects.path` or `current_dir`), and parenthetical phrasing `"(or index comparison for staged items)"` leaves staged/untracked file handling unresolved.
* **Actionable Fix:**
  1. Change signature to: `pub fn capture_working_tree_diff(action_id: &ActionId, repo_root: &Path, files: &[String]) -> KnoxResult<Vec<ActionDelta>>`.
  2. Specify that `repo_root` is resolved from the active session's project path in SQLite, falling back to `std::env::current_dir()`.
  3. Specify that diff execution invokes `git diff -U3 HEAD -- <files>` when HEAD exists, handling untracked files by checking `git status --porcelain`.

---

### 🟡 Major Issues (Must Clarify Before Implementation)

#### 6. Missing Response Schemas for New MCP Tools
* **Affected Scope:** Lines 164–180 (`src/mcp/tools.rs`)
* **Finding:** The plan defines parameters for `knox_diff_inspect`, `knox_blame_causal`, and `knox_verify_drift`, but zero response schemas. In MCP, tools must return structured JSON (`structuredContent`) and formatted Markdown.
* **Actionable Fix:** Define explicit output structs:
  * `DiffInspectResponse { action_id: ActionId, deltas: Vec<ActionDelta> }`
  * `CausalBlameResponse { file_path: String, line_number: u32, action: Option<KnoxAction>, delta: Option<ActionDelta>, commit_sha: Option<String> }`
  * `VerifyDriftResponse { is_clean: bool, untracked_files: Vec<String>, modified_files: Vec<String>, drift_summary: String }`

#### 7. Missing File Action Markers in Target Scope
* **Affected Scope:** Lines 189–210 (Section 5)
* **Finding:** Omits required `[Create]`, `[Modify]`, `[Delete]` markers and forgets root module exports: `src/lib.rs` (`pub mod git;`), `src/git/mod.rs`, `src/core/mod.rs` (`pub mod guardrails;`), `src/storage/mod.rs`, and `Cargo.toml`.
* **Actionable Fix:** Add an explicit Target Scope table with file status markers for every affected file.

#### 8. Missing `regex` Dependency in `Cargo.toml`
* **Affected Scope:** Line 138 (Section 2)
* **Finding:** Pattern matching for secrets requires the `regex` crate, which is not currently in Knox's `Cargo.toml`.
* **Actionable Fix:** Specify adding `regex = "1.10"` to `[dependencies]` in `Cargo.toml`.

#### 9. Data Model Inconsistencies in `ActionDelta`
* **Affected Scope:** Lines 96–118 (Section 1)
* **Finding:** `created_at` is defined as `String` instead of `DateTime<Utc>`. `ActionDelta.action_id` is defined as `String` instead of `ActionId`. `DeltaChangeType` lacks `#[serde(rename_all = "snake_case")]` and `Display`/`FromStr`.
* **Actionable Fix:** Update `ActionDelta` to use `ActionId` and `DateTime<Utc>`, and add `#[serde(rename_all = "snake_case")]` to `DeltaChangeType`.

#### 10. Data Flow Disconnect for Summary Diffs in `format.rs`
* **Affected Scope:** Line 203 (Task 4)
* **Finding:** `format_markdown_digest` accepts `&[KnoxAction]`, but `KnoxAction` does not contain delta statistics, nor does `format.rs` query SQLite.
* **Actionable Fix:** Add `pub delta_summary: Option<DeltaSummary>` (with `lines_added: u32, lines_deleted: u32`) to `KnoxAction`, populated via SQL `LEFT JOIN` in `src/storage/queries.rs`.

#### 11. Architectural Scope Bundling in `check_guardrails.rs`
* **Affected Scope:** Line 135 (Section 2)
* **Finding:** Section 2 links pre-write delta sanitization to `src/mcp/handlers/check_guardrails.rs`, which is an MCP query tool. Pre-write sanitization belongs in domain logic (`src/core/guardrails.rs`) and should be invoked in `record_action.rs` and `src/git/diff.rs`.
* **Actionable Fix:** Remove `check_guardrails.rs` from Section 2 and isolate secret scrubbing in `src/core/guardrails.rs`.

#### 12. Redundancy Between `knox_verify_drift` and `knox_scan_staged`
* **Affected Scope:** Lines 175–180 (Section 4)
* **Finding:** Knox already implements git scanning in `src/commands/scan.rs` (`cmd_scan`) exposed via `knox_scan_staged`. The plan does not specify how `knox_verify_drift` reuses or differentiates from `cmd_scan`.
* **Actionable Fix:** Document that `knox_verify_drift` reuses `src/commands/scan.rs` and compares dirty git files against SQLite `action_files`.

#### 13. Parameter Naming Inconsistency (`project_id` vs `project`)
* **Affected Scope:** Line 178 (Section 4)
* **Finding:** `knox_verify_drift` uses `project_id: Option<String>`, whereas all existing project-scoped tools in Knox use `project: Option<String>`.
* **Actionable Fix:** Standardize parameter name to `project: Option<String>`.

---

### 🟢 Nit Issues (Precision, Slang & Verification Commands)

1. **Incorrect Test Command:** Change `cargo test --test storage_migrations` to `cargo test storage::migrations::tests`.
2. **Stale Migration Test Assertion:** Document updating the assertion in `src/storage/migrations.rs` from `assert_eq!(version, 3)` to `assert_eq!(version, 4)`.
3. **Byte Limit Truncation:** Add explicit truncation rule for hunks exceeding `MAX_HUNK_BYTES` (16 KiB).
4. **Jargon Inflation:** Replace marketing phrasing like "zero-allocation streaming parser" with "line-by-line stream parser", and "Entropy scrubbing" with "Prefix pattern scrubbing".

---

## Conclusion & Stage 9 Resolution Path

The plan has strong architectural merit but cannot be handed directly to an engineer or coding agent without causing build failures or forced design guesses.

To achieve **PASS** status on Implementation Readiness, the 5 Blockers and 8 Majors must be resolved in `concepts_brief.md`.
