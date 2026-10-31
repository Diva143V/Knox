import React from "react";
import {
  Activity,
  Clock,
  FolderGit2,
  Calendar,
  ArrowRight,
  Database,
  Terminal,
  Shield,
  Search,
  PlusCircle,
  Network,
  PlugZap,
  LayoutDashboard,
} from "lucide-react";
import { useKnoxStore } from "@/store/useKnoxStore";
import { KpiCard } from "@/components/ui/KpiCard";
import { ActionRow } from "@/components/ui/ActionRow";
import { PageHeader } from "@/components/ui/PageHeader";
import { EmptyState } from "@/components/ui/EmptyState";
import { SkeletonKpi, SkeletonRow } from "@/components/ui/Skeleton";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/Card";
import { Button } from "@/components/ui/Button";
import { Badge } from "@/components/ui/Badge";
import { APP_VERSION } from "@/lib/navigation";

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

  const recentActions = actions.slice(0, 15);
  const topActors = Object.entries(status?.actions_by_actor || {})
    .sort((a, b) => b[1] - a[1])
    .slice(0, 4);

  return (
    <div className="p-5 sm:p-6 space-y-5 max-w-7xl mx-auto overflow-y-auto h-full">
      <PageHeader
        title="Workspace Overview"
        description="Local knowledge graph health, recent decisions, and next useful actions."
        icon={<LayoutDashboard />}
        actions={
          <>
            <Button
              variant="primary"
              size="sm"
              onClick={() => setActiveTab("graph")}
            >
              <Network className="size-3.5" />
              Open Graph
              <ArrowRight className="size-3" />
            </Button>
            <Button
              variant="outline"
              size="sm"
              onClick={() => setActiveTab("query")}
            >
              <Search className="size-3.5" />
              Search
            </Button>
            <Button
              variant="outline"
              size="sm"
              onClick={() => setActiveTab("workspace")}
            >
              <PlusCircle className="size-3.5" />
              Workspace
            </Button>
            <Button
              variant="outline"
              size="sm"
              onClick={() => setActiveTab("mcp")}
            >
              <PlugZap className="size-3.5" />
              MCP
            </Button>
          </>
        }
      />

      {/* Compact status strip — real status fields only */}
      <div className="flex flex-wrap items-center gap-x-4 gap-y-2 px-3 py-2 rounded-md border border-border bg-card text-sm font-mono text-muted-foreground">
        <span className="flex items-center gap-1.5">
          <span className="size-1.5 rounded-full bg-success" aria-hidden />
          <span className="text-foreground font-medium">Knox v{APP_VERSION}</span>
        </span>
        <span className="hidden sm:inline text-border">|</span>
        <span>
          Actions{" "}
          <span className="text-foreground font-semibold">
            {status?.total_actions ?? "—"}
          </span>
        </span>
        <span>
          Sessions{" "}
          <span className="text-foreground font-semibold">
            {status?.total_sessions ?? "—"}
          </span>
        </span>
        <span>
          Projects{" "}
          <span className="text-foreground font-semibold">
            {status?.projects_tracked ?? "—"}
          </span>
        </span>
        <span className="ml-auto truncate max-w-[240px]" title={config?.kg_path}>
          {config?.kg_path || "knox.db"}
        </span>
      </div>

      <div className="grid grid-cols-1 sm:grid-cols-2 xl:grid-cols-4 gap-3">
        {isLoading && !status ? (
          Array.from({ length: 4 }).map((_, i) => <SkeletonKpi key={i} />)
        ) : (
          <>
            <KpiCard
              title="Total Actions"
              value={status?.total_actions ?? 0}
              subtitle="Indexed in SQLite knowledge graph"
              icon={<Activity className="text-primary" />}
              trend={{
                direction: (status?.total_actions ?? 0) > 0 ? "up" : "neutral",
                label: "Indexed",
              }}
              onClick={() => setActiveTab("query")}
            />
            <KpiCard
              title="Active Sessions"
              value={status?.total_sessions ?? 0}
              subtitle="AI & human session logs"
              icon={<Clock className="text-plan" />}
              trend={{ direction: "neutral", label: "Tracked" }}
              onClick={() => setActiveTab("stats")}
            />
            <KpiCard
              title="Projects Tracked"
              value={status?.projects_tracked ?? 0}
              subtitle="Registered codebases"
              icon={<FolderGit2 className="text-primary" />}
              onClick={() => setActiveTab("workspace")}
            />
            <KpiCard
              title="Last Activity"
              value={status?.total_actions ? "Active" : "Idle"}
              subtitle={lastUpdateStr}
              icon={<Calendar className="text-warning" />}
              trend={{
                direction: status?.total_actions ? "up" : "neutral",
                label: "Status",
              }}
            />
          </>
        )}
      </div>

      <div className="grid grid-cols-1 lg:grid-cols-3 gap-4">
        <Card className="lg:col-span-2">
          <CardHeader className="flex-row items-center justify-between space-y-0 pb-0">
            <CardTitle className="flex items-center gap-2">
              <Activity className="size-4 text-primary" aria-hidden />
              Recent Activity
              <Badge variant="secondary" className="font-mono">
                {recentActions.length}
              </Badge>
            </CardTitle>
            <Button
              variant="ghost"
              size="sm"
              onClick={() => setActiveTab("query")}
              className="text-primary"
            >
              View all
              <ArrowRight className="size-3" />
            </Button>
          </CardHeader>
          <CardContent className="pt-4 space-y-1.5">
            {isLoading && actions.length === 0 ? (
              Array.from({ length: 6 }).map((_, i) => <SkeletonRow key={i} />)
            ) : recentActions.length === 0 ? (
              <EmptyState
                compact
                icon={<Activity />}
                title="No graph actions yet"
                description="Import a transcript or connect an agent via MCP to start building memory."
                action={
                  <div className="flex gap-2 justify-center">
                    <Button
                      variant="primary"
                      size="sm"
                      onClick={() => setActiveTab("ingest")}
                    >
                      Ingest Transcript
                    </Button>
                    <Button
                      variant="outline"
                      size="sm"
                      onClick={() => setActiveTab("mcp")}
                    >
                      Configure MCP
                    </Button>
                  </div>
                }
              />
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
          </CardContent>
        </Card>

        <div className="space-y-4">
          <Card>
            <CardHeader className="pb-0">
              <CardTitle className="flex items-center gap-2">
                <Shield className="size-4 text-primary" aria-hidden />
                Workspace Context
              </CardTitle>
            </CardHeader>
            <CardContent className="pt-4 space-y-3 text-xs">
              <div className="p-3 rounded-md border border-border bg-background space-y-1">
                <div className="flex items-center gap-1.5 text-muted-foreground">
                  <Database className="size-3.5 text-success" aria-hidden />
                  <span className="font-medium">Database</span>
                </div>
                <p className="font-mono text-foreground break-all text-sm">
                  {config?.kg_path || "knox.db"}
                </p>
              </div>

              <div className="p-3 rounded-md border border-border bg-background space-y-1">
                <div className="flex items-center gap-1.5 text-muted-foreground">
                  <Terminal className="size-3.5 text-primary" aria-hidden />
                  <span className="font-medium">MCP transport</span>
                </div>
                <p className="font-mono text-foreground text-sm">
                  knox serve --stdio
                </p>
              </div>

              <div className="p-3 rounded-md border border-border bg-background space-y-1.5 text-sm">
                <div className="flex justify-between text-muted-foreground">
                  <span>Version</span>
                  <span className="font-mono text-foreground">
                    v{APP_VERSION}
                  </span>
                </div>
                <div className="flex justify-between text-muted-foreground">
                  <span>Engine</span>
                  <span className="text-foreground">Rust + SQLite</span>
                </div>
                <div className="flex justify-between text-muted-foreground">
                  <span>Session timeout</span>
                  <span className="font-mono text-foreground">
                    {config?.session_timeout_seconds
                      ? `${config.session_timeout_seconds}s`
                      : "1800s"}
                  </span>
                </div>
              </div>

              {topActors.length > 0 && (
                <div className="space-y-1.5">
                  <span className="text-overline">Top actors</span>
                  <div className="flex flex-wrap gap-1.5">
                    {topActors.map(([actor, count]) => (
                      <Badge key={actor} variant="secondary">
                        {actor}
                        <span className="text-muted-foreground ml-1">
                          {count}
                        </span>
                      </Badge>
                    ))}
                  </div>
                </div>
              )}

              <div className="pt-1 space-y-2">
                <Button
                  variant="outline"
                  size="sm"
                  className="w-full"
                  onClick={() => setActiveTab("mcp")}
                >
                  MCP Configuration
                </Button>
                <Button
                  variant="outline"
                  size="sm"
                  className="w-full"
                  onClick={() => setActiveTab("scanner")}
                >
                  Human Edit Scan
                </Button>
              </div>
            </CardContent>
          </Card>
        </div>
      </div>
    </div>
  );
};
