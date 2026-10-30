import React from "react";
import type { KnoxAction } from "@/types/knox";
import { Activity } from "lucide-react";

interface RecentTrendChartProps {
  actions: KnoxAction[];
}

interface DayData {
  dayName: string;
  dateStr: string;
  count: number;
  isToday: boolean;
}

export const RecentTrendChart: React.FC<RecentTrendChartProps> = ({ actions }) => {
  // Aggregate last 7 days of activity
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

  // SVG dimensions
  const svgWidth = 600;
  const svgHeight = 160;
  const padLeft = 35;
  const padRight = 35;
  const padTop = 25;
  const padBottom = 35;
  const plotWidth = svgWidth - padLeft - padRight;
  const plotHeight = svgHeight - padTop - padBottom;

  const points = days.map((d, index) => {
    const x = padLeft + (index / (days.length - 1)) * plotWidth;
    const y = svgHeight - padBottom - (d.count / maxCount) * plotHeight;
    return { x, y, data: d };
  });

  const polylinePoints = points.map((p) => `${p.x},${p.y}`).join(" ");

  // Closed area path for gradient fill
  const areaPath = `M ${points[0].x},${svgHeight - padBottom} ` +
    points.map((p) => `L ${p.x},${p.y}`).join(" ") +
    ` L ${points[points.length - 1].x},${svgHeight - padBottom} Z`;

  return (
    <div className="rounded-xl border border-border/70 bg-card p-5 space-y-4">
      {/* Chart Title & Stat Summary */}
      <div className="flex flex-wrap items-center justify-between gap-3">
        <div className="flex items-center gap-2">
          <Activity className="w-4 h-4 text-primary" />
          <h2 className="text-sm font-semibold tracking-tight text-foreground">
            7-Day Activity Trend
          </h2>
          <span className="text-[10px] text-muted-foreground font-mono bg-card border border-border/60 px-1.5 py-0.5 rounded">
            Last 7 Days
          </span>
        </div>

        <div className="flex items-center gap-4 text-xs font-mono">
          <div className="flex items-center gap-1.5 text-muted-foreground">
            <span>7-day total:</span>
            <span className="font-semibold text-foreground">{total7Days}</span>
          </div>
          <div className="flex items-center gap-1.5 text-muted-foreground">
            <span>Daily avg:</span>
            <span className="font-semibold text-foreground">{dailyAvg}</span>
          </div>
        </div>
      </div>

      {/* Inline SVG Sparkline */}
      <div className="w-full overflow-hidden">
        <svg
          viewBox={`0 0 ${svgWidth} ${svgHeight}`}
          className="w-full h-40 overflow-visible"
        >
          <defs>
            <linearGradient id="trendGradient" x1="0" y1="0" x2="0" y2="1">
              <stop offset="0%" stopColor="#6366f1" stopOpacity="0.35" />
              <stop offset="100%" stopColor="#6366f1" stopOpacity="0.0" />
            </linearGradient>
            <linearGradient id="lineGradient" x1="0" y1="0" x2="1" y2="0">
              <stop offset="0%" stopColor="#818cf8" />
              <stop offset="100%" stopColor="#6366f1" />
            </linearGradient>
          </defs>

          {/* Horizontal Grid lines */}
          <line
            x1={padLeft}
            y1={padTop}
            x2={svgWidth - padRight}
            y2={padTop}
            stroke="var(--border)"
            strokeDasharray="3 3"
            strokeOpacity="0.4"
          />
          <line
            x1={padLeft}
            y1={padTop + plotHeight / 2}
            x2={svgWidth - padRight}
            y2={padTop + plotHeight / 2}
            stroke="var(--border)"
            strokeDasharray="3 3"
            strokeOpacity="0.4"
          />
          <line
            x1={padLeft}
            y1={svgHeight - padBottom}
            x2={svgWidth - padRight}
            y2={svgHeight - padBottom}
            stroke="var(--border)"
            strokeOpacity="0.6"
          />

          {/* Gradient Area Fill */}
          <path d={areaPath} fill="url(#trendGradient)" />

          {/* Polylines for line stroke */}
          <polyline
            fill="none"
            stroke="url(#lineGradient)"
            strokeWidth="2.5"
            strokeLinecap="round"
            strokeLinejoin="round"
            points={polylinePoints}
          />

          {/* Interactive / visual Data Points */}
          {points.map((p, idx) => (
            <g key={idx} className="group cursor-pointer">
              {/* Outer pulsing ring for non-zero points */}
              {p.data.count > 0 && (
                <circle
                  cx={p.x}
                  cy={p.y}
                  r={5}
                  fill="none"
                  stroke="#6366f1"
                  strokeWidth="1.5"
                  className="opacity-40"
                />
              )}
              {/* Center point circle */}
              <circle
                cx={p.x}
                cy={p.y}
                r={p.data.isToday ? 4.5 : 3.5}
                fill={p.data.isToday ? "#a5b4fc" : "#6366f1"}
                stroke="var(--card)"
                strokeWidth="2"
                className="transition-all hover:r-6"
              />

              {/* Count label above point */}
              <text
                x={p.x}
                y={p.y - 10}
                textAnchor="middle"
                fill="var(--foreground)"
                className="text-[10px] font-mono font-medium"
              >
                {p.data.count}
              </text>

              {/* Day Name label below bottom line */}
              <text
                x={p.x}
                y={svgHeight - padBottom + 16}
                textAnchor="middle"
                fill={p.data.isToday ? "var(--primary)" : "var(--muted-foreground)"}
                className={`text-[11px] ${
                  p.data.isToday ? "font-semibold" : "font-normal"
                }`}
              >
                {p.data.dayName}
              </text>

              {/* Date string subtle subtitle */}
              <text
                x={p.x}
                y={svgHeight - padBottom + 27}
                textAnchor="middle"
                fill="var(--muted-foreground)"
                className="text-[9px] font-mono opacity-60"
              >
                {p.data.dateStr}
              </text>
            </g>
          ))}
        </svg>
      </div>
    </div>
  );
};
