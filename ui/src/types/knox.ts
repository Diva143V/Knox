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
  // V4 Causal Code Delta Fields
  commit_sha?: string | null;
  delta_summary?: DeltaSummary | null;
  project_id?: string | null;
}

export interface DeltaSummary {
  total_files: number;
  lines_added: number;
  lines_deleted: number;
}

export type DeltaChangeType = "added" | "modified" | "deleted" | "renamed";

export interface ActionDelta {
  id: string;
  action_id: string;
  file_path: string;
  change_type: DeltaChangeType | string;
  old_path?: string | null;
  lines_added: number;
  lines_deleted: number;
  diff_hunk?: string | null;
  deleted_code?: string | null;
  deletion_reason?: string | null;
  created_at: string;
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
  remote?: string | null;
  created_at: string;
  egress?: boolean;
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
  model_profile: string;
  model_endpoint?: string | null;
  model_api_key_masked?: string | null;
  embedding_model?: string | null;
  chat_model?: string | null;
  allow_cloud_egress: boolean;
  stray_db_prompted: boolean;
}

export interface DbStats {
  db_size_bytes: number;
  wal_size_bytes: number;
  page_count: number;
  page_size: number;
  total_actions: number;
  total_sessions: number;
  total_relations: number;
  total_edges: number;
  schema_version: number;
}

export type SettingsTabId =
  | "general"
  | "storage"
  | "model"
  | "graph"
  | "mcp"
  | "scanner"
  | "ingestion"
  | "system"
  | "shortcuts"
  | "about";

export interface StrayDatabaseDto {
  project_path: string;
  db_path: string;
  action_count: number;
}

export interface ConsolidationReport {
  source_path: string;
  target_path: string;
  projects_moved: number;
  sessions_moved: number;
  actions_moved: number;
  actions_skipped: number;
  edges_moved: number;
}

export interface ModelSettingsDto {
  profile: string;
  endpoint?: string | null;
  api_key?: string | null;
  embedding_model?: string | null;
  chat_model?: string | null;
  allow_cloud_egress: boolean;
}

export interface ModelTestResultDto {
  success: boolean;
  latency_ms: number;
  embedding_ok: boolean;
  embedding_dim: number;
  answer_preview?: string | null;
  error?: string | null;
}

export interface UserSettings {
  // Appearance
  theme: "dark" | "light" | "system";
  uiDensity: "comfortable" | "compact";
  enableGlowFx: boolean;
  /** Custom accent (#rrggbb); null follows the theme's own accent. */
  accentColor: string | null;
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
  accentColor: null,
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

export interface UpdateActionPayload {
  id: string;
  summary?: string;
  action_type?: string;
  actor?: string;
  files?: string[];
  tags?: string[];
  symbols?: string[];
  rationale?: string | null;
  context_summary?: string | null;
  rejected_alternatives?: string[];
  significance?: string;
  is_resolved?: boolean;
  resolution_note?: string | null;
  parent_action_id?: string | null;
  relations?: ActionRelation[];
  add_relations?: ActionRelation[];
  remove_relations?: string[];
}

export interface ModelPreset {
  id: string;
  name: string;
  profile: "local" | "cloud";
  endpoint: string;
  embeddingModel: string;
  chatModel: string;
  requiresApiKey: boolean;
  defaultPort?: number;
  description: string;
  troubleshootHelp?: string;
}

export const LOCAL_PRESETS: ModelPreset[] = [
  {
    id: "ollama",
    name: "Ollama",
    profile: "local",
    endpoint: "http://localhost:11434",
    embeddingModel: "nomic-embed-text",
    chatModel: "llama3.2",
    requiresApiKey: false,
    defaultPort: 11434,
    description: "Default local daemon on port 11434 with nomic-embed-text and llama3.2.",
    troubleshootHelp:
      "Ensure Ollama is running (`ollama serve`) and pull models (`ollama pull nomic-embed-text && ollama pull llama3.2`).",
  },
  {
    id: "lm-studio",
    name: "LM Studio",
    profile: "local",
    endpoint: "http://localhost:1234/v1",
    embeddingModel: "text-embedding-nomic-embed-text-v1.5",
    chatModel: "meta-llama-3.1-8b-instruct",
    requiresApiKey: false,
    defaultPort: 1234,
    description: "Local GUI model server on port 1234 with OpenAI-compatible /v1 endpoints.",
    troubleshootHelp:
      "Open LM Studio, start the Local Server on port 1234, and ensure both embedding and chat models are loaded.",
  },
  {
    id: "localai",
    name: "LocalAI / vLLM",
    profile: "local",
    endpoint: "http://localhost:8080/v1",
    embeddingModel: "text-embedding-ada-002",
    chatModel: "gpt-3.5-turbo",
    requiresApiKey: false,
    defaultPort: 8080,
    description: "Self-hosted OpenAI-compatible inference server on port 8080.",
    troubleshootHelp: "Ensure LocalAI or vLLM container is running on port 8080.",
  },
];

export const CLOUD_PRESETS: ModelPreset[] = [
  {
    id: "openai",
    name: "OpenAI",
    profile: "cloud",
    endpoint: "https://api.openai.com/v1",
    embeddingModel: "text-embedding-3-small",
    chatModel: "gpt-4o-mini",
    requiresApiKey: true,
    description: "Official OpenAI cloud models with high accuracy and fast embeddings.",
    troubleshootHelp:
      "Ensure your OpenAI API key (sk-...) has active credits and that Cloud Egress is authorized.",
  },
  {
    id: "openrouter",
    name: "OpenRouter",
    profile: "cloud",
    endpoint: "https://openrouter.ai/api/v1",
    embeddingModel: "openai/text-embedding-3-small",
    chatModel: "meta-llama/llama-3.2-3b-instruct",
    requiresApiKey: true,
    description: "Unified AI gateway routing to multiple frontier and open-weights models.",
    troubleshootHelp: "Verify your OpenRouter API key and confirm credit balance at openrouter.ai.",
  },
  {
    id: "groq",
    name: "Groq",
    profile: "cloud",
    endpoint: "https://api.groq.com/openai/v1",
    embeddingModel: "nomic-embed-text",
    chatModel: "llama-3.2-3b-preview",
    requiresApiKey: true,
    description: "Ultra-low latency LPU cloud inference.",
    troubleshootHelp: "Check your Groq API key at console.groq.com.",
  },
  {
    id: "gemini",
    name: "Google Gemini",
    profile: "cloud",
    endpoint: "https://generativelanguage.googleapis.com/v1beta/openai/",
    embeddingModel: "text-embedding-004",
    chatModel: "gemini-1.5-flash",
    requiresApiKey: true,
    description: "Google Gemini OpenAI-compatible endpoint with fast multimodal synthesis.",
    troubleshootHelp:
      "Check your Gemini API key from Google AI Studio and ensure Cloud Egress is enabled.",
  },
];

export interface LocalDaemonStatus {
  port: number;
  is_online: boolean;
  name: string;
}

export interface AskCitationDto {
  action_id: string;
  summary: string;
  file?: string | null;
}

export interface AskResultDto {
  question: string;
  answer: string;
  citations: AskCitationDto[];
  citations_count: number;
  model: string;
  latency_ms: number;
  candidates_evaluated: number;
  grounded: boolean;
  confidence: number;
}

export interface DoctorReport {
  empty_rate: number;
  empty_breakdown: {
    no_records: number;
    no_match: number;
    out_of_scope: number;
  };
  zombie_sessions: number;
  top_rejected_edge_types: [string, number][];
  promotion_candidates: {
    attempted_type: string;
    rejection_count: number;
    threshold: number;
    promotion_path: string;
  }[];
  db_size_bytes: number;
  wal_size_bytes: number;
  page_count: number;
  page_size: number;
  integrity_check: string;
  schema_version: number;
  relation_coverage: number;
  bound_models: string[];
}
