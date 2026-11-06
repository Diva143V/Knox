import React from "react";
import {
  Filter,
  Search,
  LayoutGrid,
  ChevronDown,
  X,
  FileCode,
  Zap,
  Tag,
  LayoutDashboard,
  Orbit,
  Atom,
  Flame,
  Maximize2,
  RotateCcw,
  Eye,
  Map as MapIcon,
  RefreshCw,
} from "lucide-react";
import { Button } from "@/components/ui/Button";
import { Input } from "@/components/ui/Input";
import { ActionBadge } from "@/components/ui/ActionBadge";
import { ActorBadge } from "@/components/ui/ActorBadge";
import { cn } from "@/lib/cn";
import type { KnoxAction } from "@/types/knox";
import type { LayoutEngine } from "@/hooks/useGraphLayout";

export const LAYOUTS: {
  id: LayoutEngine;
  label: string;
  title: string;
  icon: React.ReactNode;
}[] = [
  {
    id: "hierarchical",
    label: "ELK",
    title: "Hierarchical layered layout (ELK)",
    icon: <LayoutGrid className="size-3.5" />,
  },
  {
    id: "dagre",
    label: "Dagre",
    title: "Fast hierarchical layout (Dagre)",
    icon: <LayoutGrid className="size-3.5" />,
  },
  {
    id: "force",
    label: "Force",
    title: "Force-directed layout (D3)",
    icon: <LayoutGrid className="size-3.5" />,
  },
  {
    id: "grid",
    label: "Grid",
    title: "Compact grid layout",
    icon: <LayoutGrid className="size-3.5" />,
  },
];

const selectClass =
  "h-7 bg-background border border-border rounded-md px-2 text-sm text-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-focus-ring cursor-pointer font-mono";

export interface SearchMatch {
  action: KnoxAction;
  score: number;
  matchField: "summary" | "symbol" | "file" | "tag" | "actor" | "type" | "rationale";
  matchDetail: string;
}

interface GraphToolbarsProps {
  toolbarContainerRef: React.RefObject<HTMLDivElement | null>;
  searchContainerRef: React.RefObject<HTMLDivElement | null>;
  isFiltersOpen: boolean;
  setIsFiltersOpen: React.Dispatch<React.SetStateAction<boolean>>;
  activeFilterCount: number;
  handleResetFilters: () => void;
  actorFilter: string;
  setActorFilter: (val: string) => void;
  availableActors: string[];
  typeFilter: string;
  setTypeFilter: (val: string) => void;
  availableTypes: string[];
  currentOnly: boolean;
  setCurrentOnly: (val: boolean) => void;
  showFileNodes: boolean;
  setShowFileNodes: (val: boolean | ((prev: boolean) => boolean)) => void;
  isLayoutOpen: boolean;
  setIsLayoutOpen: React.Dispatch<React.SetStateAction<boolean>>;
  layoutEngine: LayoutEngine;
  setLayoutEngine: (val: LayoutEngine) => void;
  viewMode: "cards" | "cosmos";
  handleToggleViewMode: (mode: "cards" | "cosmos") => void;
  livePhysics: boolean;
  isPhysicsRunning: boolean;
  handleTogglePhysics: () => void;
  reheat: (alpha: number) => void;
  handleFitView: () => void;
  handleRelayout: () => void;
  isLayouting: boolean;
  nodesLength: number;
  showLegend: boolean;
  handleToggleLegend: () => void;
  showMinimap: boolean;
  handleToggleMinimap: () => void;
  searchHighlight: string;
  setSearchHighlight: (val: string) => void;
  isSearchDropdownOpen: boolean;
  setIsSearchDropdownOpen: (val: boolean) => void;
  activeResultIndex: number;
  setActiveResultIndex: React.Dispatch<React.SetStateAction<number>>;
  searchMatches: SearchMatch[];
  handleSelectSearchResult: (action: KnoxAction) => void;
  isLoading: boolean;
}

export const GraphToolbars: React.FC<GraphToolbarsProps> = ({
  toolbarContainerRef,
  searchContainerRef,
  isFiltersOpen,
  setIsFiltersOpen,
  activeFilterCount,
  handleResetFilters,
  actorFilter,
  setActorFilter,
  availableActors,
  typeFilter,
  setTypeFilter,
  availableTypes,
  currentOnly,
  setCurrentOnly,
  showFileNodes,
  setShowFileNodes,
  isLayoutOpen,
  setIsLayoutOpen,
  layoutEngine,
  setLayoutEngine,
  viewMode,
  handleToggleViewMode,
  livePhysics,
  isPhysicsRunning,
  handleTogglePhysics,
  reheat,
  handleFitView,
  handleRelayout,
  isLayouting,
  nodesLength,
  showLegend,
  handleToggleLegend,
  showMinimap,
  handleToggleMinimap,
  searchHighlight,
  setSearchHighlight,
  isSearchDropdownOpen,
  setIsSearchDropdownOpen,
  activeResultIndex,
  setActiveResultIndex,
  searchMatches,
  handleSelectSearchResult,
  isLoading,
}) => {
  return (
    <div
      ref={toolbarContainerRef}
      className="absolute top-3 left-3 z-30 flex flex-col items-start gap-2 pointer-events-none max-w-[calc(100vw-1.5rem)]"
    >
      {/* Floating Trigger Bar */}
      <div className="pointer-events-auto flex items-center gap-1.5 bg-card/95 backdrop-blur-md border border-border p-1 rounded-md shadow-md">
        {/* Filters Toggle Button */}
        <button
          type="button"
          onClick={() => setIsFiltersOpen((v) => !v)}
          aria-expanded={isFiltersOpen}
          aria-controls="graph-filters-panel"
          className={cn(
            "flex items-center gap-1.5 px-2.5 py-1 rounded-sm text-xs font-medium transition-all cursor-pointer select-none",
            "focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-focus-ring",
            isFiltersOpen
              ? "bg-primary text-primary-foreground font-semibold shadow-xs"
              : "text-foreground hover:bg-muted"
          )}
          title={isFiltersOpen ? "Collapse filters panel (Esc)" : "Expand filters panel"}
        >
          <Filter className={cn("size-3.5", isFiltersOpen ? "text-primary-foreground" : "text-primary")} />
          <span>Filters</span>
          {activeFilterCount > 0 && (
            <span
              className={cn(
                "font-mono text-2xs px-1 py-0.2 rounded-xs font-semibold",
                isFiltersOpen
                  ? "bg-primary-foreground/20 text-primary-foreground"
                  : "bg-primary-muted text-primary"
              )}
            >
              {activeFilterCount}
            </span>
          )}
          <ChevronDown
            className={cn(
              "size-3 opacity-70 transition-transform duration-200",
              isFiltersOpen && "rotate-180"
            )}
          />
        </button>

        <div className="w-px h-4 bg-border" aria-hidden />

        {/* Layout Toggle Button */}
        <button
          type="button"
          onClick={() => setIsLayoutOpen((v) => !v)}
          aria-expanded={isLayoutOpen}
          aria-controls="graph-layout-panel"
          className={cn(
            "flex items-center gap-1.5 px-2.5 py-1 rounded-sm text-xs font-medium transition-all cursor-pointer select-none",
            "focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-focus-ring",
            isLayoutOpen
              ? "bg-primary text-primary-foreground font-semibold shadow-xs"
              : "text-foreground hover:bg-muted"
          )}
          title={isLayoutOpen ? "Collapse layout panel (Esc)" : "Expand layout panel"}
        >
          <LayoutGrid className={cn("size-3.5", isLayoutOpen ? "text-primary-foreground" : "text-primary")} />
          <span>Layout</span>
          <span
            className={cn(
              "font-mono text-2xs px-1.5 py-0.2 rounded-xs font-semibold uppercase",
              isLayoutOpen
                ? "bg-primary-foreground/20 text-primary-foreground"
                : "bg-muted text-muted-foreground"
            )}
          >
            {layoutEngine === "hierarchical" ? "ELK" : layoutEngine}
          </span>
          <ChevronDown
            className={cn(
              "size-3 opacity-70 transition-transform duration-200",
              isLayoutOpen && "rotate-180"
            )}
          />
        </button>

        <div className="w-px h-4 bg-border" aria-hidden />

        {/* Interactive Search Bar directly in toolbar */}
        <div ref={searchContainerRef} className="relative">
          <Search
            className="size-3.5 text-muted-foreground absolute left-2.5 top-1/2 -translate-y-1/2 pointer-events-none"
            aria-hidden
          />
          <Input
            type="text"
            value={searchHighlight}
            onChange={(e) => {
              setSearchHighlight(e.target.value);
              setIsSearchDropdownOpen(true);
              setActiveResultIndex(0);
            }}
            onFocus={() => {
              if (searchHighlight.trim().length > 0) {
                setIsSearchDropdownOpen(true);
              }
            }}
            onKeyDown={(e) => {
              if (e.key === "ArrowDown") {
                e.preventDefault();
                setIsSearchDropdownOpen(true);
                setActiveResultIndex((prev) =>
                  prev < searchMatches.length - 1 ? prev + 1 : 0
                );
              } else if (e.key === "ArrowUp") {
                e.preventDefault();
                setIsSearchDropdownOpen(true);
                setActiveResultIndex((prev) =>
                  prev > 0 ? prev - 1 : searchMatches.length - 1
                );
              } else if (e.key === "Enter") {
                e.preventDefault();
                if (searchMatches.length > 0) {
                  const target =
                    activeResultIndex >= 0 && activeResultIndex < searchMatches.length
                      ? searchMatches[activeResultIndex].action
                      : searchMatches[0].action;
                  handleSelectSearchResult(target);
                }
              } else if (e.key === "Escape") {
                setIsSearchDropdownOpen(false);
              }
            }}
            placeholder="Search nodes, files, symbols…"
            aria-label="Search and select matching nodes"
            className="pl-8 pr-7 h-7 w-44 sm:w-64 text-xs font-mono transition-all duration-200 focus:w-64 sm:focus:w-80"
          />
          {searchHighlight && (
            <button
              type="button"
              onClick={() => {
                setSearchHighlight("");
                setIsSearchDropdownOpen(false);
              }}
              className="absolute right-2 top-1/2 -translate-y-1/2 text-muted-foreground hover:text-foreground cursor-pointer p-0.5 rounded-sm hover:bg-muted transition-colors"
              title="Clear search"
            >
              <X className="size-3" />
            </button>
          )}

          {/* Interactive Search Results Dropdown */}
          {isSearchDropdownOpen && searchMatches.length > 0 && (
            <div className="absolute top-full left-0 mt-1.5 w-80 sm:w-96 max-h-80 overflow-y-auto bg-card/95 border border-border/80 rounded-lg shadow-2xl backdrop-blur-md z-50 py-1 divide-y divide-border/40 animate-in fade-in zoom-in-95 duration-150">
              <div className="px-2.5 py-1 text-3xs font-mono text-muted-foreground flex items-center justify-between">
                <span>Matching Nodes ({searchMatches.length})</span>
                <span className="text-3xs">Use ↑↓ to navigate • Enter to jump</span>
              </div>
              <div className="py-0.5">
                {searchMatches.map((match, idx) => {
                  const isSelected = idx === activeResultIndex;
                  return (
                    <button
                      key={match.action.id}
                      type="button"
                      onMouseEnter={() => setActiveResultIndex(idx)}
                      onClick={() => handleSelectSearchResult(match.action)}
                      className={cn(
                        "w-full text-left px-3 py-2 flex flex-col gap-1 transition-colors cursor-pointer group",
                        isSelected
                          ? "bg-primary/10 text-foreground"
                          : "hover:bg-muted/60 text-muted-foreground hover:text-foreground"
                      )}
                    >
                      <div className="flex items-center gap-1.5 flex-wrap">
                        <ActionBadge
                          type={match.action.action_type}
                          className="text-3xs px-1 py-0"
                        />
                        <ActorBadge
                          actor={match.action.actor}
                          className="text-3xs px-1 py-0"
                        />
                        {match.action.significance === "major" && (
                          <span className="text-[9px] font-mono uppercase bg-warning/15 text-warning px-1 rounded-xs border border-warning/30">
                            ADR
                          </span>
                        )}
                        {match.matchField !== "summary" && match.matchDetail && (
                          <span className="inline-flex items-center gap-1 text-3xs font-mono bg-background/80 text-primary border border-border px-1.5 py-0.5 rounded-xs ml-auto truncate max-w-[140px]">
                            {match.matchField === "file" && <FileCode className="size-2.5 shrink-0" />}
                            {match.matchField === "symbol" && <Zap className="size-2.5 shrink-0" />}
                            {match.matchField === "tag" && <Tag className="size-2.5 shrink-0" />}
                            <span className="truncate">{match.matchDetail}</span>
                          </span>
                        )}
                      </div>
                      <p className="text-xs line-clamp-2 font-medium text-foreground group-hover:text-primary transition-colors">
                        {match.action.summary}
                      </p>
                    </button>
                  );
                })}
              </div>
            </div>
          )}
        </div>

        {(isLayouting || isLoading) && (
          <RefreshCw
            className="size-3 text-primary animate-spin ml-1 mr-0.5"
            aria-label="Layout in progress"
          />
        )}
      </div>

      {/* Floating Panels (Independent & Non-overlapping) */}
      {(isFiltersOpen || isLayoutOpen) && (
        <div className="flex flex-col md:flex-row items-start gap-2.5 max-w-[calc(100vw-1.5rem)] motion-safe:animate-in motion-safe:fade-in-50 motion-safe:duration-150">
          {/* 1. Graph Filters Panel */}
          {isFiltersOpen && (
            <div
              id="graph-filters-panel"
              className="pointer-events-auto bg-card/95 backdrop-blur-md border border-border rounded-md p-3 shadow-xl flex flex-col gap-2.5 w-72 sm:w-80 max-w-full"
              role="region"
              aria-label="Filter controls"
            >
              {/* Panel Header */}
              <div className="flex items-center justify-between pb-1.5 border-b border-border">
                <div className="flex items-center gap-1.5">
                  <Filter className="size-3.5 text-primary" />
                  <span className="text-xs font-semibold text-foreground">Graph Filters</span>
                  {activeFilterCount > 0 && (
                    <span className="font-mono text-2xs text-primary bg-primary-muted px-1.5 py-0.2 rounded-sm font-semibold">
                      {activeFilterCount} active
                    </span>
                  )}
                </div>
                <div className="flex items-center gap-1">
                  {activeFilterCount > 0 && (
                    <button
                      type="button"
                      onClick={handleResetFilters}
                      className="text-2xs text-muted-foreground hover:text-foreground hover:underline cursor-pointer px-1 py-0.5"
                    >
                      Reset
                    </button>
                  )}
                  <button
                    type="button"
                    onClick={() => setIsFiltersOpen(false)}
                    className="size-5 rounded flex items-center justify-center text-muted-foreground hover:text-foreground hover:bg-muted cursor-pointer"
                    title="Close panel"
                    aria-label="Close filters panel"
                  >
                    <X className="size-3" />
                  </button>
                </div>
              </div>

              {/* Dropdowns Grid */}
              <div className="grid grid-cols-2 gap-2">
                <div className="flex flex-col gap-1">
                  <label className="text-2xs text-muted-foreground font-medium uppercase tracking-wider">
                    Actor
                  </label>
                  <select
                    value={actorFilter}
                    onChange={(e) => setActorFilter(e.target.value)}
                    className={cn(selectClass, "w-full truncate text-xs h-7.5")}
                    aria-label="Filter by actor"
                  >
                    <option value="all">All Actors</option>
                    {availableActors.map((actor) => (
                      <option key={actor} value={actor}>
                        {actor}
                      </option>
                    ))}
                  </select>
                </div>

                <div className="flex flex-col gap-1">
                  <label className="text-2xs text-muted-foreground font-medium uppercase tracking-wider">
                    Type
                  </label>
                  <select
                    value={typeFilter}
                    onChange={(e) => setTypeFilter(e.target.value)}
                    className={cn(selectClass, "w-full truncate text-xs h-7.5")}
                    aria-label="Filter by type"
                  >
                    <option value="all">All Types</option>
                    {availableTypes.map((type) => (
                      <option key={type} value={type}>
                        {type}
                      </option>
                    ))}
                  </select>
                </div>
              </div>

              {/* Checkbox Toggles */}
              <div className="flex items-center justify-between pt-1 border-t border-border/60">
                <label className="flex items-center gap-1.5 text-xs text-muted-foreground hover:text-foreground cursor-pointer select-none">
                  <input
                    type="checkbox"
                    checked={currentOnly}
                    onChange={(e) => setCurrentOnly(e.target.checked)}
                    className="rounded border-border accent-primary cursor-pointer size-3.5"
                  />
                  <span>Current Branch</span>
                </label>

                <label className="flex items-center gap-1 text-xs text-muted-foreground hover:text-foreground cursor-pointer select-none">
                  <input
                    type="checkbox"
                    checked={showFileNodes}
                    onChange={(e) => setShowFileNodes(e.target.checked)}
                    className="rounded border-border accent-primary cursor-pointer size-3.5"
                  />
                  <FileCode className="size-3 text-plan" aria-hidden />
                  <span>File Nodes</span>
                </label>
              </div>
            </div>
          )}

          {/* 2. Graph Layout Panel */}
          {isLayoutOpen && (
            <div
              id="graph-layout-panel"
              className="pointer-events-auto bg-card/95 backdrop-blur-md border border-border rounded-md p-3 shadow-xl flex flex-col gap-3 w-80 sm:w-88 max-w-full"
              role="region"
              aria-label="Layout and view controls"
            >
              {/* Panel Header */}
              <div className="flex items-center justify-between pb-1.5 border-b border-border">
                <div className="flex items-center gap-1.5">
                  <LayoutGrid className="size-3.5 text-primary" />
                  <span className="text-xs font-semibold text-foreground">Graph Layout & Display</span>
                </div>
                <button
                  type="button"
                  onClick={() => setIsLayoutOpen(false)}
                  className="size-5 rounded flex items-center justify-center text-muted-foreground hover:text-foreground hover:bg-muted cursor-pointer"
                  title="Close panel"
                  aria-label="Close layout panel"
                >
                  <X className="size-3" />
                </button>
              </div>

              {/* View Mode Toggle: Cards vs Cosmos */}
              <div className="flex flex-col gap-1.5">
                <span className="text-2xs text-muted-foreground font-medium uppercase tracking-wider">
                  Node View Mode
                </span>
                <div className="grid grid-cols-2 gap-1.5 bg-muted/60 p-1 rounded-sm border border-border">
                  <button
                    type="button"
                    onClick={() => handleToggleViewMode("cards")}
                    className={cn(
                      "flex items-center justify-center gap-1.5 py-1 px-2 rounded-xs text-xs font-medium transition-colors cursor-pointer",
                      viewMode === "cards"
                        ? "bg-card text-foreground shadow-xs font-semibold"
                        : "text-muted-foreground hover:text-foreground"
                    )}
                    title="Structured card nodes view"
                    aria-pressed={viewMode === "cards"}
                  >
                    <LayoutDashboard className="size-3.5 text-primary" />
                    <span>Cards</span>
                  </button>
                  <button
                    type="button"
                    onClick={() => handleToggleViewMode("cosmos")}
                    className={cn(
                      "flex items-center justify-center gap-1.5 py-1 px-2 rounded-xs text-xs font-medium transition-colors cursor-pointer",
                      viewMode === "cosmos"
                        ? "bg-primary text-primary-foreground shadow-xs font-semibold"
                        : "text-muted-foreground hover:text-foreground"
                    )}
                    title="Cosmos knowledge graph orb view"
                    aria-pressed={viewMode === "cosmos"}
                  >
                    <Orbit className="size-3.5" />
                    <span>Cosmos</span>
                  </button>
                </div>
              </div>

              {/* Live Physics Toggle */}
              <div className="flex flex-col gap-1.5">
                <span className="text-2xs text-muted-foreground font-medium uppercase tracking-wider">
                  Force Physics Engine
                </span>
                <div className="flex items-center gap-1.5">
                  <button
                    type="button"
                    onClick={handleTogglePhysics}
                    aria-pressed={livePhysics}
                    className={cn(
                      "flex-1 flex items-center justify-center gap-1.5 py-1 px-2 rounded-xs text-xs font-medium border transition-all cursor-pointer",
                      livePhysics
                        ? "bg-warning/20 text-warning border-warning/40 shadow-[0_0_10px_rgba(245,158,11,0.2)] font-semibold"
                        : "border-border text-muted-foreground hover:text-foreground hover:bg-muted"
                    )}
                  >
                    <Atom className={cn("size-3.5", livePhysics && isPhysicsRunning && "animate-spin text-warning")} />
                    <span>{livePhysics ? "Live Physics Running" : "Physics Off"}</span>
                  </button>
                  {livePhysics && (
                    <Button
                      type="button"
                      variant="outline"
                      size="sm"
                      className="h-7 text-xs text-warning hover:text-warning hover:bg-warning/10 cursor-pointer px-2"
                      onClick={() => reheat(0.55)}
                      title="Shake simulation"
                      aria-label="Shake simulation"
                    >
                      <Flame className="size-3 text-warning mr-1" />
                      <span>Shake</span>
                    </Button>
                  )}
                </div>
              </div>

              {/* Layout Engines */}
              <div className="flex flex-col gap-1.5">
                <span className="text-2xs text-muted-foreground font-medium uppercase tracking-wider">
                  Layout Algorithm
                </span>
                <div className="grid grid-cols-2 gap-1.5">
                  {LAYOUTS.map((layout) => (
                    <button
                      key={layout.id}
                      type="button"
                      onClick={() => setLayoutEngine(layout.id)}
                      title={layout.title}
                      aria-pressed={layoutEngine === layout.id}
                      className={cn(
                        "flex items-center gap-1.5 px-2 py-1.5 rounded-sm text-xs font-medium border transition-colors cursor-pointer",
                        layoutEngine === layout.id
                          ? "bg-primary-muted border-primary/40 text-primary font-semibold shadow-2xs"
                          : "border-border text-muted-foreground hover:text-foreground hover:bg-muted"
                      )}
                    >
                      {layout.icon}
                      <span>{layout.label}</span>
                    </button>
                  ))}
                </div>
              </div>

              {/* Canvas Display Actions */}
              <div className="flex flex-col gap-1.5 pt-1 border-t border-border">
                <span className="text-2xs text-muted-foreground font-medium uppercase tracking-wider">
                  Display & Helpers
                </span>
                <div className="grid grid-cols-4 gap-1.5">
                  <Button
                    type="button"
                    variant="outline"
                    size="sm"
                    className="h-8 flex flex-col items-center justify-center p-1 gap-0.5 text-2xs cursor-pointer"
                    onClick={handleFitView}
                    title="Fit to view"
                    aria-label="Fit graph to view"
                  >
                    <Maximize2 className="size-3.5" />
                    <span>Fit</span>
                  </Button>

                  <Button
                    type="button"
                    variant="outline"
                    size="sm"
                    className="h-8 flex flex-col items-center justify-center p-1 gap-0.5 text-2xs cursor-pointer"
                    onClick={handleRelayout}
                    title="Re-run layout"
                    aria-label="Re-run layout"
                    disabled={isLayouting || nodesLength === 0}
                  >
                    <RotateCcw
                      className={cn("size-3.5", isLayouting && "animate-spin text-primary")}
                    />
                    <span>Relayout</span>
                  </Button>

                  <Button
                    type="button"
                    variant="outline"
                    size="sm"
                    className={cn(
                      "h-8 flex flex-col items-center justify-center p-1 gap-0.5 text-2xs cursor-pointer",
                      showLegend && "bg-primary-muted border-primary/40 text-primary font-semibold"
                    )}
                    onClick={handleToggleLegend}
                    title={showLegend ? "Hide legend" : "Show legend"}
                    aria-label={showLegend ? "Hide legend" : "Show legend"}
                    aria-pressed={showLegend}
                  >
                    <Eye className="size-3.5" />
                    <span>Legend</span>
                  </Button>

                  <Button
                    type="button"
                    variant="outline"
                    size="sm"
                    className={cn(
                      "h-8 flex flex-col items-center justify-center p-1 gap-0.5 text-2xs cursor-pointer",
                      showMinimap && "bg-primary-muted border-primary/40 text-primary font-semibold"
                    )}
                    onClick={handleToggleMinimap}
                    title={showMinimap ? "Hide minimap" : "Show minimap"}
                    aria-label={showMinimap ? "Hide minimap" : "Show minimap"}
                    aria-pressed={showMinimap}
                  >
                    <MapIcon className="size-3.5" />
                    <span>Minimap</span>
                  </Button>
                </div>
              </div>
            </div>
          )}
        </div>
      )}
    </div>
  );
};

