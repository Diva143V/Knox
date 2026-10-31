import React from "react";
import { cn } from "@/lib/cn";

export interface FilterBarProps extends React.HTMLAttributes<HTMLDivElement> {
  /** Primary search / control row */
  primary?: React.ReactNode;
  /** Secondary filters row */
  secondary?: React.ReactNode;
  /** Optional meta strip below filters (counts, hints) */
  meta?: React.ReactNode;
}

/**
 * Shared sticky filter chrome for Search / Graph toolbars.
 * Screens adopt this in later phases; shipped here as the design-system primitive.
 */
export const FilterBar: React.FC<FilterBarProps> = ({
  primary,
  secondary,
  meta,
  className,
  children,
  ...props
}) => {
  return (
    <div
      className={cn(
        "shrink-0 border-b border-border bg-card/80",
        className
      )}
      {...props}
    >
      {(primary || children) && (
        <div className="px-4 pt-3 pb-2 flex flex-wrap items-center gap-2">
          {primary ?? children}
        </div>
      )}
      {secondary && (
        <div className="px-4 pb-3 flex flex-wrap items-center gap-2.5 text-xs">
          {secondary}
        </div>
      )}
      {meta && (
        <div className="px-4 py-2 border-t border-border-subtle bg-background/50 flex items-center justify-between text-sm text-muted-foreground font-mono">
          {meta}
        </div>
      )}
    </div>
  );
};
