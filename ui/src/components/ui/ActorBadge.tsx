import React from "react";
import { cn } from "@/lib/cn";

interface ActorBadgeProps {
  actor: string;
  className?: string;
}

/** Actor hue lives in a 6px dot; the name itself stays muted mono text. */
const DOT_BY_ACTOR: Record<string, string> = {
  human: "bg-swatch-d",
  gemini: "bg-swatch-b",
  cursor: "bg-swatch-c",
  antigravity: "bg-swatch-a",
  copilot: "bg-swatch-e",
};

export const ActorBadge: React.FC<ActorBadgeProps> = ({
  actor,
  className = "",
}) => {
  const key = (actor || "").toLowerCase();
  const dot = DOT_BY_ACTOR[key] ?? "bg-swatch-f";

  return (
    <span
      className={cn(
        "inline-flex min-w-0 items-center gap-1.5 text-[11px] text-muted-foreground",
        className
      )}
      title={actor}
    >
      <span
        className={cn("size-1.5 shrink-0 rounded-full", dot)}
        aria-hidden
      />
      <span className="truncate font-mono">{actor}</span>
    </span>
  );
};
