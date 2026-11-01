import { describe, it } from "node:test";
import assert from "node:assert/strict";

interface MockAction {
  id: string;
  action_type: string;
  significance?: "major" | "minor";
  is_resolved?: boolean;
  summary: string;
}

function filterByFacet(
  actions: MockAction[],
  activeFacet: "all" | "doubts" | "major" | "minor" | "dialogue"
): MockAction[] {
  return actions.filter((a) => {
    if (activeFacet === "doubts") {
      return (a.action_type === "doubt" || a.action_type === "question") && !a.is_resolved;
    }
    if (activeFacet === "major") {
      return a.action_type === "decide" && a.significance !== "minor";
    }
    if (activeFacet === "minor") {
      return a.significance === "minor";
    }
    if (activeFacet === "dialogue") {
      return a.action_type === "dialogue" || a.action_type === "conversation";
    }
    return true;
  });
}

function computeNextIndex(
  direction: "up" | "down",
  currentIndex: number,
  displayedLength: number
): number {
  if (displayedLength === 0) return -1;
  if (direction === "down") {
    return currentIndex < 0 ? 0 : Math.min(currentIndex + 1, displayedLength - 1);
  } else {
    return currentIndex < 0 ? displayedLength - 1 : Math.max(currentIndex - 1, 0);
  }
}

describe("Query Console Keyboard Navigation & Facet Filter Logic", () => {
  const mockActions: MockAction[] = [
    { id: "act-1", action_type: "write", summary: "Write parser" },
    { id: "act-2", action_type: "doubt", is_resolved: false, summary: "Unresolved parser doubt" },
    { id: "act-3", action_type: "decide", significance: "major", summary: "ADR: Use SQLite WAL" },
    { id: "act-4", action_type: "decide", significance: "minor", summary: "Choose port 4000" },
    { id: "act-5", action_type: "dialogue", summary: "User requested refactor" },
    { id: "act-6", action_type: "doubt", is_resolved: true, summary: "Resolved question" },
  ];

  it("filters accurately across each cognitive facet", () => {
    const all = filterByFacet(mockActions, "all");
    assert.equal(all.length, 6);

    const doubts = filterByFacet(mockActions, "doubts");
    assert.equal(doubts.length, 1);
    assert.equal(doubts[0].id, "act-2");

    const major = filterByFacet(mockActions, "major");
    assert.equal(major.length, 1);
    assert.equal(major[0].id, "act-3");

    const minor = filterByFacet(mockActions, "minor");
    assert.equal(minor.length, 1);
    assert.equal(minor[0].id, "act-4");

    const dialogue = filterByFacet(mockActions, "dialogue");
    assert.equal(dialogue.length, 1);
    assert.equal(dialogue[0].id, "act-5");
  });

  it("navigates down correctly within displayedResults bounds", () => {
    const displayed = filterByFacet(mockActions, "all"); // 6 items
    let idx = -1;

    // First ArrowDown from unselected state selects index 0
    idx = computeNextIndex("down", idx, displayed.length);
    assert.equal(idx, 0);
    assert.equal(displayed[idx].id, "act-1");

    // Stepping down
    idx = computeNextIndex("down", idx, displayed.length);
    assert.equal(idx, 1);
    assert.equal(displayed[idx].id, "act-2");

    // Clamps at last index (5)
    idx = 5;
    idx = computeNextIndex("down", idx, displayed.length);
    assert.equal(idx, 5);
  });

  it("navigates up correctly within displayedResults bounds", () => {
    const displayed = filterByFacet(mockActions, "all"); // 6 items
    let idx = -1;

    // ArrowUp from unselected starts at the bottom
    idx = computeNextIndex("up", idx, displayed.length);
    assert.equal(idx, 5);
    assert.equal(displayed[idx].id, "act-6");

    // Step up
    idx = computeNextIndex("up", idx, displayed.length);
    assert.equal(idx, 4);

    // Clamps at top (0)
    idx = 0;
    idx = computeNextIndex("up", idx, displayed.length);
    assert.equal(idx, 0);
  });

  it("safely handles keyboard navigation when displayed list is empty", () => {
    const emptyList: MockAction[] = [];
    assert.equal(computeNextIndex("down", -1, emptyList.length), -1);
    assert.equal(computeNextIndex("up", -1, emptyList.length), -1);
  });

  it("ensures facet filtering prevents selecting out-of-facet actions", () => {
    // Before: When unfiltered results had 6 items and doubts had only 1,
    // indexing results[1] would select act-2, but results[2] was act-3 (a decision!).
    // Now: with displayedResults, doubts only has 1 item, so idx clamps to 0.
    const doubts = filterByFacet(mockActions, "doubts");
    assert.equal(doubts.length, 1);

    let idx = -1;
    idx = computeNextIndex("down", idx, doubts.length);
    assert.equal(idx, 0);
    assert.equal(doubts[idx].id, "act-2");

    // Another ArrowDown cannot advance past 0 because length is 1
    idx = computeNextIndex("down", idx, doubts.length);
    assert.equal(idx, 0);
    assert.equal(doubts[idx].action_type, "doubt");
  });
});

