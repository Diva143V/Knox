import React from "react";
import type { KnoxAction } from "@/types/knox";
import { ActionBadge } from "./ActionBadge";
import { ActorBadge } from "./ActorBadge";
import { FileCode } from "lucide-react";

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
  let formattedTime = "--:--";
  if (action?.timestamp) {
    try {
      const d = new Date(action.timestamp);
      if (!isNaN(d.getTime())) {
        formattedTime = d.toLocaleTimeString([], {
          hour: "2-digit",
          minute: "2-digit",
        });
      }
    } catch {
      formattedTime = "--:--";
    }
  }

  return (
    <div
      onClick={() => onSelect?.(action)}
      className={`group flex items-center justify-between px-3 py-2.5 rounded-lg border text-xs transition-all cursor-pointer ${
        isSelected
          ? "bg-primary/10 border-primary/50 text-foreground shadow-sm"
          : "bg-card/40 border-border/50 hover:bg-card hover:border-border text-foreground"
      } ${className}`}
    >
      {/* Left Column: Badges & Summary */}
      <div className="flex items-center gap-3 min-w-0 flex-1 mr-3">
        <div className="flex items-center gap-1.5 shrink-0">
          <ActorBadge actor={action.actor} />
          <ActionBadge type={action.action_type} />
        </div>

        <p className="font-medium truncate text-foreground group-hover:text-primary transition-colors">
          {action.summary}
        </p>

        {action.files && action.files.length > 0 && (
          <span className="hidden md:flex items-center gap-1 text-[11px] font-mono text-muted-foreground shrink-0">
            <FileCode className="w-3 h-3" />
            <span className="truncate max-w-[120px]">{action.files[0]}</span>
            {action.files.length > 1 && (
              <span className="text-[10px] bg-muted/60 px-1 py-0.2 rounded">
                +{action.files.length - 1}
              </span>
            )}
          </span>
        )}
      </div>

      {/* Right Column: State & Time */}
      <div className="flex items-center gap-2.5 shrink-0 font-mono text-[11px] text-muted-foreground">
        {!action.is_current && (
          <span className="text-[9px] px-1 py-0.2 rounded bg-amber-500/10 text-amber-400 border border-amber-500/20">
            STALE
          </span>
        )}
        <span>{formattedTime}</span>
      </div>
    </div>
  );
};

