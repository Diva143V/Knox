import { memo } from "react";
import { Handle, Position } from "@xyflow/react";
import { useKnoxStore } from "@/store/useKnoxStore";
import type { KnoxAction } from "@/types/knox";
import { cn } from "@/lib/cn";
import {
  FolderGit2,
  FileCode,
  Zap,
  Compass,
  Scale,
  Bug,
  Layers,
} from "lucide-react";

export interface CircleNodeData {
  kind: "action" | "project" | "file";
  title: string;
  subtitle?: string;
  action?: KnoxAction;
  filePath?: string;
  actionCount?: number;
  fileCount?: number;
  isHighlighted?: boolean;
  isDimmed?: boolean;
  selected?: boolean;
  [key: string]: unknown;
}

interface RoleVisual {
  color: string;
  border: string;
  bg: string;
  glow: string;
  ring: string;
  text: string;
  Icon: React.ComponentType<{ className?: string }>;
  radius: number;
}

function getVisual(kind: CircleNodeData["kind"], action?: KnoxAction): RoleVisual {
  if (kind === "project") {
    return {
      color: "#38bdf8",
      border: "border-sky-400/70",
      bg: "bg-sky-500/20",
      glow: "shadow-[0_0_22px_rgba(56,189,248,0.4)]",
      ring: "ring-sky-400",
      text: "text-sky-300",
      Icon: FolderGit2,
      radius: 26, // 52px diameter
    };
  }

  if (kind === "file") {
    return {
      color: "#94a3b8",
      border: "border-slate-400/60",
      bg: "bg-slate-500/20",
      glow: "shadow-[0_0_12px_rgba(148,163,184,0.25)]",
      ring: "ring-slate-400",
      text: "text-slate-300",
      Icon: FileCode,
      radius: 18, // 36px diameter
    };
  }

  const type = (action?.action_type || "").toLowerCase();
  if (type.includes("decide")) {
    return {
      color: "#f59e0b",
      border: "border-amber-400/80",
      bg: "bg-amber-500/20",
      glow: "shadow-[0_0_20px_rgba(245,158,11,0.45)]",
      ring: "ring-amber-400",
      text: "text-amber-300",
      Icon: Scale,
      radius: 23, // 46px diameter
    };
  }
  if (type.includes("plan")) {
    return {
      color: "#06b6d4",
      border: "border-cyan-400/70",
      bg: "bg-cyan-500/20",
      glow: "shadow-[0_0_18px_rgba(6,182,212,0.35)]",
      ring: "ring-cyan-400",
      text: "text-cyan-300",
      Icon: Compass,
      radius: 21,
    };
  }
  if (type.includes("write")) {
    return {
      color: "#10b981",
      border: "border-emerald-400/70",
      bg: "bg-emerald-500/20",
      glow: "shadow-[0_0_18px_rgba(16,185,129,0.35)]",
      ring: "ring-emerald-400",
      text: "text-emerald-300",
      Icon: Zap,
      radius: 21,
    };
  }
  if (type.includes("debug")) {
    return {
      color: "#ef4444",
      border: "border-rose-400/70",
      bg: "bg-rose-500/20",
      glow: "shadow-[0_0_18px_rgba(239,68,68,0.35)]",
      ring: "ring-rose-400",
      text: "text-rose-300",
      Icon: Bug,
      radius: 21,
    };
  }

  return {
    color: "#6366f1",
    border: "border-indigo-400/60",
    bg: "bg-indigo-500/20",
    glow: "shadow-[0_0_16px_rgba(99,102,241,0.35)]",
    ring: "ring-indigo-400",
    text: "text-indigo-300",
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

  const handleClick = (e: React.MouseEvent) => {
    e.stopPropagation();
    if (kind === "action" && action) {
      setSelectedAction(action);
    }
  };

  return (
    <div
      onClick={handleClick}
      className={cn(
        "relative flex flex-col items-center justify-center group cursor-pointer select-none",
        "focus-visible:outline-none",
        data.isDimmed && !isSelected && !isHighlighted && "opacity-25 transition-opacity duration-300"
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

      {/* Outer Pulse/Selection Ring */}
      <div
        className={cn(
          "rounded-full flex items-center justify-center transition-all duration-300 relative",
          visual.border,
          visual.bg,
          visual.glow,
          "border-2 backdrop-blur-xs",
          isSelected && `ring-4 ${visual.ring} scale-110`,
          isHighlighted && !isSelected && "ring-3 ring-primary animate-pulse scale-105"
        )}
        style={{
          width: diameter,
          height: diameter,
        }}
      >
        {/* Inner Lucide Glyph */}
        <IconComponent className={cn("size-5 transition-transform duration-200 group-hover:scale-115", visual.text)} />

        {/* Small badge for relations or files count */}
        {action && action.relations && action.relations.length > 0 && (
          <span
            className="absolute -top-1 -right-1 size-4 rounded-full bg-primary text-primary-foreground text-[9px] font-mono font-bold flex items-center justify-center border border-background shadow-xs"
            title={`${action.relations.length} relations`}
          >
            {action.relations.length}
          </span>
        )}

        {kind === "project" && (
          <span
            className="absolute -bottom-1 -right-1 px-1 rounded-sm bg-sky-600 text-white text-[9px] font-mono font-bold border border-background shadow-xs"
          >
            PKG
          </span>
        )}
      </div>

      {/* Floating Ambient Label */}
      <div className="mt-1.5 flex flex-col items-center text-center max-w-[140px] pointer-events-none">
        <span
          className={cn(
            "text-[11px] font-medium leading-tight line-clamp-2 px-1 rounded-sm transition-colors",
            isSelected ? "text-primary font-semibold bg-primary/10" : "text-foreground/90 group-hover:text-foreground"
          )}
          title={title}
        >
          {title}
        </span>
        {subtitle && (
          <span className="text-[9px] font-mono text-muted-foreground truncate max-w-[120px] mt-0.5">
            {subtitle}
          </span>
        )}
      </div>
    </div>
  );
});

CircleNode.displayName = "CircleNode";
