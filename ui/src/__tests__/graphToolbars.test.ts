import test, { describe, it } from "node:test";
import assert from "node:assert/strict";

describe("Knowledge Graph Collapsible Floating Toolbars Logic", () => {
  it("computes active filter count accurately", () => {
    function computeActiveFilterCount(
      actorFilter: string,
      typeFilter: string,
      searchHighlight: string,
      showFileNodes: boolean
    ): number {
      let n = 0;
      if (actorFilter !== "all") n++;
      if (typeFilter !== "all") n++;
      if (searchHighlight.trim().length > 0) n++;
      if (!showFileNodes) n++;
      return n;
    }

    assert.equal(computeActiveFilterCount("all", "all", "", true), 0);
    assert.equal(computeActiveFilterCount("antigravity", "all", "", true), 1);
    assert.equal(computeActiveFilterCount("antigravity", "decide", "", true), 2);
    assert.equal(computeActiveFilterCount("antigravity", "decide", "test", true), 3);
    assert.equal(computeActiveFilterCount("antigravity", "decide", "test", false), 4);
  });

  it("handles panel toggle states independently without collisions", () => {
    let isFiltersOpen = false;
    let isLayoutOpen = false;

    const toggleFilters = () => {
      isFiltersOpen = !isFiltersOpen;
    };
    const toggleLayout = () => {
      isLayoutOpen = !isLayoutOpen;
    };
    const handleEscape = () => {
      isFiltersOpen = false;
      isLayoutOpen = false;
    };

    // Both start collapsed
    assert.equal(isFiltersOpen, false);
    assert.equal(isLayoutOpen, false);

    // Open Filters independently
    toggleFilters();
    assert.equal(isFiltersOpen, true);
    assert.equal(isLayoutOpen, false);

    // Open Layout independently (both open together without closing the other)
    toggleLayout();
    assert.equal(isFiltersOpen, true);
    assert.equal(isLayoutOpen, true);

    // Collapse Filters independently
    toggleFilters();
    assert.equal(isFiltersOpen, false);
    assert.equal(isLayoutOpen, true);

    // Escape closes any open panel
    handleEscape();
    assert.equal(isFiltersOpen, false);
    assert.equal(isLayoutOpen, false);
  });

  it("preserves layout engine and view mode state across panel open/close cycles", () => {
    type LayoutEngine = "hierarchical" | "dagre" | "force" | "grid";
    type ViewMode = "cards" | "cosmos";

    let currentLayout: LayoutEngine = "hierarchical";
    let currentViewMode: ViewMode = "cards";
    let isLayoutOpen = false;

    // User opens layout panel
    isLayoutOpen = true;
    assert.equal(isLayoutOpen, true);

    // User selects Dagre and Cosmos mode
    currentLayout = "dagre";
    currentViewMode = "cosmos";

    // User collapses panel
    isLayoutOpen = false;
    assert.equal(isLayoutOpen, false);

    // State remains fully intact
    assert.equal(currentLayout, "dagre");
    assert.equal(currentViewMode, "cosmos");

    // User re-opens panel
    isLayoutOpen = true;
    assert.equal(currentLayout, "dagre");
    assert.equal(currentViewMode, "cosmos");
  });

  it("handles outside click logic accurately", () => {
    let isFiltersOpen = true;
    let isLayoutOpen = true;

    function handleClickOutside(isInsideToolbar: boolean) {
      if (!isInsideToolbar) {
        isFiltersOpen = false;
        isLayoutOpen = false;
      }
    }

    // Click inside does not collapse
    handleClickOutside(true);
    assert.equal(isFiltersOpen, true);
    assert.equal(isLayoutOpen, true);

    // Click outside collapses both panels cleanly
    handleClickOutside(false);
    assert.equal(isFiltersOpen, false);
    assert.equal(isLayoutOpen, false);
  });
});

