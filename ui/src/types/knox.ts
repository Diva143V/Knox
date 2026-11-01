//! TypeScript type definitions mirroring Knox Rust domain types.

export interface ActionRelation {
  target_id: string;
  relation_type: string;
}

export interface KnoxAction {
  id: string;
  actor: string;
  action_type: string;
  summary: string;
  files: string[];
  tags: string[];
  confidence: number | null;
  timestamp: string;
  session_id: string | null;
  parent_action_id: string | null;
  supersedes: string | null;
  is_current: boolean;
  // V2 Knowledge Graph Fields
  relations?: ActionRelation[];
  symbols?: string[];
  rationale?: string | null;
  rejected_alternatives?: string[];
  context_summary?: string | null;
  // V3 Cognitive Memory & Doubts Fields
  significance?: "major" | "minor" | string;
  is_resolved?: boolean;
  resolution_note?: string | null;
}

export interface Session {
  id: string;
  actor: string;
  project: string | null;
  started_at: string;
  ended_at: string | null;
  is_open: boolean;
}

export interface Project {
  id: string;
  name: string;
  path: string;
  created_at: string;
}

export interface StatusReport {
  total_actions: number;
  total_sessions: number;
  projects_tracked: number;
  last_action_timestamp: string | null;
  actions_by_actor: Record<string, number>;
  actions_by_type: Record<string, number>;
}

export interface QueryResult {
  results: KnoxAction[];
  total_count: number;
  has_more: boolean;
}

export interface ScanReport {
  human_edits_detected: number;
  actions_recorded: number;
  files_scanned: string[];
  diff_stat: string | null;
}

export interface IngestReport {
  actions_imported: number;
  files_touched: number;
  sessions_created: number;
  duplicates_skipped?: number;
  tool_detected?: string;
}

export interface IngestPreview {
  tool_detected: string;
  total_actions: number;
  plans_count: number;
  writes_count: number;
  debugs_count: number;
  decisions_count: number;
  files_touched: string[];
  duplicates_count: number;
  sample_actions: KnoxAction[];
}

export interface DetectedTranscript {
  path: string;
  source: string;
  size_bytes: number;
  last_modified: string;
  estimated_actions: number;
}

export interface KnoxConfig {
  kg_path: string;
  session_timeout_seconds: number;
  global_mode: boolean;
}

export interface DbStats {
  db_size_bytes: number;
  wal_size_bytes: number;
  page_count: number;
  page_size: number;
  total_actions: number;
  total_sessions: number;
  total_relations: number;
  schema_version: number;
}

export type SettingsTabId =
  | "general"
  | "storage"
  | "graph"
  | "mcp"
  | "scanner"
  | "ingestion"
  | "system"
  | "shortcuts"
  | "about";

export interface UserSettings {
  // Appearance
  theme: "dark" | "light" | "system";
  uiDensity: "comfortable" | "compact";
  enableGlowFx: boolean;
  defaultStartupView: ViewTab;

  // Graph Engine
  defaultGraphMode: "cards" | "cosmos";
  defaultLivePhysics: boolean;
  defaultLayoutEngine: "hierarchical" | "dagre" | "force" | "grid";
  showFileNodes: boolean;
  showLegend: boolean;
  showEdgeLabels: boolean;
  chargeStrength: number;
  linkDistance: number;
  collisionPadding: number;

  // AI Agents & MCP
  mcpCommandMode: "system" | "absolute";
  guardrailsMode: "strict" | "advisory" | "off";
  enableMarkdownDigest: boolean;

  // Git & Scanner
  defaultAuthorName: string;
  scannerIgnorePatterns: string[];
  strictGitHook: boolean;
  diffScanDepth: number;

  // Ingestion
  ingestWatchPaths: string[];
  deduplicationMode: "skip" | "supersede";
  requireIngestPreview: boolean;

  // System & Shell
  closeToTray: boolean;
  startOnBoot: boolean;
  showDesktopNotifications: boolean;
}

export const DEFAULT_USER_SETTINGS: UserSettings = {
  theme: "dark",
  uiDensity: "comfortable",
  enableGlowFx: true,
  defaultStartupView: "dashboard",

  defaultGraphMode: "cards",
  defaultLivePhysics: false,
  defaultLayoutEngine: "hierarchical",
  showFileNodes: true,
  showLegend: true,
  showEdgeLabels: true,
  chargeStrength: -380,
  linkDistance: 140,
  collisionPadding: 28,

  mcpCommandMode: "system",
  guardrailsMode: "advisory",
  enableMarkdownDigest: true,

  defaultAuthorName: "human",
  scannerIgnorePatterns: [
    "node_modules/**",
    "target/**",
    "dist/**",
    "build/**",
    "*.lock",
    "*.min.js",
    ".git/**",
  ],
  strictGitHook: false,
  diffScanDepth: 1000,

  ingestWatchPaths: [],
  deduplicationMode: "skip",
  requireIngestPreview: true,

  closeToTray: true,
  startOnBoot: false,
  showDesktopNotifications: true,
};

export type ViewTab =
  | "dashboard"
  | "graph"
  | "query"
  | "stats"
  | "ingest"
  | "scanner"
  | "mcp"
  | "workspace";

export const VALID_TABS: readonly ViewTab[] = [
  "dashboard",
  "graph",
  "query",
  "stats",
  "ingest",
  "scanner",
  "mcp",
  "workspace",
];

export const isValidTab = (tab: string): tab is ViewTab => {
  return (VALID_TABS as readonly string[]).includes(tab);
};

export type McpSchemaFormat = "standard" | "nested_mcp" | "zed" | "custom";

export interface DetectedMcpApp {
  id: string;
  name: string;
  category: "ide" | "cli" | "extension" | "custom";
  file_path: string;
  exists: boolean;
  is_configured: boolean;
  format: McpSchemaFormat;
  last_modified: string | null;
  is_custom: boolean;
}

export type McpCommandMode = "absolute" | "system";

export type McpScanLogLevel = "info" | "scan" | "found" | "config" | "skip" | "error";

export interface McpScanLog {
  level: McpScanLogLevel;
  message: string;
  timestamp: string;
}

export interface McpResource {
  uri: string;
  name: string;
  description: string;
  mimeType: string;
}

export interface McpPromptArgument {
  name: string;
  description: string;
  required: boolean;
}

export interface McpPrompt {
  name: string;
  description: string;
  arguments?: McpPromptArgument[];
}
