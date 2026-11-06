import React from "react";
import { useKnoxStore } from "@/store/useKnoxStore";
import { cn } from "@/lib/cn";

export const GraphTab: React.FC = () => {
  const { userSettings, updateUserSettings } = useKnoxStore();

  return (
    <div className="space-y-6 animate-in fade-in-50 duration-150">
      <div>
        <h3 className="text-sm font-semibold text-foreground">Knowledge Graph Canvas</h3>
        <p className="text-xs text-muted-foreground">
          Customize the default visual presentation, continuous D3 physics, and layout parameters
        </p>
      </div>

      <div className="space-y-4">
        {/* Default View Mode */}
        <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-3 p-3.5 rounded-lg border border-border bg-card/60">
          <div>
            <label className="text-xs font-medium text-foreground block">Default Node Mode</label>
            <span className="text-xs text-muted-foreground">
              Cards mode shows structured rectangular summaries; Cosmos mode renders glowing spherical nodes
            </span>
          </div>
          <div className="flex items-center gap-1.5 shrink-0">
            {(["cards", "cosmos"] as const).map((mode) => (
              <button
                key={mode}
                type="button"
                onClick={() => updateUserSettings({ defaultGraphMode: mode })}
                className={cn(
                  "px-3 py-1.5 rounded-md text-xs font-medium capitalize border transition-all cursor-pointer",
                  userSettings.defaultGraphMode === mode
                    ? "bg-primary text-primary-foreground border-primary shadow-xs font-semibold"
                    : "bg-muted/70 text-muted-foreground border-border hover:text-foreground hover:bg-muted"
                )}
              >
                {mode === "cards" ? "Cards (Structured)" : "Cosmos (Orbs)"}
              </button>
            ))}
          </div>
        </div>

        {/* Continuous Live Physics Toggle */}
        <div className="flex items-center justify-between p-3.5 rounded-lg border border-border bg-card/60">
          <div>
            <label className="text-xs font-medium text-foreground block">Continuous D3 Force Physics</label>
            <span className="text-xs text-muted-foreground">
              Keep live force simulation active continuously for spring-like organic node interaction
            </span>
          </div>
          <input
            type="checkbox"
            checked={userSettings.defaultLivePhysics}
            onChange={(e) => updateUserSettings({ defaultLivePhysics: e.target.checked })}
            className="size-4 rounded border-border text-primary focus:ring-primary cursor-pointer"
          />
        </div>

        {/* Default Layout Engine */}
        <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-3 p-3.5 rounded-lg border border-border bg-card/60">
          <div>
            <label className="text-xs font-medium text-foreground block">Default Layout Engine</label>
            <span className="text-xs text-muted-foreground">
              Initial graph layout algorithm applied when rendering new subgraphs
            </span>
          </div>
          <select
            value={userSettings.defaultLayoutEngine}
            onChange={(e) =>
              updateUserSettings({
                defaultLayoutEngine: e.target.value as "hierarchical" | "dagre" | "force" | "grid",
              })
            }
            className="h-8 px-2.5 rounded-md border border-border bg-background text-xs font-mono text-foreground focus:outline-none focus:ring-1 focus:ring-primary cursor-pointer"
          >
            <option value="hierarchical">Hierarchical (ELK Layered)</option>
            <option value="dagre">Dagre (Fast DAG)</option>
            <option value="force">D3 Force Directed</option>
            <option value="grid">Compact Grid</option>
          </select>
        </div>

        {/* Overlays: File Nodes, Legend, Edge Labels */}
        <div className="p-3.5 rounded-lg border border-border bg-card/60 space-y-3">
          <span className="text-xs font-medium text-foreground block">Canvas Elements & Overlays</span>
          <div className="space-y-2">
            <label className="flex items-center gap-2.5 text-xs text-foreground cursor-pointer">
              <input
                type="checkbox"
                checked={userSettings.showFileNodes}
                onChange={(e) => updateUserSettings({ showFileNodes: e.target.checked })}
                className="size-4 rounded border-border text-primary focus:ring-primary"
              />
              <span>Display touched File nodes alongside Action nodes</span>
            </label>
            <label className="flex items-center gap-2.5 text-xs text-foreground cursor-pointer">
              <input
                type="checkbox"
                checked={userSettings.showLegend}
                onChange={(e) => updateUserSettings({ showLegend: e.target.checked })}
                className="size-4 rounded border-border text-primary focus:ring-primary"
              />
              <span>Show interactive floating color & role legend</span>
            </label>
            <label className="flex items-center gap-2.5 text-xs text-foreground cursor-pointer">
              <input
                type="checkbox"
                checked={userSettings.showEdgeLabels}
                onChange={(e) => updateUserSettings({ showEdgeLabels: e.target.checked })}
                className="size-4 rounded border-border text-primary focus:ring-primary"
              />
              <span>Show directional relationship badges on causal edges (implements, depends_on)</span>
            </label>
          </div>
        </div>
      </div>
    </div>
  );
};

