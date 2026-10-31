import type { ViewTab } from "@/types/knox";

export interface NavItemDef {
  id: ViewTab;
  /** Compact sidebar label */
  label: string;
  /** Full name for tooltips / header / palette */
  fullLabel: string;
  description: string;
  shortcut: string;
  /** Lucide icon name key used by consumers */
  icon:
    | "dashboard"
    | "graph"
    | "query"
    | "stats"
    | "ingest"
    | "scanner"
    | "mcp"
    | "workspace";
}

export interface NavGroupDef {
  id: "explore" | "capture" | "connect";
  label: string;
  items: NavItemDef[];
}

/** Canonical navigation — route IDs and shortcuts unchanged. */
export const NAV_GROUPS: NavGroupDef[] = [
  {
    id: "explore",
    label: "Explore",
    items: [
      {
        id: "dashboard",
        label: "Dashboard",
        fullLabel: "Dashboard",
        description: "System overview, KPIs, and recent activity",
        shortcut: "1",
        icon: "dashboard",
      },
      {
        id: "graph",
        label: "Graph",
        fullLabel: "Knowledge Graph",
        description: "Interactive causal graph of decisions and writes",
        shortcut: "2",
        icon: "graph",
      },
      {
        id: "query",
        label: "Search",
        fullLabel: "Query Console",
        description: "FTS5 search and structured multi-filter lookup",
        shortcut: "3",
        icon: "query",
      },
      {
        id: "stats",
        label: "Statistics",
        fullLabel: "Metrics & Statistics",
        description: "Actor and type distributions with 7-day trend",
        shortcut: "4",
        icon: "stats",
      },
    ],
  },
  {
    id: "capture",
    label: "Capture",
    items: [
      {
        id: "ingest",
        label: "Ingest",
        fullLabel: "Ingest Transcripts",
        description: "Import session transcripts into the knowledge graph",
        shortcut: "5",
        icon: "ingest",
      },
      {
        id: "scanner",
        label: "Scan",
        fullLabel: "Human Edit Scan",
        description: "Detect staged human edits and manage git hooks",
        shortcut: "6",
        icon: "scanner",
      },
    ],
  },
  {
    id: "connect",
    label: "Connect",
    items: [
      {
        id: "mcp",
        label: "MCP",
        fullLabel: "MCP Configuration",
        description: "Model Context Protocol tools and editor snippets",
        shortcut: "7",
        icon: "mcp",
      },
      {
        id: "workspace",
        label: "Workspace",
        fullLabel: "Workspace",
        description: "Initialize knox.md and register repository hooks",
        shortcut: "8",
        icon: "workspace",
      },
    ],
  },
];

export const ALL_NAV_ITEMS: NavItemDef[] = NAV_GROUPS.flatMap((g) => g.items);

export const VIEW_BY_ID: Record<ViewTab, NavItemDef> = ALL_NAV_ITEMS.reduce(
  (acc, item) => {
    acc[item.id] = item;
    return acc;
  },
  {} as Record<ViewTab, NavItemDef>
);

export const APP_VERSION = "1.2.1";
