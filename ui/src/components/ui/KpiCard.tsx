import React from "react";
import { TrendingUp, TrendingDown, Minus } from "lucide-react";

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
  return (
    <div
      onClick={onClick}
      className={`relative overflow-hidden rounded-xl border border-border/70 bg-card p-4 transition-all duration-150 hover:border-border hover:shadow-md ${
        onClick ? "cursor-pointer hover:bg-card/90" : ""
      } ${className}`}
    >
      <div className="flex items-center justify-between">
        <span className="text-[11px] font-semibold tracking-wider text-muted-foreground uppercase">
          {title}
        </span>
        {icon && <span className="text-muted-foreground/80">{icon}</span>}
      </div>

      <div className="mt-2.5 flex items-baseline gap-2">
        <span className="text-2xl font-bold tracking-tight text-foreground font-mono">
          {value}
        </span>
        {badge && (
          <span className="text-[10px] px-1.5 py-0.5 rounded bg-primary/10 text-primary border border-primary/20 font-mono">
            {badge}
          </span>
        )}
      </div>

      <div className="mt-1.5 flex items-center justify-between text-xs text-muted-foreground">
        {subtitle && <p className="truncate max-w-[200px]">{subtitle}</p>}
        {trend && (
          <div
            className={`flex items-center gap-0.5 text-[11px] font-medium shrink-0 ${
              trend.direction === "up"
                ? "text-emerald-400"
                : trend.direction === "down"
                ? "text-rose-400"
                : "text-muted-foreground"
            }`}
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
