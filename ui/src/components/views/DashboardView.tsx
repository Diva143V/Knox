import React from "react";
import {
  Activity,
  Clock,
  FolderGit2,
  Calendar,
  Sparkles,
  ArrowRight,
  Database,
  Terminal,
  Shield,
  Search,
  PlusCircle,
  Cpu,
  Layers,
} from "lucide-react";
import { useKnoxStore } from "@/store/useKnoxStore";
import { KpiCard } from "@/components/ui/KpiCard";
import { ActionRow } from "@/components/ui/ActionRow";

export const DashboardView: React.FC = () => {
  const {
    status,
    actions,
    config,
    isLoading,
    setActiveTab,
    selectedAction,
    setSelectedAction,
  } = useKnoxStore();

  const lastUpdateStr = status?.last_action_timestamp
    ? new Date(status.last_action_timestamp).toLocaleString(undefined, {
        month: "short",
        day: "numeric",
        hour: "2-digit",
        minute: "2-digit",
      })
    : "No records yet";

  // Recent 15 actions feed as specified in v1.2.0.md
  const recentActions = actions.slice(0, 15);

  return (
    <div className="p-6 space-y-6 max-w-7xl mx-auto overflow-y-auto h-full">
      {/* Welcome Banner & Quick Action Buttons */}
      <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-4">
        <div>
          <h1 className="text-xl font-bold tracking-tight text-foreground flex items-center gap-2">
            <span>Welcome to Knox</span>
            <span className="text-[11px] px-2 py-0.5 rounded-full bg-primary/10 text-primary border border-primary/20 font-mono">
              v1.2.0 Desktop
            </span>
          </h1>
          <p className="text-xs text-muted-foreground mt-1">
            Persistent context, causal graph memory, and cross-session audit trail for AI coding agents.
          </p>
        </div>

        {/* Quick Action Navigation Bar */}
        <div className="flex flex-wrap items-center gap-2">
          <button
            onClick={() => setActiveTab("graph")}
            className="flex items-center gap-1.5 px-3 py-1.5 rounded-lg bg-primary text-primary-foreground text-xs font-medium hover:bg-primary/90 transition cursor-pointer shadow-sm shadow-primary/25"
          >
            <Sparkles className="w-3.5 h-3.5" />
            <span>Open Graph</span>
            <ArrowRight className="w-3 h-3 ml-0.5" />
          </button>

          <button
            onClick={() => setActiveTab("query")}
            className="flex items-center gap-1.5 px-3 py-1.5 rounded-lg border border-border bg-card hover:bg-background text-xs font-medium text-foreground transition cursor-pointer"
          >
            <Search className="w-3.5 h-3.5 text-indigo-400" />
            <span>Query Console</span>
          </button>

          <button
            onClick={() => setActiveTab("workspace")}
            className="flex items-center gap-1.5 px-3 py-1.5 rounded-lg border border-border bg-card hover:bg-background text-xs font-medium text-foreground transition cursor-pointer"
          >
            <PlusCircle className="w-3.5 h-3.5 text-emerald-400" />
            <span>Init Workspace</span>
          </button>

          <button
            onClick={() => setActiveTab("mcp")}
            className="flex items-center gap-1.5 px-3 py-1.5 rounded-lg border border-border bg-card hover:bg-background text-xs font-medium text-foreground transition cursor-pointer"
          >
            <Cpu className="w-3.5 h-3.5 text-purple-400" />
            <span>MCP Config</span>
          </button>
        </div>
      </div>

      {/* KPI Cards Row (4 Cards) */}
      <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-4">
        {isLoading && !status ? (
          Array.from({ length: 4 }).map((_, i) => (
            <div
              key={i}
              className="h-24 rounded-xl border border-border/50 bg-card/60 animate-pulse p-4 flex flex-col justify-between"
            >
              <div className="h-3 w-20 bg-muted/50 rounded" />
              <div className="h-6 w-12 bg-muted/70 rounded" />
              <div className="h-3 w-32 bg-muted/40 rounded" />
            </div>
          ))
        ) : (
          <>
            <KpiCard
              title="Total Actions"
              value={status?.total_actions ?? 0}
              subtitle="Indexed in SQLite knowledge graph"
              icon={<Activity className="w-4 h-4 text-emerald-400" />}
              trend={{ direction: (status?.total_actions ?? 0) > 0 ? "up" : "neutral", label: "Indexed" }}
              onClick={() => setActiveTab("query")}
            />
            <KpiCard
              title="Active Sessions"
              value={status?.total_sessions ?? 0}
              subtitle="AI & human session logs"
              icon={<Clock className="w-4 h-4 text-indigo-400" />}
              trend={{ direction: "neutral", label: "Tracked" }}
              onClick={() => setActiveTab("stats")}
            />
            <KpiCard
              title="Projects Tracked"
              value={status?.projects_tracked ?? 0}
              subtitle="Registered codebases"
              icon={<FolderGit2 className="w-4 h-4 text-purple-400" />}
              onClick={() => setActiveTab("workspace")}
            />
            <KpiCard
              title="Last Activity"
              value={status?.total_actions ? "Active" : "Idle"}
              subtitle={lastUpdateStr}
              icon={<Calendar className="w-4 h-4 text-amber-400" />}
              trend={{ direction: status?.total_actions ? "up" : "neutral", label: "Status" }}
            />
          </>
        )}
      </div>

      {/* 2-Column Split: Recent 15 Actions & System Diagnostics */}
      <div className="grid grid-cols-1 lg:grid-cols-3 gap-6">
        {/* Recent Actions Feed (2 cols) */}
        <div className="lg:col-span-2 rounded-xl border border-border/70 bg-card p-5 space-y-4">
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-2">
              <Activity className="w-4 h-4 text-primary" />
              <h2 className="text-sm font-semibold tracking-tight text-foreground">
                Recent Graph Decisions & Actions
              </h2>
              <span className="text-[10px] font-mono text-muted-foreground bg-background px-1.5 py-0.5 rounded border border-border/50">
                Top {recentActions.length}
              </span>
            </div>

            <button
              onClick={() => setActiveTab("query")}
              className="text-xs text-primary hover:underline flex items-center gap-1 cursor-pointer font-medium"
            >
              <span>View all in Query Console</span>
              <ArrowRight className="w-3 h-3" />
            </button>
          </div>

          <div className="space-y-2">
            {isLoading && actions.length === 0 ? (
              Array.from({ length: 6 }).map((_, i) => (
                <div
                  key={i}
                  className="h-11 rounded-lg border border-border/40 bg-background/40 animate-pulse px-3 py-2.5 flex items-center justify-between"
                >
                  <div className="flex items-center gap-2.5">
                    <div className="h-4 w-14 bg-muted/60 rounded" />
                    <div className="h-4 w-12 bg-muted/50 rounded" />
                    <div className="h-4 w-48 bg-muted/40 rounded" />
                  </div>
                  <div className="h-3 w-12 bg-muted/40 rounded" />
                </div>
              ))
            ) : recentActions.length === 0 ? (
              <div className="py-12 text-center space-y-2 border border-dashed border-border/60 rounded-lg p-6">
                <Layers className="w-8 h-8 text-muted-foreground/40 mx-auto" />
                <p className="text-xs font-medium text-foreground">No graph actions recorded yet</p>
                <p className="text-[11px] text-muted-foreground max-w-sm mx-auto">
                  Import an Antigravity transcript in Ingest Transcripts, or configure your coding agent via the MCP Server.
                </p>
                <div className="pt-2 flex justify-center gap-2">
                  <button
                    onClick={() => setActiveTab("ingest")}
                    className="px-3 py-1.5 rounded-md bg-primary text-primary-foreground text-xs font-medium hover:bg-primary/90 transition cursor-pointer"
                  >
                    Ingest Transcript
                  </button>
                </div>
              </div>
            ) : (
              recentActions.map((action) => (
                <ActionRow
                  key={action.id}
                  action={action}
                  isSelected={selectedAction?.id === action.id}
                  onSelect={(act) => setSelectedAction(act)}
                />
              ))
            )}
          </div>
        </div>

        {/* System Diagnostics & Config Panel (1 col) */}
        <div className="rounded-xl border border-border/70 bg-card p-5 space-y-5">
          <h2 className="text-sm font-semibold tracking-tight text-foreground flex items-center gap-2">
            <Shield className="w-4 h-4 text-emerald-400" />
            <span>System Diagnostics</span>
          </h2>

          <div className="space-y-3.5 text-xs">
            {/* Database Path */}
            <div className="p-3 rounded-lg border border-border/50 bg-background/50 space-y-1">
              <div className="flex items-center gap-1.5 text-muted-foreground">
                <Database className="w-3.5 h-3.5 text-emerald-400" />
                <span className="font-medium">Knowledge Graph Database:</span>
              </div>
              <p className="font-mono text-foreground break-all text-[11px] pl-5">
                {config?.kg_path || "knox.db"}
              </p>
            </div>

            {/* MCP Transport */}
            <div className="p-3 rounded-lg border border-border/50 bg-background/50 space-y-1">
              <div className="flex items-center gap-1.5 text-muted-foreground">
                <Terminal className="w-3.5 h-3.5 text-indigo-400" />
                <span className="font-medium">MCP Server Protocol:</span>
              </div>
              <p className="font-mono text-foreground text-[11px] pl-5">
                knox serve --stdio
              </p>
            </div>

            {/* Application Environment */}
            <div className="p-3 rounded-lg border border-border/50 bg-background/50 space-y-1.5 text-[11px]">
              <div className="flex justify-between text-muted-foreground">
                <span>Version:</span>
                <span className="font-mono text-foreground">v1.2.0</span>
              </div>
              <div className="flex justify-between text-muted-foreground">
                <span>Core Engine:</span>
                <span className="text-foreground">Rust (tokio + rusqlite)</span>
              </div>
              <div className="flex justify-between text-muted-foreground">
                <span>Session Timeout:</span>
                <span className="font-mono text-foreground">
                  {config?.session_timeout_seconds ? `${config.session_timeout_seconds}s` : "1800s"}
                </span>
              </div>
            </div>

            {/* Quick Action Navigation */}
            <div className="pt-2 space-y-2">
              <button
                onClick={() => setActiveTab("mcp")}
                className="w-full py-2 px-3 rounded-lg border border-border text-center text-xs font-medium text-foreground hover:bg-background transition cursor-pointer"
              >
                Inspect MCP Configuration
              </button>
              <button
                onClick={() => setActiveTab("scanner")}
                className="w-full py-2 px-3 rounded-lg border border-border text-center text-xs font-medium text-foreground hover:bg-background transition cursor-pointer"
              >
                Human Edit Scanner & Hooks
              </button>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
};
