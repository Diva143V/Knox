# Knox v1.2.0 — Code Production Guide

> **This document defines how Knox code must be written. It is the source of truth for module boundaries, IPC contracts, frontend conventions, and verification. Every AI agent and human contributor MUST read it before changing Knox.**

---

## Non-Negotiable Architecture

Knox v1.2.0 is a consumer desktop application built from three deliberately separated parts:

```text
┌──────────────────────────────────────────────────────────────────┐
│ ui/ — React 19 + Vite + Tailwind v4 + shadcn/ui                  │
│ Owns presentation, interaction, client-side UI state, layout.    │
└──────────────────────────┬───────────────────────────────────────┘
                           │ typed Tauri invoke only
┌──────────────────────────▼───────────────────────────────────────┐
│ src-tauri/ — Tauri v2 application shell                          │
│ Owns IPC boundary, desktop capabilities, process/window wiring.  │
└──────────────────────────┬───────────────────────────────────────┘
                           │ typed Rust calls only
┌──────────────────────────▼───────────────────────────────────────┐
│ src/ — Knox Rust library and CLI                                 │
│ Owns domain data, storage, graph, ingest, commands, and MCP.     │
└──────────────────────────────────────────────────────────────────┘
```

The governing principle is: **Rust owns the data; React owns the experience.**

### Protected Core Rule

Layers 0–5 are stable domain code. Do not modify `src/core/`, `src/storage/`, `src/graph/`, `src/ingest/`, `src/commands/`, `src/mcp/`, or their integration tests while implementing desktop UI features unless the requested work explicitly requires a domain capability that cannot be exposed through the existing command API. Never move business logic into `src-tauri/` or `ui/` for convenience.

`src/gui/` is obsolete. Never add, restore, or reference `egui`, `eframe`, `rfd`, `handle_gui`, `Commands::Gui`, or `GuiArgs`.

---

## Workspace Layout

```text
Knox/
├── Cargo.toml                    # Workspace root; `.` and `src-tauri` members
├── src/                          # Knox library + slim CLI binary
│   ├── lib.rs                    # Public Rust library modules; no gui module
│   ├── main.rs                   # clap CLI dispatch only
│   ├── core/                     # Layer 0: types, ids, config, errors
│   ├── storage/                  # Layer 1: SQLite access and migrations
│   ├── graph/                    # Layer 2: read-only petgraph loading/traversal
│   ├── ingest/                   # Layer 2: transcript mapping
│   ├── commands/                 # Layer 3: domain operation orchestration
│   ├── mcp/                      # Layer 4: MCP transport and handlers
│   └── cli/                      # Layer 5: clap argument definitions
├── src-tauri/                    # Tauri v2 desktop shell
│   ├── Cargo.toml
│   ├── build.rs
│   ├── tauri.conf.json
│   ├── capabilities/default.json
│   └── src/
│       ├── main.rs               # Calls knox_desktop_lib::run()
│       ├── lib.rs                # Builder, plugins, AppState, command registration
│       └── commands/             # One typed IPC wrapper per domain
├── ui/                           # Vite React frontend
│   ├── package.json
│   ├── vite.config.ts
│   ├── public/fonts/             # Bundled Inter and JetBrains Mono
│   └── src/
│       ├── main.tsx
│       ├── App.tsx
│       ├── globals.css           # Tailwind v4 and design tokens
│       ├── services/api.ts       # Only frontend IPC boundary
│       ├── types/knox.ts         # Rust DTO mirrors
│       ├── store/useKnoxStore.ts # Zustand UI state
│       ├── hooks/
│       └── components/
│           ├── layout/
│           ├── graph/
│           ├── views/
│           └── ui/               # Vendored shadcn primitives/shared UI
├── tests/                        # Rust cross-module integration tests
└── fixtures/                     # Stable test inputs
```

### Ownership and Dependency Rules

| Area | May depend on | Must not depend on |
|---|---|---|
| `src/core` | External crates | Any Knox module |
| `src/storage` | `core` | Graph, ingest, commands, MCP, Tauri, UI |
| `src/graph`, `src/ingest` | `core`, `storage` | Commands, MCP, Tauri, UI |
| `src/commands` | Core, storage, graph, ingest | MCP, Tauri, UI, CLI |
| `src/mcp`, `src/cli` | Commands and lower layers | Tauri, UI |
| `src-tauri` | Public Knox library APIs | Private Knox internals, UI implementation |
| `ui` | `services/api.ts`, UI types/components | Rust source, SQLite, filesystem, shell APIs directly |

Additional rules:

- The Tauri shell is an adapter, never a second domain layer.
- React never reads the DB or invokes shell/filesystem APIs except through a reviewed Tauri command or approved plugin API.
- Tauri commands never contain raw SQL, transcript parsing, graph traversal rules, or duplicated command business logic.
- `src-tauri` must depend on the root `knox` library crate, not copy its code.
- `src/graph/` remains read-only; only storage owns SQLite connections and raw SQL.
- `mod.rs` files re-export modules only. Do not hide business logic there.

---

## Desktop Application Configuration

The root Cargo manifest is a workspace with `members = [".", "src-tauri"]` and `resolver = "2"`. The root `knox` package exposes both a library (`src/lib.rs`) and the CLI binary (`src/main.rs`). The desktop crate consumes the root library via `knox = { path = ".." }`.

### Required Tauri Shell Configuration

`src-tauri/tauri.conf.json` must describe a v2 application with:

| Setting | Required value or intent |
|---|---|
| Product | `Knox`, version `1.2.0`, identifier `dev.knox.desktop` |
| Frontend | `../ui/dist` for release and `http://localhost:5173` for development |
| Window | Default 1280×820; minimum 960×640; centered; background `#0d1117` |
| Titlebar | Custom frameless titlebar (`decorations: false`) with a React `data-tauri-drag-region` |
| Bundle | Active for all supported platform targets; icons from `src-tauri/icons/` |
| Security | Do not broaden CSP or capabilities without a specific need and review |

The system tray is part of the v1.2 desktop behavior: closing the window hides it to the tray while the MCP server can remain alive. Auto-updates are deferred; do not add an updater implementation as incidental work.

### Capabilities and Plugins

Use only the Tauri plugins specified for this app: dialog, filesystem, and shell. Their permissions must appear explicitly in `src-tauri/capabilities/default.json` alongside `core:default`. Add the narrowest available permission; do not replace scoped permissions with unrestricted access.

### Frontend Toolchain

The UI package is ESM and uses these product dependencies:

| Concern | Package |
|---|---|
| Desktop IPC | `@tauri-apps/api`, `@tauri-apps/plugin-dialog` |
| Rendering | `react`, `react-dom` |
| Graph | `@xyflow/react`, `d3-force`, `elkjs` |
| Command palette | `cmdk` |
| State | `zustand` |
| UI | `lucide-react`, `clsx`, `tailwind-merge`, vendored shadcn/ui primitives |
| Build/style | Vite, TypeScript, `@tailwindcss/vite`, Tailwind v4 |

`vite.config.ts` uses React and Tailwind plugins, maps `@` to `ui/src`, runs on port 5173 with `strictPort: true`, ignores `src-tauri` in watch mode, and builds for the Tauri WebView targets. Do not introduce a browser-only API or remote backend assumption.

---

## Rust Core Standards

### Layer Contracts

| Layer | Responsibility | Contract |
|---|---|---|
| Core | Data types, IDs, errors, config | No Knox-layer imports; no business logic. |
| Storage | SQLite schema, migrations, CRUD | The only DB gatekeeper. |
| Graph | Read-only graph loading and traversal | Never writes the DB. |
| Ingest | Transcript detection and mapping | Produces typed `KnoxAction` values. |
| Commands | Domain orchestration | Calls storage/graph/ingest; is the public domain operation boundary. |
| MCP | JSON-RPC transport | Deserializes, validates, calls commands, serializes. |
| CLI | Argument parsing and presentation | Dispatches only; no domain logic. |

### Rust Coding Rules

- Use `KnoxError` and `KnoxResult<T>` in library code. `anyhow` is allowed only at binary/application entry points for top-level presentation.
- Propagate expected failures with `?`. Do not `unwrap()` or `expect()` in production core code.
- Public functions need `///` documentation describing behavior, returned data, and failure modes.
- Use typed structs between layers. Deserialize JSON only at a transport boundary (MCP or Tauri), then pass typed values.
- Keep functions below 50 lines where practical. At more than five parameters, define an input struct.
- Default to private; make an item `pub` only when a higher layer needs it.
- Do not create `utils.rs` or `helpers.rs`. Put shared domain utilities in their proper core/domain module.
- Do not silently discard errors (`let _ = ...`). Propagate, intentionally handle, or log them.
- Avoid unnecessary `clone()`; borrow unless ownership is genuinely required.

### Persistence Rules

- No raw SQL outside `src/storage/`.
- Migrations are versioned; never edit an already-released migration to change production data shape.
- Knox history is append-only: do not add destructive deletes for actions. Use `is_current` and supersession links.
- Store UUIDs as text and timestamps as UTC RFC 3339 text.
- Rust storage tests use `KnoxDb::open_memory()` whenever filesystem behavior is not under test.

### Core Data Contract

These are domain-owned Rust concepts. Tauri and the UI mirror their serialized shape; neither redefines their semantics.

| Type | Required meaning |
|---|---|
| `KnoxAction` | One recorded action: ID, actor, free-form action type, summary, files, tags, confidence, timestamp, optional session/parent/supersedes links, and `is_current`. |
| `Session` | Continuous actor work block with optional project, start/end timestamps, and open/closed state. |
| `QueryFilter` | Optional file/project/session/actor/since filters, tags, current-only switch, and bounded limit. |
| `QueryResult` | Flat action list plus `total_count` and `has_more`. |
| `StatusReport` | Aggregate counts and distributions required by the desktop status/dashboard views. |
| `IngestReport` | Imported-action count, touched-file count, created-session count, and relevant timing/preview metadata. |
| `ScanReport` | Human-edit detection summary and associated action/file results. |

`ActionId`, `SessionId`, and `ProjectId` remain UUID-v4 string wrappers. They are not interchangeable raw strings inside the Rust core.

### Existing Domain Interfaces

Keep the module contracts explicit:

```rust
// Storage owns the connection and all schema work.
pub struct KnoxDb { /* private Connection */ }
pub fn insert_action(db: &KnoxDb, action: &KnoxAction) -> KnoxResult<ActionId>;
pub fn get_action(db: &KnoxDb, id: &ActionId) -> KnoxResult<Option<KnoxAction>>;
pub fn query_actions(db: &KnoxDb, filter: &QueryFilter) -> KnoxResult<QueryResult>;

// Graph loads and traverses persisted data; it does not mutate it.
pub fn load_subgraph(db: &KnoxDb, anchors: &[ActionId], max_depth: usize)
    -> KnoxResult<KnoxGraph>;
pub fn load_full_graph(db: &KnoxDb) -> KnoxResult<KnoxGraph>;

// Ingest detects and maps a transcript into typed domain actions.
pub trait TranscriptMapper: Send + Sync {
    fn tool_name(&self) -> &str;
    fn can_parse(&self, raw: &[u8]) -> bool;
    fn map(&self, raw: &[u8]) -> KnoxResult<Vec<KnoxAction>>;
}
```

---

---

## Tauri IPC Standards

`src-tauri` translates desktop requests into public Knox APIs. It may manage shared app state and plugins, but it must remain thin and domain-free.

### Command Organization

Use one command file per domain:

```text
src-tauri/src/commands/
├── status.rs
├── query.rs
├── graph.rs
├── ingest.rs
├── scan.rs
├── sessions.rs
├── workspace.rs
├── config.rs
└── mod.rs
```

Each command follows this shape:

1. Receive a serializable, typed UI input.
2. Acquire only the state required for the operation.
3. Call a public `knox` command/storage API.
4. Return a serializable typed DTO, converting `KnoxError` to a user-safe string at the IPC boundary.

```rust
#[tauri::command]
pub async fn get_status(
    state: tauri::State<'_, AppState>,
) -> Result<StatusReport, String> {
    let db = state.db.lock().await;
    knox::commands::cmd_status(&db).map_err(|error| error.to_string())
}
```

Rules:

- Register every command explicitly in `tauri::generate_handler!` in `src-tauri/src/lib.rs`.
- Put shared desktop state in `AppState`; use `Arc<tokio::sync::Mutex<KnoxDb>>` for the shared DB connection as specified by the application shell.
- Request the minimum capability/plugin permission required. Update `capabilities/default.json` when an IPC feature needs a new permission.
- Do not expose generic file, shell, SQL, or arbitrary-command execution endpoints.
- Keep `tauri.conf.json` and CSS background colors aligned (`#0d1117`) to prevent startup flashing.
- Desktop-only work belongs in Tauri commands/plugins, not React event handlers.

### IPC Contract Discipline

- Tauri command names are `snake_case`; UI wrapper methods are `camelCase`.
- Rust response structs and TypeScript interfaces must have matching serialized field names and nullability.
- Make changes in this order: Rust domain DTO/API → Tauri command → `ui/src/types/knox.ts` → `ui/src/services/api.ts` → view/hook.
- Never call `invoke()` outside `ui/src/services/api.ts`. This preserves one searchable IPC boundary.
- Return actionable, safe errors. Frontend code must surface async failures via the standard toast/error state; never swallow them.

### Required Desktop Command Surface

The following is the supported IPC inventory. Keep each mapping one-to-one; do not create a broad “do anything” command.

| Invoke name | Command module | Rust operation / purpose |
|---|---|---|
| `get_status` | `status` | `cmd_status`; dashboard/statistics aggregates |
| `query_actions` | `query` | Query actions with typed filters and bounded results |
| `get_action` | `query` | Retrieve one action for the inspector |
| `get_graph_data` | `graph` | Load graph-safe data for the React Flow canvas |
| `ingest_transcript` | `ingest` | `cmd_ingest` for a selected transcript path |
| `init_project` | `workspace` | `cmd_init` for the chosen workspace |
| `scan_staged` | `scan` | `cmd_scan` for staged human edits |
| `install_hook` | `workspace` | Install the project hook through the command layer |
| `uninstall_hook` | `workspace` | Remove the project hook through the command layer |
| `start_session` | `sessions` | Start a persisted session |
| `end_session` | `sessions` | End a persisted session |
| `get_sessions` | `sessions` | List open/recent sessions |
| `get_config` | `config` | Return safe configuration required by the UI |
| `open_file_dialog` | `dialog` | Native file selection only; never arbitrary file access |

If a requested UI control needs an operation absent from this table, first determine whether it can be derived from existing data. If not, add a narrowly named domain command, DTO, Tauri wrapper, frontend type, and API method together. Do not fake a working UI control or use a placeholder shell command.

### Tauri File Responsibilities

| File | Responsibility |
|---|---|
| `src-tauri/src/main.rs` | Minimal binary entry; calls `knox_desktop_lib::run()`. |
| `src-tauri/src/lib.rs` | Constructs `AppState`, initializes plugins/tray/window lifecycle, and registers commands. |
| `src-tauri/src/commands/mod.rs` | Declares command modules only. |
| `src-tauri/src/commands/*.rs` | One serializable desktop adapter domain per file. |
| `src-tauri/build.rs` | Only invokes `tauri_build::build()`. |
| `src-tauri/capabilities/default.json` | Explicit permission declaration, not runtime logic. |

---

## React Frontend Standards

### Foundation

Use React 19, TypeScript, Vite, Tailwind CSS v4, shadcn/ui primitives, Zustand, and `@xyflow/react` for the graph. Keep the app desktop-first; it does not require a web router or server API.

- Use the `@/` alias for `ui/src` imports.
- Use `type` imports for values used only in type position.
- Put all Knox DTO mirrors in `ui/src/types/knox.ts`; do not redefine them separately per component.
- Put all invoke wrappers in `ui/src/services/api.ts` as `KnoxAPI` methods.
- Zustand stores global UI state only: active view, loaded status, selected action, palette/modal visibility, and other cross-view state. Keep transient field input local to the relevant component.
- Prefer composition of shadcn primitives and shared Knox components over copy/pasted markup.
- Use Lucide icons; do not introduce an uncoordinated icon library.

### Component Boundaries

| Directory | Purpose |
|---|---|
| `components/layout/` | App shell, title/header, sidebar, hash router |
| `components/views/` | Route-level dashboard, query, stats, ingest, scan, MCP, workspace views |
| `components/graph/` | React Flow canvas, nodes, edges, inspector, graph-only controls |
| `components/ui/` | Vendored shadcn primitives and reusable Knox UI such as badges, rows, KPI cards |
| `hooks/` | Data lifecycle, keyboard map, graph layout; no generic dumping ground |
| `services/` | Typed IPC calls only |

Naming:

- React components: `PascalCase.tsx` and `PascalCase` exports.
- Hooks: `useSomething.ts` and `useSomething` exports.
- Non-component TypeScript modules: `camelCase.ts`.
- Tauri commands/Rust files: `snake_case`.

### TypeScript IPC Contract

`ui/src/types/knox.ts` mirrors Rust serialized DTOs. The canonical action shape is:

```ts
export interface KnoxAction {
  id: string;
  actor: string;
  action_type: string;
  summary: string;
  files: string[];
  tags: string[];
  confidence: number | null;
  timestamp: string;
  session_id: string | null;
  parent_action_id: string | null;
  supersedes: string | null;
  is_current: boolean;
}
```

`Session`, `StatusReport`, `QueryResult`, `ScanReport`, and `IngestReport` belong in this same file. Do not use `any`, broad index signatures, or UI-only substitutes for a known IPC shape.

`ui/src/services/api.ts` is the only invoke adapter. Its methods return typed promises and retain the Rust snake-case argument keys used by Tauri:

```ts
export const KnoxAPI = {
  getStatus: () => invoke<StatusReport>("get_status"),
  queryActions: (params: QueryActionsInput) =>
    invoke<QueryResult>("query_actions", params),
  ingestTranscript: (path: string) =>
    invoke<IngestReport>("ingest_transcript", { path }),
  scanStaged: (project: string) =>
    invoke<ScanReport>("scan_staged", { project }),
};
```

Keep TypeScript request-input interfaces adjacent to this API layer when they are only transport inputs. Promote a type to `types/knox.ts` only when it is a stable shared DTO.

### Required App Shell

The primary layout is a three-zone desktop shell:

```text
Sidebar (w-56) | Main content (flex-1) | Detail inspector (w-80, conditional)
```

The sidebar contains Dashboard, Graph, Query, Stats, Ingest, Scan, MCP, and Workspace navigation plus DB/MCP status. The inspector presents the selected action and must be driven from shared selection state so graph and query views behave consistently.

Use a hash-based desktop router unless a change explicitly introduces a router dependency. View navigation must be keyboard-accessible and represented in the command palette.

### Visual System

The UI is dark-first and dense: Linear/Raycast/Cursor quality, never generic dashboard styling.

- Base colors use the defined OKLCH tokens in `globals.css`: background, card, popover, sidebar, border, primary, success, warning, danger, plan, muted, foreground.
- Use elevation through color steps and 1px `border-border` strokes rather than large drop shadows. Command palette is the deliberate exception and may use a frosted backdrop.
- Use Inter Variable for UI text and JetBrains Mono for paths, IDs, JSON, and timestamps. Bundle fonts in `ui/public/fonts/`.
- Keep the compact scale: metadata around 10px, body 12–13px, headings around 14px.
- Use `focus-visible` states and sufficient contrast. Never rely on hue alone to communicate action state.
- Use the semantic action colors consistently: write/success green, plan violet, decide/warning amber, debug/danger red.
- Avoid arbitrary raw hex values in components; add/use semantic tokens instead.

#### Design Tokens

Define these values once in `ui/src/globals.css` and reference them through Tailwind’s inline theme mapping:

| Token | OKLCH | Role |
|---|---|---|
| `--background` | `oklch(0.128 0.005 285)` | Window floor |
| `--card` | `oklch(0.162 0.006 285)` | Raised panels and cards |
| `--popover` | `oklch(0.180 0.006 285)` | Menus, modals, popovers |
| `--sidebar` | `oklch(0.145 0.005 285)` | Navigation rail |
| `--border` | `oklch(0.245 0.006 285)` | Structural 1px lines |
| `--muted-foreground` | `oklch(0.440 0.010 285)` | Secondary text and hints |
| `--primary` | `oklch(0.623 0.214 260)` | Indigo selection, actions, focus |
| `--success` | `oklch(0.620 0.160 160)` | Write/success state |
| `--warning` | `oklch(0.740 0.180 80)` | Decide/warning state |
| `--danger` | `oklch(0.580 0.200 25)` | Debug/error state |
| `--plan` | `oklch(0.620 0.185 295)` | Plan actions |

Use 4px, 6px, 8px, and 12px as the standard radius scale. The only intentional visual depth levels are base background, card, popover, and backdrop-blurred overlay. Avoid oversized corner radii, gradients, and marketing-site whitespace.

### Async UX Requirements

Every invoke-driven interaction must handle all three states:

- loading: disabled relevant control plus a skeleton/spinner;
- success: update the owning state and show a concise toast when useful;
- failure: preserve recoverable input and show a human-readable error.

Provide intentional empty states for every data view. Do not leave blank panels or silently fail requests.

### Keyboard and Accessibility

- `Ctrl+K` / `Cmd+K` opens the command palette from any view.
- `G D`, `G G`, `G Q`, `G S`, `G I`, `G C`, `G M`, `G W` navigate to Dashboard, Graph, Query, Stats, Ingest, Scan, MCP, and Workspace.
- `Escape` closes the current palette/dialog; `F5` refreshes current view data.
- Use native semantic controls or accessible shadcn primitives. Tooltips alone never supply an accessible name.
- Support keyboard navigation in tables, list rows, filters, dialogs, and graph selection.

---

## View-Specific Implementation Contracts

| View | Required behavior |
|---|---|
| Dashboard | Status KPI cards, recent actions, diagnostics, and quick actions sourced through `KnoxAPI`. |
| Graph | React Flow canvas with custom action/file nodes, causal edges, filters, search highlight, minimap, controls, layout switcher, and selected-action inspector. |
| Query | Search plus file/actor/type/date/current filters, bounded results, and the shared action inspector. |
| Stats | Status-derived KPI cards, actor/type distributions, and inline-SVG seven-day trend. |
| Ingest | `.jsonl` drag/drop and native picker, format indication, disabled-until-selected ingest, report and preview. |
| Scan | Workspace path, staged-edit scan, detected-file results, and hook management. |
| MCP | Status, transport/config copy, tool documentation, and start/stop actions only when supported by an explicit backend command. |
| Workspace | Path setup, initialization, hook setup, and generated `knox.md` preview through supported commands. |

For the graph, keep custom nodes memoized. Use React Flow nodes/edges as render data, not as a second database. Layout selection is presentation behavior: D3 force, ELK hierarchy, and Dagre are allowed; domain relationship derivation remains Rust-owned.

### Dashboard and Stats Detail

Dashboard uses four top-level metrics: total actions, sessions, tracked projects, and last update. Below, render a recent-actions feed (latest 15 actions) and diagnostics (database path, version, hook status). Quick actions initiate only supported operations such as init, ingest, and MCP setup.

Stats repeats the KPI layer and adds actor/type horizontal distributions plus a seven-day action trend rendered as an inline SVG. Do not add a charting library for these simple charts. Distribution colors must reuse the shared actor/action badge mappings.

### Query and Detail Inspector Detail

The Query Console has a full-width search input and a compact filter row: file-prefix filter, actor and action-type multi-selects, date range, current-only switch, and a 20/50/100 limit. Search is explicit (`Enter` or Search button), results are bounded/virtualized or safely scrollable, and pagination honours `has_more`.

`ActionDetail` is shared by query and graph contexts. It shows full action metadata, files, tags, confidence, session/timestamp, and causal links. It never independently fetches or recomputes graph semantics when the selected action already supplies them.

### Ingest Detail

The ingest view accepts `.jsonl` via drag/drop and the native dialog. It displays filename, size, detected mapper format, an ingest button disabled until a valid input is selected, progress while invoking, and an `IngestReport` afterward. Show at most the first five parsed actions in the preview; never render an unbounded transcript in the UI.

### Knowledge Graph Detail

`KnoxGraphCanvas` fills the content zone and supplies `fitView`, pan/zoom, a minimap, built-in controls, filters, search highlighting, and layout selection.

- `ActionNode`: 180px wide, 8px radius, action-color border, type icon, actor badge, truncated summary, file-count chip, muted timestamp/session, top target handle, and bottom source handle.
- `FileNode`: compact path pill with a truncated path (about 30 characters) and connection count.
- `CausalEdge`: smooth-step relation with a small relationship label. Use dashed strokes for supersession and solid strokes for follows/targets.
- Layout selector: Force (D3), Hierarchical (ELK), and Dagre. Any layout calculation transforms only UI node positions.
- Inspector: node selection updates the global selected action and opens/refreshes the shared detail rail.

Target graph performance is smooth interaction at 200+ nodes. Memoize custom nodes and avoid recomputing all layouts on unrelated UI-state changes.

### Scan, MCP, and Workspace Detail

Scan accepts a project path, provides a native folder chooser, invokes the staged-edit scan, and reports detected files with diff statistics and recorded action association. Hook install/uninstall controls reflect actual backend status and must not optimistically claim success.

MCP presents the transport/status, an expandable documented tool list, a copyable connection configuration, and the `knox serve --stdio` command. Start/stop controls require explicit backend support; never invent a local process-management path in the UI.

Workspace is a four-step flow: select path, initialize, install hook, verify. It shows the generated `knox.md` preview only from backend data and retains path/error state until the user resolves it.

---

## Implementation Order

Build vertically in phases. Do not begin a phase that consumes an unstable lower layer, and keep each phase independently buildable.

| Phase | Deliverable | Required completion signal |
|---|---|---|
| 0 | Cargo workspace restructuring; remove legacy GUI; root becomes library + CLI | `cargo check` and `cargo test` succeed with no GUI references. |
| 1 | `src-tauri` crate, config, capabilities, `AppState`, and IPC command adapters | `cargo check -p knox-desktop` succeeds. |
| 2 | `ui/` Vite/React foundation, aliases, types, and typed API service | `pnpm --dir ui build` succeeds. |
| 3 | Tokens, fonts, vendored primitives, global Zustand store, titlebar and three-zone shell | `pnpm tauri dev` opens a dark native shell without white flash. |
| 4 | Dashboard and Stats populated from `get_status` | KPI cards and distributions show live data. |
| 5 | Query, Ingest, and shared detail inspector | Search/filter and ingest report work end-to-end. |
| 6 | React Flow graph canvas and layouts | Nodes, edges, selection, and ELK layout work. |
| 7 | Scan, MCP configuration, and Workspace/hook views | Commands return verified desktop results. |
| 8 | Command palette, keyboard map, motion/empty/error polish | `Ctrl+K` and navigation shortcuts work globally. |
| 9 | Icons, CI, release packaging, cross-platform validation | `pnpm tauri build` produces platform artifacts. |

### Expected Creation Sequence

For a clean implementation, create files in this dependency order:

1. Root workspace configuration; remove `src/gui`; remove old GUI exports and CLI dispatch.
2. Tauri manifest, build script, application configuration, capabilities, and empty command module tree.
3. Tauri status/query/graph/ingest/workspace/scan/sessions/config adapters, then app builder and binary entry.
4. UI package manifest, Vite/TypeScript configuration, HTML entrypoint, React entrypoint, types, and API service.
5. Tokens/fonts/store, then layout shell and vendored shadcn components.
6. Shared `KpiCard`, action/actor badges, and action rows before dashboard/stats.
7. Query, ingest, and action detail before graph-specific renderer work.
8. Graph canvas, nodes, edges, and graph-layout hook.
9. Scan, MCP, workspace, command palette, keyboard hook, icons, then CI/release work.

Never create the UI views first and backfill their contracts later. The typed contract and shell must exist before a view consumes them.

---

## Testing and Verification

Run the smallest relevant checks while developing, then run the broad checks appropriate to the change.

| Change | Minimum verification |
|---|---|
| Core/storage/graph/ingest/commands | `cargo test` |
| Tauri command or shell | `cargo check -p knox-desktop` and relevant Rust tests |
| Frontend TypeScript/UI | `pnpm --dir ui build` |
| Full desktop integration | `pnpm tauri dev` for manual verification; release changes use `pnpm tauri build` |
| Visual changes | Inspect all changed states: normal, hover, focus, loading, empty, error, and populated. |

Testing rules:

- Unit tests live at the bottom of the Rust file they test; cross-module tests live in `tests/`.
- Test public contracts rather than private implementation details.
- Use in-memory DBs for non-filesystem Rust tests.
- Add tests when changing a command, serialization contract, migration, or graph/ingest behavior.
- Frontend changes must at least type-check and build cleanly; avoid introducing unused exports or console errors.

### Manual Desktop Checkpoints

| Phase | Checkpoint | Pass criterion |
|---|---|---|
| 0 | Core workspace | The old GUI directory/dispatch are absent; Rust checks are clean. |
| 1 | IPC bridge | All registered commands compile against the root `knox` library. |
| 2 | Vite | The UI starts at `localhost:5173` and renders. |
| 3 | Native shell | Window opens with an immediate dark background, custom titlebar, and sidebar. |
| 4 | Dashboard | Four live KPIs and a populated recent-action feed. |
| 5 | Query/Ingest | Keyword/file filters work; valid JSONL imports report results. |
| 6 | Graph | Pan/zoom, node click, inspector, edges, and auto-layout function. |
| 7 | Operations | Scan and hook actions accurately report backend outcomes. |
| 8 | Keyboard polish | Palette is keyboard navigable; all listed global shortcuts work. |
| 9 | Packaging | Release build produces the intended installer/artifact for each target. |

Before a visual phase is complete, inspect normal, hover, focus-visible, disabled, loading, empty, error, and populated states. Confirm Inter is loaded instead of a system fallback, the command palette has its intended blur, and tables/lists can be navigated by keyboard.

---

## Change Checklist

Before finishing any Knox change, verify:

- [ ] The work respects the Rust core / Tauri shell / React UI boundary.
- [ ] No legacy `egui`, `eframe`, `rfd`, or `src/gui` code was introduced.
- [ ] All new UI-to-Rust communication is typed and flows through `KnoxAPI` and a named Tauri command.
- [ ] Rust and TypeScript serialized types agree.
- [ ] New permissions are minimal and declared in the Tauri capability manifest.
- [ ] Loading, empty, error, hover, and keyboard/focus states are intentional.
- [ ] Relevant Rust checks and/or `pnpm --dir ui build` passed.
- [ ] New files have one clear concern and live in the appropriate layer.

## Anti-Patterns — Do Not

| Anti-pattern | Required alternative |
|---|---|
| React calling `invoke` directly in arbitrary components | Add a typed method to `services/api.ts`. |
| UI querying SQLite, parsing transcripts, or calculating causal graphs | Add/expose a Rust command and consume its typed result. |
| Tauri command with 200 lines of business logic | Delegate to a public Knox command; extract narrow adapter code only. |
| Reintroducing the `src/gui/` tree or eframe dependencies | Implement the experience in `ui/` and bridge it with Tauri. |
| Raw colors and duplicated action badge styling | Use design tokens and shared UI components. |
| A generic `helpers.ts`, `utils.rs`, or catch-all component | Put code in its owning domain/module. |
| Silent async errors or `let _ =` error discards | Propagate/handle and surface a useful error. |
| `pub` everywhere or JSON passed between Rust layers | Preserve encapsulation and typed Rust interfaces. |
| Large visual change without checking focus/loading/empty/error states | Verify the complete interaction state set. |

---

*If code disagrees with this guide, update the code or deliberately revise this guide in the same change. The architecture boundary is not optional.*
