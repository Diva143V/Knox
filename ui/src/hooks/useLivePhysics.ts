import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import type { Node, Edge } from "@xyflow/react";
import {
  forceCollide,
  forceLink,
  forceManyBody,
  forceSimulation,
  forceX,
  forceY,
  type Force,
  type Simulation,
  type SimulationLinkDatum,
  type SimulationNodeDatum,
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
  /** Element that owns the canvas; used for coordinates and heat handles. */
  containerRef?: React.RefObject<HTMLElement | null>;
  /** Screen → flow coordinates for the pointer repulsion field. */
  clientToWorld?: (clientX: number, clientY: number) => { x: number; y: number } | null;
}

/** Rendered sizes stand in until React Flow has measured a node. */
const FALLBACK_SIZE: Record<string, { w: number; h: number }> = {
  projectRootNode: { w: 288, h: 150 },
  actionNode: { w: 240, h: 130 },
  fileNode: { w: 200, h: 32 },
  circleNode: { w: 80, h: 96 },
};

/**
 * Circular collision radius from the painted box. The old fixed 65/32 was half
 * the real card width, so cards sank into each other and never separated.
 */
function radiusFor(node: Node): number {
  const measured = (
    node as Node & { measured?: { width?: number; height?: number } }
  ).measured;
  const fallback =
    FALLBACK_SIZE[node.type ?? "actionNode"] ?? FALLBACK_SIZE.actionNode;
  const width = measured?.width ?? node.width ?? fallback.w;
  const height = measured?.height ?? node.height ?? fallback.h;
  return (Math.hypot(width, height) / 2) * 0.82;
}

export function useLivePhysics({
  enabled,
  nodes,
  edges,
  setNodes,
  chargeStrength = -380,
  linkDistance = 110,
  collisionPadding = 25,
  containerRef,
  clientToWorld,
}: UseLivePhysicsOptions) {
  const simulationRef = useRef<Simulation<PhysicsNode, PhysicsLink> | null>(null);
  const simNodesRef = useRef<Map<string, PhysicsNode>>(new Map());
  const nodesRef = useRef<Node[]>(nodes);
  const draggingRef = useRef<Set<string>>(new Set());
  const heatRef = useRef<Map<string, number>>(new Map());
  const elementRef = useRef<Map<string, HTMLElement>>(new Map());
  const pointerRef = useRef({ x: 0, y: 0, active: false });
  const lastCommitRef = useRef(0);
  const [isRunning, setIsRunning] = useState(false);

  useEffect(() => {
    nodesRef.current = nodes;
  }, [nodes]);

  const reheat = useCallback((alpha = 0.45) => {
    const simulation = simulationRef.current;
    if (!simulation) return;
    simulation.alpha(Math.max(simulation.alpha(), alpha)).restart();
    setIsRunning(true);
  }, []);

  /** Clear every heat ring once the graph comes to rest. */
  const coolDown = useCallback(() => {
    for (const element of elementRef.current.values()) {
      element.style.setProperty("--knox-heat", "0");
    }
    heatRef.current.clear();
  }, []);

  // Only structural changes rebuild the simulation; moving positions must not.
  const structureKey = useMemo(
    () =>
      `${nodes.map((n) => n.id).join("|")}::${edges.map((e) => e.id).join("|")}`,
    [nodes, edges]
  );

  useEffect(() => {
    if (!enabled || nodes.length === 0) {
      simulationRef.current?.stop();
      simulationRef.current = null;
      setIsRunning(false);
      return;
    }

    const simNodesMap = simNodesRef.current;
    const activeIds = new Set(nodes.map((n) => n.id));

    const simNodes: PhysicsNode[] = nodes.map((node) => {
      const radius = radiusFor(node);
      const existing = simNodesMap.get(node.id);
      if (existing) {
        existing.radius = radius;
        return existing;
      }
      const created: PhysicsNode = {
        id: node.id,
        x: node.position.x,
        y: node.position.y,
        vx: 0,
        vy: 0,
        radius,
      };
      simNodesMap.set(node.id, created);
      return created;
    });

    for (const id of Array.from(simNodesMap.keys())) {
      if (!activeIds.has(id)) simNodesMap.delete(id);
    }

    const links: PhysicsLink[] = edges
      .filter(
        (edge) =>
          activeIds.has(String(edge.source)) && activeIds.has(String(edge.target))
      )
      .map((edge) => ({
        source: String(edge.source),
        target: String(edge.target),
      }));

    // Weak gravity toward the centroid keeps disconnected islands from drifting
    // away; forceCenter alone cannot hold separate components together.
    const centroidX =
      simNodes.reduce((sum, n) => sum + n.x, 0) / (simNodes.length || 1);
    const centroidY =
      simNodes.reduce((sum, n) => sum + n.y, 0) / (simNodes.length || 1);

    simulationRef.current?.stop();
    simulationRef.current = null;

    let pointerNodes: PhysicsNode[] = [];
    const pointerForce = ((alpha: number) => {
      const pointer = pointerRef.current;
      if (!pointer.active) return;
      const reach = 190;
      for (const node of pointerNodes) {
        const dx = (node.x ?? 0) - pointer.x;
        const dy = (node.y ?? 0) - pointer.y;
        const distanceSquared = dx * dx + dy * dy;
        if (distanceSquared > reach * reach || distanceSquared < 1) continue;
        const distance = Math.sqrt(distanceSquared);
        const falloff = 1 - distance / reach;
        const push = falloff * falloff * 24 * alpha;
        node.vx = (node.vx ?? 0) + (dx / distance) * push;
        node.vy = (node.vy ?? 0) + (dy / distance) * push;
      }
    }) as Force<PhysicsNode, PhysicsLink>;
    pointerForce.initialize = (initial: PhysicsNode[]) => {
      pointerNodes = initial;
    };

    const simulation = forceSimulation<PhysicsNode>(simNodes)
      .force(
        "charge",
        forceManyBody<PhysicsNode>().strength(chargeStrength).distanceMax(900)
      )
      .force(
        "link",
        forceLink<PhysicsNode, PhysicsLink>(links)
          .id((d) => d.id)
          .distance(linkDistance)
          .strength(0.32)
      )
      .force(
        "collide",
        forceCollide<PhysicsNode>((d) => d.radius + collisionPadding)
          .iterations(2)
          .strength(0.9)
      )
      .force("x", forceX<PhysicsNode>(centroidX).strength(0.035))
      .force("y", forceY<PhysicsNode>(centroidY).strength(0.035))
      .force("pointer", pointerForce)
      .velocityDecay(0.42)
      .alphaMin(0.002)
      .alphaDecay(0.024)
      .alpha(0.55);

    simulationRef.current = simulation;
    elementRef.current.clear();
    lastCommitRef.current = 0;
    setIsRunning(true);

    /**
     * Commit from the simulation tick (time-gated to ~60fps) instead of a rAF
     * loop: frames can be throttled while the window is busy or occluded, which
     * used to freeze the graph mid-motion with no way to recover.
     */
    const commit = (force = false) => {
      const now = performance.now();
      if (!force && now - lastCommitRef.current < 12) return;
      lastCommitRef.current = now;

      const dragging = draggingRef.current;

      setNodes((previous) => {
        let changed = false;
        const next = previous.map((node) => {
          if (dragging.has(node.id)) return node;
          const simNode = simNodesMap.get(node.id);
          if (!simNode || simNode.x == null || simNode.y == null) return node;
          if (
            Math.abs(simNode.x - node.position.x) < 0.15 &&
            Math.abs(simNode.y - node.position.y) < 0.15
          ) {
            return node;
          }
          changed = true;
          return { ...node, position: { x: simNode.x, y: simNode.y } };
        });
        // Same reference when nothing moved: React Flow skips the whole pass.
        return changed ? next : previous;
      });

      const wrapper = containerRef?.current;
      if (!wrapper) return;

      for (const node of nodesRef.current) {
        const simNode = simNodesMap.get(node.id);
        if (!simNode) continue;
        const speed = Math.hypot(simNode.vx ?? 0, simNode.vy ?? 0);
        const heat = Math.round(Math.min(speed / 9, 1) * 20) / 20;
        if ((heatRef.current.get(node.id) ?? 0) === heat) continue;
        heatRef.current.set(node.id, heat);

        let element = elementRef.current.get(node.id);
        if (!element) {
          const escaped = node.id.replace(/["\\]/g, "\\$&");
          element =
            wrapper.querySelector<HTMLElement>(
              `.react-flow__node[data-id="${escaped}"]`
            ) ?? undefined;
          if (element) elementRef.current.set(node.id, element);
        }
        element?.style.setProperty("--knox-heat", String(heat));
      }
    };

    simulation.on("tick", () => commit());
    simulation.on("end", () => {
      commit(true);
      coolDown();
      setIsRunning(false);
    });

    return () => {
      simulation.stop();
      simulation.on("tick", null);
      simulation.on("end", null);
      simulationRef.current = null;
      setIsRunning(false);
    };
  }, [
    enabled,
    structureKey,
    nodes.length,
    chargeStrength,
    linkDistance,
    collisionPadding,
    setNodes,
    containerRef,
    coolDown,
  ]);

  // Pointer repulsion: stirring the field wakes the simulation without a click.
  useEffect(() => {
    if (!enabled || !clientToWorld) return;
    const element = containerRef?.current;
    if (!element) return;

    let lastX = 0;
    let lastY = 0;

    const handleMove = (event: PointerEvent) => {
      const world = clientToWorld(event.clientX, event.clientY);
      if (!world) {
        pointerRef.current.active = false;
        return;
      }
      pointerRef.current = { x: world.x, y: world.y, active: true };

      if (Math.hypot(event.clientX - lastX, event.clientY - lastY) > 12) {
        lastX = event.clientX;
        lastY = event.clientY;
        const simulation = simulationRef.current;
        if (simulation && simulation.alpha() < 0.12) {
          simulation.alpha(0.12).restart();
          setIsRunning(true);
        }
      }
    };

    const handleLeave = () => {
      pointerRef.current.active = false;
    };

    element.addEventListener("pointermove", handleMove);
    element.addEventListener("pointerleave", handleLeave);
    return () => {
      element.removeEventListener("pointermove", handleMove);
      element.removeEventListener("pointerleave", handleLeave);
      pointerRef.current.active = false;
    };
  }, [enabled, containerRef, clientToWorld]);

  const onNodeDragStart = useCallback(
    (_: unknown, node: Node) => {
      if (!enabled) return;
      const simulation = simulationRef.current;
      const simNode = simNodesRef.current.get(node.id);
      if (!simulation || !simNode) return;
      draggingRef.current.add(node.id);
      simNode.fx = node.position.x;
      simNode.fy = node.position.y;
      simulation.alphaTarget(0.12).restart();
      setIsRunning(true);
    },
    [enabled]
  );

  const onNodeDrag = useCallback(
    (_: unknown, node: Node) => {
      if (!enabled) return;
      const simNode = simNodesRef.current.get(node.id);
      if (!simNode) return;
      simNode.fx = node.position.x;
      simNode.fy = node.position.y;
    },
    [enabled]
  );

  const onNodeDragStop = useCallback(
    (_: unknown, node: Node) => {
      if (!enabled) return;
      const simulation = simulationRef.current;
      const simNode = simNodesRef.current.get(node.id);
      if (!simNode) return;

      draggingRef.current.delete(node.id);
      simNode.fx = null;
      simNode.fy = null;
      // Hand the dropped position back so it never snaps to a stale coordinate.
      simNode.x = node.position.x;
      simNode.y = node.position.y;
      simNode.vx = 0;
      simNode.vy = 0;
      simulation?.alphaTarget(0);
      reheat(0.18);
    },
    [enabled, reheat]
  );

  return {
    isRunning,
    reheat,
    onNodeDragStart,
    onNodeDrag,
    onNodeDragStop,
  };
}
