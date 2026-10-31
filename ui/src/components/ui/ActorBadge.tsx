import React from "react";
import { cn } from "@/lib/cn";
import { actorClasses } from "@/lib/semantics";

interface ActorBadgeProps {
  actor: string;
  className?: string;
}

export const ActorBadge: React.FC<ActorBadgeProps> = ({
  actor,
  className = "",
}) => {
  return (
    <span
      className={cn(
        "inline-flex items-center gap-1 px-1.5 py-0.5 rounded-sm text-xs font-medium border font-mono",
        actorClasses(actor),
        className
      )}
    >
      <span className="w-1.5 h-1.5 rounded-full bg-current opacity-70" aria-hidden />
      {actor}
    </span>
  );
};
