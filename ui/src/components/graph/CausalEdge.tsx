import { memo } from "react";
import {
  BaseEdge,
  EdgeLabelRenderer,
  getSmoothStepPath,
  type EdgeProps,
} from "@xyflow/react";

/** Token-aligned edge colors (Graphite Vault). */
const RELATION_STYLE: Record<
  string,
  { stroke: string; dashed?: boolean; label: string }
> = {
  follows: { stroke: "var(--primary)", label: "follows" },
  caused: { stroke: "var(--primary)", label: "follows" },
  supersedes: { stroke: "var(--warning)", dashed: true, label: "supersedes" },
  superseded_by: { stroke: "var(--warning)", dashed: true, label: "supersedes" },
  targets: { stroke: "var(--plan)", label: "targets" },
  project: { stroke: "var(--info, #38bdf8)", label: "project" },
  contains: { stroke: "var(--success, #10b981)", label: "contains" },
  origin: { stroke: "var(--purple, #8b5cf6)", label: "origin" },
};

export const CausalEdge = memo(function CausalEdge({
  id,
  sourceX,
  sourceY,
  targetX,
  targetY,
  sourcePosition,
  targetPosition,
  data,
  markerEnd,
  selected,
}: EdgeProps) {
  const [edgePath, labelX, labelY] = getSmoothStepPath({
    sourceX,
    sourceY,
    sourcePosition,
    targetX,
    targetY,
    targetPosition,
  });

  const relation = ((data?.relation as string) || "follows").toLowerCase();
  const style = RELATION_STYLE[relation] || RELATION_STYLE.follows;

  return (
    <>
      <BaseEdge
        id={id}
        path={edgePath}
        markerEnd={markerEnd}
        style={{
          stroke: style.stroke,
          strokeWidth: selected ? 2.25 : 1.5,
          strokeDasharray: style.dashed ? "5 4" : undefined,
          opacity: selected ? 1 : 0.7,
        }}
      />
      <EdgeLabelRenderer>
        <div
          style={{
            position: "absolute",
            transform: `translate(-50%, -50%) translate(${labelX}px,${labelY}px)`,
            pointerEvents: "none",
          }}
          className="px-1 py-0.5 rounded-sm text-2xs font-mono uppercase tracking-wider bg-card/95 border border-border text-muted-foreground select-none nodrag nopan"
        >
          {style.label}
        </div>
      </EdgeLabelRenderer>
    </>
  );
});
