import { memo } from "react";
import {
  BaseEdge,
  EdgeLabelRenderer,
  getSmoothStepPath,
  type EdgeProps,
} from "@xyflow/react";

/** Token-aligned edge colors (Achromatic Ledger / Data Terminal). */
const RELATION_STYLE: Record<
  string,
  { stroke: string; dashed?: boolean; label: string }
> = {
  follows: { stroke: "var(--primary)", label: "follows" },
  caused: { stroke: "var(--primary)", label: "follows" },
  supersedes: { stroke: "var(--warning)", dashed: true, label: "supersedes" },
  superseded_by: { stroke: "var(--warning)", dashed: true, label: "supersedes" },
  targets: { stroke: "var(--muted-foreground)", label: "targets" },
  project: { stroke: "var(--primary)", label: "project" },
  contains: { stroke: "var(--success)", label: "contains" },
  origin: { stroke: "var(--primary)", label: "origin" },
  depends_on: { stroke: "var(--warning)", dashed: true, label: "depends on" },
  implements: { stroke: "var(--success)", label: "implements" },
  verifies: { stroke: "var(--primary)", dashed: true, label: "verifies" },
  relates_to: { stroke: "var(--muted-foreground)", dashed: true, label: "relates to" },
  reverts: { stroke: "var(--danger)", label: "reverts" },
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
          strokeWidth: selected ? 2 : 1.25,
          strokeDasharray: style.dashed ? "4 3" : undefined,
          opacity: selected ? 1 : 0.75,
        }}
      />
      <EdgeLabelRenderer>
        <div
          style={{
            position: "absolute",
            transform: `translate(-50%, -50%) translate(${labelX}px,${labelY}px)`,
            pointerEvents: "none",
          }}
          className="px-1 py-0.5 rounded-[var(--radius-control)] text-3xs font-mono bg-card border border-border text-muted-foreground select-none nodrag nopan"
        >
          {style.label}
        </div>
      </EdgeLabelRenderer>
    </>
  );
});
