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
import { ProjectRootNode } from "./ProjectRootNode";
import { CausalEdge } from "./CausalEdge";
import { CircleNode, type CircleNodeData } from "./CircleNode";
import { useGraphLayout, type LayoutEngine } from "@/hooks/useGraphLayout";
import { useLivePhysics } from "@/hooks/useLivePhysics";
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
  LayoutDashboard,
  Orbit,
  Atom,
  Flame,
  Map as MapIcon,
  X,
  ChevronDown,
  LayoutGrid,
} from "lucide-react";
import type { KnoxAction, Project } from "@/types/knox";

const nodeTypes = {
  projectRootNode: ProjectRootNode,
  actionNode: ActionNode,
  fileNode: FileNode,
  circleNode: CircleNode,
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
  project: { type: MarkerType.ArrowClosed, color: "var(--info, #38bdf8)", width: 16, height: 16 },
  contains: { type: MarkerType.ArrowClosed, color: "var(--success, #10b981)", width: 16, height: 16 },
  origin: { type: MarkerType.ArrowClosed, color: "var(--purple, #8b5cf6)", width: 16, height: 16 },
  depends_on: { type: MarkerType.ArrowClosed, color: "var(--warning, #f59e0b)", width: 16, height: 16 },
  implements: { type: MarkerType.ArrowClosed, color: "var(--success, #10b981)", width: 16, height: 16 },
  verifies: { type: MarkerType.ArrowClosed, color: "var(--plan, #06b6d4)", width: 16, height: 16 },
  relates_to: { type: MarkerType.ArrowClosed, color: "var(--muted-foreground, #94a3b8)", width: 16, height: 16 },
  reverts: { type: MarkerType.ArrowClosed, color: "var(--danger, #ef4444)", width: 16, height: 16 },
} as const;

export const KnoxGraphCanvas: React.FC = () => {
  const {
    actions,
    selectedAction,
    setSelectedAction,
    projects,
    selectedProject,
    setSelectedProject,
    getProjectForAction,
    recentProjects,
    currentOnly,
    setCurrentOnly,
    setActiveTab,
    refreshData,
    activeProjectScope,
    config,
    isLoading,
  } = useKnoxStore();

  const [actorFilter, setActorFilter] = useState<string>("all");
  const [typeFilter, setTypeFilter] = useState<string>("all");
  const [searchHighlight, setSearchHighlight] = useState<string>("");
  const [showFileNodes, setShowFileNodes] = useState<boolean>(true);
  const [showLegend, setShowLegend] = useState<boolean>(() => {
    try {
      const saved = localStorage.getItem("knox_graph_show_legend");
      if (saved !== null) return saved === "true";
    } catch {}
    return true;
  });
  const [showMinimap, setShowMinimap] = useState<boolean>(() => {
    try {
      const saved = localStorage.getItem("knox_graph_show_minimap");
      if (saved !== null) return saved === "true";
    } catch {}
    return true;
  });
  const handleToggleLegend = () => {
    setShowLegend((prev) => {
      const next = !prev;
      try {
        localStorage.setItem("knox_graph_show_legend", String(next));
      } catch {}
      return next;
    });
  };
  const handleToggleMinimap = () => {
    setShowMinimap((prev) => {
      const next = !prev;
      try {
        localStorage.setItem("knox_graph_show_minimap", String(next));
      } catch {}
      return next;
    });
  };
  const rfRef = useRef<ReactFlowInstance | null>(null);
  const [isFiltersOpen, setIsFiltersOpen] = useState<boolean>(false);
  const [isLayoutOpen, setIsLayoutOpen] = useState<boolean>(false);
  const toolbarContainerRef = useRef<HTMLDivElement | null>(null);

  useEffect(() => {
    const handleClickOutside = (e: MouseEvent) => {
      if (
        (isFiltersOpen || isLayoutOpen) &&
        toolbarContainerRef.current &&
        !toolbarContainerRef.current.contains(e.target as globalThis.Node)
      ) {
        setIsFiltersOpen(false);
        setIsLayoutOpen(false);
      }
    };

    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === "Escape" && (isFiltersOpen || isLayoutOpen)) {
        setIsFiltersOpen(false);
        setIsLayoutOpen(false);
      }
    };

    document.addEventListener("mousedown", handleClickOutside);
    document.addEventListener("keydown", handleKeyDown);
    return () => {
      document.removeEventListener("mousedown", handleClickOutside);
      document.removeEventListener("keydown", handleKeyDown);
    };
  }, [isFiltersOpen, isLayoutOpen]);

  type ViewMode = "cards" | "cosmos";
  const [viewMode, setViewMode] = useState<ViewMode>(() => {
    try {
      const saved = localStorage.getItem("knox_graph_view_mode");
      if (saved === "cosmos" || saved === "cards") return saved;
    } catch {}
    return "cards";
  });

  const [livePhysics, setLivePhysics] = useState<boolean>(() => {
    try {
      const saved = localStorage.getItem("knox_graph_live_physics");
      if (saved !== null) return saved === "true";
    } catch {}
    return false;
  });

  const handleToggleViewMode = (mode: ViewMode) => {
    setViewMode(mode);
    try {
      localStorage.setItem("knox_graph_view_mode", mode);
    } catch {}
    if (mode === "cosmos" && !livePhysics) {
      setLivePhysics(true);
      try {
        localStorage.setItem("knox_graph_live_physics", "true");
      } catch {}
    }
  };

  const handleTogglePhysics = () => {
    setLivePhysics((prev) => {
      const next = !prev;
      try {
        localStorage.setItem("knox_graph_live_physics", String(next));
      } catch {}
      return next;
    });
  };

  const { layoutEngine, setLayoutEngine, isLayouting, applyLayout } =
    useGraphLayout();

  const [nodes, setNodes, onNodesChange] = useNodesState<Node>([]);
  const [edges, setEdges, onEdgesChange] = useEdgesState<Edge>([]);

  const {
    isRunning: isPhysicsRunning,
    reheat,
    onNodeDragStart,
    onNodeDrag,
    onNodeDragStop,
  } = useLivePhysics({
    enabled: livePhysics,
    nodes,
    edges,
    setNodes,
    chargeStrength: viewMode === "cosmos" ? -340 : -450,
    linkDistance: viewMode === "cosmos" ? 110 : 180,
    collisionPadding: viewMode === "cosmos" ? 22 : 35,
  });

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

      // Filter actions
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

      // Global DB root node
      const globalDbPath = config?.kg_path || "~/.knox/knox.db";
      const globalNodeId = "root-global-db";

      // 1. Determine active project branches to display
      let activeProjects: Project[] = [];
      if (projects && projects.length > 0) {
        activeProjects = [...projects];
      } else if (recentProjects && recentProjects.length > 0) {
        activeProjects = recentProjects.map((r, idx) => ({
          id: `recent-${idx}`,
          name: r.name,
          path: r.path,
          created_at: new Date().toISOString(),
        }));
      } else {
        activeProjects = [
          {
            id: "default-knox",
            name: "Knox",
            path: "D:\\Knox",
            created_at: new Date().toISOString(),
          },
        ];
      }

      // If activeProjectScope is selected (e.g. D:\Knox), filter to that project branch
      if (activeProjectScope) {
        const normScope = activeProjectScope.toLowerCase().replace(/\\/g, "/");
        const matched = activeProjects.filter(
          (p) => p.path.toLowerCase().replace(/\\/g, "/") === normScope
        );
        if (matched.length > 0) {
          activeProjects = matched;
        } else {
          const folderName = normScope.split("/").filter(Boolean).pop() || activeProjectScope;
          activeProjects = [
            {
              id: "scoped-custom",
              name: folderName,
              path: activeProjectScope,
              created_at: new Date().toISOString(),
            },
          ];
        }
      }

      // 2. Add Global Knowledge Graph Top Anchor Node
      const allFileSet = new Set<string>();
      filteredActions.forEach((a) => a.files?.forEach((f) => allFileSet.add(f)));

      if (viewMode === "cosmos") {
        rawNodes.push({
          id: globalNodeId,
          type: "circleNode",
          position: { x: 0, y: 0 },
          data: {
            kind: "project",
            title: "Global KG",
            subtitle: "Global Graph Root",
            actionCount: filteredActions.length,
            fileCount: allFileSet.size,
          },
        });
      } else {
        rawNodes.push({
          id: globalNodeId,
          type: "projectRootNode",
          position: { x: 0, y: 0 },
          data: {
            name: "Global Knowledge Graph",
            path: globalDbPath,
            isGlobal: true,
            actionCount: filteredActions.length,
            fileCount: allFileSet.size,
          },
        });
      }

      // 3. Partition actions into their respective projects
      const projectActionMap = new Map<string, KnoxAction[]>();
      activeProjects.forEach((p) => projectActionMap.set(p.path, []));

      filteredActions.forEach((action) => {
        const ownerProj = getProjectForAction(action);
        const targetProj =
          ownerProj && projectActionMap.has(ownerProj.path)
            ? ownerProj
            : activeProjects[0];
        const list = projectActionMap.get(targetProj.path);
        if (list) {
          list.push(action);
        } else {
          projectActionMap.set(targetProj.path, [action]);
        }
      });

      const lowerSearch = searchHighlight.trim().toLowerCase();

      // 4. For each active project, create Project Folder Node & Sub-KG
      activeProjects.forEach((proj) => {
        const projNodeId = `project:${proj.path}`;
        const projActions = projectActionMap.get(proj.path) || [];
        const projActionIds = new Set(projActions.map((a) => a.id));

        // Skip empty projects that have no actions unless explicitly focused or sole project
        if (
          projActions.length === 0 &&
          !activeProjectScope &&
          activeProjects.length > 1 &&
          proj.name !== "Knox"
        ) {
          return;
        }

        // Calculate files for this project
        const projFileCounts = new Map<string, number>();
        projActions.forEach((a) => {
          a.files?.forEach((f) => {
            projFileCounts.set(f, (projFileCounts.get(f) || 0) + 1);
          });
        });

        // Add Project Folder Node
        if (viewMode === "cosmos") {
          rawNodes.push({
            id: projNodeId,
            type: "circleNode",
            position: { x: 0, y: 0 },
            data: {
              kind: "project",
              title: proj.name,
              subtitle: `${projActions.length} actions`,
              project: proj,
              isSelected: selectedProject?.path === proj.path,
            },
          });
        } else {
          rawNodes.push({
            id: projNodeId,
            type: "projectRootNode",
            position: { x: 0, y: 0 },
            data: {
              name: proj.name,
              path: proj.path,
              isGlobal: false,
              actionCount: projActions.length,
              fileCount: projFileCounts.size,
              project: proj,
              isSelected: selectedProject?.path === proj.path,
            },
          });
        }

        // Directed Edge: Global DB -> Project Folder Branch
        rawEdges.push({
          id: `proj-edge-${globalNodeId}->${projNodeId}`,
          source: globalNodeId,
          target: projNodeId,
          type: "causalEdge",
          data: { relation: "project" },
          markerEnd: MARKER.project,
        });

        // 4a. Add File Nodes belonging to this project (if showFileNodes is true)
        if (showFileNodes) {
          const sortedFiles = Array.from(projFileCounts.entries())
            .sort((a, b) => b[1] - a[1])
            .slice(0, 15);

          sortedFiles.forEach(([filePath, count]) => {
            const fileNodeId = `file:${proj.name}:${filePath}`;
            if (viewMode === "cosmos") {
              rawNodes.push({
                id: fileNodeId,
                type: "circleNode",
                position: { x: 0, y: 0 },
                data: {
                  kind: "file",
                  title: filePath.split(/[/\\]/).pop() || filePath,
                  subtitle: filePath,
                  filePath,
                  count,
                },
              });
            } else {
              rawNodes.push({
                id: fileNodeId,
                type: "fileNode",
                position: { x: 0, y: 0 },
                data: { filePath, count },
              });
            }

            // Project Folder -> File ("contains")
            rawEdges.push({
              id: `contains-${projNodeId}->${fileNodeId}`,
              source: projNodeId,
              target: fileNodeId,
              type: "causalEdge",
              data: { relation: "contains" },
              markerEnd: MARKER.contains,
            });

            // Action -> File ("targets")
            projActions.forEach((action) => {
              if (action.files && action.files.includes(filePath)) {
                rawEdges.push({
                  id: `targets-${action.id}->${fileNodeId}`,
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

        // 4b. Add Action Nodes for this project
        projActions.forEach((action) => {
          const isHighlighted =
            lowerSearch.length > 0 &&
            (action.summary.toLowerCase().includes(lowerSearch) ||
              action.actor.toLowerCase().includes(lowerSearch) ||
              (action.files &&
                action.files.some((f) => f.toLowerCase().includes(lowerSearch))));

          if (viewMode === "cosmos") {
            rawNodes.push({
              id: action.id,
              type: "circleNode",
              position: { x: 0, y: 0 },
              data: {
                kind: "action",
                title: action.summary,
                subtitle: `${action.actor} • ${action.action_type}`,
                action,
                selected: selectedAction?.id === action.id,
                isHighlighted,
              },
            });
          } else {
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
          }

          // Causal edge to parent or Project Root
          if (action.parent_action_id && projActionIds.has(action.parent_action_id)) {
            rawEdges.push({
              id: `edge-${action.parent_action_id}->${action.id}`,
              source: action.parent_action_id,
              target: action.id,
              type: "causalEdge",
              data: { relation: "follows" },
              markerEnd: MARKER.follows,
            });
          } else {
            // Action has no parent in this project: anchor to Project Root ("origin")
            rawEdges.push({
              id: `origin-${projNodeId}->${action.id}`,
              source: projNodeId,
              target: action.id,
              type: "causalEdge",
              data: { relation: "origin" },
              markerEnd: MARKER.origin,
            });
          }

          // Edge for supersede
          if (action.supersedes && projActionIds.has(action.supersedes)) {
            rawEdges.push({
              id: `sup-${action.id}->${action.supersedes}`,
              source: action.id,
              target: action.supersedes,
              type: "causalEdge",
              data: { relation: "supersedes" },
              markerEnd: MARKER.supersedes,
            });
          }

          // Semantic Relations (depends_on, implements, verifies, relates_to, reverts)
          if (action.relations && action.relations.length > 0) {
            action.relations.forEach((rel) => {
              const targetId = rel.target_id;
              rawEdges.push({
                id: `rel-${action.id}->${targetId}:${rel.relation_type}`,
                source: action.id,
                target: targetId,
                type: "causalEdge",
                data: { relation: rel.relation_type },
                markerEnd:
                  (MARKER as Record<string, any>)[rel.relation_type] ||
                  MARKER.follows,
              });
            });
          }
        });
      });

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
  }, [
    actions,
    activeProjectScope,
    projects,
    selectedProject?.path,
    getProjectForAction,
    recentProjects,
    config?.kg_path,
    currentOnly,
    actorFilter,
    typeFilter,
    searchHighlight,
    showFileNodes,
    layoutEngine,
    applyLayout,
    setNodes,
    setEdges,
    viewMode,
  ]);

  const onNodeClick = useCallback(
    (_: React.MouseEvent, node: Node) => {
      if (node.type === "actionNode") {
        const action = (node.data as { action: KnoxAction }).action;
        setSelectedAction(action);
      } else if (node.type === "circleNode") {
        const cData = node.data as CircleNodeData;
        if (cData.kind === "action" && cData.action) {
          setSelectedAction(cData.action);
        } else if (cData.kind === "project") {
          const proj = (cData as Record<string, unknown>).project as Project | undefined;
          if (proj) setSelectedProject(proj);
        }
      } else if (node.type === "projectRootNode") {
        const pData = node.data as { isGlobal?: boolean; project?: Project };
        if (!pData.isGlobal && pData.project) {
          setSelectedProject(pData.project);
        }
      }
    },
    [setSelectedAction, setSelectedProject]
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
      {/* Top Collapsible Floating Toolbars */}
      <div
        ref={toolbarContainerRef}
        className="absolute top-3 left-3 z-30 flex flex-col items-start gap-2 pointer-events-none max-w-[calc(100vw-1.5rem)]"
      >
        {/* Compact Master Toggle Bar */}
        <div
          className="pointer-events-auto flex items-center gap-1.5 bg-card/95 backdrop-blur-md border border-border p-1 rounded-md shadow-md"
          role="toolbar"
          aria-label="Knowledge Graph Toolbars"
        >
          {/* Filters Toggle Button */}
          <button
            type="button"
            onClick={() => setIsFiltersOpen((v) => !v)}
            aria-expanded={isFiltersOpen}
            aria-controls="graph-filters-panel"
            className={cn(
              "flex items-center gap-1.5 px-2.5 py-1 rounded-sm text-xs font-medium transition-all cursor-pointer select-none",
              "focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-focus-ring",
              isFiltersOpen
                ? "bg-primary text-primary-foreground font-semibold shadow-xs"
                : "text-foreground hover:bg-muted"
            )}
            title={isFiltersOpen ? "Collapse filters panel (Esc)" : "Expand filters panel"}
          >
            <Filter className={cn("size-3.5", isFiltersOpen ? "text-primary-foreground" : "text-primary")} />
            <span>Filters</span>
            {activeFilterCount > 0 && (
              <span
                className={cn(
                  "font-mono text-2xs px-1 py-0.2 rounded-xs font-semibold",
                  isFiltersOpen
                    ? "bg-primary-foreground/20 text-primary-foreground"
                    : "bg-primary-muted text-primary"
                )}
              >
                {activeFilterCount}
              </span>
            )}
            <ChevronDown
              className={cn(
                "size-3 opacity-70 transition-transform duration-200",
                isFiltersOpen && "rotate-180"
              )}
            />
          </button>

          <div className="w-px h-4 bg-border" aria-hidden />

          {/* Layout Toggle Button */}
          <button
            type="button"
            onClick={() => setIsLayoutOpen((v) => !v)}
            aria-expanded={isLayoutOpen}
            aria-controls="graph-layout-panel"
            className={cn(
              "flex items-center gap-1.5 px-2.5 py-1 rounded-sm text-xs font-medium transition-all cursor-pointer select-none",
              "focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-focus-ring",
              isLayoutOpen
                ? "bg-primary text-primary-foreground font-semibold shadow-xs"
                : "text-foreground hover:bg-muted"
            )}
            title={isLayoutOpen ? "Collapse layout panel (Esc)" : "Expand layout panel"}
          >
            <LayoutGrid className={cn("size-3.5", isLayoutOpen ? "text-primary-foreground" : "text-primary")} />
            <span>Layout</span>
            <span
              className={cn(
                "font-mono text-2xs px-1.5 py-0.2 rounded-xs font-semibold uppercase",
                isLayoutOpen
                  ? "bg-primary-foreground/20 text-primary-foreground"
                  : "bg-muted text-muted-foreground"
              )}
            >
              {layoutEngine === "hierarchical" ? "ELK" : layoutEngine}
            </span>
            <ChevronDown
              className={cn(
                "size-3 opacity-70 transition-transform duration-200",
                isLayoutOpen && "rotate-180"
              )}
            />
          </button>

          {(isLayouting || isLoading) && (
            <RefreshCw
              className="size-3 text-primary animate-spin ml-1 mr-0.5"
              aria-label="Layout in progress"
            />
          )}
        </div>

        {/* Floating Panels (Independent & Non-overlapping) */}
        {(isFiltersOpen || isLayoutOpen) && (
          <div className="flex flex-col md:flex-row items-start gap-2.5 max-w-[calc(100vw-1.5rem)] motion-safe:animate-in motion-safe:fade-in-50 motion-safe:duration-150">
            {/* 1. Graph Filters Panel */}
            {isFiltersOpen && (
              <div
                id="graph-filters-panel"
                className="pointer-events-auto bg-card/95 backdrop-blur-md border border-border rounded-md p-3 shadow-xl flex flex-col gap-2.5 w-72 sm:w-80 max-w-full"
                role="region"
                aria-label="Filter controls"
              >
                {/* Panel Header */}
                <div className="flex items-center justify-between pb-1.5 border-b border-border">
                  <div className="flex items-center gap-1.5">
                    <Filter className="size-3.5 text-primary" />
                    <span className="text-xs font-semibold text-foreground">Graph Filters</span>
                    {activeFilterCount > 0 && (
                      <span className="font-mono text-2xs text-primary bg-primary-muted px-1.5 py-0.2 rounded-sm font-semibold">
                        {activeFilterCount} active
                      </span>
                    )}
                  </div>
                  <div className="flex items-center gap-1">
                    {activeFilterCount > 0 && (
                      <button
                        type="button"
                        onClick={handleResetFilters}
                        className="text-2xs text-muted-foreground hover:text-foreground hover:underline cursor-pointer px-1 py-0.5"
                      >
                        Reset
                      </button>
                    )}
                    <button
                      type="button"
                      onClick={() => setIsFiltersOpen(false)}
                      className="size-5 rounded flex items-center justify-center text-muted-foreground hover:text-foreground hover:bg-muted cursor-pointer"
                      title="Close panel"
                      aria-label="Close filters panel"
                    >
                      <X className="size-3" />
                    </button>
                  </div>
                </div>

                {/* Highlight Field */}
                <div className="flex flex-col gap-1">
                  <label className="text-2xs text-muted-foreground font-medium uppercase tracking-wider">
                    Search Highlight
                  </label>
                  <div className="relative">
                    <Search
                      className="size-3 text-muted-foreground absolute left-2.5 top-1/2 -translate-y-1/2 pointer-events-none"
                      aria-hidden
                    />
                    <Input
                      type="text"
                      value={searchHighlight}
                      onChange={(e) => setSearchHighlight(e.target.value)}
                      placeholder="Highlight matching nodes…"
                      aria-label="Highlight matching nodes"
                      className="pl-7 h-7.5 text-xs w-full"
                    />
                  </div>
                </div>

                {/* Dropdowns Grid */}
                <div className="grid grid-cols-2 gap-2">
                  <div className="flex flex-col gap-1">
                    <label className="text-2xs text-muted-foreground font-medium uppercase tracking-wider">
                      Actor
                    </label>
                    <select
                      value={actorFilter}
                      onChange={(e) => setActorFilter(e.target.value)}
                      className={cn(selectClass, "w-full truncate text-xs h-7.5")}
                      aria-label="Filter by actor"
                    >
                      <option value="all">All Actors</option>
                      {availableActors.map((actor) => (
                        <option key={actor} value={actor}>
                          {actor}
                        </option>
                      ))}
                    </select>
                  </div>

                  <div className="flex flex-col gap-1">
                    <label className="text-2xs text-muted-foreground font-medium uppercase tracking-wider">
                      Type
                    </label>
                    <select
                      value={typeFilter}
                      onChange={(e) => setTypeFilter(e.target.value)}
                      className={cn(selectClass, "w-full truncate text-xs h-7.5")}
                      aria-label="Filter by type"
                    >
                      <option value="all">All Types</option>
                      {availableTypes.map((type) => (
                        <option key={type} value={type}>
                          {type}
                        </option>
                      ))}
                    </select>
                  </div>
                </div>

                {/* Checkbox Toggles */}
                <div className="flex items-center justify-between pt-1 border-t border-border/60">
                  <label className="flex items-center gap-1.5 text-xs text-muted-foreground hover:text-foreground cursor-pointer select-none">
                    <input
                      type="checkbox"
                      checked={currentOnly}
                      onChange={(e) => setCurrentOnly(e.target.checked)}
                      className="rounded border-border accent-primary cursor-pointer size-3.5"
                    />
                    <span>Current Branch</span>
                  </label>

                  <label className="flex items-center gap-1 text-xs text-muted-foreground hover:text-foreground cursor-pointer select-none">
                    <input
                      type="checkbox"
                      checked={showFileNodes}
                      onChange={(e) => setShowFileNodes(e.target.checked)}
                      className="rounded border-border accent-primary cursor-pointer size-3.5"
                    />
                    <FileCode className="size-3 text-plan" aria-hidden />
                    <span>File Nodes</span>
                  </label>
                </div>
              </div>
            )}

            {/* 2. Graph Layout Panel */}
            {isLayoutOpen && (
              <div
                id="graph-layout-panel"
                className="pointer-events-auto bg-card/95 backdrop-blur-md border border-border rounded-md p-3 shadow-xl flex flex-col gap-3 w-80 sm:w-88 max-w-full"
                role="region"
                aria-label="Layout and view controls"
              >
                {/* Panel Header */}
                <div className="flex items-center justify-between pb-1.5 border-b border-border">
                  <div className="flex items-center gap-1.5">
                    <LayoutGrid className="size-3.5 text-primary" />
                    <span className="text-xs font-semibold text-foreground">Graph Layout & Display</span>
                  </div>
                  <button
                    type="button"
                    onClick={() => setIsLayoutOpen(false)}
                    className="size-5 rounded flex items-center justify-center text-muted-foreground hover:text-foreground hover:bg-muted cursor-pointer"
                    title="Close panel"
                    aria-label="Close layout panel"
                  >
                    <X className="size-3" />
                  </button>
                </div>

                {/* View Mode Toggle: Cards vs Cosmos */}
                <div className="flex flex-col gap-1.5">
                  <span className="text-2xs text-muted-foreground font-medium uppercase tracking-wider">
                    Node View Mode
                  </span>
                  <div className="grid grid-cols-2 gap-1.5 bg-muted/60 p-1 rounded-sm border border-border">
                    <button
                      type="button"
                      onClick={() => handleToggleViewMode("cards")}
                      className={cn(
                        "flex items-center justify-center gap-1.5 py-1 px-2 rounded-xs text-xs font-medium transition-colors cursor-pointer",
                        viewMode === "cards"
                          ? "bg-card text-foreground shadow-xs font-semibold"
                          : "text-muted-foreground hover:text-foreground"
                      )}
                      title="Structured card nodes view"
                      aria-pressed={viewMode === "cards"}
                    >
                      <LayoutDashboard className="size-3.5 text-primary" />
                      <span>Cards</span>
                    </button>
                    <button
                      type="button"
                      onClick={() => handleToggleViewMode("cosmos")}
                      className={cn(
                        "flex items-center justify-center gap-1.5 py-1 px-2 rounded-xs text-xs font-medium transition-colors cursor-pointer",
                        viewMode === "cosmos"
                          ? "bg-primary text-primary-foreground shadow-xs font-semibold"
                          : "text-muted-foreground hover:text-foreground"
                      )}
                      title="Cosmos knowledge graph orb view"
                      aria-pressed={viewMode === "cosmos"}
                    >
                      <Orbit className="size-3.5" />
                      <span>Cosmos</span>
                    </button>
                  </div>
                </div>

                {/* Layout Engines */}
                <div className="flex flex-col gap-1.5">
                  <span className="text-2xs text-muted-foreground font-medium uppercase tracking-wider">
                    Layout Engine
                  </span>
                  <div className="grid grid-cols-2 gap-1.5">
                    {LAYOUTS.map((layout) => (
                      <button
                        key={layout.id}
                        type="button"
                        onClick={() => setLayoutEngine(layout.id)}
                        title={layout.title}
                        aria-pressed={layoutEngine === layout.id}
                        className={cn(
                          "flex items-center gap-2 px-2.5 py-1.5 rounded-sm text-xs font-medium transition-colors cursor-pointer border text-left",
                          "focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-focus-ring",
                          layoutEngine === layout.id
                            ? "bg-primary-muted border-primary/40 text-primary font-semibold shadow-2xs"
                            : "bg-muted/30 border-border text-muted-foreground hover:text-foreground hover:bg-muted"
                        )}
                      >
                        {layout.icon}
                        <div className="flex flex-col min-w-0">
                          <span className="truncate">{layout.label}</span>
                        </div>
                      </button>
                    ))}
                  </div>
                </div>

                {/* Physics Simulation */}
                <div className="flex flex-col gap-1.5">
                  <span className="text-2xs text-muted-foreground font-medium uppercase tracking-wider">
                    Live Physics
                  </span>
                  <div className="flex items-center gap-2">
                    <button
                      type="button"
                      onClick={handleTogglePhysics}
                      aria-pressed={livePhysics}
                      className={cn(
                        "flex-1 flex items-center justify-center gap-2 py-1.5 px-3 rounded-sm text-xs font-medium transition-all cursor-pointer border",
                        livePhysics
                          ? "bg-warning/20 text-warning border-warning/40 shadow-[0_0_10px_rgba(245,158,11,0.2)] font-semibold"
                          : "bg-muted/30 border-border text-muted-foreground hover:text-foreground hover:bg-muted"
                      )}
                      title={livePhysics ? "Pause physics simulation" : "Start live dynamic physics simulation"}
                    >
                      <Atom className={cn("size-3.5", livePhysics && isPhysicsRunning && "animate-spin text-warning")} />
                      <span>{livePhysics ? "Physics Active" : "Enable Physics"}</span>
                    </button>

                    {livePhysics && (
                      <Button
                        type="button"
                        variant="ghost"
                        size="sm"
                        className="h-8 px-2.5 text-warning hover:text-warning hover:bg-warning/10 border border-warning/30"
                        onClick={() => reheat(0.55)}
                        title="Shake / Reheat live physics motion"
                        aria-label="Shake simulation"
                      >
                        <Flame className="size-3.5 mr-1" />
                        <span>Reheat</span>
                      </Button>
                    )}
                  </div>
                </div>

                {/* Utilities & Actions */}
                <div className="flex flex-col gap-1.5 pt-1 border-t border-border">
                  <span className="text-2xs text-muted-foreground font-medium uppercase tracking-wider">
                    Actions & Overlays
                  </span>
                  <div className="grid grid-cols-4 gap-1.5">
                    <Button
                      type="button"
                      variant="outline"
                      size="sm"
                      className="h-8 flex flex-col items-center justify-center p-1 gap-0.5 text-2xs cursor-pointer"
                      onClick={handleFitView}
                      title="Fit to view"
                      aria-label="Fit graph to view"
                    >
                      <Maximize2 className="size-3.5" />
                      <span>Fit</span>
                    </Button>

                    <Button
                      type="button"
                      variant="outline"
                      size="sm"
                      className="h-8 flex flex-col items-center justify-center p-1 gap-0.5 text-2xs cursor-pointer"
                      onClick={handleRelayout}
                      title="Re-run layout"
                      aria-label="Re-run layout"
                      disabled={isLayouting || nodes.length === 0}
                    >
                      <RotateCcw
                        className={cn("size-3.5", isLayouting && "animate-spin text-primary")}
                      />
                      <span>Relayout</span>
                    </Button>

                    <Button
                      type="button"
                      variant="outline"
                      size="sm"
                      className={cn(
                        "h-8 flex flex-col items-center justify-center p-1 gap-0.5 text-2xs cursor-pointer",
                        showLegend && "bg-primary-muted border-primary/40 text-primary font-semibold"
                      )}
                      onClick={handleToggleLegend}
                      title={showLegend ? "Hide legend" : "Show legend"}
                      aria-label={showLegend ? "Hide legend" : "Show legend"}
                      aria-pressed={showLegend}
                    >
                      <Eye className="size-3.5" />
                      <span>Legend</span>
                    </Button>

                    <Button
                      type="button"
                      variant="outline"
                      size="sm"
                      className={cn(
                        "h-8 flex flex-col items-center justify-center p-1 gap-0.5 text-2xs cursor-pointer",
                        showMinimap && "bg-primary-muted border-primary/40 text-primary font-semibold"
                      )}
                      onClick={handleToggleMinimap}
                      title={showMinimap ? "Hide minimap" : "Show minimap"}
                      aria-label={showMinimap ? "Hide minimap" : "Show minimap"}
                      aria-pressed={showMinimap}
                    >
                      <MapIcon className="size-3.5" />
                      <span>Minimap</span>
                    </Button>
                  </div>
                </div>
              </div>
            )}
          </div>
        )}
      </div>

      {/* Legend */}
      {showLegend && actions.length > 0 && (
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
              onClick={handleToggleLegend}
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
              <span className="w-4 h-px bg-[var(--purple,#8b5cf6)] shrink-0" />
              <span className="text-muted-foreground">branch origin</span>
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
          onNodeDragStart={onNodeDragStart}
          onNodeDrag={onNodeDrag}
          onNodeDragStop={onNodeDragStop}
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
            className="!shadow-sm !m-0 !bottom-3 !left-3"
          />
          {showMinimap && (
            <MiniMap
              position="bottom-right"
              nodeColor={(n) =>
                n.type === "circleNode"
                  ? "var(--warning)"
                  : n.type === "actionNode"
                  ? "var(--primary)"
                  : "var(--plan)"
              }
              maskColor="color-mix(in oklch, var(--background) 80%, transparent)"
              className="!shadow-sm !m-0 !bottom-3 !right-3"
              style={{
                width: "min(180px, 30vw)",
                height: "min(120px, 20vh)",
              }}
              pannable
              zoomable
            />
          )}
        </ReactFlow>
      </div>
    </div>
  );
};
