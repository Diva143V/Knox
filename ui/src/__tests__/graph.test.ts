import test, { describe, it } from "node:test";
import assert from "node:assert/strict";
import type { KnoxAction } from "../types/knox";

interface SearchMatch {
  action: KnoxAction;
  score: number;
  matchField: "summary" | "symbol" | "file" | "tag" | "actor" | "type" | "rationale";
  matchDetail: string;
}

function scoreActionSearch(actions: KnoxAction[], query: string): SearchMatch[] {
  const q = query.trim().toLowerCase();
  if (!q) return [];
  return actions
    .map((action) => {
      let score = 0;
      let matchField: "summary" | "symbol" | "file" | "tag" | "actor" | "type" | "rationale" = "summary";
      let matchDetail = "";

      if (action.summary.toLowerCase().includes(q)) {
        score += 10;
        matchField = "summary";
      }
      if (action.symbols?.some((s) => s.toLowerCase().includes(q))) {
        score += 8;
        if (score <= 10) matchField = "symbol";
        matchDetail = action.symbols.find((s) => s.toLowerCase().includes(q)) || "";
      }
      if (action.files?.some((f) => f.toLowerCase().includes(q))) {
        score += 6;
        if (score <= 10) matchField = "file";
        matchDetail = action.files.find((f) => f.toLowerCase().includes(q)) || "";
      }
      if (action.tags?.some((t) => t.toLowerCase().includes(q))) {
        score += 4;
        if (score <= 10) matchField = "tag";
        matchDetail = action.tags.find((t) => t.toLowerCase().includes(q)) || "";
      }
      if (action.actor.toLowerCase().includes(q)) {
        score += 3;
        if (score <= 10) matchField = "actor";
      }
      if (action.action_type.toLowerCase().includes(q)) {
        score += 2;
        if (score <= 10) matchField = "type";
      }
      if (action.rationale?.toLowerCase().includes(q)) {
        score += 1;
        if (score <= 10) matchField = "rationale";
      }

      return { action, score, matchField, matchDetail };
    })
    .filter((m) => m.score > 0)
    .sort((a, b) => b.score - a.score)
    .slice(0, 8);
}

const SAMPLE_ACTIONS: KnoxAction[] = [
  {
    id: "act-1",
    actor: "antigravity",
    action_type: "decide",
    summary: "Adopt SQLite WAL mode for fast concurrent queries",
    rationale: "Prevents reader/writer locks",
    files: ["src/storage/db.rs"],
    symbols: ["KnoxDb"],
    tags: ["architecture", "database"],
    is_current: true,
    created_at: 1000,
    superseded_by: null,
  },
  {
    id: "act-2",
    actor: "human",
    action_type: "patch",
    summary: "Fix compiler warning in sqlite driver",
    rationale: "Type mismatch on row id",
    files: ["src/storage/db.rs"],
    symbols: ["open_db"],
    tags: ["bugfix"],
    is_current: false,
    created_at: 1100,
    superseded_by: null,
  },
  {
    id: "act-3",
    actor: "cursor",
    action_type: "execute",
    summary: "Add MCP stdio transport loop",
    rationale: "Connect agents via stdin/stdout",
    files: ["src/mcp/server.rs"],
    symbols: ["run_mcp_server_loop"],
    tags: ["mcp", "agent"],
    is_current: true,
    created_at: 1200,
    superseded_by: null,
  },
];

describe("Graph Search & Matching Logic", () => {
  it("scores summary matches with highest priority (10 pts)", () => {
    const matches = scoreActionSearch(SAMPLE_ACTIONS, "WAL mode");
    assert.equal(matches.length, 1);
    assert.equal(matches[0].action.id, "act-1");
    assert.equal(matches[0].matchField, "summary");
    assert.ok(matches[0].score >= 10);
  });

  it("finds symbol and file matches when query matches secondary attributes", () => {
    const matches = scoreActionSearch(SAMPLE_ACTIONS, "server.rs");
    assert.equal(matches.length, 1);
    assert.equal(matches[0].action.id, "act-3");
    assert.equal(matches[0].matchField, "file");
    assert.equal(matches[0].matchDetail, "src/mcp/server.rs");
  });

  it("returns empty list for empty query or no match", () => {
    assert.equal(scoreActionSearch(SAMPLE_ACTIONS, "").length, 0);
    assert.equal(scoreActionSearch(SAMPLE_ACTIONS, "nonexistent-word").length, 0);
  });
});

