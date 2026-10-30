import React, { useState, useEffect, useCallback, useMemo } from "react";
import {
  ReactFlow,
  Background,
  Controls,
  MiniMap,
  useNodesState,
  useEdgesState,
  type Node,
  type Edge,
  MarkerType,
} from "@xyflow/react";
import "@xyflow/react/dist/style.css";

import { useKnoxStore } from "@/store/useKnoxStore";
import { ActionNode } from "./ActionNode";
import { FileNode } from "./FileNode";
import { CausalEdge } from "./CausalEdge";
import { useGraphLayout } from "@/hooks/useGraphLayout";
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
} from "lucide-react";
import type { KnoxAction } from "@/types/knox";

const nodeTypes = {
  actionNode: ActionNode,
  fileNode: FileNode,
};

const edgeTypes = {
  causalEdge: CausalEdge,
};

export const KnoxGraphCanvas: React.FC = () => {
  const {
    actions,
    selectedAction,
    setSelectedAction,
    currentOnly,
    setCurrentOnly,
    setActiveTab,
    refreshData,
  } = useKnoxStore();

  const [actorFilter, setActorFilter] = useState<string>("all");
  const [typeFilter, setTypeFilter] = useState<string>("all");
  const [searchHighlight, setSearchHighlight] = useState<string>("");
  const [showFileNodes, setShowFileNodes] = useState<boolean>(true);

  const { layoutEngine, setLayoutEngine, isLayouting, applyLayout } = useGraphLayout();

  const [nodes, setNodes, onNodesChange] = useNodesState<Node>([]);
  const [edges, setEdges, onEdgesChange] = useEdgesState<Edge>([]);

  // Unique actors and action types for filter dropdowns
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

  // Compute graph structure
  useEffect(() => {
    let isCancelled = false;

    const buildAndLayoutGraph = async () => {
      const rawNodes: Node[] = [];
      const rawEdges: Edge[] = [];
      const fileCounts = new Map<string, number>();

      // Filter actions
      const filteredActions = actions.filter((a) => {
        if (currentOnly && !a.is_current) return false;
        if (actorFilter !== "all" && a.actor.toLowerCase() !== actorFilter.toLowerCase()) return false;
        if (typeFilter !== "all" && a.action_type.toLowerCase() !== typeFilter.toLowerCase()) return false;
        return true;
      });

      const filteredActionIds = new Set(filteredActions.map((a) => a.id));

      // Calculate file associations
      if (showFileNodes) {
        for (const action of filteredActions) {
          for (const f of action.files || []) {
            fileCounts.set(f, (fileCounts.get(f) || 0) + 1);
          }
        }
      }

      // 1. Build Action Nodes
      const lowerSearch = searchHighlight.trim().toLowerCase();

      filteredActions.forEach((action) => {
        const isHighlighted =
          lowerSearch.length > 0 &&
          (action.summary.toLowerCase().includes(lowerSearch) ||
            action.actor.toLowerCase().includes(lowerSearch) ||
            (action.files && action.files.some((f) => f.toLowerCase().includes(lowerSearch))));

        rawNodes.push({
          id: action.id,
          type: "actionNode",
          position: { x: 0, y: 0 },
          data: {
            action,
            selected: selectedAction?.id === action.id,
            isHighlighted,
          },
        });

        // Causal edge to parent
        if (action.parent_action_id && filteredActionIds.has(action.parent_action_id)) {
          rawEdges.push({
            id: `edge-${action.parent_action_id}->${action.id}`,
            source: action.parent_action_id,
            target: action.id,
            type: "causalEdge",
            data: { relation: "follows" },
            markerEnd: {
              type: MarkerType.ArrowClosed,
              color: "#6366f1",
            },
          });
        }

        // Edge for supersede
        if (action.supersedes && filteredActionIds.has(action.supersedes)) {
          rawEdges.push({
            id: `sup-${action.id}->${action.supersedes}`,
            source: action.id,
            target: action.supersedes,
            type: "causalEdge",
            data: { relation: "supersedes" },
            markerEnd: {
              type: MarkerType.ArrowClosed,
              color: "#f59e0b",
            },
          });
        }
      });

      // 2. Build File Nodes (top connected files)
      if (showFileNodes) {
        // Take top 25 most frequent files to keep canvas uncluttered
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

          // Connect actions touching this file
          filteredActions.forEach((action) => {
            if (action.files && action.files.includes(filePath)) {
              rawEdges.push({
                id: `target-${action.id}->${fileNodeId}`,
                source: action.id,
                target: fileNodeId,
                type: "causalEdge",
                data: { relation: "targets" },
                markerEnd: {
                  type: MarkerType.ArrowClosed,
                  color: "#06b6d4",
                },
              });
            }
          });
        });
      }

      // Run automatic layout engine
      const positioned = await applyLayout(rawNodes, rawEdges, layoutEngine);
      if (!isCancelled) {
        setNodes(positioned);
        setEdges(rawEdges);
      }
    };

    buildAndLayoutGraph();

    return () => {
      isCancelled = true;
    };
  }, [
    actions,
    selectedAction?.id,
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

  const handleResetFilters = useCallback(() => {
    setActorFilter("all");
    setTypeFilter("all");
    setSearchHighlight("");
    setCurrentOnly(false);
  }, [setCurrentOnly]);

  return (
    <div className="relative w-full h-full flex overflow-hidden select-none bg-[#0b0f15]">
      {/* Top Left Floating Filter & Search Overlay */}
      <div className="absolute top-3 left-3 z-20 flex flex-wrap items-center gap-2 bg-card/90 backdrop-blur border border-border/80 px-3 py-1.5 rounded-xl shadow-xl">
        <div className="flex items-center gap-1.5 text-xs text-muted-foreground mr-1">
          <Filter className="w-3.5 h-3.5 text-primary" />
          <span>Graph:</span>
        </div>

        {/* Real-time search highlight input */}
        <div className="relative">
          <Search className="w-3 h-3 text-muted-foreground absolute left-2 top-2 pointer-events-none" />
          <input
            type="text"
            value={searchHighlight}
            onChange={(e) => setSearchHighlight(e.target.value)}
            placeholder="Highlight node…"
            className="bg-background border border-border rounded-md pl-6 pr-2 py-1 text-xs text-foreground placeholder:text-muted-foreground focus:outline-none focus:border-primary font-mono w-32"
          />
        </div>

        {/* Actor filter */}
        <select
          value={actorFilter}
          onChange={(e) => setActorFilter(e.target.value)}
          className="bg-background border border-border rounded-md px-2 py-1 text-xs text-foreground focus:outline-none cursor-pointer font-mono"
        >
          <option value="all">All Actors</option>
          {availableActors.map((actor) => (
            <option key={actor} value={actor}>
              {actor}
            </option>
          ))}
        </select>

        {/* Type filter */}
        <select
          value={typeFilter}
          onChange={(e) => setTypeFilter(e.target.value)}
          className="bg-background border border-border rounded-md px-2 py-1 text-xs text-foreground focus:outline-none cursor-pointer font-mono"
        >
          <option value="all">All Types</option>
          {availableTypes.map((type) => (
            <option key={type} value={type}>
              {type}
            </option>
          ))}
        </select>

        {/* Current / Active Only Toggle */}
        <label className="flex items-center gap-1.5 text-xs text-muted-foreground cursor-pointer pl-1 hover:text-foreground transition">
          <input
            type="checkbox"
            checked={currentOnly}
            onChange={(e) => setCurrentOnly(e.target.checked)}
            className="rounded border-border text-primary focus:ring-0 cursor-pointer"
          />
          <span>Active Only</span>
        </label>

        {/* Show File Nodes Toggle */}
        <label className="flex items-center gap-1.5 text-xs text-muted-foreground cursor-pointer pl-1 border-l border-border/50 pl-2 hover:text-foreground transition">
          <input
            type="checkbox"
            checked={showFileNodes}
            onChange={(e) => setShowFileNodes(e.target.checked)}
            className="rounded border-border text-cyan-400 focus:ring-0 cursor-pointer"
          />
          <span className="flex items-center gap-1">
            <FileCode className="w-3 h-3 text-cyan-400" />
            <span>Files</span>
          </span>
        </label>
      </div>

      {/* Top Right Floating Layout Selector */}
      <div className="absolute top-3 right-3 z-20 flex items-center gap-1 bg-card/90 backdrop-blur border border-border/80 p-1 rounded-xl shadow-xl">
        <button
          onClick={() => setLayoutEngine("hierarchical")}
          className={`flex items-center gap-1.5 px-2.5 py-1 rounded-lg text-xs font-medium transition cursor-pointer ${
            layoutEngine === "hierarchical"
              ? "bg-primary text-primary-foreground shadow-sm"
              : "text-muted-foreground hover:text-foreground hover:bg-background"
          }`}
          title="Hierarchical layered DAG layout (ELK Layered)"
        >
          <GitBranch className="w-3.5 h-3.5" />
          <span>ELK</span>
        </button>

        <button
          onClick={() => setLayoutEngine("dagre")}
          className={`flex items-center gap-1.5 px-2.5 py-1 rounded-lg text-xs font-medium transition cursor-pointer ${
            layoutEngine === "dagre"
              ? "bg-primary text-primary-foreground shadow-sm"
              : "text-muted-foreground hover:text-foreground hover:bg-background"
          }`}
          title="Fast hierarchical layout (Dagre)"
        >
          <Workflow className="w-3.5 h-3.5" />
          <span>Dagre</span>
        </button>

        <button
          onClick={() => setLayoutEngine("force")}
          className={`flex items-center gap-1.5 px-2.5 py-1 rounded-lg text-xs font-medium transition cursor-pointer ${
            layoutEngine === "force"
              ? "bg-primary text-primary-foreground shadow-sm"
              : "text-muted-foreground hover:text-foreground hover:bg-background"
          }`}
          title="Force-directed organic physics layout (D3 Force)"
        >
          <Network className="w-3.5 h-3.5" />
          <span>Force</span>
        </button>

        <button
          onClick={() => setLayoutEngine("grid")}
          className={`flex items-center gap-1.5 px-2.5 py-1 rounded-lg text-xs font-medium transition cursor-pointer ${
            layoutEngine === "grid"
              ? "bg-primary text-primary-foreground shadow-sm"
              : "text-muted-foreground hover:text-foreground hover:bg-background"
          }`}
          title="Compact uniform grid layout"
        >
          <Grid className="w-3.5 h-3.5" />
          <span>Grid</span>
        </button>

        {isLayouting && (
          <div className="px-1 text-primary animate-spin">
            <RefreshCw className="w-3.5 h-3.5" />
          </div>
        )}
      </div>

      {/* Empty State: 0 Actions in Database */}
      {actions.length === 0 && !isLayouting && (
        <div className="absolute inset-0 z-10 flex flex-col items-center justify-center pointer-events-none p-4">
          <div className="p-6 rounded-2xl bg-card/95 backdrop-blur border border-border text-center max-w-sm shadow-2xl pointer-events-auto space-y-3">
            <div className="w-12 h-12 rounded-xl bg-primary/10 border border-primary/20 flex items-center justify-center mx-auto text-primary">
              <Network className="w-6 h-6" />
            </div>
            <div>
              <h3 className="text-sm font-semibold text-foreground">Empty Knowledge Graph</h3>
              <p className="text-xs text-muted-foreground mt-1 leading-relaxed">
                No actions recorded yet. Ingest an agent transcript or run CLI commands to populate nodes and causal edges.
              </p>
            </div>
            <div className="pt-2 flex items-center justify-center gap-2">
              <button
                onClick={() => setActiveTab("ingest")}
                className="flex items-center gap-1.5 px-3 py-1.5 rounded-lg bg-primary text-primary-foreground text-xs font-medium hover:bg-primary/90 transition cursor-pointer"
              >
                <UploadCloud className="w-3.5 h-3.5" />
                <span>Ingest Transcript</span>
              </button>
              <button
                onClick={() => refreshData()}
                className="flex items-center gap-1.5 px-3 py-1.5 rounded-lg bg-secondary text-secondary-foreground text-xs font-medium hover:bg-secondary/80 border border-border transition cursor-pointer"
              >
                <RefreshCw className="w-3.5 h-3.5" />
                <span>Refresh</span>
              </button>
            </div>
          </div>
        </div>
      )}

      {/* Empty Filter State: Actions exist, but filters return 0 nodes */}
      {actions.length > 0 && nodes.length === 0 && !isLayouting && (
        <div className="absolute top-16 left-1/2 -translate-x-1/2 z-10 pointer-events-none">
          <div className="flex items-center gap-3 px-4 py-2 rounded-xl bg-card/95 backdrop-blur border border-amber-500/30 text-amber-200 shadow-xl pointer-events-auto text-xs">
            <span>No actions match current filters.</span>
            <button
              onClick={handleResetFilters}
              className="flex items-center gap-1 px-2 py-0.5 rounded bg-amber-500/20 hover:bg-amber-500/30 border border-amber-500/40 text-[11px] font-medium text-amber-300 transition cursor-pointer"
            >
              <RotateCcw className="w-3 h-3" />
              <span>Reset Filters</span>
            </button>
          </div>
        </div>
      )}

      {/* Main React Flow Canvas */}
      <div className="flex-1 w-full h-full">
        <ReactFlow
          nodes={nodes}
          edges={edges}
          onNodesChange={onNodesChange}
          onEdgesChange={onEdgesChange}
          onNodeClick={onNodeClick}
          nodeTypes={nodeTypes}
          edgeTypes={edgeTypes}
          fitView
          minZoom={0.05}
          maxZoom={4.0}
          proOptions={{ hideAttribution: true }}
        >
          <Background color="#1e2530" gap={24} size={1} />
          <Controls position="bottom-left" showInteractive={false} />
          <MiniMap
            position="bottom-right"
            nodeColor={(n) => (n.type === "actionNode" ? "#6366f1" : "#06b6d4")}
            maskColor="rgba(11, 15, 21, 0.75)"
          />
        </ReactFlow>
      </div>
    </div>
  );
};
