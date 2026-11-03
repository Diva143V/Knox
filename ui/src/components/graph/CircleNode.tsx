import React, { memo } from "react";
import { Handle, Position } from "@xyflow/react";
import {
  FolderGit2,
  FileCode,
  Scale,
  Compass,
  Zap,
  Bug,
  Layers,
} from "lucide-react";
import { useKnoxStore } from "@/store/useKnoxStore";
import type { KnoxAction } from "@/types/knox";
import { cn } from "@/lib/cn";

export interface CircleNodeData {
  kind: "project" | "file" | "action";
  title: string;
  subtitle?: string;
  action?: KnoxAction;
  project?: import("@/types/knox").Project;
  isDimmed?: boolean;
  isHighlighted?: boolean;
  [key: string]: unknown;
}

interface RoleVisual {
  color: string;
  border: string;
  bg: string;
  ring: string;
  text: string;
  Icon: React.ComponentType<{ className?: string }>;
  radius: number;
}

function getVisual(kind: CircleNodeData["kind"], action?: KnoxAction): RoleVisual {
  if (kind === "project") {
    return {
      color: "var(--primary)",
      border: "border-primary/60",
      bg: "bg-primary/15",
      ring: "ring-primary",
      text: "text-primary",
      Icon: FolderGit2,
      radius: 26,
    };
  }

  if (kind === "file") {
    return {
      color: "var(--muted-foreground)",
      border: "border-border",
      bg: "bg-muted/50",
      ring: "ring-muted-foreground",
      text: "text-muted-foreground",
      Icon: FileCode,
      radius: 18,
    };
  }

  const type = (action?.action_type || "").toLowerCase();
  if (type.includes("decide")) {
    return {
      color: "var(--warning)",
      border: "border-warning/70",
      bg: "bg-warning/15",
      ring: "ring-warning",
      text: "text-warning",
      Icon: Scale,
      radius: 23,
    };
  }
  if (type.includes("plan")) {
    return {
      color: "var(--primary)",
      border: "border-primary/60",
      bg: "bg-primary/15",
      ring: "ring-primary",
      text: "text-primary",
      Icon: Compass,
      radius: 21,
    };
  }
  if (type.includes("write")) {
    return {
      color: "var(--success)",
      border: "border-success/60",
      bg: "bg-success/15",
      ring: "ring-success",
      text: "text-success",
      Icon: Zap,
      radius: 21,
    };
  }
  if (type.includes("debug")) {
    return {
      color: "var(--danger)",
      border: "border-danger/60",
      bg: "bg-danger/15",
      ring: "ring-danger",
      text: "text-danger",
      Icon: Bug,
      radius: 21,
    };
  }

  return {
    color: "var(--primary)",
    border: "border-primary/50",
    bg: "bg-primary/10",
    ring: "ring-primary",
    text: "text-primary",
    Icon: Layers,
    radius: 20,
  };
}

export const CircleNode = memo(({ data }: { data: CircleNodeData }) => {
  const setSelectedAction = useKnoxStore((s) => s.setSelectedAction);
  const selectedId = useKnoxStore((s) => s.selectedAction?.id);

  const { kind, title, subtitle, action, isHighlighted } = data;
  const isSelected = kind === "action" && selectedId === action?.id;

  const visual = getVisual(kind, action);
  const IconComponent = visual.Icon;
  const diameter = visual.radius * 2;

  let delaySeed = 0;
  for (let i = 0; i < title.length; i++) delaySeed += title.charCodeAt(i);

  const handleClick = (e: React.MouseEvent) => {
    e.stopPropagation();
    if (kind === "action" && action) {
      setSelectedAction(action);
    }
  };

  return (
    <div
      data-ui="graph-node"
      onClick={handleClick}
      className={cn(
        "relative flex flex-col items-center justify-center group cursor-pointer select-none",
        "focus-visible:outline-none",
        data.isDimmed && !isSelected && !isHighlighted && "opacity-25 transition-opacity duration-200"
      )}
      style={{ width: diameter + 40, minHeight: diameter + 32 }}
      role="button"
      tabIndex={0}
      aria-label={`${kind}: ${title}`}
    >
      {/* Invisible Handles at Cardinal directions for flexible 2D routing */}
      <Handle
        type="target"
        position={Position.Top}
        className="!size-2 !bg-primary/80 !border-0 !opacity-0 group-hover:!opacity-100 transition-opacity"
      />
      <Handle
        type="source"
        position={Position.Bottom}
        className="!size-2 !bg-primary/80 !border-0 !opacity-0 group-hover:!opacity-100 transition-opacity"
      />
      <Handle
        type="target"
        position={Position.Left}
        id="target-left"
        className="!size-2 !bg-primary/80 !border-0 !opacity-0 group-hover:!opacity-100 transition-opacity"
      />
      <Handle
        type="source"
        position={Position.Right}
        id="source-right"
        className="!size-2 !bg-primary/80 !border-0 !opacity-0 group-hover:!opacity-100 transition-opacity"
      />

      {/* Outer Selection Ring */}
      <div
        className={cn(
          "obsidian-orbit rounded-full flex items-center justify-center relative transition-transform duration-150",
          visual.border,
          visual.bg,
          "border",
          isSelected && `ring-2 ${visual.ring} scale-105`,
          isHighlighted && !isSelected && "ring-1 ring-primary scale-105"
        )}
        style={{
          width: diameter,
          height: diameter,
          "--globe-delay": `-${delaySeed % 8}s`,
        } as React.CSSProperties}
      >
        {/* Inner Lucide Glyph */}
        <IconComponent className={cn("size-4 transition-transform duration-150 group-hover:scale-110", visual.text)} />

        {/* Small badge for relations or files count */}
        {action && action.relations && action.relations.length > 0 && (
          <span
            className="absolute -top-1 -right-1 size-3.5 rounded-full bg-primary text-primary-foreground text-[8px] font-mono font-medium flex items-center justify-center border border-background"
            title={`${action.relations.length} relations`}
          >
            {action.relations.length}
          </span>
        )}

        {kind === "project" && (
          <span className="absolute -bottom-1 -right-1 px-1 rounded-xs bg-muted text-foreground text-[8px] font-mono border border-border">
            PKG
          </span>
        )}
      </div>

      {/* Floating Ambient Label */}
      <div className="mt-1 flex flex-col items-center text-center max-w-[130px] pointer-events-none">
        <span
          className={cn(
            "text-3xs font-normal leading-tight line-clamp-2 px-1 rounded-xs transition-colors",
            isSelected ? "text-primary font-medium bg-primary-muted" : "text-foreground/90 group-hover:text-foreground"
          )}
          title={title}
        >
          {title}
        </span>
        {subtitle && (
          <span className="text-[9px] font-mono text-muted-foreground truncate max-w-[110px] mt-0.5">
            {subtitle}
          </span>
        )}
      </div>
    </div>
  );
});

CircleNode.displayName = "CircleNode";
