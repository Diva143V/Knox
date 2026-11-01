# Knox UI/UX Redesign — Final Report (Phase 9)

**Branch:** `feature/ui-ux-redesign`  
**Design system:** Graphite Vault (Dark) + Graphite Light  
**Date:** 2026-10-31  
**Scope:** Frontend UI/UX (`ui/`) + design documentation (`docs/`) only  

**Status:** Phase 9 + Phase 9B (native Tauri theme verification) complete. **Not committed / not pushed.** Awaiting approval.

---

## 1. Executive summary

Knox’s desktop React UI was redesigned end-to-end under the approved Graphite Vault brief: deep graphite surfaces, restrained teal accents, Inter + JetBrains Mono, border-led chrome, and shared primitives (`PageHeader`, `EmptyState`, `CodeBlock`, `Toast`, etc.).

All eight application views, shell chrome (titlebar, sidebar, header, inspector, command palette), and the onboarding wizard now share one visual language. Existing Tauri IPC contracts, Rust backend, SQLite schema, and MCP tool surfaces were **not** modified.

Phase 8 closed polish/QA; Phase 8B verified native Windows after SAC. Phase 9 adds complete light/dark themes, a titlebar appearance switcher with localStorage persistence, and a raised typography scale across the app.

---

## 2. Completed phases

| Phase | Goal | Outcome |
|-------|------|---------|
| 2B | Design brief | `docs/UI_UX_REDESIGN_BRIEF.md` approved |
| 3 | Design system / tokens / primitives | Graphite Vault tokens + shared UI components |
| 4 | Shell & navigation | Titlebar, sidebar groups, header, inspector, palette, toast |
| 5 | Dashboard / Search / Stats | Status-first dashboard, Query polish, Stats alignment |
| 6 | Knowledge graph | Canvas, nodes, edges, layouts, empty/loading |
| 7 | Ingest / Scan / MCP / Workspace / Onboarding | Remaining screens + onboarding hooks fix |
| 8 | Polish, regression, QA | Visual QA + Phase 8B native Windows |
| 9 | Light/Dark themes + typography | This addendum |

---

## 3. Final design system (Graphite Vault)

- **Surfaces:** Deep graphite backgrounds (`--background`, `--card`, `--sidebar`)
- **Accent:** Restrained teal primary (`--primary`) — no purple/indigo chrome leftovers
- **Typography:** Inter (UI), JetBrains Mono (paths, code, badges)
- **Shape:** Border-led cards; modest radii (`--radius-md` / `--radius-xl` for onboarding)
- **Motion:** `motion-safe:*` utilities + global `prefers-reduced-motion` kill-switch in `globals.css`
- **Focus:** Visible `--focus-ring` on interactive controls
- **Semantics:** Actor/action colors constrained via `ui/src/lib/semantics.ts`

---

## 4. Screens redesigned

1. **Dashboard** — KPI strip, recent activity, next actions  
2. **Knowledge Graph** — ELK / Dagre / Force / Grid, filters, legend, empty CTA  
3. **Query (Search)** — FTS filters, keyboard listbox, empty CTAs  
4. **Statistics** — KPI + actor/type distributions + 7-day trend  
5. **Ingest** — Antigravity-only honesty, drop zone, report/preview  
6. **Human Edit Scan** — path/browse/scan, hook controls, results table  
7. **MCP Configuration** — docs/copy only (not a live server manager), 9 tools  
8. **Workspace** — init / hook / knox.md preview + real `getConfig()` path  
9. **Onboarding Wizard** — welcome → optional workspace → MCP → ready  

Shell: Custom titlebar, Sidebar (Explore / Capture / Connect), Header, Detail Inspector, Command Palette, Toast.

---

## 5. Components changed

### Modified (tracked)
Shell, all views, graph nodes/edges/canvas, store toast variants, `globals.css`, command palette.

### New (untracked under `ui/`)
- `ui/src/lib/cn.ts`, `semantics.ts`, `navigation.ts`
- `ui/src/components/ui/{PageHeader,EmptyState,Skeleton,FilterBar,CodeBlock,Toast,index}.ts(x)`

### Phase 8 polish deltas
- Nav grouping aligned to brief: **Capture** = Ingest + Scan; **Connect** = MCP + Workspace  
- Scan/Workspace path fields: full-width input + button row (better at ≤1000px)  
- Query empty-no-data CTAs → Ingest / MCP  
- Dashboard CTA label “Human Edit Scan”  
- Removed duplicate Scan empty-state primary button  

---

## 6. Accessibility improvements

- Icon-only controls labeled (`aria-label` / `title`)
- Form labels associated with Scan/Workspace/Onboarding path inputs
- Toast: `role="status"` / `role="alert"` + assertive live region for errors
- Onboarding: `role="dialog"`, `aria-modal`, Esc dismiss
- Command palette: dialog semantics + Esc / backdrop dismiss
- Inspector: overlay scrim on narrow viewports; labeled close
- Focus-visible rings via tokens + Button/Input primitives
- Reduced-motion honored globally

---

## 7. Performance improvements

- Graph: selection updates avoid forcing full ELK relayout (Zustand in nodes; Phase 6)
- Shared primitives reduce one-off style churn
- No new npm dependencies added in this redesign

**Note:** Production JS bundle remains large (~2.2 MB) due to React Flow / graph stack — pre-existing class of cost, not introduced as a new dependency.

---

## 8. Regression test results

### Frontend automated
| Check | Result |
|-------|--------|
| `pnpm typecheck` | **Pass** |
| `pnpm build:ui` | **Pass** |
| Frontend unit tests in `ui/` | **None present** |

### Functional (Vite preview + mock / IPC stubs)
| Area | Result |
|------|--------|
| All 8 routes via hash / sidebar | Pass |
| Dashboard KPIs + recent actions | Pass (mock status) |
| Query results list + selection | Pass (mock) |
| Graph layouts ELK/Dagre/Force/Grid + zoom controls | Pass (mock actions) |
| Ingest Antigravity-only copy + empty state | Pass |
| Scan UI + hook status presentation | Pass (IPC stubbed in browser) |
| MCP 9 tools + copy affordances | Pass (clipboard may depend on browser perms) |
| Workspace stepper + config path | Pass |
| Onboarding open / Esc / reopen / shell remains | **Pass** (hooks bug fixed in Phase 7) |

### Native desktop IPC (file/folder pickers, real git scan, hooks)
**Not verified** — see §9 / §12 (Tauri launch blocked).

---

## 9. Rust validation results

| Check | Result |
|-------|--------|
| Diff touch `src/`, `src-tauri/`, `Cargo.toml`, `Cargo.lock` | **None** (backend protected) |
| `cargo fmt --all -- --check` | Reports pre-existing formatting drift in `src/cli/dispatch.rs` and `src-tauri/src/main.rs` — **not fixed** (would expand scope into Rust) |
| `cargo test --workspace` | **Blocked** — Windows Application Control (os error **4551**) prevents executing crate build scripts (`parking_lot_core`) |
| `cargo clippy --workspace --all-targets` | **Blocked** — same Application Control policy |
| `pnpm tauri dev` | **Blocked** — same policy during Rust compile |

These failures are **environment / policy**, not UI redesign regressions.

---

## 10. CodeRabbit review findings

**CodeRabbit extension / CLI:** Not installed or discoverable in this environment (no Cursor/VS Code extension match, no `coderabbit` binary, no MCP tools).

**Manual review substitutes (valid findings addressed):**

| Finding | Action |
|---------|--------|
| Onboarding `useEffect` after early `return null` crashed Vite root | Fixed in Phase 7 (hooks always run) |
| Workspace incorrectly grouped under Capture vs brief | Fixed in Phase 8 (`navigation.ts`) |
| Query empty “no data” lacked next-step CTAs | Fixed |
| Scan path controls stacked awkwardly on narrow widths | Fixed layout |
| Duplicate Scan staged CTA on empty state | Removed |
| Legacy purple/indigo utility classes in UI chrome | Cleared (grep clean aside from comments) |

Suggestions that would alter IPC/behavior or invent features were not applied.

---

## 11. Visual verification

### Method
**Vite production preview** (`http://127.0.0.1:4173`) with CDP viewport overrides.  
**Not** native Tauri window screenshots.

### Viewports exercised
- 1400×900 — primary screenshot set  
- 960×640 — sidebar collapses to icon rail; Workspace/Graph remain usable  

Also spot-checked conceptually for 1200×800 / 1000×700 (same breakpoints: `lg` sidebar labels, `xl` inspector dock).

### Screenshot artifacts
Stored under `docs/ui-ux-screenshots/` (Vite preview):

| File | Screen |
|------|--------|
| `p8-vite-dashboard-1400x900.png` | Dashboard |
| `p8-vite-graph-1400x900.png` | Knowledge Graph |
| `p8-vite-query-1400x900.png` | Query / Search |
| `p8-vite-stats-1400x900.png` | Statistics |
| `p8-vite-ingest-1400x900.png` | Ingest |
| `p8-vite-scan-1400x900.png` | Scan |
| `p8-vite-mcp-1400x900.png` | MCP |
| `p8-vite-workspace-1400x900.png` | Workspace |
| `p8-vite-onboarding-1400x900.png` | Onboarding |
| `p8-vite-workspace-960x640.png` | Workspace @ 960×640 |
| `p8-vite-graph-960x640.png` | Graph @ 960×640 |

---

## 12. Known limitations

1. **No native Tauri verification on this machine** — Application Control blocks Rust build-script execution (`os error 4551`). Titlebar drag, tray hide, native dialogs, and real IPC need a machine/policy that allows `cargo` build scripts (or a prebuilt `knox-desktop` binary).  
2. **Browser preview uses API mocks** when `__TAURI__` is absent — ingest/scan/hook success paths not exercised against SQLite.  
3. **Clipboard copy** depends on browser/Tauri permission context.  
4. **`cargo fmt --check`** would fail on pre-existing Rust import order (out of redesign scope).  
5. **Onboarding modal** at very short heights (~640) is scroll-constrained (`max-h-[90vh]`); usable but denser.  
6. **CodeRabbit** unavailable for automated PR-style review.

---

## 13. Files changed

### Modified (32 tracked files under `ui/`)
Includes views, layout, graph, UI primitives hardening, `globals.css`, `useKnoxStore.ts`, `CommandPalette.tsx`, `OnboardingWizard.tsx`.

### Untracked (to include in future commit)
- `docs/UI_UX_REDESIGN_BRIEF.md`
- `docs/FINAL_UI_UX_REDESIGN_REPORT.md` (this file)
- `docs/ui-ux-screenshots/*`
- `ui/src/lib/*`
- `ui/src/components/ui/{CodeBlock,EmptyState,FilterBar,PageHeader,Skeleton,Toast,index}.*`

### Explicitly untouched
- `src/**`, `src-tauri/**`, `Cargo.toml`, `Cargo.lock`, DB schema, MCP protocol Rust side

---

## 14. Git diff statistics

```
32 files changed, 3898 insertions(+), 2727 deletions(-)
```

Plus untracked `docs/` and new `ui/` primitives/`lib` (not counted in `--stat` until added).

Branch: `feature/ui-ux-redesign` — working tree dirty; **no commit performed**.

---

## 15. Final readiness assessment

| Criterion | Ready? |
|-----------|--------|
| Cohesive Graphite Vault UI across all screens | **Yes** |
| No backend / IPC / schema changes | **Yes** |
| `pnpm typecheck` + `pnpm build:ui` | **Yes** |
| Onboarding dismiss no longer blanks app | **Yes** |
| Brief acceptance checklist (visual + a11y chrome) | **Mostly yes** |
| Native Tauri + cargo test green on this host | **Yes (Phase 8B)** |
| CodeRabbit automated review | **No — unavailable** |
| Commit / PR | **Pending explicit approval** |

**Recommendation:** Redesign is ready for review/commit of UI + docs only. See Phase 8B addendum below.

---

## 16. Phase 8B addendum — Native Windows verification (2026-10-31)

Smart App Control disabled by user. Re-ran native build + Rust validation.

### Environment
- `rustc` / `cargo` 1.99.1  
- `node` v24.20.0  
- `pnpm` 12.16.1  
- Branch: `feature/ui-ux-redesign` (unchanged; no commit/push)

### Native desktop (`pnpm tauri dev`)
- **Launch:** Succeeded (~11m45s first compile). Window title `Knox 🛡️ — Knowledge Graph`.
- **Live data:** Real SQLite (`C:\Users\chvna\.knox\knox.db`) — Actions 18, Sessions 5, Projects 4.
- **Verified:** Custom titlebar + native min/max/close controls; sidebar Explore/Capture/Connect; Ctrl+K command palette; Detail Inspector on action select; onboarding first-run + Esc dismiss (shell remains); Ingest Windows file dialog (`Transcripts *.jsonl;*.json`); keyboard/sidebar navigation across Dashboard, Graph, Search, Statistics, Ingest, Scan, MCP, Workspace.
- **Partial / flaky under automation:** Folder Browse dialog click targeting; Setup-button reopen of onboarding; maximize click once hit wrong window (Chrome). Controls are present; not all automation paths confirmed.

### Rust validation
| Check | Result |
|-------|--------|
| `cargo fmt --all -- --check` | **Fail** — pre-existing import-order drift in `src/cli/dispatch.rs`, `src-tauri/src/main.rs` (not fixed; out of UI scope) |
| `cargo test --workspace` | **Pass** — 122 tests, 0 failed |
| `cargo clippy --workspace --all-targets` | **Pass** (exit 0, Finished) |

### Git protection
- Accidental Tauri regen of `src-tauri/gen/schemas/*.json` was **reverted**.
- Working tree remains UI + docs only. No intentional Rust/IPC edits.

---

**Phase 8B complete.** Phase 9 continued below.

---

## 17. Phase 9 — Light Mode, Dark Mode & Typography

### 17.1 Initial architecture review

| Area | Finding |
|------|---------|
| Theme architecture | Single `:root` Graphite Vault token set in `globals.css`; Tailwind v4 `@theme inline` maps CSS variables. No prior light theme. |
| Hardcoded colors | Few: `index.html` FOUC `#0d1117`, onboarding `bg-black/70`, close `hover:text-white`, scrollbar hover oklch. Components otherwise use token classes (`bg-card`, `text-muted-foreground`, etc.). |
| Typography | Root `font-size: 13px`; many `text-[10px]` / `text-[11px]` arbitrary sizes; page title via `.text-display` at ~1.125rem. |
| Persistence | Zustand + `localStorage` already used for onboarding (`knox_onboarding_completed`). No Tauri store plugin required. |
| Safety | Theme + typography can ship entirely in `ui/` with `data-theme` + `localStorage` — no Rust/IPC/schema changes. |

**Verdict:** Safe to implement without backend approval.

### 17.2 Theme architecture

- Document root uses `data-theme="dark" | "light"` (default dark).
- Token blocks: `:root, [data-theme="dark"]` and `[data-theme="light"]`.
- Helpers: `ui/src/lib/theme.ts` (`getStoredTheme`, `applyTheme`).
- Zustand: `theme` + `setTheme` in `useKnoxStore` (applies DOM + persists).
- Tailwind remains variable-driven; no scattered theme-specific hardcodes.

### 17.3 Light-mode palette (Graphite Light)

Starting hex refined to oklch tokens:

| Token | Role |
|-------|------|
| `--background` | `#F5F7FA`-class cool gray |
| `--sidebar` / `--card` / `--popover` | White surfaces |
| `--muted` | `#EDF1F5`-class secondary |
| `--foreground` | `#18212B`-class primary text |
| `--muted-foreground` | `#566273`-class secondary |
| `--border` | `#DCE3EA`-class |
| `--primary` / `--primary-hover` | Teal `#0D9488` / `#0F766E` range |
| Semantic + swatches | Darkened vs dark theme for WCAG contrast on light surfaces |
| `--overlay` | Soft graphite scrim (replaces `bg-black/70`) |

### 17.4 Dark-mode preservation

Approved Graphite Vault dark tokens preserved under `:root` / `[data-theme="dark"]` (deep graphite, restrained teal, existing semantics). No intentional redesign of dark surfaces beyond shared typography scale and tokenized overlay/scrollbar/shadows.

### 17.5 Titlebar appearance switcher

- `ThemeSwitcher` in `CustomTitlebar` (left of window controls, `no-drag`).
- Sun / Moon icons; dropdown: Light / Dark with checkmark + `menuitemradio`.
- Keyboard: focusable trigger, Escape closes, aria-expanded/controls.
- Compact command-palette trigger below `lg` so appearance control stays visible.

### 17.6 Theme persistence

- Key: `localStorage.knox_theme` (`light` | `dark`).
- Default: **dark**.
- FOUC guard: inline script in `ui/index.html` sets `data-theme` + interim bg/fg before CSS/JS.
- `main.tsx` calls `applyTheme(getStoredTheme())` once CSS loads (clears interim inline colors).
- Storage failures fall back to dark for the session.
- Verified: switch → reload retains theme (`data-theme` + stored value).

### 17.7 Typography improvements

| Change | Detail |
|--------|--------|
| Root | `13px` → **`15px`**, line-height 1.5 |
| Scale | `@theme` rem steps: 2xs≈12.2 · xs≈13.1 · sm≈14 · base≈15 · lg≈17 · xl≈19 · 2xl≈24 |
| `.text-display` | **~24px** page titles |
| `.text-overline` | Floor raised (no sub-12px overlines) |
| Arbitrary sizes | `text-[8–11px]` → `text-2xs` / `text-xs` / `text-sm` across views |
| Controls | Buttons `text-sm`; inputs `text-base` / `h-9`; sidebar nav `text-sm` |
| Graph | ActionNode width 240px, summary `text-sm`; FileNode `text-sm`; edge labels `text-2xs` |

Hierarchy preserved (display > section > card > body > meta).

### 17.8 Graph theme support

- Canvas / Background / Controls / MiniMap already use CSS variables — inherit both themes.
- Edge markers/strokes use `var(--primary|warning|plan)`.
- Light mode: white cards, darker borders via semantic tokens, readable node/edge labels.
- Dark mode: unchanged Graphite Vault node chrome.

### 17.9 Accessibility

- Focus ring tokenized per theme; ThemeSwitcher keyboard operable.
- Contrast-oriented light semantic colors; disabled opacity retained.
- `prefers-reduced-motion` unchanged.
- Overlay token for modal scrim.

### 17.10 Files modified (Phase 9 highlights)

**New:** `ui/src/lib/theme.ts`, `ui/src/components/ui/ThemeSwitcher.tsx`  

**Core:** `ui/src/globals.css`, `ui/index.html`, `ui/src/main.tsx`, `ui/src/store/useKnoxStore.ts`, `ui/src/components/layout/CustomTitlebar.tsx`, `ui/src/components/layout/Sidebar.tsx`  

**Primitives:** Button, Card, Input, KpiCard, PageHeader, EmptyState, Toast, index export  

**Views / graph / shell:** typography class bumps across Dashboard, Query, Stats, Graph (ActionNode/FileNode/CausalEdge/Canvas/ActionDetail), Ingest, Scan, MCP, Workspace, Onboarding, CommandPalette, Header, badges/rows/filters/code  

**Docs:** this report + `docs/ui-ux-screenshots/p9-*`

**Untouched:** Rust, Tauri IPC, DB schema, MCP protocol.

### 17.11 Test results

| Check | Result |
|-------|--------|
| `pnpm typecheck` | **Pass** |
| `pnpm build:ui` | **Pass** (rebuilt after Phase 9) |
| `cargo test --workspace` | **Pass** — 122 tests, 0 failed |
| Theme switch dark↔light | **Pass** (Vite preview + Appearance menu) |
| Navigation after switch | **Pass** (dashboard ↔ graph) |
| Reload persistence | **Pass** (`knox_theme=dark` after reload) |
| Startup FOUC script | **Present** (`index.html`); interim colors cleared by `applyTheme` |
| Graph readability both themes | **Pass** (screenshots) |
| Viewports 1400×900, 1200×800, 1000×700, 960×640 | **Exercised** via CDP metrics + screenshots |
| Native Tauri titlebar theme control | **Deferred to Phase 9B** → **Pass** (see §18) |
| Rust code changes | **None** |

### 17.12 Screenshots

Under `docs/ui-ux-screenshots/`:

- `p9-dark-dashboard-1400x900.png` / `p9-dark-dashboard-1400x900-b.png`
- `p9-light-dashboard-1400x900.png`
- `p9-dark-graph-1400x900.png` / `p9-light-graph-1400x900.png`
- `p9-dark-dashboard-1200x800.png`
- `p9-dark-dashboard-1000x700.png`
- `p9-light-dashboard-960x640.png`

### 17.13 Known issues

1. Browser automation screenshots sometimes crop the far-right titlebar chrome; Appearance control is confirmed via a11y (`aria-label="Appearance"`) and CDP bounding boxes — native Tauri re-check recommended after approval.
2. Vite preview uses mock API data (no Tauri); theme/typography behavior is frontend-complete.
3. Pre-existing `cargo fmt --check` drift in Rust (Phase 8B) left untouched.
4. First-run onboarding still uses `knox_onboarding_completed`; independent of theme key.

### 17.14 Concise summary

Phase 9 adds a complete **Graphite Light** theme beside preserved **Graphite Vault** dark, a titlebar **Appearance** switcher, **`localStorage` persistence** with FOUC prevention, and a **raised typography scale** (15px root, larger hierarchy) across shell, views, and graph — all within `ui/` + docs, no backend changes.

---

## 18. Phase 9B — Native Tauri desktop theme verification (2026-10-31)

**Runtime:** `pnpm tauri dev` → native `knox.exe` WebView2 window (not Chrome/Edge, not Vite preview).  
**Branch:** `feature/ui-ux-redesign` (uncommitted work preserved).  
**Vite preview on port 4173:** left running (HTTP 200); not used as evidence for this phase.  
**Backend:** no Rust / IPC / schema changes.

### 18.1 Native application launch

| Check | Result |
|-------|--------|
| `pnpm tauri dev` | **Pass** — Vite + cargo run; Knox window titled “Knox — Knowledge Graph” |
| Native vs browser | **Pass** — frameless Tauri (`decorations: false`), custom titlebar, process `knox.exe` |
| Compilation | **Pass** — no compile failure during this session |

### 18.2 Theme switcher (titlebar Appearance)

| Check | Result |
|-------|--------|
| Appearance control visible | **Pass** — sun/moon + chevron left of minimize |
| Opens Light / Dark menu | **Pass** |
| Select Light → immediate apply | **Pass** (titlebar, sidebar, content, overlays) |
| Select Dark → immediate apply | **Pass** |
| Menu closes after selection | **Pass** |
| No overlap with window controls | **Pass** — control sits left of min/max/close |
| Theme control is `no-drag` | **Pass** — clicks open menu (does not steal drag) |

### 18.3 Persistence across real process restarts

Exact sequence performed with **process kill / `pnpm tauri dev` relaunch** (not page reload):

1. Dark selected → switch to Light → close Knox process → relaunch → **Light persisted** (sun icon).  
2. Switch to Dark → close → relaunch → **Dark persisted** (moon icon).  

| Check | Result |
|-------|--------|
| Light survives restart | **Pass** |
| Dark survives restart | **Pass** |
| Wrong-theme flash / FOUC | **Pass** — no visible wrong-theme flash on relaunch |
| Unexpected reset to default | **Pass** — none observed |
| localStorage / init errors | **Pass** — none observed in UI |

### 18.4 Light Mode screen verification (native)

| Screen | Result | Notes |
|--------|--------|-------|
| Dashboard | **Pass** | KPIs 13/6/4, recent activity readable |
| Knowledge Graph | **Partial** | Shell/toolbar/legend OK; node density depends on layout/fit (also verified earlier in session) |
| Query | **Pass** | Filters + FTS results readable |
| Statistics | Spot-checked via Dark primarily; Light shell OK on Dashboard/Scan |
| Ingest | Spot-checked Dark primarily; Light shell consistent |
| Human Edit Scan | **Pass** | Browse / Scan staged / hook status readable |
| MCP Configuration | Spot-checked Dark primarily |
| Workspace | Spot-checked Dark primarily |
| Onboarding | **Pass** | Esc dismiss; Setup reopen; themed surfaces |
| Command palette | **Pass** | Ctrl+K overlay themed correctly |
| Detail inspector | **Pass** | Light inspector verified |
| Dialogs / dropdowns | **Pass** | Appearance menu; filter dropdowns |

No hardcoded dark surfaces observed on verified Light screens. Text contrast readable.

### 18.5 Dark Mode screen verification (native)

| Screen | Result | Notes |
|--------|--------|-------|
| Dashboard | **Pass** | Graphite Vault surfaces + live KPIs |
| Knowledge Graph | **Pass** | Toolbar, ELK/Dagre/Force/Grid, legend; nodes visible when layout populated |
| Query | **Pass** | 13 live actions listed |
| Statistics | **Pass** | KPI cards, 7-day trend, By Actor / By Type |
| Ingest | **Pass** | Drop zone + honesty callouts |
| Human Edit Scan | **Pass** | Path/Browse/Scan staged empty state |
| MCP Configuration | **Pass** | Code blocks + client tabs + tool filters |
| Workspace | **Pass** | DB path, stepper, Browse, “No knox.md” |
| Onboarding | **Pass** | Welcome step themed |
| Command palette | **Pass** | |

No hardcoded white/light surfaces observed on verified Dark screens.

### 18.6 Typography verification

Target root **15px** confirmed readable in native WebView2:

- Nav labels, page headings, KPI metrics, Query results — **Pass**  
- Graph toolbar / legend / inspector metadata — **Pass**  
- MCP code blocks / Workspace paths — **Pass**  
- Dialogs / tooltips — **Pass** (tooltip contrast noted as dark tooltip on Light chrome — intentional/existing)  
- No unwanted horizontal scroll or clipped primary labels at tested sizes — **Pass**

### 18.7 Window size testing (native)

| Size | Theme | Result |
|------|-------|--------|
| ~1400×900 | Light + Dark | **Pass** — primary verification size |
| 1200×800 | Light | **Pass** — sidebar + KPIs hold |
| 1000×700 | — | **Partial** — not resized to exact 1000×700 this pass; covered by 960–1200 band |
| 960×640 (min) | Light | **Pass** — compact but readable; no broken chrome |
| Maximized | Light | **Pass** — restore/maximize control works |

### 18.8 IPC and SQLite verification

| Check | Result |
|-------|--------|
| Live DB path shown | **Pass** — `C:\Users\chvna\.knox\knox.db` |
| Sidebar counts | **Pass** — Actions **13**, Sessions **6**, Projects **4** |
| Dashboard / Query / Statistics data | **Pass** — matches live SQLite (not Vite mocks) |
| Workspace config path | **Pass** — global mode + db path |
| Rust / IPC / schema modified | **None** |

### 18.9 Native controls verification

| Control | Result |
|---------|--------|
| Native/custom titlebar | **Pass** |
| Theme dropdown | **Pass** |
| Window drag | **Partial** — titlebar drag region present; not exhaustively exercised every session |
| Minimize | **Partial** — control present; not every click path re-proven this wrap-up |
| Maximize / restore | **Pass** |
| Close / hide-to-tray | **Partial** — close can hide Tauri window (`Vis=False`) while process stays alive; restore via ShowWindow on `Tauri Window` HWND worked |
| File picker | **Not opened** this pass (Select file / Browse buttons present on Ingest/Scan/Workspace) |
| Folder picker | **Not opened** this pass (Browse present) |
| Clipboard | **Not exercised** (MCP Copy buttons present) |
| Navigation / palette / inspector / onboarding | **Pass** |

### 18.10 Screenshots (native only)

Directory: `docs/ui-ux-screenshots/native-phase9b/`  
These are **native Tauri** captures — distinct from Phase 9 Vite preview shots under `docs/ui-ux-screenshots/` root.

| File | Content |
|------|---------|
| `01-light-dashboard.webp` | Light Dashboard |
| `02-dark-dashboard.webp` | Dark Dashboard |
| `03-dark-query.webp` | Dark Query |
| `04-dark-statistics.webp` | Dark Statistics |
| `05-dark-ingest.webp` | Dark Ingest |
| `06-light-dashboard-1200x800.webp` | Light Dashboard @ 1200×800 |
| `07-light-dashboard-960x640.webp` | Light Dashboard @ 960×640 |
| `08-light-dashboard-maximized.webp` | Light Dashboard maximized |
| `09-light-after-restart.webp` | Light after process restart |
| `10-dark-after-restart.webp` | Dark after process restart |
| `11-light-dashboard-inspector.webp` | Light + inspector |
| `12-light-query.webp` | Light Query |

Additional Graph / titlebar-menu frames were observed live; not every intermediate capture was retained as a named file.

### 18.11 Failed or partially tested items

1. Exact **1000×700** resize not dedicated.  
2. **File / folder pickers** and **clipboard Copy** not opened end-to-end.  
3. **Minimize** and full **drag** interaction only partially proven.  
4. **Close → hide-to-tray** behavior can leave the Tauri HWND hidden while `knox.exe` remains; automation must restore the `Tauri Window` class HWND (not the empty-title Tao helper window).  
5. Dedicated named screenshots for Light Graph, Dark Graph, and open titlebar menus are thinner than the live verification coverage.  
6. Toast notifications not deliberately triggered this pass.

### 18.12 Remaining issues

1. Graph canvas may briefly show empty layout until Fit / layout settle — theme chrome still correct.  
2. Pre-existing Rust `cargo fmt --check` drift untouched.  
3. Onboarding still independent of theme key (`knox_onboarding_completed`).  
4. Writing screenshots under `docs/` can trigger Tauri’s full-project file watcher rebuild — prefer staging outside the repo during live `tauri dev`.

### 18.13 Concise Phase 9B verdict

Native Windows Knox (`pnpm tauri dev`) confirms **Light / Dark switching**, **localStorage persistence across real restarts**, **15px typography**, **live SQLite IPC**, and **readable chrome on all eight screens** in Dark (with Light fully proven on Dashboard, Query, Scan, shell, overlays). Window sizing holds at 1400×900, 1200×800, 960×640, and maximized. Remaining gaps are picker/clipboard automation and a few named screenshot slots — not theme-system failures.

---

**STOP — Phase 9B verification complete. Waiting for approval before any commit, push, or PR.**
