import React from "react";
import {
  BarChart2,
  Users,
  Layers,
  Activity,
  RefreshCw,
  AlertTriangle,
  FolderGit2,
} from "lucide-react";
import { useKnoxStore } from "@/store/useKnoxStore";
import { KpiCard } from "@/components/ui/KpiCard";
import { ActionBadge } from "@/components/ui/ActionBadge";
import { ActorBadge } from "@/components/ui/ActorBadge";
import { PageHeader } from "@/components/ui/PageHeader";
import { EmptyState } from "@/components/ui/EmptyState";
import { SkeletonKpi } from "@/components/ui/Skeleton";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/Card";
import { Button } from "@/components/ui/Button";
import { Badge } from "@/components/ui/Badge";
import { actorBarClass, actionTypeBarClass } from "@/lib/semantics";
import { RecentTrendChart } from "./RecentTrendChart";

export const StatsView: React.FC = () => {
  const { status, actions, isLoading, refreshData, setActiveTab } =
    useKnoxStore();

  const totalActions = status?.total_actions || 1;
  const staleCount = actions.filter((a) => !a.is_current).length;

  const actorEntries = Object.entries(status?.actions_by_actor || {}).sort(
    (a, b) => b[1] - a[1]
  );
  const typeEntries = Object.entries(status?.actions_by_type || {}).sort(
    (a, b) => b[1] - a[1]
  );

  const hasDistribution = actorEntries.length > 0 || typeEntries.length > 0;

  return (
    <div className="p-5 sm:p-6 space-y-5 max-w-7xl mx-auto overflow-y-auto h-full">
      <PageHeader
        title="Metrics & Graph Health"
        description="Aggregate action metrics, actor volume, type distribution, and 7-day velocity."
        icon={<BarChart2 />}
        actions={
          <Button
            variant="outline"
            size="sm"
            onClick={() => refreshData()}
            disabled={isLoading}
            loading={isLoading}
          >
            {!isLoading && <RefreshCw className="size-3.5" />}
            Refresh
          </Button>
        }
      />

      <div className="grid grid-cols-1 sm:grid-cols-2 xl:grid-cols-4 gap-3">
        {isLoading && !status ? (
          Array.from({ length: 4 }).map((_, i) => <SkeletonKpi key={i} />)
        ) : (
          <>
            <KpiCard
              title="Total Actions"
              value={status?.total_actions ?? 0}
              subtitle="Persistent knowledge graph nodes"
              icon={<Activity className="text-primary" />}
              trend={{
                direction: (status?.total_actions ?? 0) > 0 ? "up" : "neutral",
                label: "Indexed",
              }}
            />
            <KpiCard
              title="Active Sessions"
              value={status?.total_sessions ?? 0}
              subtitle="Agent work contexts"
              icon={<Users className="text-plan" />}
            />
            <KpiCard
              title="Projects Tracked"
              value={status?.projects_tracked ?? 0}
              subtitle="Distinct codebases"
              icon={<FolderGit2 className="text-primary" />}
            />
            <KpiCard
              title="Superseded / Stale"
              value={staleCount}
              subtitle="Overwritten decisions/writes"
              icon={<AlertTriangle className="text-warning" />}
              badge={
                staleCount > 0
                  ? `${Math.round((staleCount / totalActions) * 100)}%`
                  : undefined
              }
            />
          </>
        )}
      </div>

      <RecentTrendChart actions={actions} />

      {!hasDistribution && !isLoading ? (
        <EmptyState
          icon={<BarChart2 />}
          title="No distribution data yet"
          description="Ingest a transcript or record agent actions to populate actor and type breakdowns."
          action={
            <Button
              variant="primary"
              size="sm"
              onClick={() => setActiveTab("ingest")}
            >
              Go to Ingest
            </Button>
          }
        />
      ) : (
        <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
          <Card>
            <CardHeader className="flex-row items-center justify-between space-y-0 pb-0">
              <CardTitle className="flex items-center gap-2">
                <Users className="size-4 text-plan" aria-hidden />
                By Actor
              </CardTitle>
              <Badge variant="secondary" className="font-mono">
                {actorEntries.length} unique
              </Badge>
            </CardHeader>
            <CardContent className="pt-4 space-y-3.5">
              {actorEntries.length === 0 ? (
                <EmptyState
                  compact
                  title="No actor data"
                  description="Ingest a transcript to populate actor volume."
                />
              ) : (
                actorEntries.map(([actor, count]) => {
                  const pct = Math.round((count / totalActions) * 100);
                  const barColor = actorBarClass(actor);

                  return (
                    <div key={actor} className="space-y-1.5 text-xs">
                      <div className="flex items-center justify-between gap-2">
                        <ActorBadge actor={actor} />
                        <span className="font-mono text-muted-foreground text-sm shrink-0">
                          <span className="text-foreground font-semibold">
                            {count}
                          </span>{" "}
                          ({pct}%)
                        </span>
                      </div>
                      <div
                        className="h-1.5 w-full bg-background rounded-full overflow-hidden border border-border"
                        role="meter"
                        aria-label={`${actor} ${pct}%`}
                        aria-valuenow={pct}
                        aria-valuemin={0}
                        aria-valuemax={100}
                      >
                        <div
                          className={`h-full ${barColor} rounded-full transition-[width] duration-200`}
                          style={{ width: `${Math.max(pct, 2)}%` }}
                        />
                      </div>
                    </div>
                  );
                })
              )}
            </CardContent>
          </Card>

          <Card>
            <CardHeader className="flex-row items-center justify-between space-y-0 pb-0">
              <CardTitle className="flex items-center gap-2">
                <Layers className="size-4 text-success" aria-hidden />
                By Type
              </CardTitle>
              <Badge variant="secondary" className="font-mono">
                {typeEntries.length} types
              </Badge>
            </CardHeader>
            <CardContent className="pt-4 space-y-3.5">
              {typeEntries.length === 0 ? (
                <EmptyState
                  compact
                  title="No type data"
                  description="Ingest a transcript to populate action types."
                />
              ) : (
                typeEntries.map(([type, count]) => {
                  const pct = Math.round((count / totalActions) * 100);
                  const barColor = actionTypeBarClass(type);

                  return (
                    <div key={type} className="space-y-1.5 text-xs">
                      <div className="flex items-center justify-between gap-2">
                        <ActionBadge type={type} />
                        <span className="font-mono text-muted-foreground text-sm shrink-0">
                          <span className="text-foreground font-semibold">
                            {count}
                          </span>{" "}
                          ({pct}%)
                        </span>
                      </div>
                      <div
                        className="h-1.5 w-full bg-background rounded-full overflow-hidden border border-border"
                        role="meter"
                        aria-label={`${type} ${pct}%`}
                        aria-valuenow={pct}
                        aria-valuemin={0}
                        aria-valuemax={100}
                      >
                        <div
                          className={`h-full ${barColor} rounded-full transition-[width] duration-200`}
                          style={{ width: `${Math.max(pct, 2)}%` }}
                        />
                      </div>
                    </div>
                  );
                })
              )}
            </CardContent>
          </Card>
        </div>
      )}
    </div>
  );
};
