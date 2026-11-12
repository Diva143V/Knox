# Goldfish Annotated Review Report

**Target:** `d:\Knox\knox-implementation-plan.html` (`Lines 58–140`)  
**Status:** Resolved (Edits applied to knox-implementation-plan.html)  

## Prioritized Findings & AI Suggestions

### [Issue B1] [`egm-codebase-details`] - Severity: High (Blocker)
* **Affected Scope:** [Lines 65–66](d:/Knox/knox-implementation-plan.html#L65-L66) (`T1.3`, `T1.4`, `src/core/config.rs`)
* **Discrepancy Description:** Task T1.3 updates only `resolve_db_path`, omitting `resolve_db_path_for` in `src/core/config.rs:181`, which is called directly by `determine_db_path` in `src/commands/init.rs:96`. `knox init` will continue creating isolated databases when `global_mode` is false or default.
* **Rationale:** Two independent resolvers exist in `config.rs`. Updating only one leaves the codebase with contradictory database path resolution.
* ***AI Suggestion:*** PROPOSE FIX
* ***Justification:** The finding is verified directly against `src/core/config.rs` and `src/commands/init.rs`. Both resolvers must be unified to return the global path.
* ***Proposed Edit:***
  * Location: Task T1.3 (Line 65)
  * Change: Specify updating both `resolve_db_path` and `resolve_db_path_for` in `src/core/config.rs` to always return `default_global_db_path()`, deprecating `global_mode` with a warning when set to false.

---

### [Issue B2] [`egm-implementation-readiness`, `egm-codebase-details`] - Severity: High (Blocker)
* **Affected Scope:** [Lines 67–68](d:/Knox/knox-implementation-plan.html#L67-L68) (`T1.5`, `commands/migrate.rs`)
* **Discrepancy Description:** Task T1.5 mandates `knox migrate --consolidate <path>`, but leaves table migration order across 11 tables, fallback deduplication for `dedupe_key IS NULL`, foreign key remapping for project/session UUIDs, and database attachment architecture undefined.
* **Rationale:** Violates the Zero-Decision Rule. Implementers must make independent schema and deduplication choices that risk data loss or orphaned edge records.
* ***AI Suggestion:*** PROPOSE FIX
* ***Justification:** The critique is accurate. Specifying the exact SQL queries, table order, fallback matching on `(timestamp, summary, actor)` when `dedupe_key` is null, and foreign key remapping ensures deterministic implementation.
* ***Proposed Edit:***
  * Location: Task T1.5 (Line 67)
  * Change: Define the consolidation engine: SQLite `ATTACH DATABASE`, table migration sequence (projects -> sessions -> actions -> edges/deltas/files/tags/symbols), fallback matching on `(timestamp, summary, actor)` when `dedupe_key IS NULL`, and foreign key remapping to global project UUIDs.

---

### [Issue B3] [`egm-codebase-details`, `egm-implementation-readiness`] - Severity: High (Blocker)
* **Affected Scope:** [Lines 78–83](d:/Knox/knox-implementation-plan.html#L78-L83) (`T2.1`, `T2.2`, `T2.3`)
* **Discrepancy Description:** The plan omits 4 required registry files for Phase 2: `ui/src/types/knox.ts` (`SettingsTabId` union & `KnoxConfig`), `ui/src/components/modals/SettingsModal.tsx` (`CATEGORIES` navigation list), `ui/src/services/api.ts` (`updateConfig`), and `src-tauri/src/lib.rs` (`generate_handler!`).
* **Rationale:** Adding `ModelTab.tsx` in isolation causes TypeScript compilation errors and unhandled runtime IPC rejections (`"command test_model_connection not found"`).
* ***AI Suggestion:*** PROPOSE FIX
* ***Justification:** Verified in codebase. All 4 registry files are strictly required for the feature to build and run.
* ***Proposed Edit:</a>
  * Location: Tasks T2.1, T2.2, T2.3 (Lines 78–80)
  * Change: Add `ui/src/types/knox.ts`, `ui/src/components/modals/SettingsModal.tsx`, `ui/src/services/api.ts`, and `src-tauri/src/lib.rs` to the task scopes, documenting the `"model"` tab ID, DTO additions, and IPC registrations.

---

### [Issue B4] [`egm-implementation-readiness`] - Severity: High (Blocker)
* **Affected Scope:** [Line 68](d:/Knox/knox-implementation-plan.html#L68) (`T1.6`)
* **Discrepancy Description:** "Scan known project roots" and "asked once, not silently ignored" lack data source queries, persistent state flags, and IPC command signatures.
* **Rationale:** Implementers must guess where project paths are stored and how upgrade prompt state is persisted.
* ***AI Suggestion:*** PROPOSE FIX
* ***Justification:** The query and state location must be standardized: query `SELECT DISTINCT path FROM projects WHERE path IS NOT NULL`, persist `stray_db_prompted: bool` in `KnoxConfig`, and define IPC command `scan_stray_databases()`.
* ***Proposed Edit:***
  * Location: Task T1.6 (Line 68)
  * Change: Specify `SELECT DISTINCT path FROM projects` as the root source, define `stray_db_prompted: bool` in `KnoxConfig`, and define IPC command `scan_stray_databases() -> Result<Vec<StrayDbInfo>, String>`.

---

### [Issue B5] [`egm-implementation-readiness`] - Severity: High (Blocker)
* **Affected Scope:** [Lines 78–80](d:/Knox/knox-implementation-plan.html#L78-L80) (`T2.1`, `T2.2`, `T2.3`)
* **Discrepancy Description:** Editing `model_api_key` without an explicit masking update contract risks writing masked strings (`sk-...****`) back to disk or failing to update keys.
* **Rationale:** Secret masking and testing signatures must be defined to prevent key corruption and runtime failures.
* ***AI Suggestion:*** PROPOSE FIX
* ***Justification:** Standardize secret update DTO: `model_api_key: Option<String>` where `None` preserves existing key on disk, and `Some(k)` replaces it.
* ***Proposed Edit:***
  * Location: Task T2.2 & T2.3 (Lines 79–80)
  * Change: Document `ModelSettingsDto`, define secret preservation protocol, and specify signature `test_model_connection(draft: ModelSettingsDto) -> Result<ModelTestResultDto, String>`.

---

### [Issue B6] [`egm-codebase-details`, `egm-implementation-readiness`] - Severity: High (Blocker)
* **Affected Scope:** [Line 104](d:/Knox/knox-implementation-plan.html#L104) (`T4.3`, `storage/db.rs`)
* **Discrepancy Description:** Assigning "Checkpoint on clean shutdown" solely to `storage/db.rs` is unimplementable because `KnoxDb` is an inert library struct with no process lifecycle hooks.
* **Rationale:** Clean desktop shutdown is executed in `src-tauri/src/lib.rs` tray quit (`app.exit(0);`) and `RunEvent::ExitRequested`.
* ***AI Suggestion:*** PROPOSE FIX
* ***Justification:** Verified in `src-tauri/src/lib.rs`. The clean shutdown checkpoint must be hooked into Tauri's exit events and CLI exit paths.
* ***Proposed Edit:***
  * Location: Task T4.3 (Line 104)
  * Change: Add `src-tauri/src/lib.rs` and `src-tauri/src/main.rs` to target files, define `pub fn checkpoint_truncate(&self) -> KnoxResult<()>` on `KnoxDb`, and invoke it inside Tauri's tray quit handler and `RunEvent::ExitRequested`.

---

### [Issue B7] [`egm-style`] - Severity: High (Blocker)
* **Affected Scope:** [Lines 74, 76, 80, 81, 83, 86, 94–95, 112, 114, 127–130](d:/Knox/knox-implementation-plan.html#L74-L130)
* **Discrepancy Description:** Sarcastic editorializing, informal conversational snark, and personification ("the app admitting", "neither masquerades as the other", "The model panel that does not exist") obscure technical requirements.
* **Rationale:** Violates plain-English, objective engineering documentation standards.
* ***AI Suggestion:*** PROPOSE FIX
* ***Justification:** Rewriting headings and prose in neutral, professional engineering terms improves clarity and readability.
* ***Proposed Edit:***
  * Location: Lines 74–130
  * Change: Replace informal phrases with precise technical descriptions (e.g., `Phase 2 — Model Configuration Settings Panel`, `Phase 3 — Graph Relationship Visibility`).

---

### [Issue M1–M11] [`egm-codebase-details`, `egm-implementation-readiness`, `egm-style`] - Severity: Medium (Major)
* **Affected Scope:** Lines 59–137
* **Summary of Discrepancies:**
  - **M1/M2:** Incorrect directory paths: `settings/` -> `ui/src/components/modals/settings/`; `src-tauri/commands/` -> `src-tauri/src/commands/`.
  - **M3:** CLI syntax: change `--consolidate <path>` flag to `MigrationAction::Consolidate { path: PathBuf }` subcommand, adding `src/cli/args.rs` and `src/cli/dispatch.rs`.
  - **M4:** MCP startup logging must use `eprintln!` / `tracing` to `stderr` only, preventing `stdio` JSON-RPC stream corruption.
  - **M5:** Add `total_edges: usize` to `StorageStats` (`db.rs`) and `DbStats` (`ui/src/types/knox.ts`), documenting doctor SQL formula.
  - **M6:** Add concurrency busy check and retry logic for `PRAGMA wal_checkpoint(TRUNCATE)`.
  - **M7:** Define `relocate_database` contract for the Relocate button.
  - **M8:** Clarify project scope selection for egress settings.
  - **M9:** Provide concrete automated test scenarios (`test_consolidate_idempotency`, `test_resolve_db_path_deprecation`, `test_wal_checkpoint_truncate_on_vacuum`, Vitest suites).
  - **M10/M11:** Replace colloquial acceptance criteria and unmeasurable phrases with verifiable metrics.
* ***AI Suggestion:*** PROPOSE FIX
* ***Proposed Edit:*** Apply systematic technical corrections across tasks T1.1–T4.3 and Section 10 test plan.

---

### [Issue N1–N5] [`egm-codebase-details`, `egm-implementation-readiness`, `egm-style`] - Severity: Low (Nit)
* **Affected Scope:** Lines 63, 90, 93, 118, 58–131
* **Summary of Discrepancies:** Off-by-one line ranges in `StorageTab.tsx` (150–184), missing `total_edges` in DTO, case-insensitive grep verification command, deck slide residue numbers (`05`..`12`), and imprecise coverage tracking horizon.
* ***AI Suggestion:*** PROPOSE FIX
* ***Proposed Edit:*** Apply line adjustments, DTO updates, grep fix, section header formatting, and percentage metric specification.

