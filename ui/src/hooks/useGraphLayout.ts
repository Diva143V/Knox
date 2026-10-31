import { useState, useCallback } from "react";
import type { Node, Edge } from "@xyflow/react";
import ELK from "elkjs/lib/elk.bundled.js";
import dagre from "@dagrejs/dagre";
import { forceSimulation, forceLink, forceManyBody, forceCenter } from "d3-force";

export type LayoutEngine = "hierarchical" | "dagre" | "force" | "grid";

const elk = new ELK();

const getNodeDimensions = (type?: string) => {
  if (type === "projectRootNode") return { width: 340, height: 140 };
  if (type === "fileNode") return { width: 220, height: 48 };
  if (type === "circleNode") return { width: 90, height: 90 };
  return { width: 240, height: 130 };
};

export const useGraphLayout = () => {
  const [layoutEngine, setLayoutEngine] = useState<LayoutEngine>("hierarchical");
  const [isLayouting, setIsLayouting] = useState(false);

  const applyLayout = useCallback(
    async (
      nodes: Node[],
      edges: Edge[],
      engineOverride?: LayoutEngine
    ): Promise<Node[]> => {
      const engine = engineOverride || layoutEngine;
      if (nodes.length === 0) return [];

      setIsLayouting(true);
      try {
        if (engine === "hierarchical") {
          // ELK Layered Directed Acyclic Graph Layout
          const elkNodes = nodes.map((n) => {
            const dims = getNodeDimensions(n.type);
            return {
              id: n.id,
              width: dims.width,
              height: dims.height,
            };
          });

          const elkEdges = edges.map((e) => ({
            id: e.id,
            sources: [e.source],
            targets: [e.target],
          }));

          const graph = await elk.layout({
            id: "root",
            layoutOptions: {
              "elk.algorithm": "layered",
              "elk.direction": "DOWN",
              "elk.spacing.nodeNode": "50",
              "elk.layered.spacing.nodeNodeBetweenLayers": "80",
              "elk.layered.spacing.edgeNodeBetweenLayers": "40",
            },
            children: elkNodes,
            edges: elkEdges,
          });

          const posMap = new Map<string, { x: number; y: number }>();
          for (const child of graph.children || []) {
            posMap.set(child.id, { x: child.x || 0, y: child.y || 0 });
          }

          return nodes.map((node) => {
            const pos = posMap.get(node.id);
            return pos
              ? { ...node, position: { x: pos.x + 60, y: pos.y + 60 } }
              : node;
          });
        }

        if (engine === "dagre") {
          // Dagre Hierarchical Layout
          const g = new dagre.graphlib.Graph();
          g.setGraph({ rankdir: "TB", nodesep: 60, ranksep: 90 });
          g.setDefaultEdgeLabel(() => ({}));

          nodes.forEach((n) => {
            const dims = getNodeDimensions(n.type);
            g.setNode(n.id, {
              width: dims.width,
              height: dims.height,
            });
          });

          edges.forEach((e) => {
            g.setEdge(e.source, e.target);
          });

          dagre.layout(g);

          return nodes.map((node) => {
            const nodeWithPos = g.node(node.id);
            const dims = getNodeDimensions(node.type);
            return {
              ...node,
              position: {
                x: (nodeWithPos?.x ?? 0) - dims.width / 2 + 60,
                y: (nodeWithPos?.y ?? 0) - dims.height / 2 + 60,
              },
            };
          });
        }

        if (engine === "force") {
          // D3 Physics Force-Directed Layout
          const simNodes = nodes.map((n) => ({
            id: n.id,
            x: n.position.x || Math.random() * 600,
            y: n.position.y || Math.random() * 600,
          }));

          const simLinks = edges
            .filter(
              (e) =>
                nodes.some((n) => n.id === e.source) &&
                nodes.some((n) => n.id === e.target)
            )
            .map((e) => ({ source: e.source, target: e.target }));

          const sim = forceSimulation(simNodes as any)
            .force(
              "link",
              forceLink(simLinks as any)
                .id((d: any) => d.id)
                .distance(160)
            )
            .force("charge", forceManyBody().strength(-600))
            .force("center", forceCenter(450, 350))
            .stop();

          for (let i = 0; i < 140; i++) sim.tick();

          const simMap = new Map<string, { x: number; y: number }>();
          for (const sn of simNodes) {
            simMap.set(sn.id, { x: sn.x, y: sn.y });
          }

          return nodes.map((node) => {
            const pos = simMap.get(node.id);
            return pos ? { ...node, position: { x: pos.x, y: pos.y } } : node;
          });
        }

        // Default: Compact Grid
        const cols = Math.max(3, Math.ceil(Math.sqrt(nodes.length)));
        const spacingX = 300;
        const spacingY = 180;

        return nodes.map((node, i) => {
          const col = i % cols;
          const row = Math.floor(i / cols);
          return {
            ...node,
            position: { x: col * spacingX + 60, y: row * spacingY + 60 },
          };
        });
      } catch (err) {
        console.warn("Graph layout engine error, falling back to grid:", err);
        const cols = Math.max(3, Math.ceil(Math.sqrt(nodes.length)));
        return nodes.map((node, i) => ({
          ...node,
          position: {
            x: (i % cols) * 300 + 60,
            y: Math.floor(i / cols) * 180 + 60,
          },
        }));
      } finally {
        setIsLayouting(false);
      }
    },
    [layoutEngine]
  );

  return {
    layoutEngine,
    setLayoutEngine,
    isLayouting,
    applyLayout,
  };
};
