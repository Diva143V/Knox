import test, { describe, it } from "node:test";
import assert from "node:assert/strict";
import type { SettingsTabId } from "../types/knox";

interface CategoryDef {
  id: SettingsTabId;
  label: string;
  desc: string;
}

const CATEGORIES: CategoryDef[] = [
  { id: "general", label: "Appearance", desc: "Theme, density, animations and startup view" },
  { id: "storage", label: "Storage & Database", desc: "SQLite path, timeouts, telemetry and maintenance" },
  { id: "graph", label: "Graph Canvas", desc: "Cards vs Cosmos, D3 physics and default layout" },
  { id: "mcp", label: "AI Agents & MCP", desc: "Command modes, guardrails, and detected clients" },
  { id: "scanner", label: "Git & Scanner", desc: "Human author name, ignore rules and hook policy" },
  { id: "ingestion", label: "Ingestion", desc: "Transcript deduplication and preview rules" },
  { id: "system", label: "System & Shell", desc: "Tray behavior, startup launch and notifications" },
  { id: "shortcuts", label: "Keyboard Shortcuts", desc: "Interactive keybindings reference and cheat sheet" },
  { id: "about", label: "About & Info", desc: "App version, system diagnostics and quick tours" },
];

function filterCategories(categories: CategoryDef[], query: string): CategoryDef[] {
  if (!query.trim()) return categories;
  const q = query.toLowerCase();
  return categories.filter(
    (c) => c.label.toLowerCase().includes(q) || c.desc.toLowerCase().includes(q)
  );
}

function manageIgnorePatterns(existing: string[], action: { type: "add" | "remove"; pattern: string }): { patterns: string[]; error?: string } {
  const trimmed = action.pattern.trim();
  if (action.type === "add") {
    if (!trimmed) return { patterns: existing };
    if (existing.includes(trimmed)) {
      return { patterns: existing, error: "Pattern already exists" };
    }
    return { patterns: [...existing, trimmed] };
  }
  if (action.type === "remove") {
    return { patterns: existing.filter((p) => p !== action.pattern) };
  }
  return { patterns: existing };
}

describe("Settings Modal State & Navigation Logic", () => {
  it("defines all 9 modular settings categories", () => {
    assert.equal(CATEGORIES.length, 9);
    const expectedIds: SettingsTabId[] = [
      "general",
      "storage",
      "graph",
      "mcp",
      "scanner",
      "ingestion",
      "system",
      "shortcuts",
      "about",
    ];
    assert.deepEqual(CATEGORIES.map((c) => c.id), expectedIds);
  });

  it("filters categories by label or description search query", () => {
    assert.equal(filterCategories(CATEGORIES, "").length, 9);
    assert.equal(filterCategories(CATEGORIES, "git").length, 1);
    assert.equal(filterCategories(CATEGORIES, "git")[0].id, "scanner");
    assert.equal(filterCategories(CATEGORIES, "sqlite").length, 1);
    assert.equal(filterCategories(CATEGORIES, "sqlite")[0].id, "storage");
    assert.equal(filterCategories(CATEGORIES, "nonexistent-query").length, 0);
  });

  it("adds and deduplicates scanner ignore patterns", () => {
    let patterns = ["node_modules", "target"];

    // Add new pattern
    let res = manageIgnorePatterns(patterns, { type: "add", pattern: ".git" });
    assert.equal(res.error, undefined);
    assert.deepEqual(res.patterns, ["node_modules", "target", ".git"]);
    patterns = res.patterns;

    // Reject duplicate
    res = manageIgnorePatterns(patterns, { type: "add", pattern: "target" });
    assert.equal(res.error, "Pattern already exists");
    assert.equal(res.patterns.length, 3);

    // Remove pattern
    res = manageIgnorePatterns(patterns, { type: "remove", pattern: "node_modules" });
    assert.deepEqual(res.patterns, ["target", ".git"]);
  });
});

