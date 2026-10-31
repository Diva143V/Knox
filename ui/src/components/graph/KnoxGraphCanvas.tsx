import React, { useState, useEffect, useCallback, useMemo, useRef } from "react";
import {
  ReactFlow,
  Background,
  BackgroundVariant,
  Controls,
  MiniMap,
  useNodesState,
  useEdgesState,
  MarkerType,
  type Node,
  type Edge,
  type ReactFlowInstance,
} from "@xyflow/react";
import "@xyflow/react/dist/style.css";

import { useKnoxStore } from "@/store/useKnoxStore";
import { ActionNode } from "./ActionNode";
import { FileNode } from "./FileNode";
import { CausalEdge } from "./CausalEdge";
import { useGraphLayout, type LayoutEngine } from "@/hooks/useGraphLayout";
import { Button } from "@/components/ui/Button";
import { Input } from "@/components/ui/Input";
import { EmptyState } from "@/components/ui/EmptyState";
import { cn } from "@/lib/cn";
import {
  Filter,
  Search,
  GitBranch,
  Network,
  Grid,
  RefreshCw,
  FileCode,
  Workflow,
  RotateCcw,
  UploadCloud,
  Maximize2,
  Eye,
} from "lucide-react";
import type { KnoxAction } from "@/types/knox";

const nodeTypes = {
  actionNode: ActionNode,
  fileNode: FileNode,
};

const edgeTypes = {
  causalEdge: CausalEdge,
};

const selectClass =
  "h-7 bg-background border border-border rounded-md px-2 text-sm text-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-focus-ring cursor-pointer font-mono";

const LAYOUTS: {
  id: LayoutEngine;
  label: string;
  title: string;
  icon: React.ReactNode;
}[] = [
  {
    id: "hierarchical",
    label: "ELK",
    title: "Hierarchical layered layout (ELK)",
    icon: <GitBranch className="size-3.5" />,
  },
  {
    id: "dagre",
    label: "Dagre",
    title: "Fast hierarchical layout (Dagre)",
    icon: <Workflow className="size-3.5" />,
  },
  {
    id: "force",
    label: "Force",
    title: "Force-directed layout (D3)",
    icon: <Network className="size-3.5" />,
  },
  {
    id: "grid",
    label: "Grid",
    title: "Compact grid layout",
    icon: <Grid className="size-3.5" />,
  },
];

const MARKER = {
  follows: { type: MarkerType.ArrowClosed, color: "var(--primary)", width: 16, height: 16 },
  supersedes: { type: MarkerType.ArrowClosed, color: "var(--warning)", width: 16, height: 16 },
  targets: { type: MarkerType.ArrowClosed, color: "var(--plan)", width: 16, height: 16 },
} as const;

export const KnoxGraphCanvas: React.FC = () => {
  const {
    actions,
    setSelectedAction,
    currentOnly,
    setCurrentOnly,
    setActiveTab,
    refreshData,
    isLoading,
  } = useKnoxStore();

  const [actorFilter, setActorFilter] = useState<string>("all");
  const [typeFilter, setTypeFilter] = useState<string>("all");
  const [searchHighlight, setSearchHighlight] = useState<string>("");
  const [showFileNodes, setShowFileNodes] = useState<boolean>(true);
  const [showLegend, setShowLegend] = useState<boolean>(true);
  const rfRef = useRef<ReactFlowInstance | null>(null);

  const { layoutEngine, setLayoutEngine, isLayouting, applyLayout } =
    useGraphLayout();

  const [nodes, setNodes, onNodesChange] = useNodesState<Node>([]);
  const [edges, setEdges, onEdgesChange] = useEdgesState<Edge>([]);

  const availableActors = useMemo(() => {
    const set = new Set<string>();
    actions.forEach((a) => {
      if (a.actor) set.add(a.actor);
    });
    return Array.from(set).sort();
  }, [actions]);

  const availableTypes = useMemo(() => {
    const set = new Set<string>();
    actions.forEach((a) => {
      if (a.action_type) set.add(a.action_type);
    });
    return Array.from(set).sort();
  }, [actions]);

  const activeFilterCount = useMemo(() => {
    let n = 0;
    if (actorFilter !== "all") n++;
    if (typeFilter !== "all") n++;
    if (searchHighlight.trim()) n++;
    if (currentOnly) n++;
    if (!showFileNodes) n++;
    return n;
  }, [actorFilter, typeFilter, searchHighlight, currentOnly, showFileNodes]);

  useEffect(() => {
    let isCancelled = false;

    const buildAndLayoutGraph = async () => {
      const rawNodes: Node[] = [];
      const rawEdges: Edge[] = [];
      const fileCounts = new Map<string, number>();

      const filteredActions = actions.filter((a) => {
        if (currentOnly && !a.is_current) return false;
        if (
          actorFilter !== "all" &&
          a.actor.toLowerCase() !== actorFilter.toLowerCase()
        )
          return false;
        if (
          typeFilter !== "all" &&
          a.action_type.toLowerCase() !== typeFilter.toLowerCase()
        )
          return false;
        return true;
      });

      const filteredActionIds = new Set(filteredActions.map((a) => a.id));

      if (showFileNodes) {
        for (const action of filteredActions) {
          for (const f of action.files || []) {
            fileCounts.set(f, (fileCounts.get(f) || 0) + 1);
          }
        }
      }

      const lowerSearch = searchHighlight.trim().toLowerCase();

      filteredActions.forEach((action) => {
        const isHighlighted =
          lowerSearch.length > 0 &&
          (action.summary.toLowerCase().includes(lowerSearch) ||
            action.actor.toLowerCase().includes(lowerSearch) ||
            (action.files &&
              action.files.some((f) => f.toLowerCase().includes(lowerSearch))));

        rawNodes.push({
          id: action.id,
          type: "actionNode",
          position: { x: 0, y: 0 },
          data: {
            action,
            isHighlighted,
          },
        });

        if (
          action.parent_action_id &&
          filteredActionIds.has(action.parent_action_id)
        ) {
          rawEdges.push({
            id: `edge-${action.parent_action_id}->${action.id}`,
            source: action.parent_action_id,
            target: action.id,
            type: "causalEdge",
            data: { relation: "follows" },
            markerEnd: MARKER.follows,
          });
        }

        if (action.supersedes && filteredActionIds.has(action.supersedes)) {
          rawEdges.push({
            id: `sup-${action.id}->${action.supersedes}`,
            source: action.id,
            target: action.supersedes,
            type: "causalEdge",
            data: { relation: "supersedes" },
            markerEnd: MARKER.supersedes,
          });
        }
      });

      if (showFileNodes) {
        const sortedFiles = Array.from(fileCounts.entries())
          .sort((a, b) => b[1] - a[1])
          .slice(0, 25);

        sortedFiles.forEach(([filePath, count]) => {
          const fileNodeId = `file:${filePath}`;
          rawNodes.push({
            id: fileNodeId,
            type: "fileNode",
            position: { x: 0, y: 0 },
            data: { filePath, count },
          });

          filteredActions.forEach((action) => {
            if (action.files && action.files.includes(filePath)) {
              rawEdges.push({
                id: `target-${action.id}->${fileNodeId}`,
                source: action.id,
                target: fileNodeId,
                type: "causalEdge",
                data: { relation: "targets" },
                markerEnd: MARKER.targets,
              });
            }
          });
        });
      }

      const positioned = await applyLayout(rawNodes, rawEdges, layoutEngine);
      if (!isCancelled) {
        setNodes(positioned);
        setEdges(rawEdges);
        requestAnimationFrame(() => {
          if (!isCancelled) {
            rfRef.current?.fitView({ padding: 0.18, duration: 200 });
          }
        });
      }
    };

    buildAndLayoutGraph();

    return () => {
      isCancelled = true;
    };
    // Selection is handled inside ActionNode via Zustand — omit selectedAction to avoid full relayout
  }, [
    actions,
    currentOnly,
    actorFilter,
    typeFilter,
    searchHighlight,
    showFileNodes,
    layoutEngine,
    applyLayout,
    setNodes,
    setEdges,
  ]);

  const onNodeClick = useCallback(
    (_: React.MouseEvent, node: Node) => {
      if (node.type === "actionNode") {
        const action = (node.data as { action: KnoxAction }).action;
        setSelectedAction(action);
      }
    },
    [setSelectedAction]
  );

  const onPaneClick = useCallback(() => {
    setSelectedAction(null);
  }, [setSelectedAction]);

  const handleResetFilters = useCallback(() => {
    setActorFilter("all");
    setTypeFilter("all");
    setSearchHighlight("");
    setCurrentOnly(true);
    setShowFileNodes(true);
  }, [setCurrentOnly]);

  const handleFitView = useCallback(() => {
    rfRef.current?.fitView({ padding: 0.18, duration: 200 });
  }, []);

  const handleRelayout = useCallback(() => {
    void (async () => {
      if (nodes.length === 0) return;
      const positioned = await applyLayout(nodes, edges, layoutEngine);
      setNodes(positioned);
      requestAnimationFrame(() =>
        rfRef.current?.fitView({ padding: 0.18, duration: 200 })
      );
    })();
  }, [applyLayout, nodes, edges, layoutEngine, setNodes]);

  const actionNodeCount = nodes.filter((n) => n.type === "actionNode").length;
  const fileNodeCount = nodes.filter((n) => n.type === "fileNode").length;

  return (
    <div className="relative w-full h-full flex overflow-hidden select-none bg-background">
      {/* Filter toolbar */}
      <div
        className={cn(
          "absolute top-3 left-3 z-20 flex flex-wrap items-center gap-2",
          "max-w-[min(100%-1.5rem,42rem)] bg-card border border-border px-2.5 py-1.5 rounded-md shadow-sm"
        )}
        role="toolbar"
        aria-label="Graph filters"
      >
        <div className="flex items-center gap-1.5 text-sm text-muted-foreground shrink-0">
          <Filter className="size-3.5 text-primary" aria-hidden />
          <span className="font-medium text-foreground">Graph</span>
          {activeFilterCount > 0 && (
            <span className="font-mono text-xs text-primary bg-primary-muted px-1 rounded-sm">
              {activeFilterCount}
            </span>
          )}
        </div>

        <div className="relative">
          <Search
            className="size-3 text-muted-foreground absolute left-2 top-1/2 -translate-y-1/2 pointer-events-none"
            aria-hidden
          />
          <Input
            type="text"
            value={searchHighlight}
            onChange={(e) => setSearchHighlight(e.target.value)}
            placeholder="Highlight…"
            aria-label="Highlight matching nodes"
            className="pl-6 h-7 w-28 text-sm"
          />
        </div>

        <select
          value={actorFilter}
          onChange={(e) => setActorFilter(e.target.value)}
          className={selectClass}
          aria-label="Filter by actor"
        >
          <option value="all">All Actors</option>
          {availableActors.map((actor) => (
            <option key={actor} value={actor}>
              {actor}
            </option>
          ))}
        </select>

        <select
          value={typeFilter}
          onChange={(e) => setTypeFilter(e.target.value)}
          className={selectClass}
          aria-label="Filter by type"
        >
          <option value="all">All Types</option>
          {availableTypes.map((type) => (
            <option key={type} value={type}>
              {type}
            </option>
          ))}
        </select>

        <label className="flex items-center gap-1.5 text-sm text-muted-foreground cursor-pointer hover:text-foreground">
          <input
            type="checkbox"
            checked={currentOnly}
            onChange={(e) => setCurrentOnly(e.target.checked)}
            className="rounded border-border accent-primary cursor-pointer"
          />
          <span>Current</span>
        </label>

        <label className="flex items-center gap-1 text-sm text-muted-foreground cursor-pointer hover:text-foreground border-l border-border pl-2">
          <input
            type="checkbox"
            checked={showFileNodes}
            onChange={(e) => setShowFileNodes(e.target.checked)}
            className="rounded border-border accent-primary cursor-pointer"
          />
          <FileCode className="size-3 text-plan" aria-hidden />
          <span>Files</span>
        </label>
      </div>

      {/* Layout + view toolbar */}
      <div
        className={cn(
          "absolute top-3 right-3 z-20 flex items-center gap-0.5",
          "bg-card border border-border p-1 rounded-md shadow-sm"
        )}
        role="toolbar"
        aria-label="Layout and view controls"
      >
        {LAYOUTS.map((layout) => (
          <button
            key={layout.id}
            type="button"
            onClick={() => setLayoutEngine(layout.id)}
            title={layout.title}
            aria-pressed={layoutEngine === layout.id}
            className={cn(
              "flex items-center gap-1 px-2 py-1 rounded-sm text-sm font-medium transition-colors cursor-pointer",
              "focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-focus-ring",
              layoutEngine === layout.id
                ? "bg-primary-muted text-primary"
                : "text-muted-foreground hover:text-foreground hover:bg-muted"
            )}
          >
            {layout.icon}
            <span className="hidden sm:inline">{layout.label}</span>
          </button>
        ))}

        <div className="w-px h-4 bg-border mx-0.5" aria-hidden />

        <Button
          type="button"
          variant="ghost"
          size="icon"
          className="size-7"
          onClick={handleFitView}
          title="Fit to view"
          aria-label="Fit graph to view"
        >
          <Maximize2 className="size-3.5" />
        </Button>

        <Button
          type="button"
          variant="ghost"
          size="icon"
          className="size-7"
          onClick={handleRelayout}
          title="Re-run layout"
          aria-label="Re-run layout"
          disabled={isLayouting || nodes.length === 0}
        >
          <RotateCcw
            className={cn("size-3.5", isLayouting && "animate-spin text-primary")}
          />
        </Button>

        <Button
          type="button"
          variant="ghost"
          size="icon"
          className="size-7"
          onClick={() => setShowLegend((v) => !v)}
          title="Toggle legend"
          aria-label="Toggle legend"
          aria-pressed={showLegend}
        >
          <Eye className="size-3.5" />
        </Button>

        {(isLayouting || isLoading) && (
          <RefreshCw
            className="size-3.5 text-primary animate-spin mx-1"
            aria-label="Layout in progress"
          />
        )}
      </div>

      {/* Legend */}
      {showLegend && actions.length > 0 && (
        <div
          className="absolute bottom-14 left-3 z-20 bg-card border border-border rounded-md px-3 py-2 shadow-sm text-xs space-y-1.5 max-w-[220px]"
          aria-label="Graph legend"
        >
          <div className="text-overline mb-1">Legend</div>
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
          </div>
        </div>
      )}

      {/* Empty: no data */}
      {actions.length === 0 && !isLayouting && (
        <div className="absolute inset-0 z-10 flex items-center justify-center p-4 pointer-events-none">
          <div className="pointer-events-auto w-full max-w-sm">
            <EmptyState
              icon={<Network />}
              title="Empty knowledge graph"
              description="No actions recorded yet. Ingest a transcript or connect an agent via MCP to populate nodes and causal edges."
              action={
                <div className="flex items-center justify-center gap-2">
                  <Button
                    variant="primary"
                    size="sm"
                    onClick={() => setActiveTab("ingest")}
                  >
                    <UploadCloud className="size-3.5" />
                    Ingest Transcript
                  </Button>
                  <Button
                    variant="outline"
                    size="sm"
                    onClick={() => refreshData()}
                  >
                    <RefreshCw className="size-3.5" />
                    Refresh
                  </Button>
                </div>
              }
            />
          </div>
        </div>
      )}

      {/* Empty: filters hide everything */}
      {actions.length > 0 && nodes.length === 0 && !isLayouting && (
        <div className="absolute top-16 left-1/2 -translate-x-1/2 z-10">
          <div className="flex items-center gap-3 px-3 py-2 rounded-md bg-card border border-warning/40 text-xs shadow-sm">
            <span className="text-foreground">
              No nodes match the current filters.
            </span>
            <Button
              type="button"
              variant="outline"
              size="sm"
              onClick={handleResetFilters}
            >
              <RotateCcw className="size-3" />
              Reset
            </Button>
          </div>
        </div>
      )}

      <div className="flex-1 w-full h-full">
        <ReactFlow
          nodes={nodes}
          edges={edges}
          onNodesChange={onNodesChange}
          onEdgesChange={onEdgesChange}
          onNodeClick={onNodeClick}
          onPaneClick={onPaneClick}
          onInit={(instance) => {
            rfRef.current = instance;
          }}
          nodeTypes={nodeTypes}
          edgeTypes={edgeTypes}
          fitView
          minZoom={0.05}
          maxZoom={4}
          proOptions={{ hideAttribution: true }}
          deleteKeyCode={null}
          multiSelectionKeyCode={null}
        >
          <Background
            variant={BackgroundVariant.Dots}
            color="var(--border)"
            gap={20}
            size={1}
          />
          <Controls
            position="bottom-left"
            showInteractive={false}
            className="!shadow-sm"
          />
          <MiniMap
            position="bottom-right"
            nodeColor={(n) =>
              n.type === "actionNode" ? "var(--primary)" : "var(--plan)"
            }
            maskColor="color-mix(in oklch, var(--background) 80%, transparent)"
            className="!shadow-sm"
            pannable
            zoomable
          />
        </ReactFlow>
      </div>
    </div>
  );
};
