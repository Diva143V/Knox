import { useEffect, useRef, useState, useCallback } from "react";
import type { Node, Edge } from "@xyflow/react";
import {
  forceSimulation,
  forceLink,
  forceManyBody,
  forceCollide,
  forceCenter,
  type Simulation,
  type SimulationNodeDatum,
  type SimulationLinkDatum,
} from "d3-force";

export interface PhysicsNode extends SimulationNodeDatum {
  id: string;
  x: number;
  y: number;
  radius: number;
}

export interface PhysicsLink extends SimulationLinkDatum<PhysicsNode> {
  source: string | PhysicsNode;
  target: string | PhysicsNode;
}

interface UseLivePhysicsOptions {
  enabled: boolean;
  nodes: Node[];
  edges: Edge[];
  setNodes: React.Dispatch<React.SetStateAction<Node[]>>;
  chargeStrength?: number;
  linkDistance?: number;
  collisionPadding?: number;
}

export function useLivePhysics({
  enabled,
  nodes,
  edges,
  setNodes,
  chargeStrength = -380,
  linkDistance = 110,
  collisionPadding = 25,
}: UseLivePhysicsOptions) {
  const simulationRef = useRef<Simulation<PhysicsNode, PhysicsLink> | null>(null);
  const simNodesRef = useRef<Map<string, PhysicsNode>>(new Map());
  const rafRef = useRef<number | null>(null);
  const [isRunning, setIsRunning] = useState(false);

  // Stop simulation cleanly
  const stopSimulation = useCallback(() => {
    if (simulationRef.current) {
      simulationRef.current.stop();
      simulationRef.current = null;
    }
    if (rafRef.current) {
      cancelAnimationFrame(rafRef.current);
      rafRef.current = null;
    }
    setIsRunning(false);
  }, []);

  // Reheat / restart motion
  const reheat = useCallback((alpha = 0.45) => {
    if (simulationRef.current) {
      simulationRef.current.alpha(alpha).restart();
      setIsRunning(true);
    }
  }, []);

  // Initialize or update simulation when enabled or structure changes
  useEffect(() => {
    if (!enabled || nodes.length === 0) {
      stopSimulation();
      return;
    }

    // Preserve existing node positions to prevent visual snapping
    const simNodesMap = simNodesRef.current;
    const simNodes: PhysicsNode[] = nodes.map((n) => {
      const existing = simNodesMap.get(n.id);
      const radius = n.type === "circleNode" ? 32 : 65;
      const node: PhysicsNode = {
        id: n.id,
        x: existing?.x ?? n.position.x ?? Math.random() * 500,
        y: existing?.y ?? n.position.y ?? Math.random() * 500,
        vx: existing?.vx ?? 0,
        vy: existing?.vy ?? 0,
        radius,
      };
      simNodesMap.set(n.id, node);
      return node;
    });

    // Remove deleted nodes from map
    const activeIds = new Set(nodes.map((n) => n.id));
    for (const id of simNodesMap.keys()) {
      if (!activeIds.has(id)) {
        simNodesMap.delete(id);
      }
    }

    // Compute bounding center
    const avgX =
      simNodes.reduce((acc, n) => acc + (n.x || 0), 0) / (simNodes.length || 1);
    const avgY =
      simNodes.reduce((acc, n) => acc + (n.y || 0), 0) / (simNodes.length || 1);

    const validLinks: PhysicsLink[] = edges
      .filter((e) => activeIds.has(e.source) && activeIds.has(e.target))
      .map((e) => ({
        source: e.source,
        target: e.target,
      }));

    // Create and configure D3 simulation
    const sim = forceSimulation<PhysicsNode>(simNodes)
      .force(
        "charge",
        forceManyBody<PhysicsNode>()
          .strength(chargeStrength)
          .distanceMax(650)
      )
      .force(
        "link",
        forceLink<PhysicsNode, PhysicsLink>(validLinks)
          .id((d) => d.id)
          .distance(linkDistance)
          .strength(0.35)
      )
      .force(
        "collide",
        forceCollide<PhysicsNode>((d) => d.radius + collisionPadding).iterations(2)
      )
      .force("center", forceCenter<PhysicsNode>(avgX || 400, avgY || 350))
      .velocityDecay(0.4)
      .alpha(0.6)
      .alphaDecay(0.015);

    let isUpdating = false;

    sim.on("tick", () => {
      if (isUpdating) return;
      isUpdating = true;

      rafRef.current = requestAnimationFrame(() => {
        isUpdating = false;
        setNodes((currentNodes) =>
          currentNodes.map((cn) => {
            const sn = simNodesMap.get(cn.id);
            if (!sn) return cn;
            const dx = Math.abs(cn.position.x - sn.x);
            const dy = Math.abs(cn.position.y - sn.y);
            if (dx < 0.25 && dy < 0.25) return cn;
            return {
              ...cn,
              position: { x: sn.x, y: sn.y },
            };
          })
        );
      });
    });

    sim.on("end", () => {
      setIsRunning(false);
    });

    simulationRef.current = sim;
    setIsRunning(true);

    return () => {
      sim.stop();
      if (rafRef.current) cancelAnimationFrame(rafRef.current);
    };
  }, [
    enabled,
    nodes.length,
    edges.length,
    chargeStrength,
    linkDistance,
    collisionPadding,
    setNodes,
    stopSimulation,
  ]);

  // Drag interaction handlers to allow smooth tactile grabbing & shaking
  const onNodeDragStart = useCallback(
    (_: unknown, node: Node) => {
      if (!enabled || !simulationRef.current) return;
      const simNode = simNodesRef.current.get(node.id);
      if (simNode) {
        simNode.fx = simNode.x;
        simNode.fy = simNode.y;
        simulationRef.current.alphaTarget(0.2).restart();
        setIsRunning(true);
      }
    },
    [enabled]
  );

  const onNodeDrag = useCallback(
    (_: unknown, node: Node) => {
      if (!enabled || !simulationRef.current) return;
      const simNode = simNodesRef.current.get(node.id);
      if (simNode) {
        simNode.fx = node.position.x;
        simNode.fy = node.position.y;
      }
    },
    [enabled]
  );

  const onNodeDragStop = useCallback(
    (_: unknown, node: Node) => {
      if (!enabled || !simulationRef.current) return;
      const simNode = simNodesRef.current.get(node.id);
      if (simNode) {
        simNode.fx = null;
        simNode.fy = null;
        simulationRef.current.alphaTarget(0);
      }
    },
    [enabled]
  );

  return {
    isRunning,
    reheat,
    onNodeDragStart,
    onNodeDrag,
    onNodeDragStop,
  };
}
