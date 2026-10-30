import React from "react";
import {
  BaseEdge,
  EdgeLabelRenderer,
  getSmoothStepPath,
  type EdgeProps,
} from "@xyflow/react";

export const CausalEdge: React.FC<EdgeProps> = ({
  id,
  sourceX,
  sourceY,
  targetX,
  targetY,
  sourcePosition,
  targetPosition,
  data,
  markerEnd,
}) => {
  const [edgePath, labelX, labelY] = getSmoothStepPath({
    sourceX,
    sourceY,
    sourcePosition,
    targetX,
    targetY,
    targetPosition,
  });

  const relation = (data?.relation as string) || "caused";
  let stroke = "#6366f1";
  let isDashed = false;

  if (relation === "supersedes" || relation === "superseded_by") {
    stroke = "#f59e0b";
    isDashed = true;
  } else if (relation === "targets") {
    stroke = "#06b6d4";
  }

  return (
    <>
      <BaseEdge
        id={id}
        path={edgePath}
        markerEnd={markerEnd}
        style={{
          stroke,
          strokeWidth: 1.5,
          strokeDasharray: isDashed ? "4,4" : undefined,
          opacity: 0.75,
        }}
      />
      {relation && (
        <EdgeLabelRenderer>
          <div
            style={{
              position: "absolute",
              transform: `translate(-50%, -50%) translate(${labelX}px,${labelY}px)`,
              pointerEvents: "none",
            }}
            className="px-1.5 py-0.5 rounded text-[9px] font-mono uppercase tracking-wider bg-card border border-border text-muted-foreground shadow-sm select-none"
          >
            {relation}
          </div>
        </EdgeLabelRenderer>
      )}
    </>
  );
};

