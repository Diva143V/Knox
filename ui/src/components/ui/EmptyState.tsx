import React from "react";
import { cn } from "@/lib/cn";

export interface EmptyStateProps {
  icon?: React.ReactNode;
  title: string;
  description?: React.ReactNode;
  action?: React.ReactNode;
  className?: string;
  compact?: boolean;
}

/** Guided empty placeholder with optional next-step CTA. */
export const EmptyState: React.FC<EmptyStateProps> = ({
  icon,
  title,
  description,
  action,
  className,
  compact = false,
}) => {
  return (
    <div
      className={cn(
        "flex flex-col items-center justify-center text-center border border-dashed border-border rounded-lg bg-card/40",
        compact ? "py-8 px-4 gap-2" : "py-16 px-6 gap-3",
        className
      )}
      role="status"
    >
      {icon && (
        <div className="text-muted-foreground/50 [&_svg]:size-8 mb-1">{icon}</div>
      )}
      <p className="text-base font-semibold text-foreground">{title}</p>
      {description && (
        <p className="text-sm text-muted-foreground max-w-sm leading-relaxed">
          {description}
        </p>
      )}
      {action && <div className="pt-1">{action}</div>}
    </div>
  );
};
