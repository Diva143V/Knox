//! TypeScript type definitions mirroring Knox Rust domain types.

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
}

export interface Session {
  id: string;
  actor: string;
  project: string | null;
  started_at: string;
  ended_at: string | null;
  is_open: boolean;
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
}

export interface KnoxConfig {
  kg_path: string;
  session_timeout_seconds: number;
  global_mode: boolean;
}

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


