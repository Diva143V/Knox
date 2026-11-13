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
import { PageHeader } from "@/components/ui/PageHeader";
import { Button } from "@/components/ui/Button";
import { Input } from "@/components/ui/Input";
import { FilterBar } from "@/components/ui/FilterBar";
import { EmptyState } from "@/components/ui/EmptyState";
import { SkeletonRow } from "@/components/ui/Skeleton";
import { Badge } from "@/components/ui/Badge";
import { cn } from "@/lib/cn";
import type { KnoxAction, AskResultDto } from "@/types/knox";

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
    activeProjectScope,
    refreshDataQuietly,
  } = useKnoxStore();

  const [consoleMode, setConsoleMode] = useState<"table" | "intelligence">("table");
  const [askQuestion, setAskQuestion] = useState("");
  const [isAsking, setIsAsking] = useState(false);
  const [currentAnswer, setCurrentAnswer] = useState<AskResultDto | null>(null);
  const [askHistory, setAskHistory] = useState<Array<{ question: string; result: AskResultDto }>>([]);
  const [askError, setAskError] = useState<string | null>(null);
  const [isSavingThought, setIsSavingThought] = useState(false);
  const askInputRef = useRef<HTMLTextAreaElement>(null);

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

  const handleAskSubmit = async (e?: React.FormEvent) => {
    if (e) e.preventDefault();
    const q = askQuestion.trim();
    if (!q) return;

    try {
      setIsAsking(true);
      setAskError(null);
      const res = await KnoxAPI.askKnox(q, activeProjectScope || undefined);
      setCurrentAnswer(res);
      setAskHistory((prev) => [{ question: q, result: res }, ...prev]);
    } catch (err) {
      setAskError(String(err));
      showToast(String(err), true);
    } finally {
      setIsAsking(false);
    }
  };

  const triggerSuggestion = (sug: string) => {
    setAskQuestion(sug);
    setIsAsking(true);
    setAskError(null);
    KnoxAPI.askKnox(sug, activeProjectScope || undefined)
      .then((res) => {
        setCurrentAnswer(res);
        setAskHistory((prev) => [{ question: sug, result: res }, ...prev]);
      })
      .catch((err) => {
        setAskError(String(err));
        showToast(String(err), true);
      })
      .finally(() => setIsAsking(false));
  };

  const handleCitationClick = (actionId: string) => {
    const found = actions.find((a) => a.id === actionId);
    if (found) {
      setSelectedAction(found);
    } else {
      KnoxAPI.getAction(actionId)
        .then((act) => {
          if (act) setSelectedAction(act);
        })
        .catch(() => {});
    }
  };

  const handleSaveThought = async () => {
    if (!currentAnswer) return;
    try {
      setIsSavingThought(true);
      await KnoxAPI.recordQuickThought({
        summary: `[Decision] ${currentAnswer.question}`,
        category: "thought",
        contextSummary: currentAnswer.answer,
        actor: "human",
        project: activeProjectScope || undefined,
      });
      showToast("Decision takeaway recorded as thought!");
      await refreshDataQuietly();
    } catch (e) {
      showToast(`Failed to record thought: ${String(e)}`, true);
    } finally {
      setIsSavingThought(false);
    }
  };

  const renderGroundedAnswerText = (text: string) => {
    const parts = text.split(/(\[ref:[a-zA-Z0-9_\-]+\]|\[act-[a-zA-Z0-9_\-]+\])/g);
    return parts.map((part, i) => {
      const match = part.match(/\[(?:ref|act):([a-zA-Z0-9_\-]+)\]/);
      if (match) {
        const id = match[1];
        return (
          <button
            key={i}
            type="button"
            onClick={() => handleCitationClick(id)}
            className="inline-flex items-center font-mono text-2xs text-primary bg-primary/10 hover:bg-primary/20 px-1 py-0.5 rounded mx-0.5 cursor-pointer border border-primary/30 font-semibold align-baseline"
          >
            {part}
          </button>
        );
      }
      return part;
    });
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
        <div className="px-4 py-3 border-b border-border bg-card/40 shrink-0 flex items-center justify-between gap-4">
          <PageHeader
            title={consoleMode === "intelligence" ? "Decision Intelligence & Query" : "Query Console"}
            description={
              consoleMode === "intelligence"
                ? "Ask natural-language questions and analyze architectural decisions grounded in project memory."
                : "Full-text keyword and code symbol search across recorded actions, architectural decisions, and doubts."
            }
            icon={consoleMode === "intelligence" ? <Sparkles className="size-4 text-primary" /> : <Search />}
            status={
              consoleMode === "intelligence" ? (
                <span className="font-mono text-3xs text-primary tabular-nums px-2 py-0.5 rounded-full border border-primary/30 bg-primary/10 font-semibold">
                  Grounded Reasoning
                </span>
              ) : (
                <span className="font-mono text-3xs text-muted-foreground tabular-nums px-2 py-0.5 rounded-full border border-border bg-background">
                  {results.length} result{results.length === 1 ? "" : "s"}
                </span>
              )
            }
          />

          {/* Mode Switcher */}
          <div className="flex items-center gap-1 p-1 bg-muted/60 rounded-lg border border-border shrink-0">
            <button
              type="button"
              onClick={() => setConsoleMode("table")}
              className={cn(
                "px-2.5 py-1 text-xs font-medium rounded-md transition-all flex items-center gap-1.5 cursor-pointer",
                consoleMode === "table"
                  ? "bg-background text-foreground shadow-xs font-semibold"
                  : "text-muted-foreground hover:text-foreground"
              )}
            >
              <FileText className="size-3.5" />
              <span>Actions Explorer</span>
            </button>
            <button
              type="button"
              onClick={() => {
                setConsoleMode("intelligence");
                setTimeout(() => askInputRef.current?.focus(), 50);
              }}
              className={cn(
                "px-2.5 py-1 text-xs font-medium rounded-md transition-all flex items-center gap-1.5 cursor-pointer",
                consoleMode === "intelligence"
                  ? "bg-primary text-primary-foreground shadow-xs font-semibold"
                  : "text-muted-foreground hover:text-foreground"
              )}
            >
              <Sparkles className="size-3.5" />
              <span>Decision Intelligence & Ask</span>
            </button>
          </div>
        </div>

        {consoleMode === "intelligence" ? (
          <div className="flex-1 overflow-y-auto p-4 sm:p-6 space-y-6">
            {/* Hero AI Search Bar */}
            <div className="max-w-3xl mx-auto space-y-3">
              <form
                onSubmit={handleAskSubmit}
                className="relative shadow-md rounded-xl border border-border bg-card/80 backdrop-blur-sm overflow-hidden focus-within:ring-2 focus-within:ring-primary/40 focus-within:border-primary transition-all"
              >
                <div className="p-3 sm:p-4 flex items-start gap-3">
                  <Sparkles className="size-5 text-primary shrink-0 mt-1" />
                  <textarea
                    ref={askInputRef}
                    rows={2}
                    value={askQuestion}
                    onChange={(e) => setAskQuestion(e.target.value)}
                    onKeyDown={(e) => {
                      if (e.key === "Enter" && !e.shiftKey) {
                        e.preventDefault();
                        handleAskSubmit();
                      }
                    }}
                    placeholder="Ask anything about past decisions, why things were built this way, or explore architectural trade-offs..."
                    className="w-full bg-transparent text-xs sm:text-sm text-foreground placeholder:text-muted-foreground focus:outline-none resize-none font-sans leading-relaxed"
                  />
                </div>

                <div className="flex items-center justify-between px-3 sm:px-4 py-2 border-t border-border/60 bg-muted/20 text-2xs">
                  <span className="text-muted-foreground hidden sm:inline">
                    Press <kbd className="font-mono bg-muted px-1.5 py-0.5 rounded border border-border">Enter ↵</kbd> to ask, <kbd className="font-mono bg-muted px-1.5 py-0.5 rounded border border-border">Shift+Enter</kbd> for newline
                  </span>
                  <div className="flex items-center gap-2 ml-auto">
                    {askQuestion && (
                      <Button
                        type="button"
                        variant="ghost"
                        size="sm"
                        onClick={() => setAskQuestion("")}
                        className="h-7 text-xs text-muted-foreground"
                      >
                        Clear
                      </Button>
                    )}
                    <Button
                      type="submit"
                      variant="primary"
                      size="sm"
                      loading={isAsking}
                      disabled={isAsking || !askQuestion.trim()}
                      className="gap-1.5 h-7 text-xs font-medium cursor-pointer"
                    >
                      <Sparkles className="size-3" />
                      <span>{isAsking ? "Grounding…" : "Ask Knox AI"}</span>
                    </Button>
                  </div>
                </div>
              </form>

              {/* Suggestion Chips */}
              <div className="flex items-center gap-1.5 flex-wrap">
                <span className="text-2xs font-mono uppercase text-muted-foreground mr-1">Suggestions:</span>
                {[
                  "Why did we choose SQLite over Postgres?",
                  "What are the unresolved doubts in this project?",
                  "Summarize recent architectural decisions and modified files",
                  "What are the security boundaries and egress rules?",
                ].map((sug) => (
                  <button
                    key={sug}
                    type="button"
                    onClick={() => triggerSuggestion(sug)}
                    className="px-2.5 py-1 rounded-full text-2xs bg-muted/40 hover:bg-muted text-muted-foreground hover:text-foreground border border-border transition-colors cursor-pointer text-left"
                  >
                    {sug}
                  </button>
                ))}
              </div>
            </div>

            {/* Answer Display */}
            {askError && (
              <div className="max-w-3xl mx-auto p-4 rounded-xl bg-danger/10 border border-danger/30 text-danger text-xs font-mono">
                {askError}
              </div>
            )}

            {isAsking && !currentAnswer && (
              <div className="max-w-3xl mx-auto p-6 rounded-xl border border-border bg-card/60 space-y-3 animate-pulse">
                <div className="h-4 bg-muted rounded w-1/3" />
                <div className="h-3 bg-muted rounded w-full" />
                <div className="h-3 bg-muted rounded w-5/6" />
                <div className="h-3 bg-muted rounded w-4/6" />
              </div>
            )}

            {currentAnswer && (
              <div className="max-w-3xl mx-auto space-y-4 animate-in fade-in-50 duration-150">
                {/* Result Card */}
                <div className="p-5 rounded-xl border border-border bg-card/70 shadow-sm space-y-4">
                  <div className="flex items-center justify-between border-b border-border/60 pb-3 flex-wrap gap-2">
                    <div className="flex items-center gap-2">
                      <Sparkles className="size-4 text-primary" />
                      <span className="text-xs font-semibold text-foreground font-mono">
                        Grounded Synthesis
                      </span>
                      <Badge variant="outline" className="text-3xs uppercase font-mono">
                        {currentAnswer.model}
                      </Badge>
                    </div>
                    <div className="flex items-center gap-3 text-2xs font-mono text-muted-foreground">
                      <span>Evaluated {currentAnswer.candidates_evaluated} decisions</span>
                      <span>•</span>
                      <span>{currentAnswer.latency_ms}ms</span>
                      <Button
                        type="button"
                        variant="outline"
                        size="sm"
                        onClick={handleSaveThought}
                        loading={isSavingThought}
                        className="h-6 text-2xs gap-1 border-primary/30 hover:bg-primary/10 text-primary cursor-pointer ml-1"
                      >
                        <CheckCircle2 className="size-3" />
                        <span>Record as Thought</span>
                      </Button>
                    </div>
                  </div>

                  {/* Markdown Answer Text */}
                  <div className="text-xs text-foreground leading-relaxed whitespace-pre-wrap font-sans">
                    {renderGroundedAnswerText(currentAnswer.answer)}
                  </div>

                  {/* Citations & Evidence */}
                  {currentAnswer.citations.length > 0 && (
                    <div className="pt-3 border-t border-border/60 space-y-2">
                      <div className="flex items-center gap-1.5 text-2xs font-mono uppercase text-muted-foreground font-semibold">
                        <FileText className="size-3 text-primary" />
                        <span>Cited Project Memory ({currentAnswer.citations.length})</span>
                      </div>
                      <div className="grid grid-cols-1 sm:grid-cols-2 gap-2">
                        {currentAnswer.citations.map((c) => (
                          <div
                            key={c.action_id}
                            onClick={() => handleCitationClick(c.action_id)}
                            className="p-2.5 rounded-lg border border-border/70 bg-muted/30 hover:bg-muted/70 hover:border-primary/40 transition-all cursor-pointer text-left space-y-1"
                          >
                            <div className="flex items-center justify-between">
                              <span className="font-mono text-3xs font-semibold text-primary">
                                {c.action_id}
                              </span>
                              {c.file && (
                                <span className="text-3xs font-mono text-muted-foreground truncate max-w-[150px]">
                                  {c.file}
                                </span>
                              )}
                            </div>
                            <p className="text-2xs text-foreground line-clamp-2">
                              {c.summary}
                            </p>
                          </div>
                        ))}
                      </div>
                    </div>
                  )}
                </div>

                {/* Question History list */}
                {askHistory.length > 1 && (
                  <div className="pt-4 space-y-3">
                    <span className="text-2xs font-mono uppercase text-muted-foreground font-semibold">
                      Previous Inquiries in this Session ({askHistory.length - 1})
                    </span>
                    <div className="space-y-2">
                      {askHistory.slice(1).map((h, i) => (
                        <div
                          key={i}
                          onClick={() => setCurrentAnswer(h.result)}
                          className="p-3 rounded-lg border border-border bg-card/40 hover:bg-card text-xs flex items-center justify-between gap-3 cursor-pointer transition-all"
                        >
                          <div className="min-w-0 flex-1">
                            <p className="font-medium text-foreground truncate">{h.question}</p>
                            <p className="text-2xs text-muted-foreground truncate mt-0.5">
                              {h.result.answer.slice(0, 100)}…
                            </p>
                          </div>
                          <Badge variant="outline" className="font-mono text-3xs shrink-0">
                            {h.result.citations.length} citations
                          </Badge>
                        </div>
                      ))}
                    </div>
                  </div>
                )}
              </div>
            )}
          </div>
        ) : (
          <>
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
                  aria-label="Filter by agent"
                >
                  <option value="">All Agents</option>
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
                  aria-label="Filter by kind"
                >
                  <option value="">All Kinds</option>
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
          <span className="text-xs text-muted-foreground font-medium mr-1">
            Focus:
          </span>
          <button
            type="button"
            onClick={() => {
              setActiveFacet("all");
              setSelectedIndex(-1);
            }}
            className={cn(
              "px-2.5 py-1 rounded-full text-xs font-medium border transition-colors cursor-pointer",
              activeFacet === "all"
                ? "bg-primary/15 text-primary border-primary/40"
                : "bg-muted/40 text-muted-foreground border-border hover:text-foreground"
            )}
          >
            All (<span className="tabular-nums">{results.length}</span>)
          </button>
          <button
            type="button"
            onClick={() => {
              setActiveFacet("doubts");
              setSelectedIndex(-1);
            }}
            className={cn(
              "px-2.5 py-1 rounded-full text-xs font-medium border transition-colors cursor-pointer flex items-center gap-1.5",
              activeFacet === "doubts"
                ? "bg-warning/15 text-warning border-warning/40"
                : "bg-warning/10 text-warning/80 border-warning/20 hover:text-warning"
            )}
          >
            <HelpCircle className="size-3" />
            <span>Open Doubts (<span className="tabular-nums">{doubtsCount}</span>)</span>
          </button>
          <button
            type="button"
            onClick={() => {
              setActiveFacet("major");
              setSelectedIndex(-1);
            }}
            className={cn(
              "px-2.5 py-1 rounded-full text-xs font-medium border transition-colors cursor-pointer flex items-center gap-1.5",
              activeFacet === "major"
                ? "bg-plan/15 text-plan border-plan/40"
                : "bg-plan/10 text-plan/80 border-plan/20 hover:text-plan"
            )}
          >
            <Sparkles className="size-3" />
            <span>Major ADRs (<span className="tabular-nums">{majorCount}</span>)</span>
          </button>
          <button
            type="button"
            onClick={() => {
              setActiveFacet("minor");
              setSelectedIndex(-1);
            }}
            className={cn(
              "px-2.5 py-1 rounded-full text-xs font-medium border transition-colors cursor-pointer flex items-center gap-1.5",
              activeFacet === "minor"
                ? "bg-muted text-foreground border-border"
                : "bg-muted/40 text-muted-foreground border-border hover:text-foreground"
            )}
          >
            <GitCommit className="size-3" />
            <span>Minor Choices (<span className="tabular-nums">{minorCount}</span>)</span>
          </button>
          <button
            type="button"
            onClick={() => {
              setActiveFacet("dialogue");
              setSelectedIndex(-1);
            }}
            className={cn(
              "px-2.5 py-1 rounded-full text-xs font-medium border transition-colors cursor-pointer flex items-center gap-1.5",
              activeFacet === "dialogue"
                ? "bg-primary/15 text-primary border-primary/40"
                : "bg-muted/40 text-muted-foreground border-border hover:text-foreground"
            )}
          >
            <MessageSquare className="size-3" />
            <span>Conversations (<span className="tabular-nums">{dialogueCount}</span>)</span>
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
      </>
    )}
  </div>
</div>
  );
};
