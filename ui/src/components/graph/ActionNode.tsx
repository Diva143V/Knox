import { memo } from "react";
import { Handle, Position } from "@xyflow/react";
import { ActionBadge } from "@/components/ui/ActionBadge";
import { ActorBadge } from "@/components/ui/ActorBadge";
import type { KnoxAction } from "@/types/knox";
import { FileCode, Zap, Compass, Scale, Bug } from "lucide-react";
import { useKnoxStore } from "@/store/useKnoxStore";

export interface ActionNodeData {
  action: KnoxAction;
  selected?: boolean;
  isHighlighted?: boolean;
}

export const ActionNode = memo(({ data }: { data: ActionNodeData }) => {
  const setSelectedAction = useKnoxStore((s) => s.setSelectedAction);
  const { action, selected, isHighlighted } = data;
  const norm = (action.action_type || "").toLowerCase();

  let borderColor = "border-border";
  let glowHover = "hover:border-primary/50 hover:shadow-primary/10";
  let ringColor = "ring-primary";
  let TypeIcon = Zap;

  if (norm.includes("write")) {
    borderColor = "border-emerald-500/40";
    glowHover = "hover:border-emerald-400 hover:shadow-emerald-500/10";
    ringColor = "ring-emerald-500";
    TypeIcon = Zap;
  } else if (norm.includes("plan")) {
    borderColor = "border-purple-500/40";
    glowHover = "hover:border-purple-400 hover:shadow-purple-500/10";
    ringColor = "ring-purple-500";
    TypeIcon = Compass;
  } else if (norm.includes("decide")) {
    borderColor = "border-amber-500/40";
    glowHover = "hover:border-amber-400 hover:shadow-amber-500/10";
    ringColor = "ring-amber-500";
    TypeIcon = Scale;
  } else if (norm.includes("debug")) {
    borderColor = "border-rose-500/40";
    glowHover = "hover:border-rose-400 hover:shadow-rose-500/10";
    ringColor = "ring-rose-500";
    TypeIcon = Bug;
  }

  const dateStr = new Date(action.timestamp).toLocaleDateString(undefined, {
    month: "numeric",
    day: "numeric",
  });

  const sessionShort = action.session_id
    ? action.session_id.slice(0, 8)
    : null;

  return (
    <div
      onClick={(e) => {
        e.stopPropagation();
        setSelectedAction(action);
      }}
      className={`w-[220px] rounded-lg border bg-card/95 backdrop-blur p-2.5 space-y-2 transition-all cursor-pointer shadow-md select-none ${borderColor} ${glowHover} ${
        selected
          ? `ring-2 ${ringColor} border-transparent shadow-lg`
          : isHighlighted
          ? "ring-2 ring-indigo-400 border-indigo-400 shadow-indigo-500/20"
          : ""
      }`}
    >
      <Handle
        type="target"
        position={Position.Top}
        className="w-2.5 h-2.5 bg-primary border-2 border-background"
      />

      {/* Top Header: Type Icon + Action Type & Actor Badge */}
      <div className="flex items-center justify-between gap-1.5">
        <div className="flex items-center gap-1 min-w-0">
          <TypeIcon className="w-3.5 h-3.5 text-muted-foreground shrink-0" />
          <ActionBadge type={action.action_type} className="text-[10px] px-1.5 py-0" />
        </div>
        <ActorBadge actor={action.actor} className="text-[10px] px-1.5 py-0" />
      </div>

      {/* Summary with search highlight badge */}
      <div>
        <p className="text-xs text-foreground font-medium line-clamp-2 leading-snug">
          {action.summary}
        </p>
      </div>

      {/* Files Chip */}
      {action.files && action.files.length > 0 && (
        <div className="flex items-center justify-between text-[10px] font-mono text-muted-foreground pt-1 border-t border-border/40">
          <span className="flex items-center gap-1 truncate max-w-[150px]">
            <FileCode className="w-3 h-3 text-muted-foreground shrink-0" />
            <span className="truncate">{action.files[0]}</span>
          </span>
          {action.files.length > 1 && (
            <span className="bg-background px-1 py-0.2 rounded border border-border text-[9px]">
              +{action.files.length - 1}
            </span>
          )}
        </div>
      )}

      {/* Muted Timestamp & Session ID Footer */}
      <div className="flex items-center justify-between text-[10px] font-mono text-muted-foreground/80 pt-0.5">
        <span>{dateStr}</span>
        {sessionShort && (
          <span className="truncate max-w-[90px] text-[9px] bg-muted/40 px-1 rounded">
            s:{sessionShort}
          </span>
        )}
      </div>

      <Handle
        type="source"
        position={Position.Bottom}
        className="w-2.5 h-2.5 bg-primary border-2 border-background"
      />
    </div>
  );
});

ActionNode.displayName = "ActionNode";
