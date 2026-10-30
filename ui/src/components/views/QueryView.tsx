import React, { useState, useEffect, useRef } from "react";
import {
  Search,
  Filter,
  FileText,
  CheckCircle2,
  AlertCircle,
  RotateCcw,
  Layers,
  Calendar,
} from "lucide-react";
import { KnoxAPI } from "@/services/api";
import { useKnoxStore } from "@/store/useKnoxStore";
import { ActionBadge } from "@/components/ui/ActionBadge";
import { ActorBadge } from "@/components/ui/ActorBadge";
import type { KnoxAction } from "@/types/knox";

export const QueryView: React.FC = () => {
  const { actions, status, selectedAction, setSelectedAction, showToast } = useKnoxStore();

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

  const searchInputRef = useRef<HTMLInputElement>(null);

  // Focus search input on view mount
  useEffect(() => {
    searchInputRef.current?.focus();
  }, []);

  // Sync results when global actions change if no custom search is active
  useEffect(() => {
    if (!searchInput && !filePath && !actorFilter && !typeFilter && !sinceFilter) {
      setResults(actions);
    }
  }, [actions, searchInput, filePath, actorFilter, typeFilter, sinceFilter]);

  const handleSearch = async (e?: React.FormEvent) => {
    if (e) e.preventDefault();
    try {
      setIsSearching(true);
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
      showToast(`Found ${res.results.length} matched actions.`);
    } catch (err) {
      showToast(String(err), true);
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
    setResults(actions);
    setSelectedIndex(-1);
    searchInputRef.current?.focus();
  };

  // Keyboard navigation across query results
  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === "ArrowDown") {
      e.preventDefault();
      setSelectedIndex((prev) => {
        const next = Math.min(prev + 1, results.length - 1);
        if (results[next]) setSelectedAction(results[next]);
        return next;
      });
    } else if (e.key === "ArrowUp") {
      e.preventDefault();
      setSelectedIndex((prev) => {
        const next = Math.max(prev - 1, 0);
        if (results[next]) setSelectedAction(results[next]);
        return next;
      });
    } else if (e.key === "Escape") {
      handleResetFilters();
    }
  };

  // Get dynamic actor and type options
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

  return (
    <div
      onKeyDown={handleKeyDown}
      className="flex h-full w-full overflow-hidden select-none"
    >
      {/* Left/Main Results Column */}
      <div className="flex-1 flex flex-col min-w-0 overflow-hidden">
        {/* Search & Filter Toolbar */}
        <form
          onSubmit={handleSearch}
          className="p-4 border-b border-border/70 bg-card/60 backdrop-blur space-y-3 shrink-0"
        >
          {/* Main Full-Width Search Input */}
          <div className="flex items-center gap-2">
            <div className="relative flex-1">
              <Search className="w-4 h-4 text-muted-foreground absolute left-3 top-2.5" />
              <input
                ref={searchInputRef}
                type="text"
                value={searchInput}
                onChange={(e) => setSearchInput(e.target.value)}
                placeholder="Search actions, files, decisions (FTS5 full-text index)…"
                className="w-full bg-background border border-border rounded-lg pl-9 pr-8 py-2 text-xs text-foreground placeholder:text-muted-foreground focus:outline-none focus:border-primary font-mono transition"
              />
              {searchInput && (
                <button
                  type="button"
                  onClick={() => setSearchInput("")}
                  className="absolute right-2.5 top-2.5 text-muted-foreground hover:text-foreground text-xs"
                >
                  ✕
                </button>
              )}
            </div>

            <button
              type="submit"
              disabled={isSearching}
              className="px-4 py-2 rounded-lg bg-primary text-primary-foreground font-medium text-xs hover:bg-primary/90 transition flex items-center gap-1.5 cursor-pointer shadow-sm shadow-primary/25 disabled:opacity-50 shrink-0"
            >
              <Filter className="w-3.5 h-3.5" />
              <span>{isSearching ? "Searching…" : "Filter"}</span>
            </button>

            <button
              type="button"
              onClick={handleResetFilters}
              title="Reset all filters (Esc)"
              className="p-2 rounded-lg border border-border bg-card hover:bg-background text-muted-foreground hover:text-foreground transition cursor-pointer shrink-0"
            >
              <RotateCcw className="w-3.5 h-3.5" />
            </button>
          </div>

          {/* Sub-Filters: File, Actor, Type, Current-Only, Limit */}
          <div className="flex flex-wrap items-center gap-2.5 text-xs">
            {/* File Path Filter */}
            <div className="relative flex-1 sm:flex-initial">
              <FileText className="w-3 h-3 text-muted-foreground absolute left-2.5 top-2" />
              <input
                type="text"
                value={filePath}
                onChange={(e) => setFilePath(e.target.value)}
                placeholder="File path filter…"
                className="bg-background border border-border rounded-md pl-7 pr-2.5 py-1 text-xs text-foreground placeholder:text-muted-foreground focus:outline-none focus:border-primary font-mono w-full sm:w-44"
              />
            </div>

            {/* Actor Select Dropdown */}
            <select
              value={actorFilter}
              onChange={(e) => setActorFilter(e.target.value)}
              className="bg-background border border-border rounded-md px-2.5 py-1 text-xs text-foreground focus:outline-none focus:border-primary cursor-pointer font-mono"
            >
              <option value="">All Actors</option>
              {knownActors.map((act) => (
                <option key={act} value={act}>
                  {act}
                </option>
              ))}
            </select>

            {/* Action Type Select Dropdown */}
            <select
              value={typeFilter}
              onChange={(e) => setTypeFilter(e.target.value)}
              className="bg-background border border-border rounded-md px-2.5 py-1 text-xs text-foreground focus:outline-none focus:border-primary cursor-pointer font-mono"
            >
              <option value="">All Types</option>
              {knownTypes.map((typ) => (
                <option key={typ} value={typ}>
                  {typ}
                </option>
              ))}
            </select>

            {/* Date Range / Since Filter */}
            <div className="relative">
              <Calendar className="w-3 h-3 text-muted-foreground absolute left-2 top-2 pointer-events-none" />
              <select
                value={sinceFilter}
                onChange={(e) => setSinceFilter(e.target.value)}
                className="bg-background border border-border rounded-md pl-6 pr-2 py-1 text-xs text-foreground focus:outline-none focus:border-primary cursor-pointer font-mono"
              >
                <option value="">All Time</option>
                <option value={new Date(Date.now() - 3600 * 1000).toISOString()}>Past 1 Hour</option>
                <option value={new Date(Date.now() - 24 * 3600 * 1000).toISOString()}>Past 24 Hours</option>
                <option value={new Date(Date.now() - 7 * 24 * 3600 * 1000).toISOString()}>Past 7 Days</option>
                <option value={new Date(Date.now() - 30 * 24 * 3600 * 1000).toISOString()}>Past 30 Days</option>
              </select>
            </div>

            {/* Current Only Toggle */}
            <label className="flex items-center gap-1.5 text-muted-foreground text-xs cursor-pointer ml-auto hover:text-foreground transition">
              <input
                type="checkbox"
                checked={currentOnly}
                onChange={(e) => setCurrentOnly(e.target.checked)}
                className="rounded border-border text-primary focus:ring-0 cursor-pointer"
              />
              <span>Current Only</span>
            </label>

            {/* Row Limit Select */}
            <select
              value={limit}
              onChange={(e) => setLimit(Number(e.target.value))}
              className="bg-background border border-border rounded-md px-2 py-1 text-xs text-foreground focus:outline-none cursor-pointer font-mono"
            >
              <option value={20}>20 rows</option>
              <option value={50}>50 rows</option>
              <option value={100}>100 rows</option>
              <option value={200}>200 rows</option>
            </select>
          </div>
        </form>

        {/* Results Metadata Bar */}
        <div className="px-4 py-2 border-b border-border/50 bg-background/50 flex items-center justify-between text-[11px] text-muted-foreground font-mono">
          <div className="flex items-center gap-2">
            <span>
              Showing <span className="font-semibold text-foreground">{results.length}</span> actions
            </span>
            {searchInput && (
              <span className="text-primary bg-primary/10 px-1.5 py-0.2 rounded border border-primary/20">
                matching "{searchInput}"
              </span>
            )}
          </div>

          <span className="text-[10px] text-muted-foreground/70 hidden sm:inline">
            Use ↑ / ↓ arrow keys to navigate rows
          </span>
        </div>

        {/* Results Virtualized/Scrollable List */}
        <div className="flex-1 overflow-y-auto divide-y divide-border/40">
          {isSearching ? (
            <div className="p-4 space-y-3">
              {Array.from({ length: 6 }).map((_, i) => (
                <div
                  key={i}
                  className="h-14 rounded-lg border border-border/40 bg-card/40 animate-pulse px-4 py-3 flex items-center justify-between"
                >
                  <div className="flex items-center gap-2.5">
                    <div className="h-4 w-14 bg-muted/60 rounded" />
                    <div className="h-4 w-12 bg-muted/50 rounded" />
                    <div className="h-4 w-60 bg-muted/40 rounded" />
                  </div>
                  <div className="h-3 w-16 bg-muted/40 rounded" />
                </div>
              ))}
            </div>
          ) : results.length === 0 ? (
            <div className="py-24 text-center space-y-3 px-6">
              <Layers className="w-10 h-10 text-muted-foreground/30 mx-auto" />
              <p className="text-sm font-semibold text-foreground">No matching actions found</p>
              <p className="text-xs text-muted-foreground max-w-sm mx-auto">
                No actions matched your query or filters. Try clearing the file path, actor, or search text.
              </p>
              <div className="pt-2">
                <button
                  type="button"
                  onClick={handleResetFilters}
                  className="px-3.5 py-1.5 rounded-lg border border-border bg-card hover:bg-background text-xs font-medium text-foreground transition cursor-pointer"
                >
                  Clear All Filters
                </button>
              </div>
            </div>
          ) : (
            results.map((action, idx) => {
              const isSelected =
                selectedAction?.id === action.id || selectedIndex === idx;

              return (
                <div
                  key={action.id}
                  onClick={() => {
                    setSelectedAction(action);
                    setSelectedIndex(idx);
                  }}
                  className={`p-3.5 transition cursor-pointer flex items-start justify-between gap-3 ${
                    isSelected
                      ? "bg-primary/10 border-l-2 border-primary shadow-sm"
                      : "hover:bg-card/60"
                  }`}
                >
                  <div className="space-y-1.5 min-w-0 flex-1">
                    <div className="flex items-center gap-2">
                      <ActionBadge type={action.action_type} />
                      <ActorBadge actor={action.actor} />

                      {action.is_current ? (
                        <span className="flex items-center gap-1 text-[10px] text-emerald-400 font-mono">
                          <CheckCircle2 className="w-3 h-3" />
                          <span>CURRENT</span>
                        </span>
                      ) : (
                        <span className="flex items-center gap-1 text-[10px] text-amber-400 font-mono">
                          <AlertCircle className="w-3 h-3" />
                          <span>SUPERSEDED</span>
                        </span>
                      )}

                      <span className="text-[11px] font-mono text-muted-foreground ml-auto">
                        {new Date(action.timestamp).toLocaleTimeString([], {
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
                      <div className="flex items-center gap-1.5 text-[11px] font-mono text-muted-foreground truncate pt-0.5">
                        <FileText className="w-3 h-3 text-muted-foreground/70 shrink-0" />
                        <span className="text-foreground/80">{action.files[0]}</span>
                        {action.files.length > 1 && (
                          <span className="text-[10px] bg-muted/60 px-1 py-0.2 rounded text-muted-foreground">
                            +{action.files.length - 1} more
                          </span>
                        )}
                      </div>
                    )}
                  </div>
                </div>
              );
            })
          )}
        </div>
      </div>
    </div>
  );
};
