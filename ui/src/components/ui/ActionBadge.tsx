import React from "react";
import { cn } from "@/lib/cn";

interface ActionBadgeProps {
  type: string;
  className?: string;
}

/**
 * Action type as a mono micro label rather than a filled chip. Only an open
 * doubt carries colour, because that is the one type with unfinished state.
 */
export const ActionBadge: React.FC<ActionBadgeProps> = ({
  type,
  className = "",
}) => {
  const norm = (type || "").toLowerCase();
  const isOpenQuestion =
    norm.includes("doubt") || norm.includes("question");

  return (
    <span
      className={cn(
        "inline-flex items-center font-mono text-[11px] uppercase tracking-[0.06em]",
        isOpenQuestion ? "text-warning" : "text-muted-foreground",
        className
      )}
    >
      {type}
    </span>
  );
};
