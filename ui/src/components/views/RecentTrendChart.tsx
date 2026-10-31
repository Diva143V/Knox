import React from "react";
import type { KnoxAction } from "@/types/knox";
import { Activity } from "lucide-react";
import { EmptyState } from "@/components/ui/EmptyState";

interface RecentTrendChartProps {
  actions: KnoxAction[];
}

interface DayData {
  dayName: string;
  dateStr: string;
  count: number;
  isToday: boolean;
}

export const RecentTrendChart: React.FC<RecentTrendChartProps> = ({
  actions,
}) => {
  const now = new Date();
  const days: DayData[] = [];

  for (let i = 6; i >= 0; i--) {
    const d = new Date(now);
    d.setDate(d.getDate() - i);
    d.setHours(0, 0, 0, 0);

    const nextDay = new Date(d);
    nextDay.setDate(nextDay.getDate() + 1);

    const dayName =
      i === 0
        ? "Today"
        : d.toLocaleDateString(undefined, { weekday: "short" });

    const dateStr = d.toLocaleDateString(undefined, {
      month: "short",
      day: "numeric",
    });

    const count = actions.filter((a) => {
      const actTime = new Date(a.timestamp).getTime();
      return actTime >= d.getTime() && actTime < nextDay.getTime();
    }).length;

    days.push({ dayName, dateStr, count, isToday: i === 0 });
  }

  const maxCount = Math.max(...days.map((d) => d.count), 1);
  const total7Days = days.reduce((acc, d) => acc + d.count, 0);
  const dailyAvg = (total7Days / 7).toFixed(1);

  const svgWidth = 600;
  const svgHeight = 168;
  const padLeft = 36;
  const padRight = 36;
  const padTop = 28;
  const padBottom = 40;
  const plotWidth = svgWidth - padLeft - padRight;
  const plotHeight = svgHeight - padTop - padBottom;

  const points = days.map((d, index) => {
    const x = padLeft + (index / (days.length - 1)) * plotWidth;
    const y = svgHeight - padBottom - (d.count / maxCount) * plotHeight;
    return { x, y, data: d };
  });

  const polylinePoints = points.map((p) => `${p.x},${p.y}`).join(" ");

  const areaPath =
    `M ${points[0].x},${svgHeight - padBottom} ` +
    points.map((p) => `L ${p.x},${p.y}`).join(" ") +
    ` L ${points[points.length - 1].x},${svgHeight - padBottom} Z`;

  return (
    <section
      className="rounded-lg border border-border bg-card p-5 space-y-4"
      aria-labelledby="trend-chart-title"
    >
      <div className="flex flex-wrap items-center justify-between gap-3">
        <div className="flex items-center gap-2">
          <Activity className="size-4 text-primary" aria-hidden />
          <h2
            id="trend-chart-title"
            className="text-sm font-semibold tracking-tight text-foreground"
          >
            7-Day Activity Trend
          </h2>
          <span className="text-xs text-muted-foreground font-mono bg-background border border-border px-1.5 py-0.5 rounded-sm">
            Last 7 Days
          </span>
        </div>

        <div className="flex items-center gap-4 text-xs font-mono">
          <div className="flex items-center gap-1.5 text-muted-foreground">
            <span>Total</span>
            <span className="font-semibold text-foreground">{total7Days}</span>
          </div>
          <div className="flex items-center gap-1.5 text-muted-foreground">
            <span>Daily avg</span>
            <span className="font-semibold text-foreground">{dailyAvg}</span>
          </div>
        </div>
      </div>

      {total7Days === 0 ? (
        <EmptyState
          compact
          icon={<Activity />}
          title="No activity in the last 7 days"
          description="New indexed actions will appear here as a velocity sparkline."
        />
      ) : (
        <div className="w-full overflow-hidden">
          <svg
            viewBox={`0 0 ${svgWidth} ${svgHeight}`}
            className="w-full h-40 sm:h-44"
            role="img"
            aria-label={`Activity over 7 days. Total ${total7Days}, daily average ${dailyAvg}.`}
          >
            <defs>
              <linearGradient id="trendGradient" x1="0" y1="0" x2="0" y2="1">
                <stop
                  offset="0%"
                  stopColor="var(--primary)"
                  stopOpacity="0.22"
                />
                <stop
                  offset="100%"
                  stopColor="var(--primary)"
                  stopOpacity="0"
                />
              </linearGradient>
            </defs>

            <line
              x1={padLeft}
              y1={padTop}
              x2={svgWidth - padRight}
              y2={padTop}
              stroke="var(--border)"
              strokeDasharray="3 3"
              strokeOpacity="0.5"
            />
            <line
              x1={padLeft}
              y1={padTop + plotHeight / 2}
              x2={svgWidth - padRight}
              y2={padTop + plotHeight / 2}
              stroke="var(--border)"
              strokeDasharray="3 3"
              strokeOpacity="0.5"
            />
            <line
              x1={padLeft}
              y1={svgHeight - padBottom}
              x2={svgWidth - padRight}
              y2={svgHeight - padBottom}
              stroke="var(--border)"
              strokeOpacity="0.7"
            />

            <path d={areaPath} fill="url(#trendGradient)" />

            <polyline
              fill="none"
              stroke="var(--primary)"
              strokeWidth="2"
              strokeLinecap="round"
              strokeLinejoin="round"
              points={polylinePoints}
            />

            {points.map((p, idx) => (
              <g key={idx}>
                <title>
                  {p.data.dayName} {p.data.dateStr}: {p.data.count} actions
                </title>
                {p.data.count > 0 && (
                  <circle
                    cx={p.x}
                    cy={p.y}
                    r={5}
                    fill="none"
                    stroke="var(--primary)"
                    strokeWidth="1.5"
                    opacity={0.35}
                  />
                )}
                <circle
                  cx={p.x}
                  cy={p.y}
                  r={p.data.isToday ? 4.5 : 3.5}
                  fill="var(--primary)"
                  stroke="var(--card)"
                  strokeWidth="2"
                />
                <text
                  x={p.x}
                  y={p.y - 10}
                  textAnchor="middle"
                  fill="var(--foreground)"
                  className="text-xs font-mono font-medium"
                >
                  {p.data.count}
                </text>
                <text
                  x={p.x}
                  y={svgHeight - padBottom + 16}
                  textAnchor="middle"
                  fill={
                    p.data.isToday
                      ? "var(--primary)"
                      : "var(--muted-foreground)"
                  }
                  className={`text-sm ${
                    p.data.isToday ? "font-semibold" : "font-normal"
                  }`}
                >
                  {p.data.dayName}
                </text>
                <text
                  x={p.x}
                  y={svgHeight - padBottom + 28}
                  textAnchor="middle"
                  fill="var(--muted-foreground)"
                  className="text-2xs font-mono opacity-70"
                >
                  {p.data.dateStr}
                </text>
              </g>
            ))}
          </svg>
        </div>
      )}
    </section>
  );
};
