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
import { ActionBadge } from "@/components/ui/ActionBadge";
import { ActorBadge } from "@/components/ui/ActorBadge";
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
  X,
  Tag,
  Zap,
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
  const [isSearchDropdownOpen, setIsSearchDropdownOpen] = useState<boolean>(false);
  const [activeResultIndex, setActiveResultIndex] = useState<number>(-1);
  const searchContainerRef = useRef<HTMLDivElement | null>(null);
  const hasInitialFitView = useRef<boolean>(false);
  const [showFileNodes, setShowFileNodes] = useState<boolean>(true);
  const [showLegend, setShowLegend] = useState<boolean>(true);
  const rfRef = useRef<ReactFlowInstance | null>(null);

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

  useEffect(() => {
    const handleClickOutside = (e: MouseEvent) => {
      if (
        searchContainerRef.current &&
        !searchContainerRef.current.contains(e.target as unknown as globalThis.Node)
      ) {
        setIsSearchDropdownOpen(false);
      }
    };
    document.addEventListener("mousedown", handleClickOutside);
    return () => document.removeEventListener("mousedown", handleClickOutside);
  }, []);

  const searchMatches = useMemo(() => {
    const q = searchHighlight.trim().toLowerCase();
    if (!q) return [];
    return actions
      .map((action) => {
        let score = 0;
        let matchField: "summary" | "symbol" | "file" | "tag" | "actor" | "type" | "rationale" = "summary";
        let matchDetail = "";

        if (action.summary.toLowerCase().includes(q)) {
          score += 10;
          matchField = "summary";
        }
        if (action.symbols?.some((s) => s.toLowerCase().includes(q))) {
          score += 8;
          if (score <= 10) matchField = "symbol";
          matchDetail = action.symbols.find((s) => s.toLowerCase().includes(q)) || "";
        }
        if (action.files?.some((f) => f.toLowerCase().includes(q))) {
          score += 6;
          if (score <= 10) matchField = "file";
          matchDetail = action.files.find((f) => f.toLowerCase().includes(q)) || "";
        }
        if (action.tags?.some((t) => t.toLowerCase().includes(q))) {
          score += 4;
          if (score <= 10) matchField = "tag";
          matchDetail = action.tags.find((t) => t.toLowerCase().includes(q)) || "";
        }
        if (action.actor.toLowerCase().includes(q)) {
          score += 3;
          if (score <= 10) matchField = "actor";
        }
        if (action.action_type.toLowerCase().includes(q)) {
          score += 2;
          if (score <= 10) matchField = "type";
        }
        if (action.rationale?.toLowerCase().includes(q)) {
          score += 1;
          if (score <= 10) matchField = "rationale";
        }

        return { action, score, matchField, matchDetail };
      })
      .filter((m) => m.score > 0)
      .sort((a, b) => b.score - a.score)
      .slice(0, 8);
  }, [actions, searchHighlight]);

  const handleSelectSearchResult = useCallback(
    (action: KnoxAction) => {
      if (actorFilter !== "all" && action.actor.toLowerCase() !== actorFilter.toLowerCase()) {
        setActorFilter("all");
      }
      if (typeFilter !== "all" && action.action_type.toLowerCase() !== typeFilter.toLowerCase()) {
        setTypeFilter("all");
      }
      if (currentOnly && !action.is_current) {
        setCurrentOnly(false);
      }

      setSelectedAction(action);
      setIsSearchDropdownOpen(false);

      requestAnimationFrame(() => {
        const targetNode = rfRef.current?.getNode(action.id);
        if (targetNode) {
          const width = targetNode.measured?.width ?? 240;
          const height = targetNode.measured?.height ?? 120;
          rfRef.current?.setCenter(
            targetNode.position.x + width / 2,
            targetNode.position.y + height / 2,
            { zoom: 1.35, duration: 600 }
          );
        }
      });
    },
    [actorFilter, typeFilter, currentOnly, setCurrentOnly, setSelectedAction]
  );

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
              action.action_type.toLowerCase().includes(lowerSearch) ||
              (action.files &&
                action.files.some((f) => f.toLowerCase().includes(lowerSearch))) ||
              (action.tags &&
                action.tags.some((t) => t.toLowerCase().includes(lowerSearch))) ||
              (action.symbols &&
                action.symbols.some((s) => s.toLowerCase().includes(lowerSearch))) ||
              (action.rationale &&
                action.rationale.toLowerCase().includes(lowerSearch)));

          const isDimmed = lowerSearch.length > 0 && !isHighlighted;

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
                isDimmed,
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
                isDimmed,
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
        if (!hasInitialFitView.current) {
          requestAnimationFrame(() => {
            if (!isCancelled) {
              rfRef.current?.fitView({ padding: 0.18, duration: 250 });
              hasInitialFitView.current = true;
            }
          });
        }
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

        <div ref={searchContainerRef} className="relative">
          <Search
            className="size-3.5 text-muted-foreground absolute left-2.5 top-1/2 -translate-y-1/2 pointer-events-none"
            aria-hidden
          />
          <Input
            type="text"
            value={searchHighlight}
            onChange={(e) => {
              setSearchHighlight(e.target.value);
              setIsSearchDropdownOpen(true);
              setActiveResultIndex(0);
            }}
            onFocus={() => {
              if (searchHighlight.trim().length > 0) {
                setIsSearchDropdownOpen(true);
              }
            }}
            onKeyDown={(e) => {
              if (e.key === "ArrowDown") {
                e.preventDefault();
                setIsSearchDropdownOpen(true);
                setActiveResultIndex((prev) =>
                  prev < searchMatches.length - 1 ? prev + 1 : 0
                );
              } else if (e.key === "ArrowUp") {
                e.preventDefault();
                setIsSearchDropdownOpen(true);
                setActiveResultIndex((prev) =>
                  prev > 0 ? prev - 1 : searchMatches.length - 1
                );
              } else if (e.key === "Enter") {
                e.preventDefault();
                if (searchMatches.length > 0) {
                  const target =
                    activeResultIndex >= 0 && activeResultIndex < searchMatches.length
                      ? searchMatches[activeResultIndex].action
                      : searchMatches[0].action;
                  handleSelectSearchResult(target);
                }
              } else if (e.key === "Escape") {
                setIsSearchDropdownOpen(false);
              }
            }}
            placeholder="Search nodes, files, symbols…"
            aria-label="Search and select matching nodes"
            className="pl-8 pr-7 h-7 w-44 sm:w-64 text-xs font-mono transition-all duration-200 focus:w-64 sm:focus:w-80"
          />
          {searchHighlight && (
            <button
              type="button"
              onClick={() => {
                setSearchHighlight("");
                setIsSearchDropdownOpen(false);
              }}
              className="absolute right-2 top-1/2 -translate-y-1/2 text-muted-foreground hover:text-foreground cursor-pointer p-0.5 rounded-sm hover:bg-muted transition-colors"
              title="Clear search"
            >
              <X className="size-3" />
            </button>
          )}

          {/* Interactive Search Results Dropdown */}
          {isSearchDropdownOpen && searchMatches.length > 0 && (
            <div className="absolute top-full left-0 mt-1.5 w-80 sm:w-96 max-h-80 overflow-y-auto bg-card/95 border border-border/80 rounded-lg shadow-2xl backdrop-blur-md z-50 py-1 divide-y divide-border/40 animate-in fade-in zoom-in-95 duration-150">
              <div className="px-2.5 py-1 text-[11px] font-mono text-muted-foreground flex items-center justify-between">
                <span>Matching Nodes ({searchMatches.length})</span>
                <span className="text-[10px]">Use ↑↓ to navigate • Enter to jump</span>
              </div>
              <div className="py-0.5">
                {searchMatches.map((match, idx) => {
                  const isSelected = idx === activeResultIndex;
                  return (
                    <button
                      key={match.action.id}
                      type="button"
                      onMouseEnter={() => setActiveResultIndex(idx)}
                      onClick={() => handleSelectSearchResult(match.action)}
                      className={cn(
                        "w-full text-left px-3 py-2 flex flex-col gap-1 transition-colors cursor-pointer group",
                        isSelected
                          ? "bg-primary/10 text-foreground"
                          : "hover:bg-muted/60 text-muted-foreground hover:text-foreground"
                      )}
                    >
                      <div className="flex items-center gap-1.5 flex-wrap">
                        <ActionBadge
                          type={match.action.action_type}
                          className="text-[10px] px-1 py-0"
                        />
                        <ActorBadge
                          actor={match.action.actor}
                          className="text-[10px] px-1 py-0"
                        />
                        {match.action.significance === "major" && (
                          <span className="text-[9px] font-mono uppercase bg-warning/15 text-warning px-1 rounded-xs border border-warning/30">
                            ADR
                          </span>
                        )}
                        {match.matchField !== "summary" && match.matchDetail && (
                          <span className="inline-flex items-center gap-1 text-[10px] font-mono bg-background/80 text-primary border border-border px-1.5 py-0.5 rounded-xs ml-auto truncate max-w-[140px]">
                            {match.matchField === "file" && <FileCode className="size-2.5 shrink-0" />}
                            {match.matchField === "symbol" && <Zap className="size-2.5 shrink-0" />}
                            {match.matchField === "tag" && <Tag className="size-2.5 shrink-0" />}
                            <span className="truncate">{match.matchDetail}</span>
                          </span>
                        )}
                      </div>
                      <p className="text-xs line-clamp-2 font-medium text-foreground group-hover:text-primary transition-colors">
                        {match.action.summary}
                      </p>
                    </button>
                  );
                })}
              </div>
            </div>
          )}
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
          "absolute top-3 right-3 z-20 flex flex-wrap items-center gap-1",
          "bg-card border border-border p-1 rounded-md shadow-sm"
        )}
        role="toolbar"
        aria-label="Layout and view controls"
      >
        {/* View Mode Toggle: Cards vs Cosmos */}
        <div className="flex items-center bg-muted/60 p-0.5 rounded-sm border border-border mr-0.5">
          <button
            type="button"
            onClick={() => handleToggleViewMode("cards")}
            className={cn(
              "flex items-center gap-1 px-2 py-0.5 rounded-xs text-xs font-medium transition-colors cursor-pointer",
              viewMode === "cards"
                ? "bg-card text-foreground shadow-xs font-semibold"
                : "text-muted-foreground hover:text-foreground"
            )}
            title="Card nodes view"
          >
            <LayoutDashboard className="size-3 text-primary" />
            <span className="hidden sm:inline">Cards</span>
          </button>
          <button
            type="button"
            onClick={() => handleToggleViewMode("cosmos")}
            className={cn(
              "flex items-center gap-1 px-2 py-0.5 rounded-xs text-xs font-medium transition-colors cursor-pointer",
              viewMode === "cosmos"
                ? "bg-primary text-primary-foreground shadow-xs font-semibold"
                : "text-muted-foreground hover:text-foreground"
            )}
            title="Cosmos knowledge graph orb view"
          >
            <Orbit className="size-3" />
            <span className="hidden sm:inline">Cosmos</span>
          </button>
        </div>

        {/* Live Physics Toggle */}
        <button
          type="button"
          onClick={handleTogglePhysics}
          aria-pressed={livePhysics}
          className={cn(
            "flex items-center gap-1 px-2 py-1 rounded-sm text-xs font-medium transition-all cursor-pointer mr-0.5",
            livePhysics
              ? "bg-warning/20 text-warning border border-warning/40 shadow-[0_0_10px_rgba(245,158,11,0.2)] font-semibold"
              : "text-muted-foreground hover:text-foreground hover:bg-muted"
          )}
          title={livePhysics ? "Pause live physics simulation" : "Start live dynamic physics simulation"}
        >
          <Atom className={cn("size-3.5", livePhysics && isPhysicsRunning && "animate-spin text-warning")} />
          <span className="hidden sm:inline">Physics</span>
        </button>

        {livePhysics && (
          <Button
            type="button"
            variant="ghost"
            size="icon"
            className="size-7 text-warning hover:text-warning hover:bg-warning/10"
            onClick={() => reheat(0.55)}
            title="Shake / Reheat live physics motion"
            aria-label="Shake simulation"
          >
            <Flame className="size-3.5" />
          </Button>
        )}

        <div className="w-px h-4 bg-border mx-0.5" aria-hidden />

        {LAYOUTS.map((layout) => (
          <button
            key={layout.id}
            type="button"
            onClick={() => setLayoutEngine(layout.id)}
            title={layout.title}
            aria-pressed={layoutEngine === layout.id}
            className={cn(
              "flex items-center gap-1 px-2 py-1 rounded-sm text-xs font-medium transition-colors cursor-pointer",
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
            className="!shadow-sm"
          />
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
            className="!shadow-sm"
            pannable
            zoomable
          />
        </ReactFlow>
      </div>
    </div>
  );
};
