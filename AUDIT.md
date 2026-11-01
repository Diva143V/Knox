# Knox — Development Baseline Audit

**Date:** 2026-11-01  
**Branch:** `feature/ui-ux-redesign` → tracks `fork/feature/ui-ux-redesign`  
**Commit:** `48369b3` — *feat(ui): implement overall settings window modal and universal shortcuts*  
**Working tree:** clean at audit start; `AUDIT.md` is the only new (untracked) file  
**Audit mode:** static architecture/code review. No application files changed, no commits, no pushes, no branch switch. Dependencies were not installed and no build/test command was executed (see §15).

---

## 1. Git baseline verification

| Check | Result |
|---|---|
| Current branch | `feature/ui-ux-redesign` |
| Tracking | `fork/feature/ui-ux-redesign` |
| HEAD | `48369b3` |
| `origin` | `https://github.com/Diva143V/Knox.git` |
| `fork` | `https://github.com/chvnagarjunababu14-boop/Knox.git` |
| Working tree | clean |
| Local `main` | `48369b3`, tracking `origin/main`, untouched |

No branch changes were made during the audit.

---

## 2. Executive summary

Knox is a mature Rust core with a visually ambitious but **partially wired** React shell.

- The Rust/MCP/CLI layer is substantial and well tested: **77 Rust files / ~12.8k lines / 134 test functions** across 7 integration suites.
- The desktop UI renders all **8 screens**, the Graphite Vault token system is real, and the Graphite Vault visual language is consistently applied in most places.
- However, the v1.2.1 “settings + universal shortcuts” work is largely a **presentation layer**: **25 of 26 user settings are persisted to `localStorage` and never read by the application.** Several high-visibility controls are therefore no-ops.
- The release pipeline still packages binary names that the current Cargo manifest cannot produce.
- Several “working” flows (DB relocation, ingest drag-and-drop, system theme, MCP command mode) are broken or only partially implemented.

### Top 7 findings

1. **Settings are write-only.** Only `theme` is consumed. Density, glow, startup view, graph defaults, MCP mode, guardrails, scanner, ingest, tray and notification settings do nothing. (§7.2)
2. **Database relocation is unsafe.** `update_config` changes the path and writes `config.json` but never reopens or migrates the SQLite connection; the UI reports success. (§9, B2)
3. **Release/CI artifacts are stale.** The only binary target is `knox` (from `src-tauri`), but both workflows reference `knox-desktop(.exe)`, and `cargo build -p knox` builds a library-only package. (§10, B3)
4. **Transcript drag-and-drop is broken in Tauri.** It relies on Electron’s non-standard `File.path`; Tauri v2 requires `onDragDropEvent`. (§9, B4)
5. **MCP command mode is split-brain.** Settings and MCP Studio store it in two different keys, and the “absolute path” snippet hardcodes `D:\Knox\...`. (§9, B5)
6. **System theme is not a real mode.** It applies once, has no OS listener, and disappears after reload; the titlebar switcher does not offer it. (§6, B6)
7. **Four design tokens are missing** while being used widely: `text-3xs`, `bg-accent`, `--info`, `--purple`. (§5, B7)

### What is genuinely working

- All 8 views load, route via hash, and share a consistent three-zone shell.
- Command palette (`Ctrl+K`), settings (`Ctrl+,`), quick thought (`Ctrl+Alt+N`), `1–8`, `G`-prefix navigation, `F5`, and `P` (tour peek).
- Graph canvas: React Flow, cards/Cosmos modes, four layouts, filters, search highlight, minimap, physics, legend, inspector.
- Query: FTS + actor/type/file/time filters, bounded results, shared inspector.
- Ingest: file/paste/Git ingestion with preview and report.
- Scan: staged-diff scan and pre-commit hook install/uninstall.
- MCP Studio: detect, inject, remove, custom apps, tool/resource/prompt docs, live logs.
- Workspace: 4-step setup, `knox.md` preview, hook management.
- Settings: 9 categories, live DB stats, integrity check, VACUUM, shortcut cheatsheet.
- Toasts with correct `role`/`aria-live`; root and view-level error boundaries.
- Tray + single-instance behavior; 134 Rust tests.

---

## 3. Architecture

| Layer | Technology | Location | Notes |
|---|---|---|---|
| UI | React 19, Vite 6, TypeScript 5.7, Tailwind v4, Zustand 5 | `ui/` | 62 files, ~15.8k lines |
| Graph | `@xyflow/react`, `d3-force`, Dagre, ELK | `ui/src/components/graph`, `hooks` | 4 layout engines |
| Primitives | cmdk, lucide-react, custom `Button`/`Badge`/`Input`/`Card` | `ui/src/components/ui` | No shadcn dependency at runtime |
| Desktop shell | Tauri v2 | `src-tauri/` | 35 registered IPC handlers |
| Core | Rust 2024, SQLite (WAL), petgraph, MCP stdio | `src/` | Library-only package |
| State | Single Zustand store | `ui/src/store/useKnoxStore.ts` | 428 lines |
| Routing | Hash router, no react-router | `ui/src/components/layout/Router.tsx` | `#/dashboard`, … |
| Build | Cargo workspace `[".", "src-tauri"]` | `Cargo.toml` | Single bin: `src-tauri` → `knox` |

**Data flow:** React view → `KnoxAPI` (`invoke`) → Tauri command → `AppState { db, config }` → SQLite → DTO → Zustand → view.

**Important build fact:** the root `knox` package is a **library** (`autobins = false`, no `[[bin]]`). The only executable is the `knox-desktop` package’s bin named **`knox`** (`src-tauri/Cargo.toml:10–12`). This matters for §10.

---

## 4. Screens and navigation

### 4.1 Routes

| Route | View | Size | Status | Notes |
|---|---|---|---|---|
| `#/dashboard` | `DashboardView.tsx` | 390 lines | Working | KPI cards, recent feed, diagnostics, open-doubts panel |
| `#/graph` | `KnoxGraphCanvas.tsx` | 958 lines | Working | Cards/Cosmos, ELK/Dagre/Force/Grid, filters, minimap |
| `#/query` | `QueryView.tsx` | 619 lines | Working, one bug | Facet filters + keyboard selection mismatch |
| `#/stats` | `StatsView.tsx` + `RecentTrendChart.tsx` | 230 + 238 | Working, data caveat | Distributions + 7-day SVG trend |
| `#/ingest` | `IngestView.tsx` | 928 lines | Partial | Paste/Git work; file drag-drop broken |
| `#/scanner` | `ScanView.tsx` | 332 lines | Partial | Scan/hook work; per-file attribution wrong |
| `#/mcp` | `McpView.tsx` | 1,797 lines | Working, split state | Detect/inject/remove/docs/logs |
| `#/workspace` | `WorkspaceView.tsx` | 819 lines | Working, a11y gaps | 4-step setup, `knox.md`, hook |

### 4.2 Overlays

| Overlay | File | Semantics |
|---|---|---|
| Command palette | `CommandPalette.tsx` | `role="dialog"`, `aria-modal`, `aria-label` |
| Settings | `modals/SettingsModal.tsx` | `role="dialog"`, `aria-modal`, `aria-labelledby` |
| Quick thought | `modals/QuickThoughtModal.tsx` | `role="dialog"`, autofocus textarea |
| Onboarding wizard | `OnboardingWizard.tsx` | `role="dialog"`, `aria-labelledby` |
| Tour | `OnboardingTourModal.tsx` | dialog + peek-pill variant |
| Detail inspector | `layout/DetailInspector.tsx` | `<aside aria-label>` |
| Workspace modals | `WorkspaceView.tsx:614`, `:743` | **No dialog semantics** |

### 4.3 Navigation model

- 3 groups: **Explore** (Dashboard, Graph, Query, Stats), **Capture** (Ingest, Scanner), **Connect** (MCP, Workspace).
- Responsive rail: `w-14` below `lg`, `w-56` at `lg+` (`Sidebar.tsx:47`).
- Shortcuts `1–8` and `G` + `D/G/Q/S/I/C/M/W`.
- Active state uses `aria-current="page"` and a teal rail.
- Sidebar footer shows SQLite status, counts, tour and settings. It does **not** show the KG path or MCP status required by `code.md:453`.

---

## 5. Graphite Vault design system

### 5.1 What exists

`ui/src/globals.css` defines a complete dark-first token system with a light counterpart:

- Surfaces: `background`, `sidebar`, `card`, `popover`, `border`, `border-subtle`.
- Text: `foreground`, `muted-foreground`.
- Semantic: `primary`, `success`, `warning`, `danger`, `plan`.
- Actor swatches: `swatch-a` … `swatch-f`.
- Effects: `focus-ring`, `overlay`, `shadow-sm`, `shadow-md`, scrollbar.
- Typography: Inter Variable + JetBrains Mono, custom `text-2xs` … `text-2xl` scale.
- Elevation: 1px borders + lightness steps (no large blur except palette).
- Reduced-motion media query at `globals.css:309–318`.
- Semantic maps in `lib/semantics.ts` for action types and actors.

### 5.2 What is missing or inconsistent

| Issue | Evidence | Impact |
|---|---|---|
| `--text-3xs` undefined but used 35× across 9 files | `globals.css:85–99`; `Sidebar.tsx:176`, `SettingsModal.tsx:257`, `DashboardView.tsx:162`, `WorkspaceView.tsx:244` | Metadata falls back to inherited size; hierarchy breaks |
| `--color-accent` undefined (6 uses in 4 files) | `QuickThoughtModal.tsx:186–187`, `DashboardView.tsx:171`, `IngestView.tsx:653`, `WorkspaceView.tsx:237,259` | `bg-accent` / `hover:bg-accent` render nothing |
| `--info` and `--purple` undefined | `KnoxGraphCanvas.tsx:99,101,914,918`; `CausalEdge.tsx:19,21` | Edges/legend silently use hardcoded hex fallbacks |
| Raw palette colors | `DashboardView.tsx:154–176` (amber), `IngestView.tsx:803` (indigo), `OnboardingTourModal.tsx:480` (cyan), `semantics.ts:11–14,72–73` (amber/cyan) | Light/dark consistency and token discipline broken |
| Hardcoded dark error screen | `ErrorBoundary.tsx:66–88` (`#0d1117`, `#161b22`, `#30363d`, `rose-*`, `indigo-600`) | Error UI ignores light theme |
| Startup loader is red/dark | `ui/index.html:63–93` (`#07070D`, `#FF073A`) | Clashes with teal Graphite Vault; dark flash in light mode |
| Doc palette drift | Docs: indigo `oklch(0.623 0.214 260)`; impl: teal `oklch(0.72 0.12 195)` | Design docs no longer describe the product |

### 5.3 Token drift vs. documentation

| Token | `v1.2.0.md` / `code.md` | Implemented dark | Implemented light |
|---|---|---|---|
| `--background` | `oklch(0.128 0.005 285)` | `oklch(0.145 0.008 250)` | `oklch(0.972 0.005 250)` |
| `--card` | `oklch(0.162 0.006 285)` | `oklch(0.185 0.01 250)` | `oklch(1 0 0)` |
| `--popover` | `oklch(0.180 0.006 285)` | `oklch(0.205 0.011 250)` | `oklch(1 0 0)` |
| `--sidebar` | `oklch(0.145 0.005 285)` | `oklch(0.16 0.009 250)` | `oklch(1 0 0)` |
| `--border` | `oklch(0.245 0.006 285)` | `oklch(0.28 0.012 250)` | `oklch(0.9 0.01 250)` |
| `--primary` | `oklch(0.623 0.214 260)` (indigo) | `oklch(0.72 0.12 195)` (teal) | `oklch(0.58 0.11 180)` |
| `--plan` | `oklch(0.620 0.185 295)` (violet) | `oklch(0.68 0.12 250)` (blue) | `oklch(0.5 0.12 250)` |

**Typography drift:** docs specify 13px root / 10px metadata (`v1.2.0.md:113`, `code.md:464`); implementation uses **15px root** and `--text-2xs: 0.8125rem` (~13px) (`globals.css:85–99,193`). This is a deliberate readability change (“Phase 9”) but is undocumented.

**Verdict:** the Graphite Vault system is real and mostly coherent, but it is **incomplete (missing tokens), undocumented (name appears only in code comments), and diverged from the written design spec**. It needs a canonical `DESIGN.md` and a token-completeness test.

---

## 6. Light/dark theme implementation and persistence

### 6.1 Mechanism

- `lib/theme.ts` defines `ThemeMode = "light" | "dark"`, key `knox_theme`, default dark.
- `index.html:9–29` applies a FOUC guard before CSS/JS.
- `main.tsx:9` applies the stored theme before render.
- `applyTheme` sets `data-theme`, `color-scheme`, and `.dark`/`.light` classes, then clears inline FOUC colors (`theme.ts:27–48`).
- `ThemeSwitcher` (titlebar) offers Light/Dark only.
- `SettingsModal` offers Dark/Light/System (`SettingsModal.tsx:343–357`).

### 6.2 Problems

1. **System mode is cosmetic.** `updateUserSettings` resolves `system` once via `matchMedia` and stores `theme` as `light` or `dark` (`useKnoxStore.ts:175–185`). There is no `matchMedia` change listener, so the app does not follow OS changes.
2. **Two sources of truth.** `knox_theme` stores the *applied* theme; `knox_user_settings_v1.theme` stores the *selected* mode (`system`). They can disagree after reload.
3. **Inconsistent control surface.** Titlebar has 2 options, Settings has 3.
4. **Dark flash in light mode.** `tauri.conf.json:28` hardcodes `backgroundColor: "#07070D"`, and the `index.html` loader is dark/red. Light-theme users see a dark flash on every launch.
5. **Light theme was not fully audited.** Hardcoded dark surfaces in `ErrorBoundary`, `QuickThoughtModal` (`bg-black/60`), `OnboardingTourModal` (`bg-black/20`), and fixed graph edge colors are the main risks.
6. **Contrast risk.** Light `--primary: oklch(0.58 0.11 180)` with `--primary-foreground: oklch(0.99 ...)` is likely below WCAG AA 4.5:1 for small text; needs measurement.

**Persistence:** works for light/dark, fails to preserve the *semantics* of “system”.

---

## 7. SettingsModal, universal shortcuts, titlebar

### 7.1 SettingsModal (1,271 lines, 9 categories)

Structure is good: split nav + scrollable panel, live DB telemetry, search, restore defaults, version badge.

| Category | Contents | Backend-wired? |
|---|---|---|
| Appearance | theme, density, glow, startup view | Only theme |
| Storage | DB path, browse/reveal, global mode, timeout, stats, integrity, VACUUM | Partially; relocation broken |
| Graph | mode, physics, layout, overlays, physics constants | **No** |
| AI & MCP | command mode, guardrails, digest | **No** |
| Git & Scanner | author, strict hook, diff depth, ignore globs | **No** |
| Ingestion | dedup mode, require preview | **No** |
| System | close-to-tray, notifications | **No** |
| Shortcuts | cheatsheet | Static |
| About | version, subsystem, links | Static |

**Settings search** filters only category labels/descriptions (`SettingsModal.tsx:136–142`), not individual settings — the placeholder “Search settings…” overpromises.

**No focus trap / initial focus / focus restore.** Tab can move into the app behind the modal.

### 7.2 Settings wiring matrix

| Setting | Stored | Consumed by | Status |
|---|---|---|---|
| `theme` | `knox_theme` + `knox_user_settings_v1` | `applyTheme` | Partial (system broken) |
| `uiDensity` | localStorage | — | **Dead** |
| `enableGlowFx` | localStorage | — | **Dead** |
| `defaultStartupView` | localStorage | — | **Dead** (store always starts dashboard/hash) |
| `defaultGraphMode` | localStorage | — | **Dead** (graph uses `knox_graph_view_mode`) |
| `defaultLivePhysics` | localStorage | — | **Dead** (graph uses own key) |
| `defaultLayoutEngine` | localStorage | — | **Dead** (`useGraphLayout` default) |
| `showFileNodes` | localStorage | — | **Dead** (graph local `true`) |
| `showLegend` | localStorage | — | **Dead** (graph local `true`) |
| `showEdgeLabels` | localStorage | — | **Dead** |
| `chargeStrength` | localStorage | — | **Dead** (hardcoded `-340/-450`) |
| `linkDistance` | localStorage | — | **Dead** (hardcoded `110/180`) |
| `collisionPadding` | localStorage | — | **Dead** (hardcoded `22/35`) |
| `mcpCommandMode` | localStorage | — | **Dead** (MCP uses `knox_mcp_command_mode`) |
| `guardrailsMode` | localStorage | — | **Dead** |
| `enableMarkdownDigest` | localStorage | — | **Dead** |
| `defaultAuthorName` | localStorage | — | **Dead** (backend uses `"human"`) |
| `scannerIgnorePatterns` | localStorage | — | **Dead** |
| `strictGitHook` | localStorage | — | **Dead** |
| `diffScanDepth` | localStorage | — | **Dead** |
| `ingestWatchPaths` | localStorage | — | **Dead + no UI** |
| `deduplicationMode` | localStorage | — | **Dead** |
| `requireIngestPreview` | localStorage | — | **Dead** (UI always previews) |
| `closeToTray` | localStorage | — | **Dead** (always hides to tray) |
| `startOnBoot` | localStorage | — | **Dead + no UI + no plugin** |
| `showDesktopNotifications` | localStorage | — | **Dead** (toasts always shown) |

Evidence: the only `userSettings` reads outside `types/knox.ts` and the store are in `SettingsModal.tsx` (grep results). Graph state is independent in `KnoxGraphCanvas.tsx:128–194`; MCP mode is independent in `McpView.tsx:576,595–598,713`.

### 7.3 Universal shortcuts

| Shortcut | Implemented in | Works? | Notes |
|---|---|---|---|
| `Ctrl+K` / `Cmd+K` | `CommandPalette.tsx:60–73` | Yes | Toggles palette |
| `Ctrl+,` | `useKeyboardShortcuts.ts:37–47` | Yes | Toggles settings |
| `Ctrl+Alt+N` | `Header.tsx:52–61` | Yes | Quick thought |
| `1–8` | `useKeyboardShortcuts.ts:115–132` | Yes | Fires while modals are open if focus is not an input |
| `G` + `D/G/Q/S/I/C/M/W` | `useKeyboardShortcuts.ts:73–113` | Yes | 1.2s window |
| `F5` | `useKeyboardShortcuts.ts:49–54` | Yes | Refreshes |
| `Esc` | hook + overlays | Partial | Settings → palette → action; **project inspector not closed**; Workspace modals not handled |
| `P` | `OnboardingTourModal.tsx:344–357` | Yes | Peek mode, tour only |
| `Ctrl+/` | — | **No** | Documented in `v1.2.0.md:157`; replaced by Settings tab |
| `Ctrl+Enter` | `QuickThoughtModal.tsx:83–90` | Yes | Submit thought |

`Esc` on the tour marks it completed permanently (`useTour.ts:85–87` + default `dontShowAgain = true`), which may be unintended.

### 7.4 Titlebar

- Frameless (`decorations: false`), custom drag region, project/global scope dropdown, palette button, theme switcher, window controls (`CustomTitlebar.tsx`).
- Close always hides to tray (`handle_window_event`, `lib.rs:96–102`); the `closeToTray` setting is ignored.
- `isMaximized` is queried once and after toggling; it is not updated when the user maximizes via drag/double-click/OS.
- Theme switcher is well-implemented (`role="menu"`, `aria-haspopup`, `aria-expanded`, `aria-checked`) but lacks “System”.
- Duplicate entry points: Settings (Header + Sidebar + `Ctrl+,`), Tour (Header + Sidebar + Settings About), Theme (titlebar + Settings).

---

## 8. Responsiveness and accessibility

### 8.1 Responsiveness

- Desktop-only window: `minWidth: 960`, `minHeight: 640` (`tauri.conf.json:22–23`).
- Sidebar collapses to a 56px rail below `lg` (1024px); labels hidden, `aria-label` retained.
- Inspector becomes an overlay below `xl` (1280px) with a scrim.
- Views use `sm/md/lg/xl` grids; Scan table has `overflow-x-auto`.
- MCP Studio splits into two columns only at `xl`; otherwise stacks.
- Settings modal is `90vh` capped at 720px with independent scroll regions.
- Header can get tight at 960px (badge + 4 labelled buttons), but truncation and hidden descriptions prevent overflow in practice.
- No support below 960px by design; acceptable for a desktop app, but the “responsive” claim should be scoped.

### 8.2 Accessibility

| Area | Status |
|---|---|
| Dialog roles | Good on palette, settings, thought, tour, wizard; **missing on both Workspace modals** |
| Focus trap | **None** on any overlay (no focus-trap library) |
| Focus restore | **None** |
| `focus-visible` | Present on `Button`, sidebar nav, theme menu, toast, palette; **missing on most custom pills/tabs/toolbar buttons** |
| Listbox | `QueryView` uses `role="listbox"`/`option` but the container is not focusable and has no `aria-activedescendant` |
| Live regions | Toasts use `role` + `aria-live`; view errors use `role="alert"` |
| Color-only meaning | Mostly avoided: badges include text labels; edges include labels |
| Reduced motion | Global CSS media query + loader query |
| Touch targets | Many 28px (`h-7`, `size-7`) controls; acceptable for mouse-first desktop, below 44px guidance |
| Contrast | Needs measurement, especially light-theme primary buttons and `text-warning` on light surfaces |

**Highest-impact a11y gaps:** no focus trap, incomplete `Esc` stack, missing dialog semantics on Workspace modals, and missing `focus-visible` on custom controls.

---

## 9. Bugs and incomplete features

### 9.1 Severity table

| ID | Sev | Area | Finding | Evidence |
|---|---|---|---|---|
| B1 | **P0** | Settings | 25/26 settings are write-only | `useKnoxStore.ts:165–187`; grep of `userSettings` |
| B2 | **P0** | Storage | DB relocation never reopens/migrates the DB | `src-tauri/src/commands/config.rs:32–56,75–80` |
| B3 | **P0** | Build/Release | Workflows reference non-existent `knox-desktop` binaries; `cargo build -p knox` is lib-only | `Cargo.toml:18–22`; `src-tauri/Cargo.toml:10–12`; `ci.yml:64–77,117–134`; `release.yml:25–48,90–110` |
| B4 | **P0** | Ingest | Drag-and-drop uses Electron-only `File.path` | `IngestView.tsx:184–197`; no `onDragDropEvent` anywhere |
| B5 | **P0** | MCP | Two command-mode stores + hardcoded `D:\Knox` | `McpView.tsx:576,595–598,713,938`; `SettingsModal.tsx:770–804` |
| B6 | **P1** | Theme | “System” applies once, no OS listener, lost on reload | `useKnoxStore.ts:175–185`; `theme.ts` |
| B7 | **P1** | Design system | `text-3xs`, `bg-accent`, `--info`, `--purple` undefined | `globals.css:44–100`; usages listed in §5.2 |
| B8 | **P1** | A11y | No focus trap; `Esc` doesn’t close project inspector; Workspace modals lack dialog semantics | `useKeyboardShortcuts.ts:57–71`; `WorkspaceView.tsx:614,743` |
| B9 | **P1** | Query | Keyboard selection uses `results`; rendering uses facet-filtered `displayedResults`; no pagination | `QueryView.tsx:65,137–154,537–545` |
| B10 | **P1** | Dashboard/Stats | Doubts, stale count, and 7-day trend are computed from a 350-action window, not the DB | `useKnoxStore.ts:338`; `DashboardView.tsx:152–191`; `StatsView.tsx:28–29`; `RecentTrendChart.tsx` |
| B11 | **P1** | Scan | Every staged file is labelled `human_write` if any action was recorded; author setting ignored | `ScanView.tsx:306–311`; `scan.rs:40–56` |
| B12 | **P1** | Desktop | Close-to-tray always on; no autostart/notifications/updater/window-state plugins | `lib.rs:96–102,109–122` |
| B13 | **P1** | Theme startup | Hardcoded dark window + red loader cause light-theme flash | `tauri.conf.json:28`; `index.html:63–93` |
| B14 | **P1** | Error handling | “Reset & Restart” clears **all** localStorage; error UI is dark-only | `ErrorBoundary.tsx:37–44,66–88` |
| B15 | **P1** | Onboarding | Wizard opens only *after* the tour has been completed; `Esc` dismissal is not persisted | `useKnoxStore.ts:154–155`; `OnboardingWizard.tsx:112–122,158–160` |
| B16 | **P1** | Security | `csp: null`; broad `shell:default`/`fs:default` permissions | `tauri.conf.json:14`; `capabilities/default.json:17–19` |
| B17 | **P1** | Config | `global_mode` toggle does not change the desktop DB; only CLI `init` reads it | `src/commands/init.rs:60`; `config.rs:42–44` |
| B18 | **P2** | Storage | “Reveal in Explorer” treats a non-existent DB path as a folder | `config.rs:99–106` |
| B19 | **P2** | Design system | Raw palette colors and doc drift | §5.2–5.3 |
| B20 | **P2** | Typography | 15px root vs documented 13px; `text-3xs` fallback | `globals.css:85–99,193` |
| B21 | **P2** | IA | Settings/tour/theme have duplicate entry points | Header, Sidebar, titlebar, Settings About |
| B22 | **P2** | Toasts | Timers are not cleared; rapid toasts can clear the wrong one | `useKnoxStore.ts:306–320` |
| B23 | **P2** | Data hygiene | Hardcoded `D:\Knox` fallbacks; temp-project filtering by name/path heuristic | `useKnoxStore.ts:260,364,344–354`; `McpView.tsx:938` |
| B24 | **P2** | Performance | All views bundled; 350-action refetch on every scope/filter change | `Router.tsx`; `useKnoxStore.ts:332–414` |
| B25 | **P2** | MCP | Custom apps live only in localStorage; deleting one does not remove injected config | `McpView.tsx:726–736,927–934` |
| B26 | **P2** | Settings UX | Search filters categories only | `SettingsModal.tsx:136–142` |
| B27 | **P2** | UX consistency | Native `window.confirm`/`window.prompt` mixed with custom modals | `SettingsModal.tsx:1248`; `IngestView.tsx:130–133`; `ScanView.tsx:92` |
| B28 | **P3** | Dead code | `save_config` registered but unused; mock data when not Tauri | `config.rs:58–71`; `api.ts:21–38` |
| B29 | **P3** | Desktop | No window-state persistence, auto-update, or autostart | `lib.rs` plugins list |
| B30 | **P3** | Ingest | Native picker is single-file only despite “Select File(s)” | `dialog.rs:22–24`; `IngestView.tsx:121` |

### 9.2 Detail on the P0 issues

#### B1 — Settings are write-only

`UserSettings` has 26 fields (`types/knox.ts:127–165`) with defaults (`:167–207`). `updateUserSettings` merges and writes `knox_user_settings_v1` (`useKnoxStore.ts:167–187`). A repository-wide grep shows the only consumers outside the store/types are in `SettingsModal.tsx` itself. Consequences:

- Turning off “Glow & Bloom FX” does nothing.
- Choosing a startup view does nothing.
- Changing graph defaults does nothing (the graph has its own `knox_graph_view_mode` / `knox_graph_live_physics` keys and hardcoded physics).
- Choosing “Absolute Path” in Settings does nothing (MCP Studio uses `knox_mcp_command_mode`).
- Guardrails, digest, scanner ignores, diff depth, dedup, preview, tray, and notifications are inert.

This is the single largest gap between the UI and the product promise.

#### B2 — DB relocation is unsafe

`update_config` mutates `cfg.kg_path` and calls `cfg.save_default()` (`config.rs:32–56`). It does **not**:

- lock/reopen `state.db`,
- migrate/copy the existing database,
- validate the target,
- update `get_db_stats` (which uses `cfg.kg_path` for file size while `state.db` still points at the old file).

The UI then shows “Knowledge graph database path updated” and calls `refreshData()` (`SettingsModal.tsx:146–160`), which still reads the old connection. After restart, `KnoxConfig::from_env()` loads the new path (`src/core/config.rs:37–42`), so the data appears to “move” only on restart — and if the target is a new empty file, the user silently loses visibility of the old graph. There is no confirmation or backup.

#### B3 — Release/CI binary mismatch

- Root package `knox`: `autobins = false`, no `[[bin]]` (`Cargo.toml:18–22`) → `cargo build --package knox` produces a library, not an executable.
- Only bin: `src-tauri` package, `[[bin]] name = "knox"` (`src-tauri/Cargo.toml:10–12`).
- CI expects `target/release/knox-desktop(.exe)` (`ci.yml:64–77`) and uploads it (`:123–134`).
- Release expects `.../release/knox-desktop` and `knox-desktop.exe` (`release.yml:25–48`), then `chmod`/`tar`/`Compress-Archive` them (`:90–110`).
- `pnpm tauri build` produces `knox`, not `knox-desktop`.

Static conclusion: the current workflows cannot produce the desktop artifact paths they reference. The release job will fail at packaging; the CI artifact upload will at best warn about missing files. This should be fixed before any release branch is cut.

#### B4 — Ingest drag-and-drop

`handleDrop` reads `(file as File & { path?: string }).path || file.name` (`IngestView.tsx:192`). `File.path` is an Electron extension; Tauri v2 webviews expose dropped files through `getCurrentWebview().onDragDropEvent(...)`, which is not used anywhere (grep confirms). Result: dropping a file queues a bare filename that the backend cannot resolve. The same screen also labels the picker “Select File(s)” but `open_file_dialog` uses `pick_file` (single selection).

#### B5 — MCP command mode split-brain

- Settings: `userSettings.mcpCommandMode` in `knox_user_settings_v1` (`SettingsModal.tsx:770–804`).
- MCP Studio: `commandMode` in `knox_mcp_command_mode` (`McpView.tsx:576,595–598,713`).
- The two controls can display different values simultaneously.
- The snippet preview hardcodes `D:\Knox\target\release\knox.exe` for “absolute” (`McpView.tsx:938`), which is wrong on every machine except the original developer’s.

---

## 10. Tests, build scripts, CI

### 10.1 Test inventory

| Suite | Count | Notes |
|---|---|---|
| Rust unit tests | included in 134 | Across core, storage, graph, ingest, MCP, commands |
| Rust integration tests | 7 files | `test_core`, `test_storage`, `test_graph`, `test_ingest`, `test_commands`, `test_mcp`, `test_e2e` |
| Frontend tests | **0** | No `*.test.*`, no Vitest/Jest/Testing Library, no `test` script |
| E2E | **0** | No Playwright/WebDriver/Tauri driver |
| Tauri command tests | Minimal | `test_update_config_flow`, icon/state tests; DB maintenance and relocation untested |

`cargo test --workspace` does include `src-tauri` (workspace members `[".", "src-tauri"]`), so Rust coverage is broader than the root crate alone.

### 10.2 Scripts

| Script | Command | Issue |
|---|---|---|
| `pnpm dev` | `pnpm --dir ui dev` | Fine |
| `pnpm typecheck` | `pnpm --dir ui typecheck` | Fine |
| `pnpm test` | `cargo test --workspace` | No UI tests |
| `pnpm lint` | `cargo clippy --workspace --all-targets` | No UI lint (ESLint absent) |
| `pnpm build` | `tauri build --bundles nsis` | Windows-only |
| `pnpm prebuild` | PowerShell process cleanup | **Breaks macOS/Linux local builds** |
| `pnpm build:portable` | `tauri build --no-bundle` | Fine on Windows |
| `ui build` | `tsc && vite build` | `noUnusedLocals`/`noUnusedParameters` enabled; static scan found no unused imports |

### 10.3 CI

- `lint`: rustfmt + clippy `-D warnings` + UI install + UI build.
- `test-and-build`: matrix Linux/macOS/Windows, UI build, `cargo test --workspace`, CLI build, Tauri build, artifact upload.
- Missing: UI lint, UI tests, E2E, `cargo audit`, bundle smoke test.
- Artifact names are stale (B3).
- CI triggers only on `main` pushes and PRs targeting `main`; feature-branch pushes get no CI until a PR exists.

### 10.4 Release

- Builds `cargo build --package knox` (library-only) and `pnpm tauri build` (the real binary).
- Packages both as if they were two different binaries.
- References `knox-desktop` paths that the manifest does not produce.
- No code signing, notarization, or updater manifest.

---

## 11. Comparison with the original redesign requirements

Source documents: `v1.2.0.md` (Phase 3 design system + view specs), `code.md` (architecture/UX contracts), `plan.md` (roadmap).

| Requirement | Status | Notes |
|---|---|---|
| Three-zone shell: sidebar / main / inspector | ✅ | `AppShell.tsx:25–57` |
| Sidebar `w-56` | ⚠️ | Responsive `w-14 lg:w-56` (reasonable enhancement) |
| Inspector `w-80` | ✅ | `xl:w-96 2xl:w-[420px]` |
| Hash router, no react-router | ✅ | `Router.tsx` |
| `Ctrl+K` palette | ✅ | `CommandPalette.tsx` |
| `Ctrl+/` shortcut help | ❌ | Replaced by Settings → Shortcuts |
| `G`-prefix + `1–8` navigation | ✅ | Hook + sidebar hints |
| Dark-first, dense 13px typography | ⚠️ | 15px root, `text-2xs` ~13px; `text-3xs` missing |
| Indigo primary `#6366f1` | ⚠️ | Teal `oklch(0.72 0.12 195)` |
| Semantic colors (write/plan/decide/debug) | ✅ | `semantics.ts` + badges |
| No raw hex in components | ❌ | ErrorBoundary, graph fallbacks, amber/cyan/indigo utilities |
| 4/6/8/12px radius scale | ⚠️ | 4/6/8/10px |
| Sidebar footer: KG path, row count, version | ⚠️ | Counts only; no KG path; version in titlebar |
| MCP status indicator | ❌ | Not present |
| Onboarding auto on first launch | ⚠️ | Tour first, wizard on next launch; Esc not persisted |
| Ingest `.jsonl` drag/drop + picker | ❌ | Drag/drop broken; picker single-file |
| Query file/actor/type/date/current filters | ✅ | Plus facets |
| Graph filters/search/minimap/layouts | ✅ | Exceeds spec (Cosmos, physics) |
| MCP tool docs + config copy | ✅ | Plus resources/prompts/live logs |
| Async loading/success/failure states | ✅ | Mostly; some silent catches |
| Empty states for every view | ✅ | `EmptyState` used broadly |
| Accessible dialogs/tables/lists | ⚠️ | Roles mostly present; no focus trap; listbox incomplete |
| No destructive resets | ❌ | ErrorBoundary clears all storage |

**Assessment:** the implementation has **exceeded** the visual/graph scope of the original plan (Cosmos mode, D3 physics, MCP Studio depth, 9-tab settings) while **falling short** on wiring, accessibility, and release integrity. The redesign is visually ahead of its documentation and functionally behind its controls.

---

## 12. Recommended improvements, ranked

### P0 — do first (correctness and release safety)

1. **Wire or remove every dead setting.** Prefer a typed `useUserSettings()` hook and a single settings context; either consume each value or remove the control. Add a test that every setting has a consumer.
2. **Fix DB lifecycle.** Reopen the SQLite connection on path change, validate the target, offer copy/migrate/backup, and make stats read from the active connection. Add an integration test.
3. **Fix the release pipeline.** Standardize on the single `knox` binary; update CI/release artifact names; stop building the library-only root package as a CLI; validate `--no-bundle` output.
4. **Implement Tauri-native drag-and-drop** (and multi-select) for ingest.
5. **Unify MCP command mode** into one persisted source; derive the absolute path from `current_exe` in the backend, never hardcode `D:\Knox`.
6. **Complete the token set** (`--text-3xs`, `--color-accent`, `--info`, `--purple`) and replace raw palette usage.

### P1 — next (UX integrity)

7. Implement a real focus trap + focus restore for every overlay; add an `Esc` stack that includes the project inspector and Workspace modals.
8. Make “System” theme a first-class mode with `matchMedia` subscription and a single persisted source; add it to the titlebar switcher.
9. Fix Query selection to index `displayedResults`; add pagination/“load more”.
10. Move dashboard/stats aggregations to backend queries (open doubts, stale count, 7-day trend) instead of the 350-action window.
11. Fix scan attribution per file and wire scanner settings.
12. Wire tray/close behavior, notifications, autostart, and window-state; add the corresponding plugins.
13. Remove the light-theme startup flash (theme-aware window background + neutral loader).
14. Make ErrorBoundary reset scoped (onboarding keys only) and theme-aware.
15. Fix onboarding ordering/persistence (wizard first or immediately after tour; persist dismissal).
16. Tighten CSP and reduce `shell`/`fs` capabilities to the minimum used.
17. Make `global_mode` meaningful for the desktop app or label it CLI-only.

### P2 — polish

18. Fix “Reveal in Explorer” for non-existent paths.
19. Replace raw amber/cyan/indigo/rose utilities with semantic tokens.
20. Align typography with docs or update docs to the 15px scale.
21. Remove duplicate settings/tour entry points; keep one primary surface.
22. Clear toast timers; use a single timeout ref.
23. Remove `D:\Knox` fallbacks and the temp-project name heuristic.
24. Add route-level code splitting; debounce/refetch intentionally.
25. Persist custom MCP apps in the backend and remove injected config on delete.
26. Make settings search match individual setting labels.
27. Replace `window.confirm`/`window.prompt` with themed dialogs.
28. Persist window size/position.

### P3 — hardening

29. Delete or use `save_config`; remove mock API paths from production bundles.
30. Add multi-file selection and folder ingestion.
31. Add auto-update and signing/notarization.
32. Write `DESIGN.md` and update `v1.2.0.md`/`code.md` to the shipped palette/typography.

---

## 13. Phased implementation roadmap

Each phase is independently shippable and should end with a green typecheck/build/test run.

### Phase 0 — Baseline and guardrails (0.5–1 day)

- Install dependencies; run `pnpm --dir ui typecheck`, `pnpm --dir ui build`, `cargo test --workspace`, `cargo clippy`.
- Add Vitest + React Testing Library + `test` script; add a minimal smoke test for the store and settings reducer.
- Add a UI CI job (typecheck + lint + tests).
- Fix CI/release artifact names (B3) and validate `pnpm build:portable`.
- Add a manual smoke checklist (launch, 8 routes, palette, settings, graph, ingest, MCP inject).

**Exit criteria:** all four commands pass locally and in CI; release workflow produces one `knox` artifact per OS.

### Phase 1 — Settings truth (3–5 days)

- Introduce `useUserSettings()` with typed selectors and one storage key.
- Wire: theme/system, density, startup view, graph defaults/physics/overlays, MCP command mode, guardrails/digest (or hide), scanner author/ignores/depth, dedup/preview, tray/notifications.
- Remove `startOnBoot`/`ingestWatchPaths` or implement them.
- Add a settings-consumer test/table.

**Exit criteria:** no control is inert; every setting has a visible effect; one source of truth per setting.

### Phase 2 — Database lifecycle and config integrity (2–3 days)

- Reopen `state.db` on path change; validate/open target; optional copy/migrate/backup.
- Read stats from the active connection; refresh actions after relocation.
- Add confirmation + rollback; fix `reveal_db_folder`.
- Define and test `global_mode` semantics for desktop.
- Add Tauri command tests for relocation/stats/integrity.

**Exit criteria:** relocating to an existing DB shows its data immediately; invalid targets fail safely; no data loss path.

### Phase 3 — Design system completion (3–4 days)

- Add missing tokens; define `--info`, `--purple`, `--color-accent`, `--text-3xs`.
- Replace raw palette utilities and graph fallbacks.
- Theme-aware window background + neutral loader.
- Light-theme pass: error screen, overlays, graph edges, contrast measurement.
- Write `DESIGN.md`; update docs to the shipped palette/type scale.

**Exit criteria:** no undefined token classes; no raw hex outside tokens; light theme reviewed at 960/1280/1440.

### Phase 4 — Accessibility and keyboard (3–4 days)

- Focus trap + restore for Settings, palette, thought, tour, wizard, Workspace modals.
- Central `Esc` stack (topmost overlay first); include project inspector.
- `focus-visible` on all custom interactive elements.
- Query listbox: focusable container, `aria-activedescendant`, roving tabindex.
- Dialog semantics for Workspace modals; `aria-live` for result counts.
- Keyboard test checklist.

**Exit criteria:** full keyboard-only pass; no focus escapes; Esc closes the topmost layer; axe/contrast checks clean.

### Phase 5 — Data correctness and performance (4–6 days)

- Backend aggregation endpoints for open doubts, stale count, 7-day trend, actor/type totals.
- Pagination/“load more” in Query; server-side filters.
- Fix Query selection index; persist filters consistently.
- Route-level code splitting; avoid full refetch on every toggle.
- Add loading/error states for aggregations.

**Exit criteria:** counts match the DB; trend covers all actions; Query keyboard and mouse agree; bundle split per route.

### Phase 6 — Ingest, Scan, MCP polish (4–6 days)

- Tauri drag-drop + multi-select + folder ingestion.
- Per-file scan attribution; wire scanner settings; respect diff depth/ignores.
- Persist custom MCP apps; remove injected config on delete; use backend `current_exe` for absolute paths.
- Unify MCP command mode with Settings.
- Add tests for MCP inject/remove and ingest preview.

**Exit criteria:** drop/pick/multi-ingest work; scan table is accurate; MCP config matches the running binary.

### Phase 7 — Packaging and release (2–3 days)

- One unified `knox` binary; correct artifacts and checksums.
- Window-state persistence, autostart, notifications, updater.
- CSP allowlist; minimal capabilities.
- Signing/notarization plan; cross-platform build scripts (remove PowerShell-only prebuild).
- Release smoke test on a clean VM.

**Exit criteria:** installable builds on all three OSes; no missing artifacts; update path verified.

---

## 14. Suggested definition of done for the redesign

- Every visible control changes behavior or is removed.
- No `text-3xs`, `bg-accent`, `--info`, `--purple` unresolved.
- Light and dark both pass a contrast check.
- Keyboard-only user can reach and dismiss every overlay; Esc always closes the topmost layer.
- Dashboard/Stats numbers reconcile with the DB.
- Ingest drop, picker, preview, and report work in a packaged build.
- Settings DB relocation is safe and test-covered.
- CI publishes the same `knox` binary it builds.
- A `DESIGN.md` and updated `v1.2.0.md`/`code.md` describe the shipped UI.

---

## 15. Method and limitations

- Reviewed 60+ source files across `ui/src`, `src-tauri/src`, `src/`, workflows, manifests, and the design/planning docs.
- Used targeted greps for token usage, `userSettings` consumers, ARIA, `focus-visible`, reduced motion, drag-drop APIs, and TODO markers (none).
- Counted 134 Rust test functions, 0 frontend test files.
- `node_modules/` and `target/` are absent, so no build, typecheck, or test was run. Build/release conclusions are static and should be confirmed in CI.
- No application files were modified; this report is the only new file.
- No commits, pushes, or branch changes were made. Working tree remains clean.

---

## Appendix A — IPC command inventory (35)

Registered in `src-tauri/src/lib.rs:124–160`:

`get_status`, `query_actions`, `get_action`, `get_graph_data`, `ingest_transcript`, `ingest_raw_transcript`, `preview_transcript`, `ingest_git_history`, `detect_local_transcripts`, `init_project`, `get_projects`, `install_hook`, `uninstall_hook`, `get_knox_md`, `get_hook_status`, `scan_staged`, `start_session`, `end_session`, `get_sessions`, `get_config`, `update_config`, `save_config`, `get_db_stats`, `run_db_integrity`, `run_db_vacuum`, `reveal_db_folder`, `open_file_dialog`, `open_folder_dialog`, `detect_mcp_configs`, `deep_scan_mcp_configs`, `inspect_custom_mcp_config`, `inject_mcp_config`, `remove_mcp_config`, `record_quick_thought`, `resolve_doubt`.

Notable gaps: no aggregation endpoint, no DB relocation/migration command, no multi-file ingest command, no notification/autostart/update commands.

---

## Appendix B — Quick file map

| Concern | File |
|---|---|
| Shell/layout | `ui/src/components/layout/AppShell.tsx` |
| Store | `ui/src/store/useKnoxStore.ts` |
| Theme | `ui/src/lib/theme.ts`, `ui/src/globals.css` |
| Tokens | `ui/src/globals.css:44–182` |
| Settings | `ui/src/components/modals/SettingsModal.tsx` |
| Shortcuts | `ui/src/hooks/useKeyboardShortcuts.ts` |
| Graph | `ui/src/components/graph/KnoxGraphCanvas.tsx` |
| Query | `ui/src/components/views/QueryView.tsx` |
| Ingest | `ui/src/components/views/IngestView.tsx` |
| Scan | `ui/src/components/views/ScanView.tsx` |
| MCP | `ui/src/components/views/McpView.tsx` |
| Workspace | `ui/src/components/views/WorkspaceView.tsx` |
| IPC API | `ui/src/services/api.ts` |
| Tauri config | `src-tauri/tauri.conf.json` |
| Tauri commands | `src-tauri/src/commands/*.rs` |
| Config lifecycle | `src-tauri/src/commands/config.rs`, `src/core/config.rs` |
| Build | `Cargo.toml`, `src-tauri/Cargo.toml`, `package.json`, `.github/workflows/*.yml` |
| Design docs | `v1.2.0.md`, `code.md`, `plan.md` |

---

*End of audit.*
