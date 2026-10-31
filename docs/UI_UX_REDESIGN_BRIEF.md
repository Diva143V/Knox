# Knox UI/UX Redesign Brief — Phase 2B

**Status:** Blueprint only — awaiting approval before implementation  
**Branch:** `feature/ui-ux-redesign`  
**Date:** 2026-10-31  
**Constraint:** Rust owns the data. React owns the experience. No backend / Tauri IPC / schema changes.

---

## 0. Environment verification (Phase 2B gate)

| Check | Result |
|---|---|
| Current branch | `feature/ui-ux-redesign` |
| Working tree | Clean (`nothing to commit`) |
| `origin` remote | `https://github.com/chvnagarjunababu14-boop/Knox.git` |
| Repository type | Real Git clone (`.git` directory with `objects`, `refs`, `index`) |
| HEAD | `732084e` — docs(goal): v1.2.1 unified binary / onboarding |

**GSD (Get Shit Done) extension:** Not installed in this Cursor environment. Available Cursor extensions are limited (e.g. Remote SSH). No GSD commands, skills, or docs were found under `~/.cursor`, `~/.claude`, or `~/.agents`.

**Planning substitute:** This brief uses a GSD-style phased workflow (foundation → shell → screens → polish) with explicit acceptance criteria per phase. If GSD is installed later, map Phases 3–8 below to GSD plan/execute tickets without changing scope.

---

## 1. Product design direction

### What Knox is

Knox is a **local-first knowledge graph and persistent memory system** for AI-assisted development. It records agent actions, decisions, plans, and human edits into SQLite, exposes them via MCP, and visualizes causal relationships in a desktop workspace.

### Experience promise

Knox should feel like a **premium developer instrument** — closer to a focused IDE panel or ops console than a marketing dashboard:

- High information density without clutter
- Keyboard-first navigation
- Graph exploration as a first-class craft surface
- Calm, durable visual language (hours of use, not a demo splash)

### Architectural rule (non-negotiable)

> **Rust owns the data. React owns the experience.**

UI redesign may restyle, reorganize, and improve interaction flows. It must **not**:

- Change Tauri command names, payloads, or return shapes
- Alter Rust domain types, SQLite schema, or MCP tool contracts
- Remove existing features or screens
- Add new backend dependencies for UX improvements

### Inspiration (principles, not copies)

| Source | Borrow |
|---|---|
| Linear | Clear hierarchy, restrained chrome, professional density |
| Raycast | Command palette primacy, keyboard fluency |
| Modern dev tools | Predictable panels, monospace for data, sharp focus rings |
| Knowledge-graph apps | Spatial exploration, inspector coupling, filterable canvas |

Knox must remain **visually original** — not a Linear/Raycast clone.

### Anti-patterns (explicitly rejected)

- Generic “AI product” purple-on-white / purple-indigo gradient dashboards
- Excessive glassmorphism and backdrop-blur stacks
- Card farms with large radii and multi-layer shadows
- Oversized marketing headings inside the app shell
- Decorative motion with no state meaning
- Random accent colors per icon/widget
- Warm cream + terracotta “AI landing page” light theme

---

## 2. Recommended visual theme

### Direction evaluation

#### A — Premium dark workspace (recommended)

- Deep graphite surfaces with subtle hue (cool slate, not pure black)
- Restrained **teal / cyan-electric** accent (not purple)
- Sophisticated typography; borders carry structure more than shadows
- Graph canvas inherits the same dark plane

#### B — Modern light workspace

- Warm off-white canvas, white content surfaces
- Dark typography, royal-blue or teal accent
- Strong print-like clarity for tables/forms

### Recommendation: **Direction A — Premium dark workspace**

**Why this is stronger for Knox:**

1. **Session length** — Developers leave Knox open beside an IDE; dark chrome reduces glare and matches VS Code / terminal / Tauri desktop norms.
2. **Graph primacy** — Causal edges, minimap, and node semantics read cleaner on a deep field; light canvases force competing contrast strategies.
3. **Existing product identity** — Current UI is already dark; redesign should refine identity, not flip paradigm mid-product.
4. **Differentiation from generic AI SaaS** — Light “dashboard” aesthetics are overused; a graphite instrument look supports “local memory vault” positioning.
5. **Accent discipline** — Replacing the current indigo/purple scatter with a single teal accent creates a distinct Knox brand without looking like Linear’s purple or Raycast’s pink.

**Optional later:** A light theme token set may be defined as Phase 8+ polish, but Phase 3–7 ship dark-first only.

### Knox visual identity (working name)

**“Graphite Vault”**

- Surfaces: deep graphite stack (`bg` → `sidebar` → `elevated` → `popover`)
- Accent: single electric teal for primary actions, focus, and selection
- Semantic colors: success / warning / danger / plan — used only for meaning
- Structure: 1px borders + soft elevation; almost no decorative gradient
- Density: 12–13px UI text; mono for IDs, paths, counts

---

## 3. Design tokens

### 3.1 Color tokens (OKLCH — Tailwind v4 `@theme`)

Proposed CSS variables (replace current indigo-heavy primary):

| Token | Role | Proposed value |
|---|---|---|
| `--background` | App canvas | `oklch(0.145 0.008 250)` |
| `--sidebar` | Nav rail / titlebar | `oklch(0.160 0.009 250)` |
| `--card` | Elevated panel | `oklch(0.185 0.010 250)` |
| `--popover` | Menus / palette | `oklch(0.205 0.011 250)` |
| `--border` | Structural lines | `oklch(0.280 0.012 250)` |
| `--border-subtle` | Dividers | `oklch(0.240 0.010 250)` |
| `--foreground` | Primary text | `oklch(0.95 0.005 250)` |
| `--muted` | Quiet fills | `oklch(0.230 0.010 250)` |
| `--muted-foreground` | Secondary text | `oklch(0.62 0.015 250)` |
| `--primary` | Accent (teal) | `oklch(0.72 0.12 195)` |
| `--primary-foreground` | On-accent text | `oklch(0.16 0.02 250)` |
| `--primary-muted` | Selection wash | `oklch(0.72 0.12 195 / 0.15)` |
| `--success` | Healthy / current | `oklch(0.70 0.14 155)` |
| `--warning` | Caution / superseded | `oklch(0.78 0.14 85)` |
| `--danger` | Destructive | `oklch(0.65 0.18 25)` |
| `--plan` | Plan action type | `oklch(0.68 0.12 250)` |
| `--focus-ring` | Keyboard focus | `oklch(0.72 0.12 195 / 0.55)` |

**Actor / type colors:** Keep semantic mapping, but constrain to a **6-swatch palette** derived from tokens (no ad-hoc Tailwind `purple-400` / `indigo-400` scatter). Document swatches in a `semantics.ts` or CSS map during Phase 3.

### 3.2 Typography

| Role | Family | Size / weight | Notes |
|---|---|---|---|
| UI | Inter Variable (existing) | 12–13px / 450–600 | Keep self-hosted fonts |
| Display (rare) | Inter Variable | 16–18px / 600 | View titles only; never >18px in-app |
| Mono | JetBrains Mono (existing) | 11–12px / 400–500 | Paths, IDs, counts, snippets |
| Overline | Inter | 10px / 600 | Uppercase tracking `0.06em` section labels |

**Scale:** `10 / 11 / 12 / 13 / 14 / 16 / 18` — no larger display sizes inside the shell.

### 3.3 Spacing system

Base unit: **4px**. Preferred scale: `4, 8, 12, 16, 20, 24, 32, 40, 48`.

| Context | Padding |
|---|---|
| Compact controls | `px-2.5 py-1.5` |
| Panel body | `p-4` or `p-5` |
| Page content | `p-5` / `p-6` (max-width as today) |
| Sidebar item | `px-3 py-2` |

### 3.4 Border radius

| Token | Value | Use |
|---|---|---|
| `--radius-sm` | 4px | Inputs, badges, kbd |
| `--radius-md` | 6px | Buttons, rows |
| `--radius-lg` | 8px | Panels, dialogs |
| `--radius-xl` | 10px | Rare: onboarding modal only |

Avoid `rounded-2xl` / pill clusters.

### 3.5 Shadows

Prefer **border + background delta** over shadow. When needed:

| Level | Use |
|---|---|
| `shadow-sm` | Floating menus |
| `shadow-md` | Command palette / dialog |
| None | Cards, KPI tiles, inspector |

No `shadow-2xl` + blur stacks for toasts (simplify).

### 3.6 Iconography

- **Library:** Lucide React (already in use) — keep
- **Size:** 14–16px in chrome; 18–20px only for empty-state illustrations
- **Stroke:** Default Lucide weight; do not mix filled icons casually
- **Color:** Inherit `currentColor`; accent only when communicating status

### 3.7 Component states

| State | Spec |
|---|---|
| Default | Border + quiet fill |
| Hover | Raise foreground; subtle `bg-muted` or border brighten |
| Active / selected | `bg-primary-muted` + left accent bar **or** soft fill (choose one pattern app-wide) |
| Disabled | 50% opacity; `cursor-not-allowed` |
| Loading | Skeleton blocks matching layout; spinner only on buttons |
| Error | Danger border + inline message; toast for async failures |
| Focus-visible | 2px ring using `--focus-ring`; never remove without replacement |

### 3.8 Hover & focus principles

- Mouse hover ≠ focus style
- All interactive elements keyboard reachable
- Sidebar active: quiet fill + accent indicator (not solid primary flood)
- List rows: full-row hit target; selected row uses consistent selection token

### 3.9 Animation principles

- Duration: **100–160ms** for panels/toasts; **200ms** max for modal enter
- Easing: standard ease-out
- Allowed: inspector slide, palette fade, skeleton pulse, layout engine progress
- Forbidden: continuous decorative pulses on brand (replace status pulse with static success dot unless “live syncing”)
- Prefer CSS over JS animation libraries (no new deps)

---

## 4. Information architecture

### Current structure (preserve feature set)

```
AppShell
├── CustomTitlebar (brand, DB path, Ctrl+K, window controls)
├── Sidebar (8 views + SQLite status)
├── Main
│   ├── Header (breadcrumb, setup, refresh)
│   └── Router → 8 views
├── DetailInspector (selected action)
├── CommandPalette
├── OnboardingWizard
└── Toast
```

### Proposed IA refinements (no feature removal)

| Area | Change |
|---|---|
| Nav grouping | Group sidebar: **Explore** (Dashboard, Graph, Query, Stats) · **Capture** (Ingest, Scanner) · **Connect** (MCP, Workspace) |
| Labels | Shorten: “Knowledge Graph” → “Graph”; “Query Console” → “Search”; “MCP Server Config” → “MCP”; “Workspace & Hook” → “Workspace” (tooltips keep full names) |
| Header | Drop redundant long description on small widths; keep breadcrumb + contextual actions slot |
| Inspector | Shared for Dashboard, Search, Graph, Ingest preview — one detail contract |
| Command palette | Primary navigation + action search + quick ops (refresh, toggle current-only, open setup) |
| Empty → next step | Every empty state links to Ingest, Workspace, or MCP — not dead ends |

### Navigation model

- Sidebar primary
- Number keys `1–8` preserved
- `g` then letter sequences preserved
- `Ctrl+K` / `Cmd+K` command palette
- Hash routes `#/dashboard` … `#/workspace` preserved

---

## 5. Screen-by-screen redesign plan

### 5.1 Application shell

| Surface | Redesign intent |
|---|---|
| **Titlebar** | Quieter brand mark; static DB health indicator; centered command trigger with clearer hit area; Windows-native control spacing |
| **Sidebar** | Section labels; icon + short label; shortcut glyphs muted; active = left rail + soft fill (not full primary pill); footer status denser and scannable |
| **Header** | Compact breadcrumb; view-level actions move here when needed (filters stay in-view) |
| **Inspector** | Sticky header with type/actor badges; clearer sections (Summary, Files, Relations, Meta); Esc affordance visible |
| **Toast** | Compact, top-right under titlebar; success/error variants; no heavy blur |
| **Loading / empty / error** | Shared `EmptyState`, `ErrorState`, `Skeleton` primitives |

### 5.2 Dashboard

**Job:** Orient the user in &lt;3 seconds — health, recent memory, next useful action.

**Plan:**

- Replace marketing “Welcome to Knox” with **workspace status strip** (actions / sessions / projects / last activity)
- KPI row: tighten cards (less card chrome; more metric typography)
- Recent feed: denser `ActionRow`; keyboard-openable into inspector
- Diagnostics panel: monospace config facts; demote version marketing
- Empty graph CTA remains (Ingest / MCP) but visually quieter

### 5.3 Query / Search

**Job:** Fast FTS5 + structured filter retrieval.

**Plan:**

- Rename UX to **Search** (keep route id `query`)
- Sticky filter bar with clearer hierarchy: primary search → secondary filters → results meta
- Replace native `<select>` with consistent custom select styling (same IPC)
- Results list: monospace timestamps; stronger selected row; Enter opens inspector
- Empty: distinguish “no data yet” vs “no matches”

### 5.4 Knowledge Graph

**Job:** Explore causal structure; select nodes → inspector.

**Plan:**

- Unify toolbar with shell tokens (filters, layout engines, current-only, file nodes)
- Canvas background uses `--background` (remove hardcoded `#0b0f15` drift)
- Nodes/edges: refine ActionNode / FileNode / CausalEdge to Graphite Vault palette
- Controls / MiniMap restyle to match tokens (drop glassmorphic minimap)
- Empty canvas guidance → Ingest
- Preserve dagre / elk / force layouts and filters

### 5.5 Statistics

**Job:** Health and distribution at a glance.

**Plan:**

- Shared KPI + distribution patterns with Dashboard (DRY)
- Bars use semantic token colors only
- Trend chart (`RecentTrendChart`) restyled; no chart library unless already present
- Stale/superseded callout as first-class health signal

### 5.6 Transcript Ingestion

**Job:** Get memory into Knox with confidence.

**Plan:**

- Clearer dropzone states (idle / drag / ready / ingesting / success)
- Success report as metric strip + expandable preview (already good — refine chrome)
- Keep format detection badge; reduce decorative sparkle overload

### 5.7 Human Edit Scan

**Job:** Attribute staged human edits; manage pre-commit hook.

**Plan:**

- Path + scan as primary toolbar; hook status as secondary status card
- Results table: denser, sticky header, empty staged guidance
- Destructive uninstall remains clearly danger-styled

### 5.8 MCP Configuration

**Job:** Understand tools and copy editor snippets.

**Plan:**

- Split: connection status / copy snippets | tool catalog
- Tool cards → expandable rows (less nested card chrome)
- Category filters as quiet tabs/chips
- Preserve all 9 tools and schemas

### 5.9 Workspace

**Job:** Init `knox.md` + hook in a clear stepwise flow.

**Plan:**

- Stepper visual refinement (progress, completed checks)
- Path browser + init / hook actions aligned with Scan patterns
- Preview `knox.md` in a code panel with copy
- Errors inline near the failing step

### 5.10 Command palette

**Job:** Navigate and act without leaving keyboard.

**Plan:**

- Graphite popover; clearer group headers (Views / Actions / Recent)
- Selected item uses primary-muted (not loud indigo wash)
- Show shortcuts consistently
- Keep action search → select → inspector

### 5.11 Onboarding wizard

**Job:** First-run confidence: what Knox is, connect MCP, next step.

**Plan:**

- Modal uses `--radius-lg`; reduce sparkle
- Agent picker as selectable list (not heavy cards)
- Snippet block mono + copy
- Link into Workspace / Ingest when done
- Re-openable via Setup Guide (preserve)

---

## 6. Component inventory

### 6.1 Proposed reusable inventory

| Component | Purpose | Disposition |
|---|---|---|
| `AppShell` | Three-zone layout + overlays | **Improve** |
| `CustomTitlebar` | Frameless window chrome | **Improve** |
| `Sidebar` | Primary nav + status | **Improve** (grouping) |
| `Header` | Breadcrumb + global actions | **Improve** |
| `DetailInspector` | Right panel host | **Improve** |
| `Router` | Hash view switch | **Reuse** (no IA break) |
| `Button` | Variants: primary / secondary / ghost / danger | **Improve** (use everywhere) |
| `Input` | Text fields | **Improve** + adopt in views |
| `Select` *(new)* | Styled select | **Add** (visual only) |
| `Checkbox` / `Toggle` *(new)* | Filters | **Add** |
| `Tabs` *(new)* | MCP categories, optional | **Add** |
| `Dialog` / `Modal` *(new or extract)* | Onboarding host | **Improve** |
| `Tooltip` *(new)* | Shortcut / truncated path | **Add** |
| `Card` | Panel container | **Improve** — flatter |
| `KpiCard` | Metric display | **Improve** |
| `Badge` / `ActionBadge` / `ActorBadge` | Semantic chips | **Improve** (tokenized colors) |
| `ActionRow` | Compact action list item | **Improve** |
| `Table` *(new primitive)* | Scan results etc. | **Add** |
| `EmptyState` *(new)* | Guided empties | **Add** |
| `Skeleton` *(new)* | Loading placeholders | **Add** |
| `Toast` *(extract)* | Notifications | **Extract** from AppShell |
| `CommandPalette` | cmdk wrapper | **Improve** |
| `OnboardingWizard` | First-run | **Improve** |
| `KnoxGraphCanvas` + nodes/edges | Graph experience | **Improve** |
| `ActionDetail` | Inspector body | **Improve** |
| `ScrollArea` / `Separator` | Primitives | **Reuse** |
| `PageHeader` *(new)* | Title + description + actions | **Add** — replace duplicated h1 blocks |
| `FilterBar` *(new)* | Search/Graph shared patterns | **Add** |
| `MetricStrip` *(new)* | Ingest/scan success numbers | **Add** |
| `CodeBlock` *(new)* | MCP / knox.md snippets | **Add** |
| `StatusDot` *(new)* | DB / hook health | **Add** |

### 6.2 Reuse vs improve vs replace

| Existing | Verdict |
|---|---|
| Layout shell pieces | Improve visually; keep structure |
| `Button` / `Input` / `Card` | Improve tokens; **enforce adoption** (many views still use raw `<button>` / `<input>`) |
| Badges + `ActionRow` + `KpiCard` | Improve; centralize colors |
| Graph node/edge components | Improve styling; keep XYFlow contracts |
| View files | Restyle + extract shared chrome; **do not delete features** |
| Ad-hoc toast in `AppShell` | Extract to `Toast` / `Toaster` |
| Native selects / checkboxes | Replace visually with primitives |

---

## 7. Interaction specifications

### 7.1 Keyboard

| Shortcut | Behavior | Change? |
|---|---|---|
| `Ctrl/Cmd+K` | Toggle command palette | Keep |
| `1–8` | Switch views | Keep |
| `g` then `d/g/q/s/i/c/m/w` | View jump | Keep |
| `F5` | Refresh | Keep |
| `Esc` | Close palette → clear selection → blur input | Keep; document order |
| `↑/↓` in Search | Move selection | Keep; add `Enter` to focus inspector |
| Graph | Existing XYFlow keys | Keep |

**UX improvement:** Show a Shortcuts cheatsheet entry in command palette (`?` optional in Phase 7).

### 7.2 Search experience

- Autofocus search field on view enter
- Debounced search optional later (Phase 7) — **frontend-only**; still calls existing `queryActions`
- Visible active filter chips with one-click clear
- Result count + match context in meta bar

### 7.3 Graph exploration

- Click node → inspector; Esc clears
- Filters apply client-side to loaded actions (existing)
- Layout controls remain explicit (no surprise re-layout)
- Highlight search matches without new backend APIs

### 7.4 Filtering & sorting

- Search + Graph share actor/type/current-only vocabulary
- Stats remains aggregate (no sort UI unless free with existing data)
- Default sort stays backend/store order unless UI already sorts

### 7.5 Details inspection

- One inspector for all selection sources
- Closing inspector does not clear list highlight until Esc / click away policy is unified
- Files list clickable as copy-to-clipboard (UI-only)

### 7.6 Empty-state guidance

| Context | CTA |
|---|---|
| No actions | Ingest transcript / configure MCP |
| No search hits | Clear filters |
| No staged files | `git add` guidance |
| Graph empty | Same as no actions |
| Workspace uninitialized | Run init step |

### 7.7 Onboarding

- First launch opens wizard (existing store flag)
- Skippable; re-open via Setup Guide
- Ends with concrete next action (Workspace or Ingest)

### 7.8 Window / desktop usability

- Min window width: ensure sidebar + content usable (~900px); inspector collapses/hides below breakpoint (UI-only media query)
- Drag region preserved on titlebar
- No-drag on controls
- Scroll: only content panes scroll; shell chrome fixed

### 7.9 Accessibility

See §8.

---

## 8. Accessibility considerations

- **Contrast:** Graphite + teal tuned for WCAG AA on text and critical controls
- **Focus:** Visible `:focus-visible` rings on all interactive controls
- **Semantics:** Prefer buttons/links; add `aria-label` on icon-only controls (window buttons, refresh, close inspector)
- **Command palette:** Ensure cmdk listbox semantics remain intact
- **Graph:** Provide non-graph path (Search + Inspector) for the same data — already true; keep
- **Motion:** Respect `prefers-reduced-motion` (disable slide/fade)
- **Keyboard:** Full nav without mouse for shell + search + palette + inspector
- **Toasts:** `role="status"` / `role="alert"` for errors
- **Forms:** Labels associated with inputs (Scan/Workspace path fields)

---

## 9. Implementation phases (roadmap)

> Do **not** start implementation until this brief is approved.

### Phase 3 — Foundation and design system

**Goal:** Tokenized Graphite Vault theme + core primitives.

- Rewrite `globals.css` tokens (colors, radius, focus, reduced motion)
- Harden `Button`, `Input`, `Card`, `Badge`, `KpiCard`, `ActionRow`
- Add `EmptyState`, `Skeleton`, `Toast`, `PageHeader`, `StatusDot`
- Tokenize `ActionBadge` / `ActorBadge` color maps
- Story-free visual QA via existing Vite/Tauri UI

**Exit criteria:** App boots with new tokens; no IPC changes; typecheck passes.

### Phase 4 — Application shell and navigation

**Goal:** Titlebar, sidebar grouping, header, inspector chrome, toast, palette shell.

- Restyle `CustomTitlebar`, `Sidebar`, `Header`, `DetailInspector`, `AppShell`
- Extract toast
- Command palette visual pass (behavior unchanged)
- Responsive inspector collapse rule

**Exit criteria:** All 8 routes reachable; shortcuts work; no feature regressions in chrome.

### Phase 5 — Dashboard and core screens

**Goal:** Dashboard, Search, Stats visual/UX pass.

- Dashboard status-first layout
- QueryView → Search UX polish + shared filter styling
- StatsView shared metrics language

**Exit criteria:** KPI/search/stats flows work with existing store/API.

### Phase 6 — Knowledge graph experience

**Goal:** Canvas + nodes/edges + toolbar aligned to design system.

- Tokenize React Flow overrides
- Restyle `ActionNode`, `FileNode`, `CausalEdge`, toolbar
- Empty/loading states
- Inspector coupling polish

**Exit criteria:** Layout engines, filters, selection, minimap/controls all functional.

### Phase 7 — Remaining screens and interactions

**Goal:** Ingest, Scan, MCP, Workspace, Onboarding.

- Apply `PageHeader`, `CodeBlock`, `MetricStrip`, tables
- Wizard + onboarding modal polish
- Filter chips, shortcuts help entry, copy affordances
- Unified empty/error patterns

**Exit criteria:** Every existing action still available; copy/install/scan/ingest flows unchanged at IPC layer.

### Phase 8 — Polish, accessibility, regression testing

**Goal:** Ship-quality UI.

- AA contrast audit; focus order; reduced motion
- Keyboard regression checklist
- Visual consistency pass (no leftover indigo/purple utilities)
- Resize / min-width testing on Windows
- `pnpm typecheck`, UI smoke, existing `cargo test` (backend untouched but green)

**Exit criteria:** Acceptance checklist below fully checked.

---

## 10. Testing and acceptance criteria

### Functional (must not regress)

- [ ] All 8 views render and switch via sidebar, `1–8`, `g` sequences, hash routes, command palette
- [ ] Refresh (`F5` / buttons) still calls existing store refresh
- [ ] Search filters still call `KnoxAPI.queryActions` with same parameters
- [ ] Graph layouts (dagre/elk/force), filters, file nodes, current-only toggle work
- [ ] Inspector opens/closes; Esc behavior preserved
- [ ] Ingest file dialog + ingest report still work
- [ ] Scan staged + hook install/uninstall still work
- [ ] MCP snippets copy still work
- [ ] Workspace init + knox.md preview + hook steps still work
- [ ] Onboarding open/close/copy still work
- [ ] Toasts show success and error paths

### Visual / UX

- [ ] Single accent family (teal) — no random purple/indigo leftovers in chrome
- [ ] Consistent radius, spacing, and typography scale
- [ ] Sidebar active state is not a solid primary flood
- [ ] Empty states include a next action
- [ ] Graph canvas matches shell background tokens

### Accessibility

- [ ] Focus rings visible for keyboard users
- [ ] Icon-only buttons labeled
- [ ] `prefers-reduced-motion` honored
- [ ] Error toasts announced

### Non-goals / hard stops

- [ ] No Rust file changes
- [ ] No Tauri command contract changes
- [ ] No schema migrations
- [ ] No dependency installs unless explicitly approved later
- [ ] No feature removals

---

## 11. UX improvement summary (frontend-only)

| Area | Improvement |
|---|---|
| IA | Grouped sidebar; shorter labels; tooltips for full names |
| Keyboard | Keep all shortcuts; document in palette; Enter-to-inspect in Search |
| Search | Clearer filter hierarchy; empty-state differentiation; shared inputs |
| Graph | Tokenized canvas; quieter toolbar; better empty guidance |
| Filtering | Shared visual language across Search/Graph |
| Inspector | Stronger information sections; consistent open/close |
| Empty states | Always prescribe next step |
| Onboarding | Calmer modal; ends with CTA into Workspace/Ingest |
| A11y | Focus, labels, reduced motion, contrast |
| Desktop | Inspector collapse at narrow widths; stable drag regions |

---

## 12. Risks and dependencies

| Risk | Mitigation |
|---|---|
| Accidental IPC/type drift while editing views | Touch only `ui/`; leave `services/api.ts` contracts unchanged unless types already mirror backend |
| XYFlow style regressions | Isolate canvas CSS; test each layout engine after token change |
| Scope creep into new features | Strict “restyle + UX rearrange only” rule |
| Inconsistent adoption of primitives | Phase 3 primitives first; Phase 5–7 ban raw controls in touched files |
| GSD unavailable | Use this brief’s phases as the execution plan; optionally install GSD later |
| Light-theme requests mid-flight | Defer to post–Phase 8; tokens should allow a future `.light` map without rewiring components |
| Version string drift (`v1.2.0` vs `1.2.1` in UI) | UI copy cleanup allowed as polish; read from a single constant if one already exists — do not invent backend version APIs |

**Dependencies:** None new. Existing stack (React 19, Tailwind 4, Zustand, XYFlow, cmdk, Lucide, Tauri 2) is sufficient.

---

## 13. Files created

| File | Purpose |
|---|---|
| `docs/UI_UX_REDESIGN_BRIEF.md` | This blueprint (Phase 2B deliverable) |

**Not modified:** application source, Rust, Tauri, schemas, locks, git history.

---

## 14. Approval gate

Implementation (Phases 3–8) starts only after explicit approval of:

1. **Theme:** Graphite Vault dark + teal accent  
2. **IA tweaks:** Sidebar grouping + shorter labels  
3. **Phase order:** 3 → 8 as defined  
4. **Constraint:** UI-only; no backend/IPC changes  

**Awaiting your approval before any implementation.**
