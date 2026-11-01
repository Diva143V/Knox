# Knox — Phase 0: Foundation & Baseline Verification

**Date:** 2026-11-01  
**Branch:** `feature/ui-ux-redesign` (tracking `fork/feature/ui-ux-redesign`)  
**Commit:** `48369b3`  
**Scope:** environment, dependency install, baseline checks, test infrastructure, CI/release integrity.  
**Safety:** no source edits, no branch changes, no commits, no pushes. `AUDIT.md` preserved. Build-generated EOL-only changes to `src-tauri/gen/schemas/*.json` were restored to HEAD.

---

## 1. Environment

| Tool | Version | Notes |
|---|---|---|
| Node.js | `v24.20.0` | CI pins Node 22 |
| npm | `11.19.0` | Not used for install |
| pnpm | `12.16.1` | CI pins pnpm 9; both accept lockfile v9 |
| corepack | `0.35.0` | Available if a pnpm downgrade is needed |
| rustc | `1.99.1 (58cf8f9c3 2026-10-13)` | `Cargo.toml` MSRV is 1.85 |
| cargo | `1.99.1 (5f94df478 2026-08-27)` | Workspace resolver 2 |
| rustfmt | `1.10.0-stable (58cf8f9c35 2026-10-13)` | Edition-2024 style |
| OS | Windows (win32) | PowerShell 5.1 wrapper |

**Toolchain parity risk:** local Node/pnpm are newer than CI. Installs succeeded with the existing v9 lockfiles and `--frozen-lockfile`, so this is not a current blocker. Pin versions (`packageManager`, `engines`, or CI/local alignment) to prevent future drift.

**Skill note:** no UI/UX skill (`ui-ux-pro-max` or equivalent) is installed in this environment. Phase 0 is build/backend verification and did not require one. Re-check before Phase 1 UI work.

---

## 2. Dependency installation

All installs used the existing lockfiles with `--frozen-lockfile`; **no dependency was upgraded or added**.

| Location | Command | Result | Packages |
|---|---|---|---|
| Root | `pnpm install --frozen-lockfile` | Pass (262 ms) | `+2` (`@tauri-apps/cli 2.12.4`) |
| UI | `pnpm --dir ui install --frozen-lockfile` | Pass (16.4 s) | `+143` |

Lockfiles: `pnpm-lock.yaml` and `ui/pnpm-lock.yaml`, both `lockfileVersion: '9.0'`. pnpm 12 accepted them without migration. No `.npmrc`, `pnpm-workspace.yaml`, or `packageManager` field exists.

Key resolved versions (from lockfile):

- React `19.3.1`, React DOM `19.3.1`
- TypeScript `5.9.3`, Vite `6.4.5`, `@vitejs/plugin-react 4.7.0`
- Tailwind CSS `4.3.3`, `@tailwindcss/vite 4.3.3`
- Tauri CLI `2.12.4`, `@tauri-apps/api 2.12.4`, dialog plugin `2.8.4`
- Zustand `5.0.16`, `@xyflow/react 12.12.0`, `lucide-react 1.63.0`, `cmdk 1.1.1`
- `d3-force 3.0.0`, `@dagrejs/dagre 3.1.3`, `elkjs 0.9.3`

No install warnings/errors. `node_modules/` and `target/` are gitignored.

---

## 3. Baseline verification

| Check | Command | Result | Evidence |
|---|---|---|---|
| TypeScript | `pnpm --dir ui typecheck` | **PASS** | `tsc --noEmit`, no output |
| Frontend build | `pnpm --dir ui build` | **PASS** | 2,282 modules, 26.74 s |
| Frontend tests | `ui/package.json` | **NONE** | No test script, no test files |
| Rust check | `cargo check --workspace` | **PASS** | exit 0, 4m19s first run |
| Rust tests | `cargo test --workspace` | **PASS** | **168 passed, 0 failed** |
| Rust Clippy | `cargo clippy --workspace --all-targets -- -D warnings` | **PASS** | exit 0, 19.26 s |
| Rust format | `cargo fmt --all -- --check` | **FAIL** | exit 1, 108 hunks in 35 files |

### 3.1 Frontend build output

- `ui/dist/index.html` — 8.15 kB (2.22 kB gzip)
- `ui/dist/assets/index-*.css` — 110.56 kB (16.74 kB gzip)
- `ui/dist/assets/index-*.js` — **2,408.58 kB (714.96 kB gzip), one chunk**
- No chunk-size warning was emitted, but this is a single monolithic bundle. Route-level splitting is a Phase 5 concern.

### 3.2 Rust test breakdown

| Target | Tests |
|---|---|
| `knox` library unit tests | 122 |
| `knox_desktop_lib` unit tests | 11 |
| `test_commands` | 4 |
| `test_core` | 3 |
| `test_e2e` | 2 |
| `test_graph` | 2 |
| `test_ingest` | 4 |
| `test_mcp` | 12 |
| `test_storage` | 8 |
| `knox` binary / doc-tests | 0 |
| **Total** | **168** |

The audit estimated 134 test functions by counting `#[test]` attributes; the harness reports **168**. The Phase 0 number is authoritative.

### 3.3 Formatting failure

`cargo fmt --all -- --check` reports **108 diff hunks across 35 files**, including:

- `src/ingest/*.rs` (openai, summary, cursor, claude, generic, git_log, mod)
- `src/mcp/format.rs`, `src/mcp/handlers/*.rs`
- `src/storage/{db,nodes,queries,sessions}.rs`
- `src/cli/dispatch.rs`, `src/commands/ingest.rs`
- `src-tauri/src/main.rs`, `src-tauri/src/commands/{mcp,query,thoughts}.rs`
- `tests/test_mcp.rs`

The changes are rustfmt style: import ordering, line wrapping, and edition-2024 formatting. No logic is implicated. Because CI uses `dtolnay/rust-toolchain@stable`, a current stable run would hit the same failure unless the repo is normalized or the toolchain is pinned.

### 3.4 Tooling note

PowerShell 5.1 treats cargo’s stderr progress as an error and can report a non-zero wrapper exit even when cargo succeeds. Exit codes in this report were captured through `cmd /c`; `cargo check` is genuinely **0**.

---

## 4. Failures

### 4.1 Pre-existing failures (confirmed)

1. **`cargo fmt --check` fails** — 35 files / 108 hunks. This is the only failing baseline gate.
2. **CI/release artifact names are wrong** — confirmed by execution (§5).
3. **No frontend test harness** — zero test files, no Vitest/Jest/RTL, no `test` script.
4. **No UI lint** — no ESLint/Biome configuration.
5. **CI does not run on feature-branch pushes** — triggers are `main` pushes and PRs targeting `main` only (`ci.yml:3–7`).

### 4.2 Newly discovered

1. **Tauri build dirties tracked schemas.** `pnpm tauri build` rewrites `src-tauri/gen/schemas/desktop-schema.json` and `windows-schema.json` with LF, producing EOL-only modifications. Restored after verification. Recommend `.gitattributes` normalization or ignoring generated schemas.
2. **`pnpm build` prebuild moves the binary.** The PowerShell `prebuild` moves `target/release/knox.exe` to `knox.exe.old` before rebuilding (`package.json:7`). It works on Windows but is not cross-platform.
3. **Single 2.4 MB JS chunk** (715 kB gzip). Not a failure, but a performance baseline.
4. **Audit test count was low** — 134 estimated vs 168 executed.

### 4.3 Confirmations (not failures)

- TypeScript strict mode and `noUnusedLocals`/`noUnusedParameters` pass.
- Clippy passes at `-D warnings`; no lint debt at the configured severity.
- All 168 Rust tests pass, including Tauri command tests.
- The Windows NSIS installer builds successfully.
- `pnpm 12` accepts the v9 lockfiles without changes.

**No new source-level failures were introduced** — no application source files were edited.

---

## 5. CI and release integrity (confirmed by execution)

### 5.1 Actual Cargo targets

`cargo metadata --no-deps` reports:

| Package | Targets |
|---|---|
| `knox 1.2.1` | **lib only** + 7 integration test targets; **no bin** |
| `knox-desktop 1.2.1` | lib (`staticlib,cdylib,rlib`) + **bin `knox`** + build script |

`Cargo.toml` sets `autobins = false` and defines no `[[bin]]`. `src-tauri/Cargo.toml` defines the only bin:

```toml
[[bin]]
name = "knox"
path = "src/main.rs"
```

### 5.2 Build evidence

| Command | Result | Artifacts |
|---|---|---|
| `cargo build --package knox --release` | exit 0, 3m27s | `target/release/libknox.rlib` only; **no `knox.exe`** |
| `pnpm tauri build --no-bundle` | exit 0, 13m40s | `target/release/knox.exe` (19,417,600 bytes) |
| `pnpm build` (`tauri build --bundles nsis`) | exit 0, 6m06s | `target/release/bundle/nsis/Knox_1.2.1_x64-setup.exe` (6.01 MiB) |

`target/release/knox-desktop.exe` does **not** exist after either build.

### 5.3 Workflow mismatch

**CI (`ci.yml`)**

- Matrix expects `binary_cli: target/release/knox` and `binary_desktop: target/release/knox-desktop` (`:64–77`).
- Builds with `cargo build --package knox --release` (`:117–118`) and `pnpm tauri build --no-bundle` (`:120–121`).
- `cargo build -p knox` produces no executable; `pnpm tauri build` produces `knox`, which means the **CLI artifact is actually the unified desktop binary**.
- The desktop artifact path never exists. `upload-artifact@v4` defaults to `warn`, so the job may pass while silently omitting the desktop artifact.

**Release (`release.yml`)**

- Matrix expects `target/<triple>/release/knox` and `.../knox-desktop` (`:24–48`).
- Builds `cargo build --package knox --release --target ...` (`:88–89`, no binary) and `pnpm tauri build --target ... --no-bundle` (`:91–92`, produces `knox`).
- Packaging runs `chmod +x` / `tar` / `Compress-Archive` on `knox-desktop` (`:94–112`) → **hard failure** because the file does not exist.
- Uploads two archives (`:114–121`) even though the product now has one unified binary.

**Installer:** the NSIS bundle is configured in `tauri.conf.json:32–46` and builds correctly on Windows. It is not uploaded by either workflow.

### 5.4 Conclusion

The product builds and installs correctly. The **workflows are stale**, not the build itself. Recommended architecture (to be approved before changing):

- Treat `knox` as the single unified binary (CLI + desktop).
- Publish one `knox`/`knox.exe` artifact per OS, plus the NSIS installer on Windows.
- Remove `cargo build -p knox` as a “CLI” step, or make the root package a real bin only if a separate CLI is intentionally required.
- Update artifact names and release packaging to the paths that actually exist.
- Add a CI smoke step that asserts the expected artifact exists before upload.

Do not change release architecture until this is approved.

---

## 6. Frontend testing infrastructure

### 6.1 Current status

- **Vitest is not configured.** No `vitest.config.*`, no `test` script, no `vitest`/`jsdom`/Testing Library dependencies.
- **Zero frontend test files** under `ui/src`.
- CI has no frontend test job.
- Existing verification is `tsc` + `vite build` only.

### 6.2 Proposed smallest compatible setup (not installed)

Keep it minimal and defer component testing until pure logic is covered:

```bash
pnpm --dir ui add -D vitest jsdom @testing-library/react @testing-library/jest-dom
```

- Pin a Vitest major compatible with Vite 6 (Vitest 3.x is the safe line; verify peer dependencies before install).
- Add `ui/vitest.config.ts` using the existing React plugin, `environment: "jsdom"`, and a setup file.
- Add `ui/src/test/setup.ts` with `import "@testing-library/jest-dom/vitest";`.
- Add scripts: `"test": "vitest run"`, `"test:watch": "vitest"`.
- Add a CI step after typecheck: `pnpm --dir ui test`.
- Add `@testing-library/user-event` only when interaction tests begin.

### 6.3 Critical first tests

1. `lib/theme.ts` — light/dark/system resolution, persistence, OS-change listener (once added).
2. `useKnoxStore` — settings merge/migration, theme, scope, refresh, onboarding flags.
3. `lib/semantics.ts` — action/actor mapping and token classes.
4. `useKeyboardShortcuts` — `1–8`, `G`-prefix, `Esc` precedence.
5. Query filtering/selection — extract the facet-filter + selection index into a pure helper and test it (covers the current `results` vs `displayedResults` bug).
6. `SettingsModal` — each setting has a consumer (once Phase 1 wires them).

---

## 7. Recommended fixes, ranked

### P0 — before Phase 1 implementation

1. **Normalize Rust formatting in a dedicated commit.** Run `cargo fmt --all`, verify `cargo fmt --all -- --check`, and keep it separate from logic changes. This unblocks CI’s first gate. If formatting churn is unacceptable, pin `rust-toolchain.toml` to the version used for the last formatting pass instead — but pinning must be deliberate.
2. **Fix CI/release artifact names** using the single-`knox` architecture above. Add an artifact-existence assertion. Document the decision before editing workflows.
3. **Add frontend tests to CI** after the minimal Vitest setup; start with pure logic/store tests.
4. **Add feature-branch CI coverage** (`feature/**` push or `workflow_dispatch`) so this branch gets feedback before a PR.

### P1 — baseline hardening

5. Add `.gitattributes` (`* text=auto`) or ignore `src-tauri/gen/schemas/*.json` so Tauri builds stop dirtying tracked files.
6. Align local/CI toolchains: add `packageManager` and `engines`, and document the Rust version. CI uses Node 22 / pnpm 9; local is Node 24 / pnpm 12.
7. Replace the PowerShell-only `prebuild` with a cross-platform Node script, or scope it to Windows and document it.
8. Add a UI lint step (ESLint or Biome) once a config is chosen; do not add both.
9. Add a build smoke check that asserts `target/release/knox(.exe)` exists and that the NSIS installer exists on Windows.

### P2 — performance and hygiene

10. Split the 2.4 MB JS bundle by route (Phase 5).
11. Remove or archive the stale `knox-desktop` naming from docs and workflows.
12. Update the audit’s test count from 134 to 168 when the audit is next revised (do not rewrite history now).

---

## 8. Exact next steps for Phase 1

Phase 1 is **Settings truth**. Do not start it until the P0 formatting/artifact decisions are approved.

1. **Prerequisite:** land the formatting-only commit and the artifact-name fix (or explicitly defer them with a tracked issue).
2. **Create one settings API:** `useUserSettings()` with typed selectors, a single storage key (`knox_user_settings_v1`), and a migration path. Keep `AUDIT.md` as the source of the wiring matrix.
3. **Wire in this order:**
   - Theme (including real `system` mode + `matchMedia` listener; add System to `ThemeSwitcher`).
   - `uiDensity`, `enableGlowFx`, `defaultStartupView`.
   - Graph defaults: `defaultGraphMode`, `defaultLivePhysics`, `defaultLayoutEngine`, `showFileNodes`, `showLegend`, `showEdgeLabels`, `chargeStrength`, `linkDistance`, `collisionPadding`.
   - MCP: unify `mcpCommandMode`; remove the duplicate `knox_mcp_command_mode` key or make it the single source.
   - Scanner/ingest: author, ignore patterns, diff depth, dedup, preview — wire what the backend supports; hide/remove what it does not.
   - Tray/notifications: wire `closeToTray` and `showDesktopNotifications`; remove `startOnBoot`/`ingestWatchPaths` unless the plugins/commands are added.
4. **Add tests:** settings merge/persistence, theme system resolution, and one test per wired setting group.
5. **Acceptance:** no inert controls; `grep` shows every setting has a consumer; typecheck/build/tests remain green; manual matrix in light/dark.
6. **Do not** start DB relocation (Phase 2), design-token work (Phase 3), or a11y refactors (Phase 4) in the same change set.

---

## 9. Git and artifact safety

| Item | Status |
|---|---|
| Branch | `feature/ui-ux-redesign` (unchanged) |
| HEAD | `48369b3` (unchanged) |
| Commits/pushes | None |
| `AUDIT.md` | Preserved, untracked |
| `docs/PHASE_0_REPORT.md` | New, untracked |
| `.gitignore` | Narrow exception so the required report is trackable: `docs/*` + `!docs/PHASE_0_REPORT.md` (other docs/screenshots remain ignored) |
| Source changes | None |
| Generated schema files | EOL-only changes from Tauri build; restored to HEAD |
| Build outputs | `node_modules/`, `ui/dist/`, `target/` — gitignored |
| Working tree | `.gitignore` modified; `AUDIT.md` + `docs/PHASE_0_REPORT.md` untracked |

`git status --porcelain` after verification:

```text
 M .gitignore
?? AUDIT.md
?? docs/PHASE_0_REPORT.md
```

---

## 10. Commands run (summary)

```text
node -v / pnpm -v / rustc -V / cargo -V
pnpm install --frozen-lockfile
pnpm --dir ui install --frozen-lockfile
pnpm --dir ui typecheck
pnpm --dir ui build
cargo check --workspace
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all -- --check
cargo metadata --no-deps --format-version 1
cargo build --package knox --release
pnpm tauri build --no-bundle
pnpm build
```

---

## 11. Limitations

- Only Windows was verified locally; Linux/macOS compilation and bundling were not executed.
- CI workflows were inspected and their build steps reproduced locally, but GitHub Actions itself was not run.
- The installer was built but not installed/launched; runtime UI smoke testing is not part of Phase 0.
- No database, user data, or application source was modified.

---

## 12. Phase 0.5: Toolchain and Release Alignment Outcomes

**Date:** 2026-11-01  
**Scope:** Rust formatting investigation (Task A), CI and release workflow alignment (Task B), and full toolchain verification (Task C).

### 12.1 Task A — Rust Formatting Investigation

1. **CI Configuration and Repository Toolchain History:**
   - Both `.github/workflows/ci.yml` and `.github/workflows/release.yml` have consistently used `dtolnay/rust-toolchain@stable` since introduction (`df65055`).
   - The repository has **never** contained a `rust-toolchain`, `rust-toolchain.toml`, or `rustfmt.toml` file at any commit in its Git history.
   - `Cargo.toml` specifies `edition = "2024"` and `rust-version = "1.85"`.
   - Local toolchain: `rustc 1.99.1 (58cf8f9c3 2026-10-13)`, `rustfmt 1.10.0-stable (58cf8f9c35 2026-10-13)`.

2. **Expected Rust / rustfmt Version:**
   - The project targets Rust 1.85+ stable with Edition 2024 idioms and standard stable rustfmt.

3. **Nature of the 108 Formatting Differences:**
   - The 108 diff hunks across 35 files are **not** version-specific regressions.
   - They consist of universal formatting rules:
     - Trailing whitespace removal (e.g. `src/commands/ingest.rs:32`).
     - Redundant newline removal (e.g. `src/cli/dispatch.rs:159`).
     - Expression wrapping for statements exceeding 100 characters (e.g. `src/commands/ingest.rs:92`, `src/ingest/claude.rs:23, 30`).
     - Import list sorting and multi-line grouping (e.g. `src/ingest/action_group.rs:8`).
     - Method chaining formatting (e.g. `tests/test_mcp.rs:547, 568`).
   - These unformatted lines were introduced by recent feature commits on `feature/ui-ux-redesign` (`3a7a8ca`, `35a154d`, `1802587`, `3e5a31a`, `b1dead4`, etc.) where code was committed without running `cargo fmt`. Because CI currently only runs on `main` branch pushes/PRs, CI did not intercept these commits.

4. **Recommended Solution:**
   - Execute a dedicated, formatting-only commit (`chore: format codebase with cargo fmt`) once approved.
   - Formatting changes must **not** be mixed with application or workflow changes.

### 12.2 Task B — CI and Release Alignment

Both GitHub workflows were inspected and aligned with the actual unified `knox` binary architecture and Tauri v2 build pipeline:

1. **`.github/workflows/ci.yml`:**
   - Matrix updated to build single unified binary:
     - `ubuntu-latest`: `artifact_binary: knox-linux-amd64`, `binary_path: target/release/knox`, `bundle_args: --no-bundle`.
     - `macos-latest`: `artifact_binary: knox-macos-amd64`, `binary_path: target/release/knox`, `bundle_args: --no-bundle`.
     - `windows-latest`: `artifact_binary: knox-windows-amd64`, `binary_path: target/release/knox.exe`, `bundle_args: --bundles nsis`.
   - Replaced redundant `cargo build --package knox --release` (which produced no binary) and `pnpm tauri build --no-bundle` with single build command: `pnpm tauri build ${{ matrix.bundle_args }}`.
   - Added pre-upload assertion step (`Verify build outputs exist`) to fail fast if expected binaries or installers are missing.
   - Added Windows NSIS installer artifact upload (`knox-installer-windows-amd64` from `target/release/bundle/nsis/*.exe`).
   - Removed all stale `knox-desktop` artifact and binary references.

2. **`.github/workflows/release.yml`:**
   - Matrix updated to unified archive naming (`knox-${{ github.ref_name }}-${{ matrix.target }}.tar.gz` / `.zip`).
   - Replaced redundant `cargo build -p knox` with `pnpm tauri build --target ${{ matrix.target }} ${{ matrix.bundle_args }}`.
   - Added build output verification step prior to packaging.
   - Retained Unix tarball packaging and SHA256 checksum generation for unified `knox` binary.
   - Retained Windows zip archive and SHA256 generation for `knox.exe`.
   - Added Windows NSIS installer packaging step: discovers `target/${{ matrix.target }}/release/bundle/nsis/*.exe`, generates its SHA256 checksum, and uploads the setup executable and checksum as release assets.
   - Removed all stale `knox-desktop` artifact, archive, and executable references.

3. **Local Path Verification:**
   - `target/release/knox.exe` confirmed present (19,417,600 bytes).
   - `target/release/bundle/nsis/Knox_1.2.1_x64-setup.exe` confirmed present (6,302,361 bytes).

### 12.3 Task C — Verification Results

| Check | Command | Exit Code | Result | Details |
|---|---|---|---|---|
| Rust Formatting Check | `cargo fmt --all -- --check` | 1 | FAIL (expected) | 108 hunks across 35 files; pending dedicated formatting commit |
| Cargo Check | `cargo check --workspace` | 0 | **PASS** | 0 warnings, 0.94s |
| Cargo Tests | `cargo test --workspace` | 0 | **PASS** | **168 passed, 0 failed** across 10 test suites |
| Cargo Clippy | `cargo clippy --workspace --all-targets -- -D warnings` | 0 | **PASS** | 0 warnings, 1.11s |
| Frontend Typecheck | `pnpm --dir ui typecheck` | 0 | **PASS** | `tsc --noEmit`, 0 errors |
| Frontend Build | `pnpm --dir ui build` | 0 | **PASS** | 2,282 modules transformed in 30.73s |
| Tauri Release & NSIS | `pnpm tauri build --bundles nsis` | 0 | **PASS** | Generated `knox.exe` & `Knox_1.2.1_x64-setup.exe` (6.01 MiB) in 5m47s |

---

*End of Phase 0 and Phase 0.5 reports.*
