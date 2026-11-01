import React, { useState, useEffect, useRef } from "react";
import {
  Search,
  Filter,
  FileText,
  CheckCircle2,
  AlertCircle,
  RotateCcw,
  X,
  HelpCircle,
  Sparkles,
  MessageSquare,
  GitCommit,
} from "lucide-react";
import { KnoxAPI } from "@/services/api";
import { useKnoxStore } from "@/store/useKnoxStore";
import { ActionBadge } from "@/components/ui/ActionBadge";
import { ActorBadge } from "@/components/ui/ActorBadge";
import { Button } from "@/components/ui/Button";
import { Input } from "@/components/ui/Input";
import { FilterBar } from "@/components/ui/FilterBar";
import { EmptyState } from "@/components/ui/EmptyState";
import { SkeletonRow } from "@/components/ui/Skeleton";
import { cn } from "@/lib/cn";
import type { KnoxAction } from "@/types/knox";

const selectClass =
  "h-8 bg-background border border-border rounded-md px-2.5 text-xs text-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-focus-ring cursor-pointer font-mono";

export const QueryView: React.FC = () => {
  const {
    actions,
    status,
    selectedAction,
    setSelectedAction,
    setActiveTab,
    showToast,
  } = useKnoxStore();

  const [searchInput, setSearchInput] = useState("");
  const [filePath, setFilePath] = useState("");
  const [actorFilter, setActorFilter] = useState("");
  const [typeFilter, setTypeFilter] = useState("");
  const [sinceFilter, setSinceFilter] = useState("");
  const [currentOnly, setCurrentOnly] = useState(true);
  const [limit, setLimit] = useState(50);
  const [results, setResults] = useState<KnoxAction[]>(actions);
  const [isSearching, setIsSearching] = useState(false);
  const [selectedIndex, setSelectedIndex] = useState<number>(-1);
  const [hasQueried, setHasQueried] = useState(false);
  const [searchError, setSearchError] = useState<string | null>(null);
  const [activeFacet, setActiveFacet] = useState<"all" | "doubts" | "major" | "minor" | "dialogue">("all");

  const doubtsCount = results.filter(
    (a) => (a.action_type === "doubt" || a.action_type === "question") && !a.is_resolved
  ).length;
  const majorCount = results.filter(
    (a) => a.action_type === "decide" && a.significance !== "minor"
  ).length;
  const minorCount = results.filter((a) => a.significance === "minor").length;
  const dialogueCount = results.filter(
    (a) => a.action_type === "dialogue" || a.action_type === "conversation"
  ).length;

  const displayedResults = results.filter((a) => {
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

  const searchInputRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    searchInputRef.current?.focus();
  }, []);

  useEffect(() => {
    if (!searchInput && !filePath && !actorFilter && !typeFilter && !sinceFilter) {
      setResults(actions);
      setHasQueried(false);
      setSearchError(null);
    }
  }, [actions, searchInput, filePath, actorFilter, typeFilter, sinceFilter]);

  const handleSearch = async (e?: React.FormEvent) => {
    if (e) e.preventDefault();
    try {
      setIsSearching(true);
      setSearchError(null);
      const res = await KnoxAPI.queryActions({
        search: searchInput.trim() || undefined,
        filePath: filePath.trim() || undefined,
        actor: actorFilter || undefined,
        actionType: typeFilter || undefined,
        since: sinceFilter || undefined,
        currentOnly,
        limit,
      });
      setResults(res.results);
      setSelectedIndex(-1);
      setHasQueried(true);
      showToast(`Found ${res.results.length} matched actions.`);
    } catch (err) {
      const msg = String(err);
      setSearchError(msg);
      showToast(msg, true);
    } finally {
      setIsSearching(false);
    }
  };

  const handleResetFilters = () => {
    setSearchInput("");
    setFilePath("");
    setActorFilter("");
    setTypeFilter("");
    setSinceFilter("");
    setCurrentOnly(true);
    setLimit(50);
    setActiveFacet("all");
    setResults(actions);
    setSelectedIndex(-1);
    setHasQueried(false);
    setSearchError(null);
    searchInputRef.current?.focus();
  };

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (displayedResults.length === 0) return;

    if (e.key === "ArrowDown") {
      e.preventDefault();
      setSelectedIndex((prev) => {
        const next = prev < 0 ? 0 : Math.min(prev + 1, displayedResults.length - 1);
        if (displayedResults[next]) setSelectedAction(displayedResults[next]);
        return next;
      });
    } else if (e.key === "ArrowUp") {
      e.preventDefault();
      setSelectedIndex((prev) => {
        const next = prev < 0 ? displayedResults.length - 1 : Math.max(prev - 1, 0);
        if (displayedResults[next]) setSelectedAction(displayedResults[next]);
        return next;
      });
    } else if (e.key === "Enter" && selectedIndex >= 0 && displayedResults[selectedIndex]) {
      e.preventDefault();
      setSelectedAction(displayedResults[selectedIndex]);
    } else if (e.key === "Escape") {
      if (selectedAction) {
        setSelectedAction(null);
        return;
      }
      handleResetFilters();
    }
  };

  const knownActors = Array.from(
    new Set([
      "gemini",
      "cursor",
      "antigravity",
      "human",
      "copilot",
      ...Object.keys(status?.actions_by_actor || {}),
    ])
  );

  const knownTypes = Array.from(
    new Set([
      "write",
      "plan",
      "decide",
      "debug",
      ...Object.keys(status?.actions_by_type || {}),
    ])
  );

  const activeFilterCount = [
    searchInput,
    filePath,
    actorFilter,
    typeFilter,
    sinceFilter,
    !currentOnly ? "all" : "",
  ].filter(Boolean).length;

  const emptyIsNoData = !hasQueried && actions.length === 0;

  return (
    <div
      onKeyDown={handleKeyDown}
      className="flex h-full w-full overflow-hidden"
      role="search"
      aria-label="Knowledge graph search"
    >
      <div className="flex-1 flex flex-col min-w-0 overflow-hidden">
        <form onSubmit={handleSearch}>
          <FilterBar
            primary={
              <>
                <div className="relative flex-1 min-w-[12rem]">
                  <Search
                    className="size-4 text-muted-foreground absolute left-3 top-1/2 -translate-y-1/2 pointer-events-none"
                    aria-hidden
                  />
                  <Input
                    ref={searchInputRef}
                    type="text"
                    value={searchInput}
                    onChange={(e) => setSearchInput(e.target.value)}
                    placeholder="Search actions, files, decisions (FTS5)…"
                    aria-label="Full-text search"
                    className="pl-9 pr-8 h-9"
                  />
                  {searchInput && (
                    <button
                      type="button"
                      onClick={() => setSearchInput("")}
                      className="absolute right-2 top-1/2 -translate-y-1/2 p-1 text-muted-foreground hover:text-foreground rounded-sm cursor-pointer"
                      aria-label="Clear search text"
                    >
                      <X className="size-3.5" />
                    </button>
                  )}
                </div>

                <Button
                  type="submit"
                  variant="primary"
                  size="md"
                  disabled={isSearching}
                  loading={isSearching}
                >
                  {!isSearching && <Filter className="size-3.5" />}
                  {isSearching ? "Searching…" : "Search"}
                </Button>

                <Button
                  type="button"
                  variant="outline"
                  size="icon"
                  onClick={handleResetFilters}
                  title="Reset all filters"
                  aria-label="Reset all filters"
                >
                  <RotateCcw className="size-3.5" />
                </Button>
              </>
            }
            secondary={
              <>
                <div className="relative flex-1 sm:flex-initial">
                  <FileText
                    className="size-3 text-muted-foreground absolute left-2.5 top-1/2 -translate-y-1/2 pointer-events-none"
                    aria-hidden
                  />
                  <Input
                    type="text"
                    value={filePath}
                    onChange={(e) => setFilePath(e.target.value)}
                    placeholder="File path…"
                    aria-label="Filter by file path"
                    className="pl-7 sm:w-44"
                  />
                </div>

                <select
                  value={actorFilter}
                  onChange={(e) => setActorFilter(e.target.value)}
                  className={selectClass}
                  aria-label="Filter by actor"
                >
                  <option value="">All Actors</option>
                  {knownActors.map((act) => (
                    <option key={act} value={act}>
                      {act}
                    </option>
                  ))}
                </select>

                <select
                  value={typeFilter}
                  onChange={(e) => setTypeFilter(e.target.value)}
                  className={selectClass}
                  aria-label="Filter by action type"
                >
                  <option value="">All Types</option>
                  {knownTypes.map((typ) => (
                    <option key={typ} value={typ}>
                      {typ}
                    </option>
                  ))}
                </select>

                <select
                  value={sinceFilter}
                  onChange={(e) => setSinceFilter(e.target.value)}
                  className={selectClass}
                  aria-label="Filter by time range"
                >
                  <option value="">All Time</option>
                  <option
                    value={new Date(Date.now() - 3600 * 1000).toISOString()}
                  >
                    Past 1 Hour
                  </option>
                  <option
                    value={new Date(Date.now() - 24 * 3600 * 1000).toISOString()}
                  >
                    Past 24 Hours
                  </option>
                  <option
                    value={new Date(
                      Date.now() - 7 * 24 * 3600 * 1000
                    ).toISOString()}
                  >
                    Past 7 Days
                  </option>
                  <option
                    value={new Date(
                      Date.now() - 30 * 24 * 3600 * 1000
                    ).toISOString()}
                  >
                    Past 30 Days
                  </option>
                </select>

                <label className="flex items-center gap-1.5 text-muted-foreground text-xs cursor-pointer hover:text-foreground transition-colors ml-auto">
                  <input
                    type="checkbox"
                    checked={currentOnly}
                    onChange={(e) => setCurrentOnly(e.target.checked)}
                    className="rounded border-border accent-primary cursor-pointer"
                  />
                  <span>Current only</span>
                </label>

                <select
                  value={limit}
                  onChange={(e) => setLimit(Number(e.target.value))}
                  className={selectClass}
                  aria-label="Result limit"
                >
                  <option value={20}>20 rows</option>
                  <option value={50}>50 rows</option>
                  <option value={100}>100 rows</option>
                  <option value={200}>200 rows</option>
                </select>
              </>
            }
            meta={
              <>
                <div className="flex items-center gap-2 min-w-0">
                  <span>
                    Showing{" "}
                    <span className="font-semibold text-foreground">
                      {displayedResults.length}
                    </span>{" "}
                    {displayedResults.length === 1 ? "action" : "actions"}
                    {displayedResults.length !== results.length && (
                      <span className="text-muted-foreground text-xs ml-1 font-mono">
                        (filtered from {results.length})
                      </span>
                    )}
                  </span>
                  {activeFilterCount > 0 && (
                    <span className="text-primary bg-primary-muted px-1.5 py-0.5 rounded-sm border border-primary/20">
                      {activeFilterCount} filter
                      {activeFilterCount === 1 ? "" : "s"}
                    </span>
                  )}
                  {searchInput && (
                    <span className="text-primary truncate max-w-[12rem]">
                      “{searchInput}”
                    </span>
                  )}
                </div>
                <span className="text-xs text-muted-foreground/70 hidden sm:inline shrink-0">
                  ↑↓ navigate · Enter inspect · Esc clear
                </span>
              </>
            }
          />
        </form>

        {searchError && (
          <div
            role="alert"
            className="mx-4 mt-3 px-3 py-2 rounded-md border border-danger/40 bg-danger/10 text-xs text-danger flex items-center gap-2"
          >
            <AlertCircle className="size-3.5 shrink-0" aria-hidden />
            <span className="min-w-0 flex-1">{searchError}</span>
            <Button
              type="button"
              variant="ghost"
              size="sm"
              onClick={() => setSearchError(null)}
              aria-label="Dismiss error"
            >
              Dismiss
            </Button>
          </div>
        )}

        {/* Cognitive Memory Facet Chips */}
        <div className="px-4 py-2 border-b border-border bg-card/40 flex items-center gap-1.5 overflow-x-auto text-xs shrink-0 select-none">
          <span className="text-2xs uppercase tracking-wider text-muted-foreground font-mono mr-1">
            Focus:
          </span>
          <button
            type="button"
            onClick={() => {
              setActiveFacet("all");
              setSelectedIndex(-1);
            }}
            className={cn(
              "px-2.5 py-1 rounded-full text-xs font-medium border transition-all cursor-pointer",
              activeFacet === "all"
                ? "bg-primary text-primary-foreground border-primary shadow-xs"
                : "bg-muted/60 text-muted-foreground border-border hover:text-foreground"
            )}
          >
            All ({results.length})
          </button>
          <button
            type="button"
            onClick={() => {
              setActiveFacet("doubts");
              setSelectedIndex(-1);
            }}
            className={cn(
              "px-2.5 py-1 rounded-full text-xs font-medium border transition-all cursor-pointer flex items-center gap-1.5",
              activeFacet === "doubts"
                ? "bg-amber-500 text-white border-amber-600 shadow-xs"
                : "bg-amber-500/10 text-amber-700 dark:text-amber-400 border-amber-500/30 hover:bg-amber-500/20"
            )}
          >
            <HelpCircle className="size-3" />
            <span>Open Doubts ({doubtsCount})</span>
          </button>
          <button
            type="button"
            onClick={() => {
              setActiveFacet("major");
              setSelectedIndex(-1);
            }}
            className={cn(
              "px-2.5 py-1 rounded-full text-xs font-medium border transition-all cursor-pointer flex items-center gap-1.5",
              activeFacet === "major"
                ? "bg-warning text-black border-warning shadow-xs font-semibold"
                : "bg-warning/10 text-amber-800 dark:text-warning border-warning/30 hover:bg-warning/20"
            )}
          >
            <Sparkles className="size-3" />
            <span>Major ADRs ({majorCount})</span>
          </button>
          <button
            type="button"
            onClick={() => {
              setActiveFacet("minor");
              setSelectedIndex(-1);
            }}
            className={cn(
              "px-2.5 py-1 rounded-full text-xs font-medium border transition-all cursor-pointer flex items-center gap-1.5",
              activeFacet === "minor"
                ? "bg-muted text-foreground border-primary/50 shadow-xs font-semibold"
                : "bg-muted/60 text-muted-foreground border-border hover:text-foreground"
            )}
          >
            <GitCommit className="size-3" />
            <span>Minor Choices ({minorCount})</span>
          </button>
          <button
            type="button"
            onClick={() => {
              setActiveFacet("dialogue");
              setSelectedIndex(-1);
            }}
            className={cn(
              "px-2.5 py-1 rounded-full text-xs font-medium border transition-all cursor-pointer flex items-center gap-1.5",
              activeFacet === "dialogue"
                ? "bg-cyan-600 text-white border-cyan-700 shadow-xs"
                : "bg-cyan-500/10 text-cyan-700 dark:text-cyan-400 border-cyan-500/30 hover:bg-cyan-500/20"
            )}
          >
            <MessageSquare className="size-3" />
            <span>Conversations ({dialogueCount})</span>
          </button>
        </div>
        <div
          className="flex-1 overflow-y-auto"
          role="listbox"
          aria-label="Search results"
        >
          {isSearching ? (
            <div className="p-4 space-y-2">
              {Array.from({ length: 6 }).map((_, i) => (
                <SkeletonRow key={i} />
              ))}
            </div>
          ) : results.length === 0 ? (
            <div className="p-6">
              <EmptyState
                icon={<Search />}
                title={
                  emptyIsNoData
                    ? "No actions in the knowledge graph"
                    : "No matching actions"
                }
                description={
                  emptyIsNoData
                    ? "Ingest a transcript or record actions via MCP, then search here."
                    : "Nothing matched your query or filters. Clear filters or broaden the search text."
                }
                action={
                  emptyIsNoData ? (
                    <div className="flex gap-2 justify-center">
                      <Button
                        type="button"
                        variant="primary"
                        size="sm"
                        onClick={() => setActiveTab("ingest")}
                      >
                        Ingest Transcript
                      </Button>
                      <Button
                        type="button"
                        variant="outline"
                        size="sm"
                        onClick={() => setActiveTab("mcp")}
                      >
                        Configure MCP
                      </Button>
                    </div>
                  ) : (
                    <Button
                      type="button"
                      variant="outline"
                      size="sm"
                      onClick={handleResetFilters}
                    >
                      Clear all filters
                    </Button>
                  )
                }
              />
            </div>
          ) : displayedResults.length === 0 ? (
            <div className="p-6">
              <EmptyState
                compact
                icon={<Filter />}
                title={
                  activeFacet === "doubts"
                    ? "No open doubts"
                    : activeFacet === "major"
                    ? "No major decisions (ADRs)"
                    : activeFacet === "minor"
                    ? "No minor choices"
                    : activeFacet === "dialogue"
                    ? "No conversation actions"
                    : "No matching actions"
                }
                description={`There are ${results.length} total actions in this query, but none match the "${activeFacet}" focus filter.`}
                action={
                  <Button
                    type="button"
                    variant="outline"
                    size="sm"
                    onClick={() => {
                      setActiveFacet("all");
                      setSelectedIndex(-1);
                    }}
                  >
                    Show all {results.length} actions
                  </Button>
                }
              />
            </div>
          ) : (
            <div className="divide-y divide-border-subtle">
              {displayedResults.map((action, idx) => {
                const isSelected =
                  selectedAction?.id === action.id || selectedIndex === idx;

                return (
                  <div
                    key={action.id}
                    role="option"
                    aria-selected={isSelected}
                    tabIndex={0}
                    onClick={() => {
                      setSelectedAction(action);
                      setSelectedIndex(idx);
                    }}
                    onKeyDown={(e) => {
                      if (e.key === "Enter" || e.key === " ") {
                        e.preventDefault();
                        setSelectedAction(action);
                        setSelectedIndex(idx);
                      }
                    }}
                    className={cn(
                      "p-3.5 transition-colors cursor-pointer flex items-start gap-3 border-l-2",
                      "focus-visible:outline-none focus-visible:bg-primary-muted",
                      isSelected
                        ? "bg-primary-muted border-primary"
                        : "border-transparent hover:bg-muted/50"
                    )}
                  >
                    <div className="space-y-1.5 min-w-0 flex-1">
                      <div className="flex items-center gap-2 flex-wrap">
                        <ActionBadge type={action.action_type} />
                        <ActorBadge actor={action.actor} />
                        {action.is_current ? (
                          <span className="flex items-center gap-1 text-xs text-success font-mono">
                            <CheckCircle2 className="size-3" aria-hidden />
                            CURRENT
                          </span>
                        ) : (
                          <span className="flex items-center gap-1 text-xs text-warning font-mono">
                            <AlertCircle className="size-3" aria-hidden />
                            SUPERSEDED
                          </span>
                        )}
                        <span className="text-sm font-mono text-muted-foreground ml-auto">
                          {new Date(action.timestamp).toLocaleString([], {
                            month: "short",
                            day: "numeric",
                            hour: "2-digit",
                            minute: "2-digit",
                            second: "2-digit",
                          })}
                        </span>
                      </div>

                      <p className="text-xs text-foreground font-medium leading-normal">
                        {action.summary}
                      </p>

                      {action.files && action.files.length > 0 && (
                        <div className="flex items-center gap-1.5 text-sm font-mono text-muted-foreground truncate pt-0.5">
                          <FileText className="size-3 shrink-0" aria-hidden />
                          <span className="text-foreground/80 truncate">
                            {action.files[0]}
                          </span>
                          {action.files.length > 1 && (
                            <span className="text-xs bg-muted px-1 py-0.5 rounded-sm text-muted-foreground shrink-0">
                              +{action.files.length - 1} more
                            </span>
                          )}
                        </div>
                      )}
                    </div>
                  </div>
                );
              })}
            </div>
          )}
        </div>
      </div>
    </div>
  );
};
