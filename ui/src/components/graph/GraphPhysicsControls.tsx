import React from "react";
import { Atom, Flame } from "lucide-react";
import { cn } from "@/lib/cn";

interface GraphPhysicsControlsProps {
  nodesCount: number;
  livePhysics: boolean;
  isPhysicsRunning: boolean;
  onTogglePhysics: () => void;
  onReheat: (alpha: number) => void;
}

export const GraphPhysicsControls: React.FC<GraphPhysicsControlsProps> = ({
  nodesCount,
  livePhysics,
  isPhysicsRunning,
  onTogglePhysics,
  onReheat,
}) => {
  if (nodesCount === 0) return null;

  return (
    <div className="absolute bottom-3 left-1/2 z-10 flex -translate-x-1/2 items-center gap-1 rounded-full border border-border bg-card/90 px-1.5 py-1 shadow-sm backdrop-blur-md">
      <button
        type="button"
        onClick={onTogglePhysics}
        aria-pressed={livePhysics}
        title="Toggle the force simulation"
        className={cn(
          "flex cursor-pointer items-center gap-1.5 rounded-full px-2 py-1 text-2xs font-medium transition-colors",
          livePhysics
            ? "bg-warning/15 text-warning"
            : "text-muted-foreground hover:text-foreground"
        )}
      >
        <Atom
          className={cn("size-3", livePhysics && isPhysicsRunning && "animate-spin")}
        />
        <span>{livePhysics ? "Live physics" : "Physics off"}</span>
      </button>
      {livePhysics && (
        <button
          type="button"
          onClick={() => onReheat(0.55)}
          aria-label="Shake simulation"
          title="Shake the graph"
          className="flex cursor-pointer items-center gap-1 rounded-full px-2 py-1 text-2xs font-medium text-muted-foreground transition-colors hover:bg-muted hover:text-foreground"
        >
          <Flame className="size-3" />
          <span>Shake</span>
        </button>
      )}
    </div>
  );
};

