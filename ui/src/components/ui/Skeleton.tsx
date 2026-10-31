import React from "react";
import { cn } from "@/lib/cn";

export interface SkeletonProps extends React.HTMLAttributes<HTMLDivElement> {
  /** Optional preset shapes for common loading layouts. */
  variant?: "block" | "line" | "circle" | "kpi" | "row";
}

export const Skeleton: React.FC<SkeletonProps> = ({
  className,
  variant = "block",
  ...props
}) => {
  const presets: Record<NonNullable<SkeletonProps["variant"]>, string> = {
    block: "h-24 w-full rounded-lg",
    line: "h-3 w-full rounded-sm",
    circle: "size-8 rounded-full",
    kpi: "h-24 w-full rounded-lg p-4 flex flex-col justify-between",
    row: "h-11 w-full rounded-md",
  };

  return (
    <div
      aria-hidden
      className={cn(
        "animate-pulse bg-muted/60 border border-border-subtle",
        presets[variant],
        className
      )}
      {...props}
    />
  );
};

/** Convenience KPI loading grid item matching KpiCard footprint. */
export const SkeletonKpi: React.FC<{ className?: string }> = ({ className }) => (
  <Skeleton variant="kpi" className={className}>
    <div className="h-3 w-20 bg-muted rounded-sm" />
    <div className="h-5 w-12 bg-muted/80 rounded-sm" />
    <div className="h-3 w-32 bg-muted/50 rounded-sm" />
  </Skeleton>
);

/** Convenience action-row loading placeholder. */
export const SkeletonRow: React.FC<{ className?: string }> = ({ className }) => (
  <Skeleton variant="row" className={cn("px-3 py-2.5 flex items-center gap-2.5", className)}>
    <div className="h-4 w-14 bg-muted rounded-sm" />
    <div className="h-4 w-12 bg-muted/80 rounded-sm" />
    <div className="h-4 w-48 bg-muted/50 rounded-sm" />
  </Skeleton>
);
