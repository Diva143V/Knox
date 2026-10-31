import React from "react";
import { TrendingUp, TrendingDown, Minus } from "lucide-react";
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

export const KpiCard: React.FC<KpiCardProps> = ({
  title,
  value,
  subtitle,
  icon,
  badge,
  trend,
  onClick,
  className = "",
}) => {
  const interactive = Boolean(onClick);

  return (
    <div
      onClick={onClick}
      onKeyDown={
        interactive
          ? (e) => {
              if (e.key === "Enter" || e.key === " ") {
                e.preventDefault();
                onClick?.();
              }
            }
          : undefined
      }
      role={interactive ? "button" : undefined}
      tabIndex={interactive ? 0 : undefined}
      className={cn(
        "relative rounded-lg border border-border bg-card p-4 transition-colors duration-100",
        interactive &&
          "cursor-pointer hover:border-primary/40 hover:bg-card/90 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-focus-ring",
        className
      )}
    >
      <div className="flex items-center justify-between">
        <span className="text-overline">{title}</span>
        {icon && (
          <span className="text-muted-foreground [&_svg]:size-4">{icon}</span>
        )}
      </div>

      <div className="mt-2.5 flex items-baseline gap-2">
        <span className="text-2xl font-semibold tracking-tight text-foreground font-mono">
          {value}
        </span>
        {badge && (
          <span className="text-xs px-1.5 py-0.5 rounded-sm bg-primary-muted text-primary border border-primary/20 font-mono">
            {badge}
          </span>
        )}
      </div>

      <div className="mt-1.5 flex items-center justify-between text-sm text-muted-foreground">
        {subtitle && <p className="truncate max-w-[200px]">{subtitle}</p>}
        {trend && (
          <div
            className={cn(
              "flex items-center gap-0.5 text-sm font-medium shrink-0",
              trend.direction === "up" && "text-success",
              trend.direction === "down" && "text-danger",
              trend.direction === "neutral" && "text-muted-foreground"
            )}
          >
            {trend.direction === "up" && <TrendingUp className="w-3 h-3" />}
            {trend.direction === "down" && <TrendingDown className="w-3 h-3" />}
            {trend.direction === "neutral" && <Minus className="w-3 h-3" />}
            <span>{trend.label}</span>
          </div>
        )}
      </div>
    </div>
  );
};
