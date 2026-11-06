import React, { useMemo } from "react";
import {
  Activity,
  ArrowRight,
  CircleHelp,
  FileCode,
  Network,
  Search,
  ShieldCheck,
} from "lucide-react";
import { useKnoxStore } from "@/store/useKnoxStore";
import { ActionRow, ACTION_ROW_GRID } from "@/components/ui/ActionRow";
import { EmptyState } from "@/components/ui/EmptyState";
import { SkeletonRow } from "@/components/ui/Skeleton";
import { Button } from "@/components/ui/Button";
import { PageHeader } from "@/components/ui/PageHeader";
import { cn } from "@/lib/cn";
import { APP_VERSION } from "@/lib/navigation";
import type { KnoxAction } from "@/types/knox";

const numberFormat = new Intl.NumberFormat();
const relativeFormat = new Intl.RelativeTimeFormat(undefined, {
  numeric: "auto",
});
const dayLabelFormat = new Intl.DateTimeFormat(undefined, {
  weekday: "short",
});
const dayDateFormat = new Intl.DateTimeFormat(undefined, {
  month: "short",
  day: "numeric",
});

/** "just now" / "4 minutes ago" / "yesterday" — never a raw ISO string. */
function formatAge(iso?: string | null): string {
  if (!iso) return "no activity yet";
  const time = new Date(iso).getTime();
  if (Number.isNaN(time)) return "no activity yet";

  const deltaSeconds = (time - Date.now()) / 1000;
  const magnitude = Math.abs(deltaSeconds);
  if (magnitude < 60) return "just now";
  if (magnitude < 3600)
    return relativeFormat.format(Math.round(deltaSeconds / 60), "minute");
  if (magnitude < 86400)
    return relativeFormat.format(Math.round(deltaSeconds / 3600), "hour");
  return relativeFormat.format(Math.round(deltaSeconds / 86400), "day");
}

interface DayBucket {
  label: string;
  date: string;
  count: number;
  isToday: boolean;
}

/** Seven daily columns with a real value scale and dates under each bar. */
const WeekActivity: React.FC<{ actions: KnoxAction[] }> = ({ actions }) => {
  const days = useMemo<DayBucket[]>(() => {
    const buckets: DayBucket[] = [];
    const start = new Date();
    start.setHours(0, 0, 0, 0);

    for (let offset = 6; offset >= 0; offset--) {
      const dayStart = new Date(start);
      dayStart.setDate(dayStart.getDate() - offset);
      const dayEnd = new Date(dayStart);
      dayEnd.setDate(dayEnd.getDate() + 1);

      const count = actions.filter((action) => {
        const at = new Date(action.timestamp).getTime();
        return at >= dayStart.getTime() && at < dayEnd.getTime();
      }).length;

      buckets.push({
        label: offset === 0 ? "Today" : dayLabelFormat.format(dayStart),
        date: dayDateFormat.format(dayStart),
        count,
        isToday: offset === 0,
      });
    }
    return buckets;
  }, [actions]);

  const peak = Math.max(...days.map((day) => day.count), 1);
  const total = days.reduce((sum, day) => sum + day.count, 0);
  const mid = Math.round(peak / 2);

  return (
    <div className="flex gap-3">
      <div className="flex w-6 shrink-0 flex-col justify-between pb-9 text-right font-mono text-3xs tabular-nums text-muted-foreground">
        <span>{peak}</span>
        <span>{mid}</span>
        <span>0</span>
      </div>

      <div className="min-w-0 flex-1">
        <div className="relative h-32">
          {[0, 0.5, 1].map((ratio) => (
            <div
              key={ratio}
              aria-hidden
              className={cn(
                "absolute inset-x-0 border-t",
                ratio === 1 ? "border-border" : "border-rule"
              )}
              style={{ top: `${ratio * 100}%` }}
            />
          ))}

          <div
            className="absolute inset-0 flex items-end gap-2"
            role="img"
            aria-label={`Daily activity for the last 7 days, total ${total} actions, busiest day ${peak}.`}
          >
            {days.map((day) => {
              const height =
                day.count > 0 ? `${Math.max((day.count / peak) * 100, 6)}%` : "2px";
              return (
                <div
                  key={day.date}
                  className="flex h-full min-w-0 flex-1 flex-col items-center justify-end gap-1"
                >
                  <span className="font-mono text-3xs tabular-nums text-muted-foreground">
                    {day.count}
                  </span>
                  <div
                    className={cn(
                      "w-full rounded-[2px] transition-[height] duration-200",
                      day.count === 0
                        ? "bg-muted-foreground/30"
                        : day.isToday
                        ? "bg-primary"
                        : "bg-foreground/25"
                    )}
                    style={{ height }}
                  />
                </div>
              );
            })}
          </div>
        </div>

        <div className="mt-1.5 flex gap-1 border-t border-rule pt-1.5 sm:gap-2">
          {days.map((day) => (
            <div key={day.date} className="min-w-0 flex-1 text-center">
              <div
                className={cn(
                  "truncate text-3xs leading-tight",
                  day.isToday ? "text-foreground" : "text-muted-foreground"
                )}
              >
                {day.label}
              </div>
              <div className="truncate font-mono text-3xs leading-tight text-muted-foreground/70">
                {day.date}
              </div>
            </div>
          ))}
        </div>
      </div>
    </div>
  );
};

const Metric: React.FC<{
  label: string;
  value: number | string;
  note: string;
  suffix?: string;
  tone?: "default" | "warning";
  onClick: () => void;
}> = ({ label, value, note, suffix, tone = "default", onClick }) => (
  <button
    type="button"
    onClick={onClick}
    className={cn(
      "flex min-w-0 cursor-pointer flex-col bg-card px-4 py-3.5 text-left transition-colors duration-100 hover:bg-raised",
      "focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-inset focus-visible:ring-focus-ring"
    )}
  >
    <span className="truncate text-xs text-muted-foreground">{label}</span>
    <span className="mt-2 flex items-baseline gap-1.5">
      <span className="font-mono text-2xl leading-none tabular-nums tracking-tight text-foreground">
        {typeof value === "number" ? numberFormat.format(value) : value}
      </span>
      {suffix && (
        <span className="font-mono text-3xs text-muted-foreground">
          {suffix}
        </span>
      )}
    </span>
    <span
      className={cn(
        "mt-2 truncate text-3xs",
        tone === "warning" ? "text-warning" : "text-muted-foreground"
      )}
    >
      {note}
    </span>
  </button>
);

const Fact: React.FC<{ label: string; children: React.ReactNode }> = ({
  label,
  children,
}) => (
  <div className="px-3 py-2.5">
    <dt className="truncate text-3xs text-muted-foreground">{label}</dt>
    <dd className="mt-1 truncate font-mono text-xs tabular-nums text-foreground">
      {children}
    </dd>
  </div>
);

export const DashboardView: React.FC = () => {
  const {
    status,
    actions,
    config,
    isLoading,
    setActiveTab,
    selectedAction,
    setSelectedAction,
    activeProjectScope,
  } = useKnoxStore();

  const scopeName = activeProjectScope
    ? activeProjectScope.replace(/\\/g, "/").split("/").filter(Boolean).pop() ||
      activeProjectScope
    : "Global knowledge graph";

  const openDoubts = actions.filter(
    (action) =>
      (action.action_type === "doubt" || action.action_type === "question") &&
      !action.is_resolved
  );
  const staleActions = actions.filter((action) => !action.is_current);

  const recentActions = actions.slice(0, 8);
  const latest = actions[0];

  const weekCount = useMemo(() => {
    const weekAgo = Date.now() - 7 * 24 * 60 * 60 * 1000;
    return actions.filter(
      (action) => new Date(action.timestamp).getTime() >= weekAgo
    ).length;
  }, [actions]);

  const agents = Object.keys(status?.actions_by_actor ?? {}).length;
  const totalActions = status?.total_actions ?? 0;

  const agentShares = useMemo(() => {
    const entries = Object.entries(status?.actions_by_actor ?? {}).sort(
      (a, b) => b[1] - a[1]
    );
    const top = Math.max(totalActions, 1);
    return entries.slice(0, 4).map(([actor, count]) => ({
      actor,
      count,
      share: Math.round((count / top) * 100),
    }));
  }, [status?.actions_by_actor, totalActions]);
  const databaseName = config?.kg_path
    ? config.kg_path.replace(/\\/g, "/").split("/").pop()
    : "knox.db";

  const attentionCount = openDoubts.length + staleActions.length;

  return (
    <div className="h-full overflow-y-auto">
      <div className="mx-auto max-w-6xl space-y-5 px-4 py-5 sm:px-6 sm:py-6">
        {/* Title row: scope, health pill, provenance, one primary action */}
        <PageHeader
          title={scopeName}
          status={
            <span className="inline-flex items-center gap-1.5 rounded-[var(--radius-control)] border border-border px-2 py-0.5 text-3xs text-muted-foreground">
              <span
                className={cn(
                  "size-1.5 rounded-full",
                  isLoading
                    ? "bg-warning"
                    : status
                    ? "bg-success"
                    : "bg-muted-foreground"
                )}
                aria-hidden
              />
              {isLoading ? "Syncing…" : status ? `Synced · v${APP_VERSION}` : "Database unavailable"}
            </span>
          }
          meta={
            <>
              <span translate="no" title={config?.kg_path || "knox.db"}>
                {databaseName}
              </span>
              <span aria-hidden className="text-border">/</span>
              <span translate="no">stdio</span>
              <span aria-hidden className="text-border">/</span>
              <span>updated {formatAge(status?.last_action_timestamp)}</span>
            </>
          }
          actions={
            <>
              <Button
                variant="primary"
                size="md"
                onClick={() => setActiveTab("graph")}
                className="gap-1.5 whitespace-nowrap"
              >
                <Network className="size-3.5" aria-hidden />
                Open graph
              </Button>
              <Button
                variant="ghost"
                size="md"
                onClick={() => setActiveTab("query")}
                className="gap-1.5 whitespace-nowrap"
              >
                <Search className="size-3.5" aria-hidden />
                Query
              </Button>
            </>
          }
        />

        {/* What is live now: the newest record, with its own facts */}
        <section
          className="overflow-hidden rounded-[var(--radius-card)] border border-border bg-card"
          aria-labelledby="latest-memory-heading"
        >
          <div className="flex flex-col lg:flex-row">
            <div className="min-w-0 flex-1 p-4 lg:p-5">
              <div className="flex items-center gap-2 text-3xs text-muted-foreground">
                <Activity className="size-3.5" aria-hidden />
                <h2 id="latest-memory-heading" className="font-normal">
                  Latest recorded memory
                </h2>
                {latest && (
                  <>
                    <span aria-hidden className="text-border">/</span>
                    <span className="font-mono">{formatAge(latest.timestamp)}</span>
                  </>
                )}
              </div>

              {latest ? (
                <button
                  type="button"
                  onClick={() => setSelectedAction(latest)}
                  className="group mt-2 block w-full min-w-0 cursor-pointer text-left focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-offset-4 focus-visible:ring-offset-card focus-visible:ring-focus-ring"
                >
                  <p className="line-clamp-2 text-base font-semibold leading-snug text-foreground underline-offset-4 group-hover:underline decoration-border">
                    {latest.summary}
                  </p>
                  <span className="mt-2 flex flex-wrap items-center gap-x-3 gap-y-1 font-mono text-3xs text-muted-foreground">
                    <span className="uppercase tracking-[0.06em]">
                      {latest.action_type}
                    </span>
                    <span className="text-border" aria-hidden>/</span>
                    <span translate="no">{latest.actor}</span>
                    {latest.commit_sha && (
                      <>
                        <span className="text-border" aria-hidden>/</span>
                        <span translate="no">{latest.commit_sha.slice(0, 7)}</span>
                      </>
                    )}
                    <span className="text-muted-foreground transition-colors group-hover:text-foreground">
                      Open record
                    </span>
                  </span>
                </button>
              ) : (
                <div className="mt-2">
                  <p className="text-base font-semibold text-foreground">
                    No memory recorded yet
                  </p>
                  <p className="mt-1 text-xs text-muted-foreground">
                    Connect an agent over MCP or ingest a transcript to start the graph.
                  </p>
                </div>
              )}
            </div>

            <dl className="grid grid-cols-2 divide-x divide-rule border-t border-border sm:grid-cols-4 lg:w-[25rem] lg:grid-cols-2 lg:border-l lg:border-t-0">
              <Fact label="Files">{latest?.files?.length ?? 0}</Fact>
              <Fact label="Symbols">{latest?.symbols?.length ?? 0}</Fact>
              <Fact label="Lines changed">
                {latest?.delta_summary &&
                (latest.delta_summary.lines_added > 0 ||
                  latest.delta_summary.lines_deleted > 0) ? (
                  <span className="flex items-baseline gap-1.5">
                    <span className="text-success">
                      +{latest.delta_summary.lines_added}
                    </span>
                    <span className="text-danger">
                      -{latest.delta_summary.lines_deleted}
                    </span>
                  </span>
                ) : (
                  "—"
                )}
              </Fact>
              <Fact label="Significance">
                {latest?.significance ? latest.significance : "—"}
              </Fact>
            </dl>
          </div>
        </section>

        {/* Divided metric strip */}
        <section
          className="grid grid-cols-2 gap-px overflow-hidden rounded-[var(--radius-card)] border border-border bg-rule lg:grid-cols-4"
          aria-label="Knowledge graph metrics"
        >
          <Metric
            label="Total actions"
            value={totalActions}
            suffix="nodes"
            note={
              weekCount > 0
                ? `+${numberFormat.format(weekCount)} in the last 7 days`
                : "No activity this week"
            }
            onClick={() => setActiveTab("query")}
          />
          <Metric
            label="Sessions"
            value={status?.total_sessions ?? 0}
            suffix="recorded"
            note={`${agents} agent${agents === 1 ? "" : "s"} contributing`}
            onClick={() => setActiveTab("stats")}
          />
          <Metric
            label="Projects"
            value={status?.projects_tracked ?? 0}
            suffix="roots"
            note="Registered workspaces"
            onClick={() => setActiveTab("workspace")}
          />
          <Metric
            label="Open doubts"
            value={openDoubts.length}
            suffix={openDoubts.length > 0 ? "open" : ""}
            note={
              openDoubts.length > 0
                ? "Needs developer triage"
                : "Nothing awaiting an answer"
            }
            tone={openDoubts.length > 0 ? "warning" : "default"}
            onClick={() => setActiveTab("query")}
          />
        </section>

        {/* Volume over time, beside the block that explains the exceptions */}
        <div className="grid grid-cols-1 items-stretch gap-4 lg:grid-cols-3">
          <section
            className="flex min-w-0 flex-col rounded-[var(--radius-card)] border border-border bg-card p-4 lg:col-span-2"
            aria-labelledby="activity-heading"
          >
            <div className="flex flex-wrap items-baseline justify-between gap-2">
              <h2
                id="activity-heading"
                className="text-xs font-medium text-muted-foreground"
              >
                Actions recorded, last 7 days
              </h2>
              <p className="font-mono text-3xs tabular-nums text-muted-foreground">
                <span className="text-foreground">{weekCount}</span> in 7d
                <span className="mx-1.5 text-border" aria-hidden>/</span>
                <span className="text-foreground">
                  {numberFormat.format(totalActions)}
                </span>{" "}
                total
              </p>
            </div>

            <div className="mt-4 flex-1">
              {isLoading && actions.length === 0 ? (
                <div className="h-32 animate-pulse rounded-[var(--radius-card)] bg-muted/40" />
              ) : (
                <WeekActivity actions={actions} />
              )}
            </div>
          </section>

          <section
            className="flex min-w-0 flex-col rounded-[var(--radius-card)] border border-border bg-card"
            aria-labelledby="attention-heading"
          >
            <div className="flex items-center justify-between gap-2 border-b border-rule px-4 py-3">
              <h2
                id="attention-heading"
                className="text-xs font-medium text-muted-foreground"
              >
                Needs attention
              </h2>
              <span className="font-mono text-3xs tabular-nums text-muted-foreground">
                {attentionCount}
              </span>
            </div>

            <div className="flex-1 space-y-2 p-3">
              {openDoubts.slice(0, 3).map((doubt) => (
                <button
                  key={doubt.id}
                  type="button"
                  onClick={() => setSelectedAction(doubt)}
                  className="block w-full min-w-0 cursor-pointer rounded-[var(--radius-sm)] px-2 py-1.5 text-left transition-colors hover:bg-white/[0.03] focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-inset focus-visible:ring-focus-ring"
                >
                  <span className="flex items-center gap-1.5 font-mono text-3xs text-warning">
                    <CircleHelp className="size-3 shrink-0" aria-hidden />
                    Open question
                  </span>
                  <span className="mt-1 block line-clamp-2 text-2xs leading-snug text-foreground">
                    {doubt.summary}
                  </span>
                  <span className="mt-1 block font-mono text-3xs text-muted-foreground">
                    {doubt.actor} · {formatAge(doubt.timestamp)}
                  </span>
                </button>
              ))}

              {staleActions.length > 0 && (
                <button
                  type="button"
                  onClick={() => setActiveTab("query")}
                  className="flex w-full cursor-pointer items-center justify-between gap-2 rounded-[var(--radius-sm)] px-2 py-1.5 text-left transition-colors hover:bg-white/[0.03] focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-inset focus-visible:ring-focus-ring"
                >
                  <span className="text-2xs text-foreground">
                    Outdated records
                  </span>
                  <span className="font-mono text-3xs tabular-nums text-muted-foreground">
                    {staleActions.length}
                  </span>
                </button>
              )}

              {attentionCount === 0 && !isLoading && (
                <div className="space-y-3">
                  <p className="px-2 py-1.5 text-2xs leading-relaxed text-muted-foreground">
                    No open doubts, no outdated decisions.
                  </p>

                  {agentShares.length > 0 && (
                    <div className="border-t border-rule px-2 pt-3">
                      <p className="text-3xs text-muted-foreground">
                        Written by, all time
                      </p>
                      <ul className="mt-2 space-y-2">
                        {agentShares.map(({ actor, count, share }) => (
                          <li key={actor} className="min-w-0">
                            <div className="flex items-baseline justify-between gap-2 font-mono text-3xs">
                              <span className="truncate text-foreground">
                                {actor}
                              </span>
                              <span className="shrink-0 tabular-nums text-muted-foreground">
                                {count} · {share}%
                              </span>
                            </div>
                            <div
                              className="mt-2 h-px w-full bg-rule"
                              role="meter"
                              aria-label={`${actor} ${share} percent of actions`}
                              aria-valuenow={share}
                              aria-valuemin={0}
                              aria-valuemax={100}
                            >
                              <div
                                className="h-px bg-muted-foreground/70"
                                style={{ width: `${Math.max(share, 2)}%` }}
                              />
                            </div>
                          </li>
                        ))}
                      </ul>
                    </div>
                  )}
                </div>
              )}

              {isLoading && actions.length === 0 && (
                <div className="space-y-2 px-2 py-1.5">
                  <div className="h-3 w-3/4 animate-pulse rounded-sm bg-muted/50" />
                  <div className="h-3 w-1/2 animate-pulse rounded-sm bg-muted/50" />
                </div>
              )}
            </div>

            <div className="border-t border-rule p-2">
              <Button
                variant="ghost"
                size="sm"
                onClick={() => setActiveTab("scanner")}
                className="w-full justify-between gap-2 px-2"
              >
                <span className="flex items-center gap-1.5">
                  <ShieldCheck className="size-3.5" aria-hidden />
                  Scan staged edits
                </span>
                <ArrowRight className="size-3" aria-hidden />
              </Button>
            </div>
          </section>
        </div>

        {/* Ledger: one table, no nested card, capped and linked to the full list */}
        <section
          className="overflow-hidden rounded-[var(--radius-card)] border border-border bg-card"
          aria-labelledby="ledger-heading"
        >
          <div className="flex items-center justify-between gap-3 border-b border-rule px-4 py-3">
            <div className="flex min-w-0 items-baseline gap-2">
              <h2
                id="ledger-heading"
                className="text-xs font-medium text-muted-foreground"
              >
                Recent activity
              </h2>
              <span className="font-mono text-3xs tabular-nums text-muted-foreground">
                {recentActions.length} of {numberFormat.format(totalActions)}
              </span>
            </div>
            <Button
              variant="ghost"
              size="sm"
              onClick={() => setActiveTab("query")}
              className="gap-1 px-2"
            >
              View all
              <ArrowRight className="size-3" aria-hidden />
            </Button>
          </div>

          {recentActions.length > 0 && (
            <div
              className={cn(
                ACTION_ROW_GRID,
                "h-8 border-b border-rule px-4 font-mono text-3xs uppercase tracking-[0.06em] text-muted-foreground"
              )}
              aria-hidden
            >
              <span className="order-1">Time</span>
              <span className="order-2 lg:order-4">Summary</span>
              <span className="order-3 lg:order-3">Kind</span>
              <span className="order-4 hidden lg:order-2 lg:block">Agent</span>
              <span className="order-5 hidden lg:block">File</span>
              <span className="order-6 hidden text-right lg:block">Δ</span>
            </div>
          )}

          {isLoading && actions.length === 0 ? (
            <div className="p-4 space-y-2">
              {Array.from({ length: 6 }).map((_, index) => (
                <SkeletonRow key={index} />
              ))}
            </div>
          ) : recentActions.length === 0 ? (
            <div className="p-4">
              <EmptyState
                compact
                icon={<FileCode />}
                title="No graph actions yet"
                description="Import a transcript or connect an agent over MCP to start building memory."
                action={
                  <div className="flex justify-center gap-2">
                    <Button
                      variant="primary"
                      size="sm"
                      onClick={() => setActiveTab("ingest")}
                    >
                      Ingest transcript
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
            </div>
          ) : (
            <div className="divide-y divide-rule">
              {recentActions.map((action) => (
                <ActionRow
                  key={action.id}
                  action={action}
                  isSelected={selectedAction?.id === action.id}
                  onSelect={(record) => setSelectedAction(record)}
                />
              ))}
            </div>
          )}
        </section>
      </div>
    </div>
  );
};
