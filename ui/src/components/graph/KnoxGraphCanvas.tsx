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
  useNodesInitialized,
} from "@xyflow/react";
import "@xyflow/react/dist/style.css";

import { useKnoxStore } from "@/store/useKnoxStore";
import { ActionNode } from "./ActionNode";
import { FileNode } from "./FileNode";
import { ProjectRootNode } from "./ProjectRootNode";
import { CausalEdge } from "./CausalEdge";
import { CircleNode, type CircleNodeData } from "./CircleNode";
import { GraphToolbars } from "./GraphToolbars";
import { GraphLegend } from "./GraphLegend";
import { GraphPhysicsControls } from "./GraphPhysicsControls";
import { useGraphLayout } from "@/hooks/useGraphLayout";
import { useLivePhysics } from "@/hooks/useLivePhysics";
import { Button } from "@/components/ui/Button";
import { EmptyState } from "@/components/ui/EmptyState";
import {
  Network,
  RotateCcw,
  UploadCloud,
  RefreshCw,
} from "lucide-react";
import type { KnoxAction, Project } from "@/types/knox";

const NODE_DIMENSIONS: Record<string, { width: number; height: number }> = {
  projectRootNode: { width: 288, height: 150 },
  actionNode: { width: 240, height: 130 },
  fileNode: { width: 200, height: 32 },
  circleNode: { width: 80, height: 96 },
};

/**
 * Fixed sizes give React Flow finite bounds before the first measurement pass,
 * which is what the minimap needs; without them its viewBox is NaN.
 */
function withNodeDimensions(list: Node[]): Node[] {
  return list.map((node) => {
    const dims = NODE_DIMENSIONS[node.type ?? ""];
    if (!dims) return node;
    const width = node.width ?? dims.width;
    const height = node.height ?? dims.height;
    // Seed `measured` too: React Flow derives its bounds (and the minimap
    // viewBox) from measured sizes, and NaN bounds reach the SVG before its
    // ResizeObserver has run. These sizes are authoritative for our nodes.
    const measured =
      node.measured?.width === width && node.measured?.height === height
        ? node.measured
        : { width, height };
    return { ...node, width, height, measured };
  });
}

const nodeTypes = {
  projectRootNode: ProjectRootNode,
  actionNode: ActionNode,
  fileNode: FileNode,
  circleNode: CircleNode,
};

const edgeTypes = {
  causalEdge: CausalEdge,
};

const MARKER = {
  follows: { type: MarkerType.ArrowClosed, color: "var(--primary)", width: 16, height: 16 },
  supersedes: { type: MarkerType.ArrowClosed, color: "var(--warning)", width: 16, height: 16 },
  targets: { type: MarkerType.ArrowClosed, color: "var(--plan)", width: 16, height: 16 },
  project: { type: MarkerType.ArrowClosed, color: "var(--info, #38bdf8)", width: 16, height: 16 },
  contains: { type: MarkerType.ArrowClosed, color: "var(--success)", width: 16, height: 16 },
  origin: { type: MarkerType.ArrowClosed, color: "var(--primary)", width: 16, height: 16 },
  depends_on: { type: MarkerType.ArrowClosed, color: "var(--warning)", width: 16, height: 16 },
  implements: { type: MarkerType.ArrowClosed, color: "var(--success)", width: 16, height: 16 },
  verifies: { type: MarkerType.ArrowClosed, color: "var(--plan, #06b6d4)", width: 16, height: 16 },
  relates_to: { type: MarkerType.ArrowClosed, color: "var(--muted-foreground, #94a3b8)", width: 16, height: 16 },
  reverts: { type: MarkerType.ArrowClosed, color: "var(--danger)", width: 16, height: 16 },
} as const;

/**
 * The minimap derives its viewBox from the store's measured node bounds. Before
 * measurement those are NaN, and React Flow writes them straight into the SVG
 * (MNaN,NaN… / viewBox="NaN NaN NaN NaN"). Rendering it only once every node
 * is measured keeps the SVG valid. This component sits inside ReactFlow, so
 * the store hook is available here.
 */
const GraphMiniMap: React.FC<{ visible: boolean }> = ({ visible }) => {
  const initialized = useNodesInitialized();
  if (!visible || !initialized) return null;
  return (
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
      // Numeric sizes only: MiniMap divides its bounding box by these values to
      // derive the viewBox, so a CSS min()/vh string yields NaN for every bound.
      style={{ width: 176, height: 116 }}
      pannable
      zoomable
    />
  );
};
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
  // Store selectors are recreated per render; hold them in refs so the
  // graph rebuild effect only runs when the graph content actually changes.
  const getProjectForActionRef = useRef(getProjectForAction);
  const [isFiltersOpen, setIsFiltersOpen] = useState<boolean>(false);
  const [isLayoutOpen, setIsLayoutOpen] = useState<boolean>(false);
  const toolbarContainerRef = useRef<HTMLDivElement | null>(null);
  // Guards the rebuild effect: React Flow state changes (physics commits,
  // hover, selection) must not re-run the layout and throw positions away.
  const lastBuildSigRef = useRef<string>("")

  useEffect(() => {
    const handleClickOutside = (e: MouseEvent) => {
      if (
        (isFiltersOpen || isLayoutOpen) &&
        toolbarContainerRef.current &&
        !toolbarContainerRef.current.contains(e.target as unknown as globalThis.Node)
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

  useEffect(() => {
    getProjectForActionRef.current = getProjectForAction;
  }, [getProjectForAction]);

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
    return true;
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

  const flowWrapRef = useRef<HTMLDivElement | null>(null);

  const clientToWorld = useCallback(
    (clientX: number, clientY: number) => {
      const instance = rfRef.current;
      const wrap = flowWrapRef.current;
      if (!instance || !wrap) return null;
      const rect = wrap.getBoundingClientRect();
      if (
        clientX < rect.left ||
        clientX > rect.right ||
        clientY < rect.top ||
        clientY > rect.bottom
      ) {
        return null;
      }
      return instance.screenToFlowPosition({ x: clientX, y: clientY });
    },
    []
  );

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
    containerRef: flowWrapRef,
    clientToWorld,
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
      const buildSig = [
        viewMode,
        layoutEngine,
        actorFilter,
        typeFilter,
        currentOnly,
        showFileNodes,
        searchHighlight,
        selectedAction?.id ?? "",
        actions.map((a) => a.id).join(","),
        projects.map((proj) => proj.id).join(","),
      ].join("::");
      if (lastBuildSigRef.current === buildSig) return;

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
        const ownerProj = getProjectForActionRef.current(action);
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
        setNodes(withNodeDimensions(positioned));
        setEdges(rawEdges);
        // Recorded after the commit: a cancelled (StrictMode) first pass must
        // not block the pass that actually renders.
        lastBuildSigRef.current = buildSig;
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
      lastBuildSigRef.current = "";
    };
  }, [
    actions,
    activeProjectScope,
    projects,
    selectedProject?.path,
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
      setNodes(withNodeDimensions(positioned));
      requestAnimationFrame(() =>
        rfRef.current?.fitView({ padding: 0.18, duration: 200 })
      );
    })();
  }, [applyLayout, nodes, edges, layoutEngine, setNodes]);

  const actionNodeCount = nodes.filter((n) => n.type === "actionNode").length;
  const fileNodeCount = nodes.filter((n) => n.type === "fileNode").length;

  return (
    <div className="relative w-full h-full flex overflow-hidden select-none bg-background">
      <GraphToolbars
        toolbarContainerRef={toolbarContainerRef}
        searchContainerRef={searchContainerRef}
        isFiltersOpen={isFiltersOpen}
        setIsFiltersOpen={setIsFiltersOpen}
        activeFilterCount={activeFilterCount}
        handleResetFilters={handleResetFilters}
        actorFilter={actorFilter}
        setActorFilter={setActorFilter}
        availableActors={availableActors}
        typeFilter={typeFilter}
        setTypeFilter={setTypeFilter}
        availableTypes={availableTypes}
        currentOnly={currentOnly}
        setCurrentOnly={setCurrentOnly}
        showFileNodes={showFileNodes}
        setShowFileNodes={setShowFileNodes}
        isLayoutOpen={isLayoutOpen}
        setIsLayoutOpen={setIsLayoutOpen}
        layoutEngine={layoutEngine}
        setLayoutEngine={setLayoutEngine}
        viewMode={viewMode}
        handleToggleViewMode={handleToggleViewMode}
        livePhysics={livePhysics}
        isPhysicsRunning={isPhysicsRunning}
        handleTogglePhysics={handleTogglePhysics}
        reheat={reheat}
        handleFitView={handleFitView}
        handleRelayout={handleRelayout}
        isLayouting={isLayouting}
        nodesLength={nodes.length}
        showLegend={showLegend}
        handleToggleLegend={handleToggleLegend}
        showMinimap={showMinimap}
        handleToggleMinimap={handleToggleMinimap}
        searchHighlight={searchHighlight}
        setSearchHighlight={setSearchHighlight}
        isSearchDropdownOpen={isSearchDropdownOpen}
        setIsSearchDropdownOpen={setIsSearchDropdownOpen}
        activeResultIndex={activeResultIndex}
        setActiveResultIndex={setActiveResultIndex}
        searchMatches={searchMatches}
        handleSelectSearchResult={handleSelectSearchResult}
        isLoading={isLoading}
      />

      <GraphLegend
        showLegend={showLegend}
        onToggleLegend={handleToggleLegend}
        actionNodeCount={actionNodeCount}
        fileNodeCount={fileNodeCount}
        showFileNodes={showFileNodes}
        actionsLength={actions.length}
      />

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

      <div ref={flowWrapRef} className="relative flex-1 w-full h-full">
        <GraphPhysicsControls
          nodesCount={nodes.length}
          livePhysics={livePhysics}
          isPhysicsRunning={isPhysicsRunning}
          onTogglePhysics={handleTogglePhysics}
          onReheat={reheat}
        />
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
          <GraphMiniMap visible={showMinimap && nodes.length > 0} />
        </ReactFlow>
      </div>
    </div>
  );
};
