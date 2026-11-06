import React from "react";
import { cn } from "@/lib/cn";

export interface PageHeaderProps {
  title: string;
  description?: string;
  icon?: React.ReactNode;
  actions?: React.ReactNode;
  /** Secondary mono line under the title (paths, counts, freshness). */
  meta?: React.ReactNode;
  /** Small status element rendered beside the title. */
  status?: React.ReactNode;
  className?: string;
}

/**
 * The one view title block. 24/500 title, 13 muted description, actions right,
 * mono meta line below — the widest type step on a view is always this title.
 */
export const PageHeader: React.FC<PageHeaderProps> = ({
  title,
  description,
  icon,
  actions,
  meta,
  status,
  className,
}) => {
  return (
    <div
      className={cn(
        "flex flex-col gap-3 sm:flex-row sm:items-start sm:justify-between",
        className
      )}
    >
      <div className="min-w-0">
        <div className="flex flex-wrap items-center gap-x-2 gap-y-1">
          {icon && (
            <span className="text-muted-foreground [&_svg]:size-4 shrink-0">
              {icon}
            </span>
          )}
          <h1 className="text-2xl font-medium tracking-tight text-foreground text-wrap-balance">
            {title}
          </h1>
          {status}
        </div>
        {description && (
          <p className="mt-1.5 max-w-2xl text-xs leading-relaxed text-muted-foreground">
            {description}
          </p>
        )}
        {meta && (
          <div className="mt-1.5 flex flex-wrap items-center gap-x-3 gap-y-1 font-mono text-3xs text-muted-foreground">
            {meta}
          </div>
        )}
      </div>
      {actions && (
        <div className="flex flex-wrap items-center gap-2 shrink-0">
          {actions}
        </div>
      )}
    </div>
  );
};
