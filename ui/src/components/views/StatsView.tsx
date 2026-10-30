import React from "react";
import { BarChart2, Users, Layers, Activity, RefreshCw, AlertTriangle, FolderGit2 } from "lucide-react";
import { useKnoxStore } from "@/store/useKnoxStore";
import { KpiCard } from "@/components/ui/KpiCard";
import { ActionBadge } from "@/components/ui/ActionBadge";
import { ActorBadge } from "@/components/ui/ActorBadge";
import { RecentTrendChart } from "./RecentTrendChart";

export const StatsView: React.FC = () => {
  const { status, actions, isLoading, refreshData } = useKnoxStore();

  const totalActions = status?.total_actions || 1;
  const staleCount = actions.filter((a) => !a.is_current).length;

  const actorEntries = Object.entries(status?.actions_by_actor || {}).sort(
    (a, b) => b[1] - a[1]
  );
  const typeEntries = Object.entries(status?.actions_by_type || {}).sort(
    (a, b) => b[1] - a[1]
  );

  const getActorBarColor = (actor: string) => {
    const norm = actor.toLowerCase();
    if (norm === "gemini") return "bg-blue-500";
    if (norm === "cursor") return "bg-purple-500";
    if (norm === "antigravity") return "bg-indigo-500";
    if (norm === "human") return "bg-pink-500";
    if (norm === "copilot") return "bg-sky-500";
    return "bg-slate-500";
  };

  const getTypeBarColor = (type: string) => {
    const norm = type.toLowerCase();
    if (norm.includes("write")) return "bg-emerald-500";
    if (norm.includes("plan")) return "bg-purple-500";
    if (norm.includes("decide")) return "bg-amber-500";
    if (norm.includes("debug")) return "bg-rose-500";
    return "bg-indigo-500";
  };

  return (
    <div className="p-6 space-y-6 max-w-7xl mx-auto overflow-y-auto h-full">
      {/* View Header with Title and Refresh */}
      <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-4">
        <div>
          <h1 className="text-xl font-bold tracking-tight text-foreground flex items-center gap-2">
            <BarChart2 className="w-5 h-5 text-primary" />
            <span>Metrics & Graph Health</span>
          </h1>
          <p className="text-xs text-muted-foreground mt-1">
            Aggregate action metrics, actor volume, type distribution, and 7-day velocity.
          </p>
        </div>

        <button
          onClick={() => refreshData()}
          disabled={isLoading}
          className="flex items-center gap-1.5 px-3 py-1.5 rounded-lg border border-border bg-card hover:bg-background text-xs font-medium text-foreground transition cursor-pointer self-start sm:self-auto"
        >
          <RefreshCw className={`w-3.5 h-3.5 ${isLoading ? "animate-spin text-primary" : ""}`} />
          <span>Refresh Metrics</span>
        </button>
      </div>

      {/* 4 KPI Cards Row */}
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
              subtitle="Persistent knowledge graph nodes"
              icon={<Activity className="w-4 h-4 text-emerald-400" />}
              trend={{ direction: (status?.total_actions ?? 0) > 0 ? "up" : "neutral", label: "Indexed" }}
            />
            <KpiCard
              title="Active Sessions"
              value={status?.total_sessions ?? 0}
              subtitle="Agent work contexts"
              icon={<Users className="w-4 h-4 text-indigo-400" />}
            />
            <KpiCard
              title="Projects Tracked"
              value={status?.projects_tracked ?? 0}
              subtitle="Distinct codebases"
              icon={<FolderGit2 className="w-4 h-4 text-purple-400" />}
            />
            <KpiCard
              title="Superseded / Stale"
              value={staleCount}
              subtitle="Overwritten decisions/writes"
              icon={<AlertTriangle className="w-4 h-4 text-amber-400" />}
              badge={staleCount > 0 ? `${Math.round((staleCount / totalActions) * 100)}%` : undefined}
            />
          </>
        )}
      </div>

      {/* 7-Day Velocity Sparkline Chart */}
      <RecentTrendChart actions={actions} />

      {/* 2-Column Side-by-Side Distributions */}
      <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
        {/* By Actor / Agent */}
        <div className="rounded-xl border border-border/70 bg-card p-5 space-y-4">
          <div className="flex items-center justify-between">
            <h2 className="text-sm font-semibold tracking-tight text-foreground flex items-center gap-2">
              <Users className="w-4 h-4 text-indigo-400" />
              <span>Actions by Actor / Agent</span>
            </h2>
            <span className="text-[10px] font-mono text-muted-foreground bg-background px-1.5 py-0.5 rounded border border-border/50">
              {actorEntries.length} unique
            </span>
          </div>

          <div className="space-y-3.5">
            {actorEntries.length === 0 ? (
              <p className="text-xs text-muted-foreground py-8 text-center border border-dashed border-border/50 rounded-lg">
                No actor data recorded. Ingest a transcript to populate.
              </p>
            ) : (
              actorEntries.map(([actor, count]) => {
                const pct = Math.round((count / totalActions) * 100);
                const barColor = getActorBarColor(actor);

                return (
                  <div key={actor} className="space-y-1.5 text-xs">
                    <div className="flex items-center justify-between">
                      <ActorBadge actor={actor} />
                      <span className="font-mono text-muted-foreground text-[11px]">
                        <span className="text-foreground font-semibold">{count}</span> ({pct}%)
                      </span>
                    </div>
                    <div className="h-2 w-full bg-background rounded-full overflow-hidden border border-border/40">
                      <div
                        className={`h-full ${barColor} rounded-full transition-all duration-300`}
                        style={{ width: `${Math.max(pct, 2)}%` }}
                      />
                    </div>
                  </div>
                );
              })
            )}
          </div>
        </div>

        {/* By Action Type */}
        <div className="rounded-xl border border-border/70 bg-card p-5 space-y-4">
          <div className="flex items-center justify-between">
            <h2 className="text-sm font-semibold tracking-tight text-foreground flex items-center gap-2">
              <Layers className="w-4 h-4 text-emerald-400" />
              <span>Actions by Type</span>
            </h2>
            <span className="text-[10px] font-mono text-muted-foreground bg-background px-1.5 py-0.5 rounded border border-border/50">
              {typeEntries.length} types
            </span>
          </div>

          <div className="space-y-3.5">
            {typeEntries.length === 0 ? (
              <p className="text-xs text-muted-foreground py-8 text-center border border-dashed border-border/50 rounded-lg">
                No action types recorded. Ingest a transcript to populate.
              </p>
            ) : (
              typeEntries.map(([type, count]) => {
                const pct = Math.round((count / totalActions) * 100);
                const barColor = getTypeBarColor(type);

                return (
                  <div key={type} className="space-y-1.5 text-xs">
                    <div className="flex items-center justify-between">
                      <ActionBadge type={type} />
                      <span className="font-mono text-muted-foreground text-[11px]">
                        <span className="text-foreground font-semibold">{count}</span> ({pct}%)
                      </span>
                    </div>
                    <div className="h-2 w-full bg-background rounded-full overflow-hidden border border-border/40">
                      <div
                        className={`h-full ${barColor} rounded-full transition-all duration-300`}
                        style={{ width: `${Math.max(pct, 2)}%` }}
                      />
                    </div>
                  </div>
                );
              })
            )}
          </div>
        </div>
      </div>
    </div>
  );
};
