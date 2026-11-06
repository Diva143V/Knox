import React from "react";
import { X } from "lucide-react";
import { Button } from "@/components/ui/Button";
import { cn } from "@/lib/cn";

interface GraphLegendProps {
  showLegend: boolean;
  onToggleLegend: () => void;
  actionNodeCount: number;
  fileNodeCount: number;
  showFileNodes: boolean;
  actionsLength: number;
}

export const GraphLegend: React.FC<GraphLegendProps> = ({
  showLegend,
  onToggleLegend,
  actionNodeCount,
  fileNodeCount,
  showFileNodes,
  actionsLength,
}) => {
  if (!showLegend || actionsLength === 0) return null;

  return (
    <div
      className={cn(
        "absolute bottom-[108px] left-3 z-20",
        "bg-card/95 backdrop-blur-xs border border-border rounded-md px-3 py-2 shadow-sm text-xs space-y-1.5",
        "max-w-[220px] max-h-[min(260px,calc(100vh-280px))] overflow-y-auto",
        "motion-safe:animate-in motion-safe:fade-in-50 motion-safe:duration-150"
      )}
      role="region"
      aria-label="Graph legend"
    >
      <div className="flex items-center justify-between gap-1 mb-1">
        <span className="text-overline">Legend</span>
        <Button
          type="button"
          variant="ghost"
          size="icon"
          className="size-4.5 text-muted-foreground hover:text-foreground -mr-1"
          onClick={onToggleLegend}
          title="Close legend"
          aria-label="Close legend"
        >
          <X className="size-3" />
        </Button>
      </div>
      <div className="flex items-center gap-2 text-muted-foreground">
        <span className="size-2.5 rounded-sm border border-success/50 bg-card shrink-0" />
        <span>
          Action nodes{" "}
          <span className="font-mono text-foreground">{actionNodeCount}</span>
        </span>
      </div>
      <div className="flex items-center gap-2 text-muted-foreground">
        <span className="size-2.5 rounded-sm border border-plan/50 bg-card shrink-0" />
        <span>
          File nodes{" "}
          <span className="font-mono text-foreground">{fileNodeCount}</span>
          {!showFileNodes && " (hidden)"}
        </span>
      </div>
      <div className="pt-1 border-t border-border-subtle space-y-1">
        <div className="flex items-center gap-2">
          <span className="w-4 h-px bg-primary shrink-0" />
          <span className="text-muted-foreground">follows</span>
        </div>
        <div className="flex items-center gap-2">
          <span
            className="w-4 h-px shrink-0 border-t border-dashed border-warning"
            aria-hidden
          />
          <span className="text-muted-foreground">supersedes</span>
        </div>
        <div className="flex items-center gap-2">
          <span className="w-4 h-px bg-plan shrink-0" />
          <span className="text-muted-foreground">targets file</span>
        </div>
        <div className="flex items-center gap-2">
          <span className="w-4 h-px bg-[var(--info,#38bdf8)] shrink-0" />
          <span className="text-muted-foreground">project branch</span>
        </div>
        <div className="flex items-center gap-2">
          <span className="w-4 h-px bg-primary shrink-0" />
          <span className="text-muted-foreground">branch origin</span>
        </div>
      </div>
    </div>
  );
};

