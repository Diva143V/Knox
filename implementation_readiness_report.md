# Implementation Readiness Audit Report: knox-implementation-plan.html

**Target Document**: [knox-implementation-plan.html](file:///d:/Knox/knox-implementation-plan.html#L58-L140)  
**Detailed Scope**: Lines 58–140 (Phase 1 through Phase 4, Sequence and effort, How to know it worked, Deliberately excluded, Method and limits)  
**Target Standard**: [implementation-readiness-standard.md](file:///C:/Users/Diwa/.gemini/config/plugins/elephant-goldfish-model/resources/document_standards/implementation-readiness-standard.md)  
**Review Swarm**: Tech Lead (`egm-implementation-readiness`), Codebase Auditor (`egm-codebase-details`), Copy Editor (`egm-style`)  
**Verdict**: ❌ **FAILED — NOT READY FOR IMPLEMENTATION**  
**Findings Summary**: 7 Blockers, 11 Majors, 5 Nits

---

## Executive Summary
The remediation plan outlines a sound strategic intent (enforcing a single global database, surfacing LLM configurations in UI, visualizing graph relationships, and truncating the WAL). However, the detailed specification fails the **Zero-Decision Rule**. A developer or autonomous AI agent attempting to execute lines 58–140 will encounter immediate build breaks, contradictory resolvers, corrupted MCP stdio communication, missing UI/IPC registrations, and undefined database consolidation semantics.

---

## 🔴 Blockers (Must Fix Before Execution)

### B1: Dual Resolver Omission in `core/config.rs` Breaks Phase 1 Consolidation
- **Affected Scope**: [Line 65 (T1.3), Line 66 (T1.4)](file:///d:/Knox/knox-implementation-plan.html#L65-L66)
- **Codebase Reference**: [src/core/config.rs#L169-L195](file:///d:/Knox/src/core/config.rs#L169-L195) and [src/commands/init.rs#L95-L100](file:///d:/Knox/src/commands/init.rs#L95-L100)
- **Discrepancy**: Task T1.3 states `resolve_db_path always returns the global path`. However, `src/core/config.rs` maintains **two** independent resolvers: `resolve_db_path(&self) -> PathBuf` and `resolve_db_path_for(&self, project_path: &Path) -> PathBuf`. The latter is invoked directly by `determine_db_path` in `src/commands/init.rs:96`. If only `resolve_db_path` is modified while `resolve_db_path_for` remains unchanged, running `knox init` will continue creating isolated `.knox/knox.db` files whenever `global_mode` is false or default (`global_mode: false` at line 216).
- **Remediation**: Explicitly specify the deprecation and updating of **both** `resolve_db_path` and `resolve_db_path_for` to consistently return `default_global_db_path()`.

### B2: Undefined Consolidation Engine Architecture & Relational Cascades
- **Affected Scope**: [Line 67 (T1.5), Line 68 (T1.6)](file:///d:/Knox/knox-implementation-plan.html#L67-L68)
- **Codebase Reference**: [src/commands/migrate.rs](file:///d:/Knox/src/commands/migrate.rs) and [src/storage/migrations.rs#L380-L450](file:///d:/Knox/src/storage/migrations.rs#L380-L450)
- **Discrepancy**: Task T1.5 mandates `knox migrate --consolidate <path>`, but leaves the migration engine undefined:
  1. *Relational Scope*: Knox contains 11+ tables (`actions`, `action_relations`, `edges`, `action_files`, `action_tags`, `action_symbols`, `action_deltas`, `action_state`, `sessions`, `projects`, `guardrail_rules`). The plan does not specify which tables are migrated or how relational child rows are mapped.
  2. *Deduplication Fallback*: `dedupe_key` was introduced in schema v7 and is nullable. If an action has `dedupe_key IS NULL`, what constitutes a duplicate? If skipped, are its delta summaries and edges merged or dropped?
  3. *Foreign Key Collisions*: Actions in local DBs reference local UUIDs for `project_id` and `session_id`. If the local DB has a different UUID than the global registry for the same canonical path, how are foreign keys remapped?
  4. *Execution Mechanism*: Unspecified whether this uses SQLite `ATTACH DATABASE` with SQL `INSERT OR IGNORE` or in-memory row extraction via `rusqlite`.
- **Remediation**: Detail the exact SQL queries, foreign key remapping logic, table migration order, and fallback deduplication rules (e.g. matching `(timestamp, summary, actor)` when `dedupe_key IS NULL`).

### B3: Missing End-to-End Registries for Phase 2 Model Panel (UI & Tauri IPC)
- **Affected Scope**: [Lines 78–83 (T2.1, T2.2, T2.3)](file:///d:/Knox/knox-implementation-plan.html#L78-L83)
- **Codebase Reference**:
  - [ui/src/types/knox.ts#L125-L151](file:///d:/Knox/ui/src/types/knox.ts#L125-L151)
  - [ui/src/components/modals/SettingsModal.tsx#L39-L94](file:///d:/Knox/ui/src/components/modals/SettingsModal.tsx#L39-L94)
  - [ui/src/services/api.ts#L301-L318](file:///d:/Knox/ui/src/services/api.ts#L301-L318)
  - [src-tauri/src/lib.rs#L171-L211](file:///d:/Knox/src-tauri/src/lib.rs#L171-L211)
- **Discrepancy**: The plan states *"This phase is a form and two IPC commands, not new engine work"* and lists only 4 files. An implementer following this will hit compile/runtime failures:
  1. `SettingsTabId` in `ui/src/types/knox.ts` is a closed union of 9 tab IDs; `"model"` is not included.
  2. `CATEGORIES` in `SettingsModal.tsx` hardcodes the modal navigation items. Adding `ModelTab` to `settings/index.ts` does nothing without registering it in `SettingsModal.tsx`.
  3. `KnoxConfig` in `ui/src/types/knox.ts` and `KnoxAPI.updateConfig()` in `ui/src/services/api.ts` lack model configuration fields.
  4. Any new IPC command in `src-tauri/src/commands/model.rs` (e.g. `test_model_connection`) must be registered in `tauri::generate_handler![...]` in `src-tauri/src/lib.rs`, or UI calls reject with `"command not found"`.
- **Remediation**: Add `ui/src/types/knox.ts`, `ui/src/components/modals/SettingsModal.tsx`, `ui/src/services/api.ts`, and `src-tauri/src/lib.rs` to the explicit target scope with DTO definitions.

### B4: Missing Detection Storage, Upgrade State Mechanism, and Consolidation IPC
- **Affected Scope**: [Line 68 (T1.6)](file:///d:/Knox/knox-implementation-plan.html#L68)
- **Discrepancy**:
  1. *Project Root Source*: "Scan known project roots" does not specify where roots come from (`SELECT DISTINCT path FROM projects` vs external registry).
  2. *One-Time Prompt State*: "Asked once, not silently ignored" requires persistent storage. Unspecified whether this lives in `~/.knox/config.json`, `knox_meta`, or `localStorage`.
  3. *Missing IPC Contracts*: No Tauri command signatures exist for scanning stray databases or executing consolidation with dry-run progress.
- **Remediation**: Specify `stray_db_prompted: bool` in `KnoxConfig`, the project query `SELECT DISTINCT path FROM projects WHERE path IS NOT NULL`, and define IPC signatures `scan_stray_databases()` and `consolidate_stray_database()`.

### B5: Incomplete Model IPC Contract & Secret Masking Protocol
- **Affected Scope**: [Lines 78–80 (T2.1–T2.3)](file:///d:/Knox/knox-implementation-plan.html#L78-L80)
- **Discrepancy**:
  1. *Secret Masking Hazard*: Editing `model_api_key` without an explicit protocol causes implementers to either write masked asterisks (`sk-...****`) back to disk on save or fail to update keys.
  2. *DTO Gaps*: `ConfigDto` in `src-tauri/src/commands/config.rs` and `KnoxConfig` in TypeScript lack model fields.
  3. *Unspecified Test Signature*: Does `test_model_connection` test unsaved in-flight form inputs or already-persisted settings?
- **Remediation**: Specify secret update contract: pass `model_api_key: Option<String>` where `None` leaves the existing key intact and `Some(key)` updates it. Define `test_model_connection(draft: ModelSettingsDto) -> Result<ModelTestResultDto, String>`.

### B6: Structurally Unimplementable Clean Shutdown Scope
- **Affected Scope**: [Line 104 (T4.3)](file:///d:/Knox/knox-implementation-plan.html#L104)
- **Codebase Reference**: [src/storage/db.rs#L33-L45](file:///d:/Knox/src/storage/db.rs#L33-L45) and [src-tauri/src/lib.rs#L73-L77](file:///d:/Knox/src-tauri/src/lib.rs#L73-L77)
- **Discrepancy**: Task T4.3 assigns "Checkpoint on clean shutdown" solely to `storage/db.rs`. `KnoxDb` is a library struct with no lifecycle interceptors. Desktop shutdown is handled in `src-tauri/src/lib.rs` tray quit (`app.exit(0);`), while CLI shutdown is in `src-tauri/src/main.rs`. Specifying `storage/db.rs` alone will never execute on app exit.
- **Remediation**: Add `src-tauri/src/lib.rs` and `src-tauri/src/main.rs` to target files, define `pub fn checkpoint_truncate(&self) -> KnoxResult<()>` on `KnoxDb`, and hook it into Tauri's `RunEvent::ExitRequested` and tray quit handler.

### B7: Sarcastic Editorializing & Sloganeering Across Tasks
- **Affected Scope**: [Lines 74, 76, 80, 81, 83, 86, 94–95, 112, 114, 127–130](file:///d:/Knox/knox-implementation-plan.html#L74-L130)
- **Discrepancy**: Phrases such as *"The model panel that does not exist"*, *"Nine tabs today, none of them for models"*, *"Make the graph used"*, *"the app admitting which database it is using"*, *"Destructive, and the opposite of the fix"*, and *"you have 31 actions. ... the metric decides"* violate objective engineering documentation standards and obscure technical requirements with polemical snark.
- **Remediation**: Rewrite headings and task descriptions with dry, precise technical language (e.g. `Phase 2 — Model Configuration Settings Panel`, `Phase 3 — Graph Relationship Visibility`).

---

## 🟡 Major Issues (File Paths, Syntax, and Protocol Safety)

### M1: Non-Existent UI Settings Directory Path Cited Across Plan
- **Affected Scope**: [Lines 63, 68, 78, 81, 82, 90, 91, 103, 118, 133](file:///d:/Knox/knox-implementation-plan.html#L63-L133)
- **Discrepancy**: The plan repeatedly cites `settings/StorageTab.tsx`, `settings/ModelTab.tsx`, and `settings/index.ts`. No `ui/src/components/settings/` directory exists. The actual directory is `ui/src/components/modals/settings/`.
- **Remediation**: Use workspace-relative paths: `ui/src/components/modals/settings/StorageTab.tsx` and `ui/src/components/modals/settings/index.ts`.

### M2: Non-Existent Tauri Source Path Cited Across Plan
- **Affected Scope**: [Lines 68, 79, 80, 133](file:///d:/Knox/knox-implementation-plan.html#L68-L133)
- **Discrepancy**: The plan cites `src-tauri/commands/config.rs` and `src-tauri/commands/model.rs`. The `src/` directory is omitted.
- **Remediation**: Update paths to `src-tauri/src/commands/config.rs` and `src-tauri/src/commands/model.rs`.

### M3: CLI Subcommand Syntax and Scope Misalignment for `knox migrate`
- **Affected Scope**: [Line 67 (T1.5)](file:///d:/Knox/knox-implementation-plan.html#L67)
- **Codebase Reference**: [src/cli/args.rs#L195-L209](file:///d:/Knox/src/cli/args.rs#L195-L209) and [src/cli/dispatch.rs#L163-L186](file:///d:/Knox/src/cli/dispatch.rs#L163-L186)
- **Discrepancy**: T1.5 calls for `--consolidate <path>`. In `src/cli/args.rs`, `MigrationAction` is an enum of subcommands (`Check`, `Apply`, `Restore`, `Split`), not flags. Furthermore, the task omits `src/cli/args.rs` and `src/cli/dispatch.rs` from its file list.
- **Remediation**: Update specification to `MigrationAction::Consolidate { path: PathBuf }` (`knox migrate consolidate <path>`) and add `src/cli/args.rs` and `src/cli/dispatch.rs` to modified files.

### M4: MCP stdio Protocol Corruption Hazard in Startup Logging
- **Affected Scope**: [Line 69 (T1.7), Line 119](file:///d:/Knox/knox-implementation-plan.html#L69-L119)
- **Codebase Reference**: [src/mcp/server.rs#L25-L42](file:///d:/Knox/src/mcp/server.rs#L25-L42)
- **Discrepancy**: T1.7 states `Log the resolved DB path and schema version at startup`. Knox's MCP server uses `tokio::io::stdin()` / `stdout()` for JSON-RPC 2.0 framing. Any output to `stdout` corrupts the JSON stream and crashes MCP clients (Cursor, Claude Desktop).
- **Remediation**: Explicitly mandate that startup logging must use `eprintln!` or `tracing::info!` directed exclusively to `stderr`.

### M5: Ambiguous Definition of "Edges" Metric & Coverage Formula
- **Affected Scope**: [Lines 90, 93 (T3.1, T3.4)](file:///d:/Knox/knox-implementation-plan.html#L90-L93)
- **Discrepancy**: Knox has both legacy `action_relations` (v2) and `edges` (v8). The plan instructs to "Add an edges count beside the relations count", but does not update `StorageStats` in `src/storage/db.rs` or `DbStats` in TypeScript. In T3.4, "share of actions with at least one edge" omits the SQL formula.
- **Remediation**: Add `total_edges: usize` to `StorageStats` and `DbStats`, and document the exact SQL query:  
  `SELECT CAST(COUNT(DISTINCT a.id) AS REAL) / MAX(COUNT(*), 1) FROM actions a LEFT JOIN edges e ON (a.id = e.from_action_id OR a.id = e.to_action_id)`.

### M6: Missing Concurrency / Busy Handling in WAL Checkpoint
- **Affected Scope**: [Line 102 (T4.1)](file:///d:/Knox/knox-implementation-plan.html#L102)
- **Discrepancy**: `PRAGMA wal_checkpoint(TRUNCATE)` returns `(busy, log, checkpointed)`. If active concurrent readers/writers hold locks, the checkpoint returns busy without truncating.
- **Remediation**: Specify that `db.vacuum()` must execute `PRAGMA wal_checkpoint(TRUNCATE)`, inspect the `busy` column, and apply retry logic up to `TX_MAX_RETRIES` before issuing `VACUUM`.

### M7: Unspecified Contract for "Relocate Button"
- **Affected Scope**: [Line 63 (T1.1)](file:///d:/Knox/knox-implementation-plan.html#L63)
- **Discrepancy**: The task specifies replacing the toggle with an identity card and a "Relocate button", but does not define whether this physically moves the database, `-wal`, and `-shm` sidecars, or simply modifies `kg_path` in configuration.
- **Remediation**: Define `relocate_database(new_directory: String)` IPC command that closes connections, moves database files, and updates `config.json`.

### M8: Undefined Project Scope in Global Model Settings
- **Affected Scope**: [Line 81 (T2.4)](file:///d:/Knox/knox-implementation-plan.html#L81)
- **Discrepancy**: `ModelTab.tsx` resides in the global settings modal. The task specifies a "per-project egress toggle beside it", but does not explain how project scope is selected within a global modal or how `projects.egress` is persisted.
- **Remediation**: Specify whether `ModelTab` contains a project selector dropdown or if per-project egress toggling belongs in `WorkspaceView.tsx`.

### M9: Total Absence of Automated Test Specifications and Assertions
- **Affected Scope**: [Lines 116–125 (Section 10)](file:///d:/Knox/knox-implementation-plan.html#L116-L125)
- **Discrepancy**: Violates the standard's mandatory test plan requirement. The plan lists only manual spot checks ("grep globalMode", "Press Vacuum, re-read the panel"). There are zero unit or integration test specifications.
- **Remediation**: Specify concrete test scenarios: `test_consolidate_idempotency()`, `test_resolve_db_path_deprecation()`, `test_wal_checkpoint_truncate_on_vacuum()`, and Vitest tests for `ModelTab.tsx`.

### M10: Slang and Colloquial Acceptance Criteria in Phase 1
- **Affected Scope**: [Lines 59, 61, 64, 66, 68, 70–71](file:///d:/Knox/knox-implementation-plan.html#L59-L71)
- **Discrepancy**: Phrases like *"Remove the choice, consolidate what it created"*, *"gone too"*, and *"asked once, not silently ignored"* lack measurable engineering criteria.
- **Remediation**: Replace with precise acceptance criteria (e.g. `WorkspaceView displays read-only database badge; mode switch modal removed`).

### M11: Subjective and Imprecise Verification in Phase 4
- **Affected Scope**: [Lines 100, 102, 103, 104](file:///d:/Knox/knox-implementation-plan.html#L100-L104)
- **Discrepancy**: Criteria like *"near zero"*, *"WAL is small"*, and *"stops reading as a leak"* are unmeasurable.
- **Remediation**: Specify exact thresholds: `WAL file size truncates to 0 bytes following vacuum completion`.

---

## 🟢 Nits & Minor Technical Discrepancies

### N1: Off-by-One Target Line Range for StorageTab.tsx Toggle
- **Affected Scope**: [Line 63 (T1.1)](file:///d:/Knox/knox-implementation-plan.html#L63)
- **Discrepancy**: Plan cites lines 161–181. The containing wrapper `div` starts at line 150 and ends at line 184. Replacing only lines 161–181 leaves dangling container labels.
- **Remediation**: Update target lines to `150–184`.

### N2: Storage Telemetry DTO Omission for Edge Counts
- **Affected Scope**: [Line 90 (T3.1)](file:///d:/Knox/knox-implementation-plan.html#L90)
- **Discrepancy**: `DbStats` in `ui/src/types/knox.ts` lacks `total_edges`.
- **Remediation**: Include `ui/src/types/knox.ts` in T3.1 file list.

### N3: Flawed Verification Grep Command
- **Affected Scope**: [Line 118](file:///d:/Knox/knox-implementation-plan.html#L118)
- **Discrepancy**: `grep globalMode ui/src/components returns 0` passes even if `config?.global_mode` remains.
- **Remediation**: Specify `grep -i "global_?mode" ui/src/components`.

### N4: Deck Slide Numbering Residue
- **Affected Scope**: [Lines 58, 73, 85, 97, 105, 115, 125, 131](file:///d:/Knox/knox-implementation-plan.html#L58-L131)
- **Discrepancy**: Bare numbers (`05`, `06`, `07`, `08`, `09`, `10`, `11`, `12`) appear as slide artifacts.
- **Remediation**: Format as standard markdown section headers (`### Phase 1: ...`).

### N5: Imprecise Acceptance Time Horizon in T3.4
- **Affected Scope**: [Line 93 (T3.4)](file:///d:/Knox/knox-implementation-plan.html#L93)
- **Discrepancy**: *"A number you can track over a week"* is conversational filler.
- **Remediation**: Change to `knox doctor outputs relation coverage percentage: (actions with >= 1 edge) / total actions`.

---

## Traceability Matrix

| Task ID | Blocker / Major Findings | Affected Files | Status |
| :--- | :--- | :--- | :---: |
| **T1.1** | M1, M7, N1 | `ui/src/components/modals/settings/StorageTab.tsx` | Needs fix |
| **T1.2** | M10 | `ui/src/components/views/WorkspaceView.tsx` | Needs fix |
| **T1.3** | B1 | `src/core/config.rs`, `src/commands/init.rs` | 🔴 Blocked |
| **T1.4** | B1 | `src/core/config.rs` | 🔴 Blocked |
| **T1.5** | B2, M3 | `src/commands/migrate.rs`, `src/cli/args.rs`, `src/cli/dispatch.rs` | 🔴 Blocked |
| **T1.6** | B4, M2 | `src-tauri/src/commands/config.rs`, `ui/src/services/api.ts` | 🔴 Blocked |
| **T1.7** | M4 | `src/mcp/server.rs` | Needs fix |
| **T2.1** | B3, B5, M1 | `ui/src/components/modals/settings/ModelTab.tsx`, `SettingsModal.tsx` | 🔴 Blocked |
| **T2.2** | B3, B5, M2 | `src-tauri/src/commands/config.rs`, `ui/src/types/knox.ts` | 🔴 Blocked |
| **T2.3** | B3, B5, M2 | `src-tauri/src/commands/model.rs`, `src-tauri/src/lib.rs` | 🔴 Blocked |
| **T2.4** | B5, B7, M8 | `ui/src/components/modals/settings/ModelTab.tsx` | 🔴 Blocked |
| **T3.1** | M5, N2 | `src/storage/db.rs`, `ui/src/types/knox.ts` | Needs fix |
| **T3.4** | M5, N5 | `src/commands/doctor.rs` | Needs fix |
| **T4.1** | M6 | `src/storage/db.rs` | Needs fix |
| **T4.3** | B6 | `src/storage/db.rs`, `src-tauri/src/lib.rs`, `src-tauri/src/main.rs` | 🔴 Blocked |
| **Sec 10**| M9, N3 | Automated test suite (`tests/`) | Needs fix |
