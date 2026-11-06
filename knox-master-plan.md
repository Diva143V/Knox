# Forty-six days, eight workstreams, one order

> Master plan · three plans merged · v1

The work order, the edge addendum and the model layer plan merged into a single sequence, with the file map for each workstream, the evidence that put it there, and the gates that stop the next one starting.

**Baseline** v2.0.0 · 6 tools · schema v8 · 230 tests · **Foundation** 16 days · **With models** 32–43 · with capture 35–46 · **New features** one, at the end

> **How to read this, and how big it is**
>
> Sixteen sections, about **3,300 words** — roughly **eight printed pages**, or twenty screens of scrolling once the tables are counted. Seventeen tables, one dependency diagram, one task index. Section 02 is the map of what changes where; section 03 is how each workstream earned its place; 05–12 are the workstreams in execution order; 13–15 are the calendar, the gates and the risks. If you read only three things, read 02, 04 and 14.

## 01 · What was merged, and how

Three documents, one dependency order. The merge is not concatenation — two tasks collapsed, one phase was reordered, and one plan was pushed behind a gate it originally ignored.

| Source | Contribution | How it changed in the merge |
|---|---|---|
| Work order (12 days) | Phases 0–6: truth, graph, trust, scale, identity, measurement, capture | Phase 1 absorbed the edge addendum; the model layer was appended as H rather than left parallel |
| Edge addendum (3 days) | T1.6–T1.9, T2.5, T5.5: labels, refs, direction, rejection log, promotion | Merged into Phase 1 and Phase 2 so the schema is touched once, not twice |
| Model layer plan v3 (29 days) | Five slots, two adapters, `knox_ask`, embeddings, extractor | Phase 0 of that plan *was* measurement — now Workstream F, so it is not counted twice |

The important structural decision: the model layer no longer runs in parallel. Its own Phase 0 gate — "are the misses semantic?" — is Workstream F, and F depends on the trust and scale work because a benchmark run against an unstable system measures the instability. Putting a model on top of an unread edge table and an indistinguishable empty result compounds two problems instead of solving one.

## 02 · Where the work lives

Current size of every module, which workstreams touch it, and what kind of change. This is the map to keep open while working.

| Module | Lines | Workstreams | Nature of the change |
|---|---|---|---|
| `src/core` | 1,383 | A, B, H | Rename `confidence`; add `Edge`, `Ref`, `EdgeType`; five model traits |
| `src/storage` | 3,538 | B, C, D, E, H | v9 and v10 migrations; edge projection; batch fetch; cursor; `query_log`; `projects.remote`; `embeddings` |
| `src/graph` | 844 | B, D | Loader reads `edges`; `caused_by` in traversal; node cap; direction validator |
| `src/retrieval` | 345 | F, H | Rank components; optional semantic component |
| `src/ingest` | 3,401 | G, H | Auto-capture on begin; extractor proposals |
| `src/commands` | 4,402 | A–H | Six commands touched; two new (`doctor`, `metrics`); `model` subcommands |
| `src/mcp` | 3,186 | A, C, H | Format rename; `reason` field; instructions; seventh tool |
| `src/cli` | 589 | C, F, H | Wire `doctor`, `metrics`, `model` |
| `src/git` | 424 | E | Remote URL normalisation |
| `src/models` (new) | ~1,200 | H | Traits, profiles, router, two adapters, mock backend |
| `src-tauri` | 2,028 | A, C | Remove mock path; doctor panel |
| `ui/src` | 19,132 | A | Delete the mock fallback only — no other UI work |
| `tests` | 2,509 | all | Grows to roughly 3,800 with the new suites |

The distribution is worth noticing: nineteen thousand lines of UI receive one deletion, while three thousand lines of storage receive four workstreams. That is the correct shape for this plan — the problems are in the data model and the trust surface, not the interface.

### Files created

| New file | Purpose | Workstream |
|---|---|---|
| `src/commands/doctor.rs` | Operational health: empty rate, zombie sessions, rejections, index freshness | C, B |
| `src/commands/metrics.rs` | The published numbers, each with its denominator | F |
| `src/models/*.rs` (six files) | Traits, profiles, router, adapters, mock | H |
| `tests/test_edges.rs` | Consistency between the edge table and the legacy columns | B |
| `tests/test_scale.rs` | 10k-record latency budgets | D |
| `tests/test_calibration.rs` | Precision and recall of the overlap score | F |
| `tests/queries/tune.json`, `eval.json` | Split benchmark sets | F |

## 03 · How each workstream was identified

Every workstream traces to a specific observation with a command or a line behind it. If a row here is wrong, the workstream should not exist.

| Workstream | Evidence | The finding |
|---|---|---|
| A · Truth | `grep -n "confidence" src/mcp/handlers/check_guardrails.rs`; `format_synthesized_digest`; README line 22 | A token-overlap ratio named confidence; a regrouped list named synthesis; a 70% claim measured against JSON of the same records |
| B · Graph + edges | `grep -rn "FROM edges" src/` returns nothing anywhere in `src/`; the v8 trigger writes `'follows'`; `is_causal_edge` accepts `'parent'` | The edge table is written by triggers and read by nothing, and its type name would break traversal the moment it were read |
| C · Trust | `format_markdown_digest` returns the same string for no-rows and no-match; no `query_log` table exists | An agent cannot distinguish “nothing recorded” from “nothing applies”, and nothing measures how often each happens |
| D · Scale | `fetch_query_actions` issues four child queries per row; `loader.rs` builds the component with no cap | A 20-record context response is 81 queries; a trace loads an unbounded component per call |
| E · Identity | `resolve_project_identifier` has no remote branch | A worktree or clone starts with no memory, silently |
| F · Measurement | Benchmark expected ids authored beside the ranker; no calibration set; no metrics command | Every published number is either self-referential or uncalibrated |
| G · Capture | No path writes memory unless the agent calls `record` | The High/High adoption risk from the original plan, still unmitigated |
| H · Model layer | The earlier model-layer analysis; gated on F | Retrieval quality is unproven, so the layer waits behind the measurement that decides it |

> **The test for adding a workstream later**
>
> If a future proposal cannot fill the middle column with a command and a line number, it is a preference rather than a finding, and it goes behind the release gate rather than into the plan.

## 04 · Dependencies and the critical path

```
A truth --> B graph --> D scale --> F measure --> H models
                  |                    ^
                  +--> C trust ---------+

E identity   (independent, any time after A)
```

| Edge | Why it exists | What breaks if reversed |
|---|---|---|
| A → B | The schema change in B needs the vocabulary settled in A, or you migrate twice | Two migrations for one column rename |
| B → D | The cursor encodes `(timestamp_ms, id)` over the graph-ordered result; ordering must be settled first | Cursors that skip or repeat records after the traversal change |
| C → F | Metrics are computed from `query_log`, which C adds | A metrics command with no data source |
| D → F | Benchmark timings are meaningless while the N+1 dominates them | A latency number that measures the wrong thing |
| F → H | The model layer’s own gate is F’s semantic-share measurement | Building 16 days of layer on an unproven premise |
| E — independent | Identity touches resolution only | — |

The critical path is A → B → D → F → H: twenty-nine days to the MVP, forty to the full layer, out of forty-six. C and E can run alongside D, which is why the calendar in section 13 fits the foundation into three and a half weeks rather than four.

## 05 · A — Truth pass · 1 day

| Task | File | Change | Acceptance |
|---|---|---|---|
| A1 | `check_guardrails.rs`, `check.rs` | `confidence` → `overlap` in response, struct and tool description | No `confidence` field in any response |
| A2 | `mcp/format.rs` | “Synthesized Context” → “Grouped Context”; add `mode: "grouped"` | Snapshot test on the heading |
| A3 | `README.md` | Remove the 70% figure; point at `knox metrics` | Grep test: no bare percentage in README |
| A4 | `ui/src/services/api.ts` | Delete the `!isTauri` mock branch; demo needs `VITE_DEMO=1` and shows a banner | Production bundle has no `mockActions` |
| A5 | `knox.md`, `SERVER_INSTRUCTIONS` | Align with the six real capabilities | Test asserts instructions and tool list agree |

One day, no schema change, and it removes every claim a sceptical reviewer tests first. Do it before anything else because everything else is easier to judge once the vocabulary is honest.

## 06 · B — One graph, and edge semantics · 5 days

The largest single workstream, because it is where the schema is touched once for four different reasons.

| Task | File | Change | Acceptance |
|---|---|---|---|
| B1 | migrations v9 | Correct the verb: `follows` → `caused_by` for parent edges; update both triggers and the CHECK | Existing rows migrated |
| B2 | migrations v9 | Add `label`, `provenance`, `note` to `edges` | Columns exist; defaults set |
| B3 | migrations v9 | Add `refs(id, kind, value, project_id)`; `to_action_id` → `to_id` with an integrity trigger | An edge to `file:src/db.rs` resolves |
| B4 | `graph/loader.rs` | Build the petgraph from `edges`; delete the three in-memory constructions | Identical output on the fixture |
| B5 | `graph/traversal.rs` | Accept `caused_by`; `follows` never traversed | Unit test on both |
| B6 | `commands/record.rs` | Direction validator per type; symmetric types normalised by id order | Reversed `implements` rejected with a teaching error |
| B7 | migrations v9 + `record.rs` | `edge_rejections(ts, project, attempted_type, reason)` | Unknown type logged, not dropped |
| B8 | `storage/nodes.rs` | Project `is_resolved`, `resolution_note`, `commit_sha` into edges | Consistency test covers all five sources |
| B9 | `tests/test_edges.rs` (new) | Property test: random operations keep table and columns in agreement | Fails if a trigger is dropped |

> **The one irreversible decision in the plan**
>
> B3 makes the edge endpoint polymorphic, which loses the foreign key to `actions`. The alternative — a separate `action_refs` table — keeps the FK and forces every traversal to union two tables. Take the trigger and one table if you want one query to answer “everything connected”; take two tables if you would rather never write a trigger. Decide before B3, because changing it afterwards is a second migration.

## 07 · C — Trust surface · 2 days

| Task | File | Change | Acceptance |
|---|---|---|---|
| C1 | all six commands | Every read returns `reason`: `no_records`, `no_match`, `out_of_scope` | Three distinct payloads in tests |
| C2 | migrations v9 | `query_log(ts_ms, tool, project_id, rows, tokens, latency_ms, reason)` | No query text, no file paths in the row |
| C3 | `commands/doctor.rs` (new) | Empty rate, zombie sessions, top rejected edge types, DB size, integrity, last migration, bound models | Runs on a fixture and prints every field |
| C4 | CLI + Tauri | Expose doctor in both | Present in `--help` and in the GUI |

The `reason` field is the highest-leverage change for agent behaviour in the whole plan: `no_match` means broaden the query, `no_records` means stop asking and do the work, `out_of_scope` means fix the parameter. Today all three are an empty list and all three waste a round trip.

## 08 · D — Scale · 3 days

| Task | File | Change | Acceptance |
|---|---|---|---|
| D1 | `storage/queries.rs` | Batch four child lookups into one `IN` per table | Query count per context call 1+4N → 5 |
| D2 | `commands/context.rs`, `check.rs` | Cursor `(timestamp_ms, id)` as base64; `next` on truncation | Two pages reassemble to one |
| D3 | `commands/trace.rs`, `loader.rs` | `max_nodes` 200, `truncated` flag, frontier ids | 5,000-node component in < 300 ms |
| D4 | `tests/test_scale.rs` (new) | 10,000 records; context p95 < 50 ms, trace p95 < 300 ms | Runs in CI |

## 09 · E — Identity · 1 day

| Task | File | Change | Acceptance |
|---|---|---|---|
| E1 | migrations v9 | `projects.remote TEXT`; backfill from `.git/config` | Existing projects with a remote get one |
| E2 | `storage/sessions.rs` | Resolution: id → remote (https/ssh normalised) → path → unique name | Two clones resolve to one project |
| E3 | `commands/init.rs` | Write the remote at init | `knox init` in a clone matches |
| E4 | new test | Same remote, different path, shared memory | Both paths see the same records |

Half a day is the migration; the other half is remote normalisation, which is the part that will be wrong on the first attempt — `git@host:owner/repo.git`, `https://host/owner/repo`, trailing slash, case. Two spellings of one repository become two memories, which is the same failure the path resolver already has, one level up.

## 10 · F — Measurement · 4 days

| Task | File | Change | Acceptance |
|---|---|---|---|
| F1 | `tests/queries/*.json` | Split into `tune.json` and `eval.json` | CI fails if they overlap |
| F2 | `tests/test_calibration.rs` (new) | 50 labelled pairs; precision and recall at overlap 0.4/0.6/0.8 | A table, and a threshold justified by it |
| F3 | `commands/metrics.rs` (new) | Hit rate, empty rate, duplicates, tokens per successful retrieval, latency p50/p95 — each with a denominator | Reproduces every published number |
| F4 | `README.md` | Publish the definitions, not a headline | Every number runs as one command |
| F5 | `commands/doctor.rs` | Flag a rejected edge type once it crosses the promotion threshold | A documented promotion path |

This is also the gate for H. The semantic share of misses, measured here on `eval.json`, is what decides whether sixteen days of model layer are worth starting. Everything in F has value even if H never happens.

## 11 · G — Capture · 3 days, optional

| Task | File | Change | Acceptance |
|---|---|---|---|
| G1 | `commands/begin.rs`, `ingest.rs` | Ingest transcript lines since the last offset on begin, when a path is configured | A session with zero `record` calls still produces memory |
| G2 | `commands/hook.rs` | Extend the hook to summarise the commit message into the record | Commit text appears in the record, scoped |

This closes the only finding that is a product decision rather than a fix. It is optional in the plan because it changes behaviour, but without it the tool works only when the agent remembers — which is the adoption risk the original design named High/High and never mitigated.

## 12 · H — Model layer · 16 days MVP, 11 more full

Unchanged in design, changed in position: it now starts after F, and its own Phase 0 is already done there.

| Phase | Deliverable | Days | Gate |
|---|---|---|---|
| H1 | Five traits, profiles, router, two adapters, mock backend, v10 migration | 6 | Clean-machine promise; CI green with no GPU |
| H2 | Embeddings, re-rank in both reads, related section, semantic dedupe | 4 | +5 pts recall@5; byte-identical with profiles off |
| H3 | `knox_ask`: seventh tool, grounding modes, citation verification, caller policy, injection test | 6 | ≥3x tokens, ≥90% grounded, zero escapes |
| H4 | Extractor for ingest | 5 | Ingest precision holds |
| H5 | Code corpus, `d1-2B` contradiction candidates, drift, clustering | 6 | Hand-checked precision ≥90% on 20 cases |

H1–H3 are the MVP and depend on F having shown a semantic gap. H4 and H5 are cancellable without leaving anything half-built, which is why the two most interesting models — the decision model and the code corpus — sit last.

## 13 · The calendar

| Week | Workstreams | Cumulative | Milestone |
|---|---|---|---|
| 1 | A (1d) · B begins (4d) | 5 days | Vocabulary honest; schema v9 half-done |
| 2 | B finishes (1d) · C (2d) · E (1d) · D begins (1d) | 10 days | One graph, specific empties, identity by remote |
| 3 | D finishes (2d) · F (4d) | 16 days | Foundation complete; every claim executable |
| 4 | G (3d) · H1 begins (2d) | 21 days | Memory without compliance; model runtime started |
| 5 | H1 finishes (4d) · H2 begins (1d) | 26 days | Any model runs, no rebuild |
| 6 | H2 finishes (3d) · H3 begins (2d) | 31 days | Semantic re-rank shipped behind a flag |
| 7 | H3 finishes (4d) | 35 days | `knox_ask` live; 3x token reduction |
| 8–9 | H4 (5d) · H5 (6d) | 46 days | Extraction and contradictions, cancellable |

The foundation lands at the end of week three, and that is the first point at which the product can be promoted honestly — sixteen days for a claim set that survives a sceptical reader. Everything after it is capability, and every phase of it is individually cancellable.

## 14 · Release gates

| Gate | Check | Applies to |
|---|---|---|
| No unqualified claim | Every percentage and every use of “graph” in README, `knox.md` and instructions has a mechanism or a denominator | Release 2.1 (after A) |
| Edges are read | `FROM edges` in the traversal path; consistency test green | Release 2.1 |
| Empty is specific | Three `reason` values in tests, per tool | Release 2.1 |
| Scale | 10k fixture: context p95 < 50 ms, trace p95 < 300 ms | Release 2.1 |
| Identity | Two clones resolve to one project | Release 2.1 |
| Measurement | `knox metrics` reproduces every published number | Release 2.1 |
| No fabricated surface | Production bundle contains no mock data | Release 2.1 |
| Determinism | Every profile off → byte-identical to 2.1 output | Release 2.2 (after H) |
| Groundedness | ≥90% strict on 30 hand-scored answers | Release 2.2 |
| Injection | Planted instruction never appears in an answer | Release 2.2 |

> *Ship 2.1 when every sentence in the README can be executed as a command. Ship 2.2 when turning the models off changes nothing.*

## 15 · Risk, and what to cut first

| Risk | Likelihood | If it happens | Mitigation |
|---|---|---|---|
| B3’s polymorphic endpoint is the wrong call | medium | A second migration to split it | Decide before writing it; two-table alternative costs one union |
| F shows the semantic gap is small | medium | H cancels; sixteen days saved | That is the plan working, not failing |
| Calibration shows overlap is a poor score | medium | Threshold re-derived; strict mode may narrow | Budget a day to change it, not just measure it |
| G adds noise faster than it adds memory | medium | Ingest becomes a source of duplicates | Rule table plus `seen_count`; measure duplicate rate in F |
| Remote normalisation is subtly wrong | low | Two memories for one repository | A fixture with five URL spellings of one repo |
| H3’s groundedness lands below 90% | medium | Agent access blocked; human-only | Ship as CLI-only; the tool still earns its place |

### Cut order, if the calendar slips

1. **H5** first — code corpus and drift are the most speculative and the least load-bearing.
2. **H4** second — extraction is valuable but G already captures without it.
3. **G** third — without it the tool works only when remembered, which is the current state.
4. **H3** fourth — and if this goes, H1–H2 go with it, because the runtime exists to serve the answerer.
5. **Never** cut A, B or F. Cutting those means shipping the claims without the mechanisms, which is the problem this plan exists to fix.

### Method and limits

Every workstream in section 03 traces to a command or a line in the repository at `main`. The file map in section 02 uses line counts measured from the tree today. No Rust toolchain exists in this environment: nothing was compiled, no test was run, no query executed, and every latency budget is a target rather than a measurement. The day estimates are the author’s, not observed velocity, and should be recalibrated after workstream A — one day of real work against a one-day estimate tells you more about the remaining forty-five than any plan can.

## 16 · Task index

| ID | Workstream | One line |
|---|---|---|
| A1–A5 | Truth | Rename confidence, rename synthesis, delete the 70% claim, remove mock UI, align instructions |
| B1–B9 | Graph + edges | Fix the verb, add label/provenance/note, add refs, read edges, validate direction, log rejections, project columns, consistency test |
| C1–C4 | Trust | `reason` field, `query_log`, `doctor`, surface it |
| D1–D4 | Scale | Batch fetch, cursor, node cap, 10k test |
| E1–E4 | Identity | Remote column, resolution order, init writes it, clone test |
| F1–F5 | Measurement | Split sets, calibrate overlap, metrics command, publish, promotion flag |
| G1–G2 | Capture | Transcript auto-ingest, commit-message summary |
| H1–H5 | Models | Runtime, embeddings, `ask`, extractor, decision model and code corpus |

---

*Master implementation plan · 5 November 2026 · 3 plans merged · 8 workstreams · 16 → 46 days · 2 releases*
