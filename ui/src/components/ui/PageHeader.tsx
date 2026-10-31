import React from "react";
import { cn } from "@/lib/cn";

export interface PageHeaderProps {
  title: string;
  description?: string;
  icon?: React.ReactNode;
  actions?: React.ReactNode;
  className?: string;
}

/**
 * Standard view title block. Use instead of ad-hoc h1 stacks.
 * Layout of surrounding screens is unchanged until Phase 5+.
 */
export const PageHeader: React.FC<PageHeaderProps> = ({
  title,
  description,
  icon,
  actions,
  className,
}) => {
  return (
    <div
      className={cn(
        "flex flex-col sm:flex-row sm:items-center justify-between gap-3",
        className
      )}
    >
      <div className="min-w-0">
        <h1 className="text-display text-foreground flex items-center gap-2">
          {icon && (
            <span className="text-primary [&_svg]:size-5 shrink-0">{icon}</span>
          )}
          <span className="truncate">{title}</span>
        </h1>
        {description && (
          <p className="text-sm text-muted-foreground mt-1.5 max-w-2xl leading-relaxed">
            {description}
          </p>
        )}
      </div>
      {actions && (
        <div className="flex flex-wrap items-center gap-2 shrink-0">{actions}</div>
      )}
    </div>
  );
};
