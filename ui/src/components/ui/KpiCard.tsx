import React from "react";
import { cn } from "@/lib/cn";

interface KpiCardProps {
  title: string;
  value: string | number;
  subtitle?: string;
  icon?: React.ReactNode;
  badge?: string;
  trend?: {
    direction: "up" | "down" | "neutral";
    label: string;
  };
  onClick?: () => void;
  className?: string;
}

/** Divided metric cell: 12 muted label, 22 mono figure, one line of context. */
export const KpiCard: React.FC<KpiCardProps> = ({
  title,
  value,
  subtitle,
  badge,
  trend,
  onClick,
  className = "",
}) => {
  const inner = (
    <>
      <span className="flex items-baseline justify-between gap-2">
        <span className="truncate text-xs text-muted-foreground">{title}</span>
        {badge && (
          <span className="shrink-0 rounded-[3px] bg-muted px-1 py-0.5 font-mono text-[11px] text-muted-foreground">
            {badge}
          </span>
        )}
      </span>

      <span className="mt-2 flex items-baseline gap-1.5">
        <span className="font-mono text-[22px] leading-none tabular-nums tracking-tight text-foreground">
          {value}
        </span>
      </span>

      {(subtitle || trend) && (
        <span className="mt-2 flex items-center justify-between gap-2 text-[11px] text-muted-foreground">
          {subtitle && <span className="min-w-0 truncate">{subtitle}</span>}
          {trend && (
            <span
              className={cn(
                "ml-auto shrink-0 font-mono tabular-nums",
                trend.direction === "up" && "text-success",
                trend.direction === "down" && "text-danger",
                trend.direction === "neutral" && "text-muted-foreground"
              )}
            >
              {trend.label}
            </span>
          )}
        </span>
      )}
    </>
  );

  const base = cn(
    "flex flex-col justify-between bg-card px-4 py-3.5 text-left",
    className
  );

  if (!onClick) {
    return <div className={base}>{inner}</div>;
  }

  return (
    <button
      type="button"
      onClick={onClick}
      className={cn(
        base,
        "cursor-pointer transition-colors duration-100 hover:bg-raised",
        "focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-inset focus-visible:ring-focus-ring"
      )}
    >
      {inner}
    </button>
  );
};
