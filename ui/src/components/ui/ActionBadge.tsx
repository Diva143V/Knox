import React from "react";
import { cn } from "@/lib/cn";
import { actionTypeClasses } from "@/lib/semantics";

interface ActionBadgeProps {
  type: string;
  className?: string;
}

export const ActionBadge: React.FC<ActionBadgeProps> = ({
  type,
  className = "",
}) => {
  return (
    <span
      className={cn(
        "inline-flex items-center px-1.5 py-0.5 rounded-sm text-xs font-medium tracking-wide uppercase border font-mono",
        actionTypeClasses(type),
        className
      )}
    >
      {type}
    </span>
  );
};
