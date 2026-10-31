import type { ViewTab } from "./knox";

export type TourVisualType =
  | "welcome-concept"
  | "dashboard-kpi"
  | "graph-cosmos"
  | "query-search"
  | "ingest-dropzone"
  | "scanner-diff"
  | "mcp-bridge"
  | "workspace-shortcuts"
  | "ready-completion";

export interface TourHighlight {
  iconName: string;
  label: string;
  detail: string;
}

export interface TourSlideVisual {
  type: TourVisualType;
  title?: string;
  subtitle?: string;
  badge?: string;
  data?: Record<string, unknown>;
}

export interface TourStepDef {
  id: string;
  tabId?: ViewTab;
  stepNumber: number;
  badge: string;
  badgeVariant?: "default" | "secondary" | "outline" | "success" | "warning" | "plan";
  title: string;
  subtitle: string;
  description: string;
  highlights: TourHighlight[];
  visual: TourSlideVisual;
}

