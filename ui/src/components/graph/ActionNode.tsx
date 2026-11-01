import { memo } from "react";
import { Handle, Position } from "@xyflow/react";
import { ActionBadge } from "@/components/ui/ActionBadge";
import { ActorBadge } from "@/components/ui/ActorBadge";
import type { KnoxAction } from "@/types/knox";
import { FileCode, Zap, Compass, Scale, Bug } from "lucide-react";
import { useKnoxStore } from "@/store/useKnoxStore";
import { cn } from "@/lib/cn";

export interface ActionNodeData {
  action: KnoxAction;
  isHighlighted?: boolean;
  isDimmed?: boolean;
  [key: string]: unknown;
}

function typeAccent(type: string) {
  const norm = (type || "").toLowerCase();
  if (norm.includes("write")) {
    return {
      border: "border-success/40",
      hover: "hover:border-success/70",
      ring: "ring-success",
      Icon: Zap,
    };
  }
  if (norm.includes("plan")) {
    return {
      border: "border-plan/40",
      hover: "hover:border-plan/70",
      ring: "ring-plan",
      Icon: Compass,
    };
  }
  if (norm.includes("decide")) {
    return {
      border: "border-warning/40",
      hover: "hover:border-warning/70",
      ring: "ring-warning",
      Icon: Scale,
    };
  }
  if (norm.includes("debug")) {
    return {
      border: "border-danger/40",
      hover: "hover:border-danger/70",
      ring: "ring-danger",
      Icon: Bug,
    };
  }
  return {
    border: "border-border",
    hover: "hover:border-primary/50",
    ring: "ring-primary",
    Icon: Zap,
  };
}

export const ActionNode = memo(({ data }: { data: ActionNodeData }) => {
  const setSelectedAction = useKnoxStore((s) => s.setSelectedAction);
  const selectedId = useKnoxStore((s) => s.selectedAction?.id);
  const { action, isHighlighted } = data;
  const selected = selectedId === action.id;
  const accent = typeAccent(action.action_type);
  const TypeIcon = accent.Icon;

  const dateStr = new Date(action.timestamp).toLocaleDateString(undefined, {
    month: "numeric",
    day: "numeric",
  });

  const sessionShort = action.session_id
    ? action.session_id.slice(0, 8)
    : null;

  const fileName = action.files?.[0]
    ? action.files[0].split(/[/\\]/).pop() || action.files[0]
    : null;

  return (
    <div
      onClick={(e) => {
        e.stopPropagation();
        setSelectedAction(action);
      }}
      onKeyDown={(e) => {
        if (e.key === "Enter" || e.key === " ") {
          e.preventDefault();
          e.stopPropagation();
          setSelectedAction(action);
        }
      }}
      role="button"
      tabIndex={0}
      aria-pressed={selected}
      aria-label={`${action.action_type} by ${action.actor}: ${action.summary}`}
      className={cn(
        "w-[240px] rounded-md border bg-card p-2.5 space-y-2 cursor-pointer select-none",
        "transition-colors duration-100",
        "focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-focus-ring",
        accent.border,
        accent.hover,
        selected && `ring-2 ${accent.ring} border-transparent`,
        isHighlighted && !selected && "ring-2 ring-primary border-primary/60 scale-[1.02]",
        data.isDimmed && !selected && !isHighlighted && "opacity-25 transition-opacity duration-300",
        !action.is_current && "opacity-70"
      )}
    >
      <Handle
        type="target"
        position={Position.Top}
        className="!size-2.5 !bg-primary !border-2 !border-background"
      />

      <div className="flex items-center justify-between gap-1.5">
        <div className="flex items-center gap-1 min-w-0">
          <TypeIcon className="size-3.5 text-muted-foreground shrink-0" aria-hidden />
          <ActionBadge
            type={action.action_type}
            className="text-xs px-1.5 py-0"
          />
        </div>
        <ActorBadge actor={action.actor} className="text-xs px-1.5 py-0" />
      </div>

      <p className="text-sm text-foreground font-medium line-clamp-2 leading-snug">
        {action.summary}
      </p>

      {fileName && (
        <div className="flex items-center justify-between text-xs font-mono text-muted-foreground pt-1 border-t border-border-subtle gap-1">
          <span className="flex items-center gap-1 truncate min-w-0">
            <FileCode className="size-3 shrink-0" aria-hidden />
            <span className="truncate" title={action.files[0]}>
              {fileName}
            </span>
          </span>
          {action.files.length > 1 && (
            <span className="bg-background px-1 py-0.5 rounded-sm border border-border text-2xs shrink-0">
              +{action.files.length - 1}
            </span>
          )}
        </div>
      )}

      {((action.symbols && action.symbols.length > 0) ||
        (action.relations && action.relations.length > 0)) && (
        <div className="flex items-center gap-1.5 pt-0.5 text-2xs font-mono text-muted-foreground">
          {action.symbols && action.symbols.length > 0 && (
            <span className="bg-primary/10 text-primary border border-primary/20 px-1 py-0.5 rounded-xs truncate max-w-[120px]">
              {action.symbols[0]}
              {action.symbols.length > 1 ? ` +${action.symbols.length - 1}` : ""}
            </span>
          )}
          {action.relations && action.relations.length > 0 && (
            <span className="bg-muted text-muted-foreground border border-border px-1 py-0.5 rounded-xs shrink-0">
              {action.relations.length} rel
            </span>
          )}
        </div>
      )}

      <div className="flex items-center justify-between text-xs font-mono text-muted-foreground/80">
        <span>{dateStr}</span>
        <div className="flex items-center gap-1">
          {!action.is_current && (
            <span className="text-2xs text-warning">stale</span>
          )}
          {sessionShort && (
            <span className="truncate max-w-[72px] text-2xs bg-muted px-1 rounded-sm">
              s:{sessionShort}
            </span>
          )}
        </div>
      </div>

      <Handle
        type="source"
        position={Position.Bottom}
        className="!size-2.5 !bg-primary !border-2 !border-background"
      />
    </div>
  );
});

ActionNode.displayName = "ActionNode";
