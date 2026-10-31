import React from "react";
import type { KnoxAction } from "@/types/knox";
import { ActionBadge } from "./ActionBadge";
import { ActorBadge } from "./ActorBadge";
import { FileCode } from "lucide-react";
import { cn } from "@/lib/cn";

interface ActionRowProps {
  action: KnoxAction;
  isSelected?: boolean;
  onSelect?: (action: KnoxAction) => void;
  className?: string;
}

export const ActionRow: React.FC<ActionRowProps> = ({
  action,
  isSelected = false,
  onSelect,
  className = "",
}) => {
  const formattedTime = new Date(action.timestamp).toLocaleTimeString([], {
    hour: "2-digit",
    minute: "2-digit",
  });

  const interactive = Boolean(onSelect);

  return (
    <div
      onClick={() => onSelect?.(action)}
      onKeyDown={
        interactive
          ? (e) => {
              if (e.key === "Enter" || e.key === " ") {
                e.preventDefault();
                onSelect?.(action);
              }
            }
          : undefined
      }
      role={interactive ? "button" : undefined}
      tabIndex={interactive ? 0 : undefined}
      className={cn(
        "group flex items-center justify-between px-3 py-2 rounded-md border text-xs transition-colors duration-100",
        interactive && "cursor-pointer",
        isSelected
          ? "bg-primary-muted border-primary/40 text-foreground"
          : "bg-card/40 border-border-subtle hover:bg-card hover:border-border text-foreground",
        interactive &&
          "focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-focus-ring",
        className
      )}
    >
      <div className="flex items-center gap-3 min-w-0 flex-1 mr-3">
        <div className="flex items-center gap-1.5 shrink-0">
          <ActorBadge actor={action.actor} />
          <ActionBadge type={action.action_type} />
        </div>

        <p className="font-medium truncate text-foreground">{action.summary}</p>

        {action.files && action.files.length > 0 && (
          <span className="hidden md:flex items-center gap-1 text-sm font-mono text-muted-foreground shrink-0">
            <FileCode className="w-3 h-3" />
            <span className="truncate max-w-[120px]">{action.files[0]}</span>
            {action.files.length > 1 && (
              <span className="text-xs bg-muted px-1 py-0.5 rounded-sm">
                +{action.files.length - 1}
              </span>
            )}
          </span>
        )}
      </div>

      <div className="flex items-center gap-2.5 shrink-0 font-mono text-sm text-muted-foreground">
        {!action.is_current && (
          <span className="text-2xs px-1 py-0.5 rounded-sm bg-warning/10 text-warning border border-warning/20">
            STALE
          </span>
        )}
        <span>{formattedTime}</span>
      </div>
    </div>
  );
};
