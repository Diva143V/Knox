export interface McpTool {
  name: string;
  category: "Read" | "Write" | "Query" | "Graph" | "Safety" | "Scanner" | "Lifecycle";
  description: string;
  parameters: {
    name: string;
    type: string;
    required: boolean;
    description: string;
  }[];
  schemaExample: Record<string, unknown>;
}

export interface McpResourceItem {
  uri: string;
  name: string;
  mimeType: string;
  description: string;
  example: string;
}

export interface McpPromptItem {
  name: string;
  description: string;
  arguments: {
    name: string;
    type: string;
    required: boolean;
    description: string;
  }[];
  templatePreview: string;
}

export type ConfigTarget = "cursor" | "claude" | "antigravity" | "cli";
export type CategoryFilter = "All" | McpTool["category"];

export const CATEGORY_BADGE: Record<
  McpTool["category"],
  "default" | "secondary" | "outline" | "warning" | "plan"
> = {
  Read: "outline",
  Write: "default",
  Query: "secondary",
  Graph: "plan",
  Safety: "warning",
  Scanner: "outline",
  Lifecycle: "secondary",
};

export const MCP_TOOLS: McpTool[] = [
  {
    name: "knox_get_context",
    category: "Read",
    description:
      "Returns active, non-superseded decisions, plans, and actions for a file or project in compact Markdown digest format.",
    parameters: [
      {
        name: "file_path",
        type: "string",
        required: false,
        description: "Specific repository file path to inspect",
      },
      {
        name: "project",
        type: "string",
        required: false,
        description: "Project name or root identifier",
      },
      {
        name: "current_only",
        type: "boolean",
        required: false,
        description: "Filter out superseded actions (default: true)",
      },
      {
        name: "limit",
        type: "number",
        required: false,
        description: "Max results to return (default: 50)",
      },
      {
        name: "format",
        type: "string",
        required: false,
        description: "Output format: 'markdown' (compact, default) or 'json'",
      },
    ],
    schemaExample: { file_path: "src/parser.rs", current_only: true, format: "markdown" },
  },
  {
    name: "knox_get_history",
    category: "Read",
    description:
      "Returns a chronologically ordered causal trail of actions and edits for a file or directory.",
    parameters: [
      {
        name: "file_path",
        type: "string",
        required: true,
        description: "File or directory path to inspect causal history for",
      },
      {
        name: "limit",
        type: "number",
        required: false,
        description: "Maximum history entries (default: 50)",
      },
    ],
    schemaExample: { file_path: "src/main.rs", limit: 25 },
  },
  {
    name: "knox_record_action",
    category: "Write",
    description:
      "Records an atomic agent action with touched files, code symbols, and semantic causal relations. Smart auto-session binds automatically if session_id is omitted.",
    parameters: [
      {
        name: "session_id",
        type: "string",
        required: false,
        description: "Enclosing session identifier. If omitted, Knox binds to the active session or creates one automatically.",
      },
      {
        name: "actor",
        type: "string",
        required: true,
        description: "Agent name (e.g. gemini, cursor, claude, antigravity)",
      },
      {
        name: "action_type",
        type: "string",
        required: true,
        description: "Action intent: write | debug | plan | review | refactor | decide",
      },
      {
        name: "summary",
        type: "string",
        required: true,
        description: "Concise summary of action or structural modification performed",
      },
      {
        name: "files",
        type: "string[]",
        required: false,
        description: "List of touched file paths",
      },
      {
        name: "symbols",
        type: "string[]",
        required: false,
        description: "Functions, classes, structs touched (e.g. ['useLivePhysics', 'McpServer'])",
      },
      {
        name: "relations",
        type: "object[]",
        required: false,
        description: "Causal relations: [{ target_id: string, relation_type: 'depends_on' | 'implements' | 'verifies' | 'relates_to' | 'reverts' }]",
      },
      {
        name: "context_summary",
        type: "string",
        required: false,
        description: "High-level goal or reason for the edit",
      },
      {
        name: "tags",
        type: "string[]",
        required: false,
        description: "Topic and domain tags",
      },
      {
        name: "confidence",
        type: "number",
        required: false,
        description: "Confidence score (0.0 to 1.0)",
      },
    ],
    schemaExample: {
      actor: "antigravity",
      action_type: "write",
      summary: "Implemented D3 Cosmos live physics Knowledge Graph",
      files: ["ui/src/components/graph/KnoxGraphCanvas.tsx"],
      symbols: ["useLivePhysics", "CircleNode"],
      relations: [{ target_id: "act-042", relation_type: "implements" }],
    },
  },
  {
    name: "knox_record_decision",
    category: "Write",
    description:
      "Records an architectural choice, ADR decision, or structural trade-off with rationale and rejected alternatives.",
    parameters: [
      {
        name: "session_id",
        type: "string",
        required: false,
        description: "Enclosing session identifier (auto-created if omitted)",
      },
      {
        name: "actor",
        type: "string",
        required: true,
        description: "Agent or engineer decision-maker",
      },
      {
        name: "summary",
        type: "string",
        required: true,
        description: "Decision summary or ADR title",
      },
      {
        name: "rationale",
        type: "string",
        required: false,
        description: "Reasoning and architectural context behind the decision",
      },
      {
        name: "alternatives",
        type: "string[]",
        required: false,
        description: "Design options considered but rejected and why",
      },
      {
        name: "files",
        type: "string[]",
        required: false,
        description: "Files impacted by this decision",
      },
      {
        name: "confidence",
        type: "number",
        required: false,
        description: "Confidence score (0.0 to 1.0)",
      },
    ],
    schemaExample: {
      actor: "cursor",
      summary: "Use D3 force simulation for Cosmos knowledge graph",
      rationale: "Natural element circle clustering and real-time motion physics",
      alternatives: ["Static canvas", "Full canvas rerendering"],
    },
  },
  {
    name: "knox_record_batch",
    category: "Write",
    description:
      "Atomically records multiple actions and their relations in a single transactional batch, minimizing round-trips.",
    parameters: [
      {
        name: "session_id",
        type: "string",
        required: false,
        description: "Enclosing session identifier (auto-created if omitted)",
      },
      {
        name: "actor",
        type: "string",
        required: true,
        description: "Agent performing the batch operations",
      },
      {
        name: "actions",
        type: "object[]",
        required: true,
        description: "Array of action objects (action_type, summary, files, symbols, relations)",
      },
    ],
    schemaExample: {
      actor: "gemini",
      actions: [
        { action_type: "write", summary: "Added CircleNode component", files: ["ui/src/components/graph/CircleNode.tsx"] },
        { action_type: "write", summary: "Added useLivePhysics hook", files: ["ui/src/hooks/useLivePhysics.ts"] },
      ],
    },
  },
  {
    name: "knox_get_lineage",
    category: "Graph",
    description:
      "Recursively traces causal dependencies (depends_on, implements, reverts) upstream or downstream from an action or file.",
    parameters: [
      {
        name: "action_id",
        type: "string",
        required: false,
        description: "Starting action ID",
      },
      {
        name: "file_path",
        type: "string",
        required: false,
        description: "Starting file path to resolve lineage from",
      },
      {
        name: "depth",
        type: "number",
        required: false,
        description: "Maximum recursion depth (default: 5)",
      },
      {
        name: "direction",
        type: "string",
        required: false,
        description: "Traversal direction: 'upstream', 'downstream', or 'both' (default: 'both')",
      },
    ],
    schemaExample: { action_id: "act-042", depth: 4, direction: "both" },
  },
  {
    name: "knox_get_impact",
    category: "Graph",
    description:
      "Computes the blast radius of modifying a file or action, uncovering all dependent files, actions, and active decisions.",
    parameters: [
      {
        name: "file_path",
        type: "string",
        required: false,
        description: "Target file path to evaluate blast radius for",
      },
      {
        name: "action_id",
        type: "string",
        required: false,
        description: "Target action ID to evaluate blast radius for",
      },
      {
        name: "max_depth",
        type: "number",
        required: false,
        description: "Traversal depth limit (default: 3)",
      },
    ],
    schemaExample: { file_path: "src/mcp/format.rs", max_depth: 3 },
  },
  {
    name: "knox_get_path",
    category: "Graph",
    description:
      "Finds the causal or semantic path connecting two actions or files across the Knox knowledge graph.",
    parameters: [
      {
        name: "from_action_id",
        type: "string",
        required: false,
        description: "Starting action identifier",
      },
      {
        name: "to_action_id",
        type: "string",
        required: false,
        description: "Target action identifier",
      },
      {
        name: "from_file",
        type: "string",
        required: false,
        description: "Starting file path",
      },
      {
        name: "to_file",
        type: "string",
        required: false,
        description: "Target file path",
      },
      {
        name: "max_depth",
        type: "number",
        required: false,
        description: "Maximum path depth (default: 6)",
      },
    ],
    schemaExample: { from_file: "ui/src/types/knox.ts", to_file: "ui/src/components/views/McpView.tsx" },
  },
  {
    name: "knox_check_guardrails",
    category: "Safety",
    description:
      "Validates a proposed modification against active ADRs and architectural constraints, returning warnings before code is modified.",
    parameters: [
      {
        name: "files",
        type: "string[]",
        required: false,
        description: "File paths to check against recorded ADR constraints",
      },
      {
        name: "proposed_action",
        type: "string",
        required: false,
        description: "Summary or plan of the proposed modification",
      },
      {
        name: "strict",
        type: "boolean",
        required: false,
        description: "Treat warnings as blocking violations (default: false)",
      },
    ],
    schemaExample: {
      files: ["src/storage/mod.rs"],
      proposed_action: "Replace SQLite with in-memory HashMap persistence",
    },
  },
  {
    name: "knox_search",
    category: "Query",
    description:
      "Full-text search (FTS5) across past actions, decisions, file paths, code symbols, and rationales.",
    parameters: [
      {
        name: "query",
        type: "string",
        required: true,
        description: "Search keyword, file path, symbol, or FTS expression",
      },
      {
        name: "limit",
        type: "number",
        required: false,
        description: "Maximum number of results (default: 20)",
      },
    ],
    schemaExample: { query: "live physics simulation", limit: 10 },
  },
  {
    name: "knox_list_stale",
    category: "Query",
    description:
      "Finds files whose code has evolved since an agent last recorded an action or assumption.",
    parameters: [
      {
        name: "project",
        type: "string",
        required: false,
        description: "Project identifier",
      },
      {
        name: "threshold_days",
        type: "number",
        required: false,
        description: "Days without updates to consider stale (default: 7)",
      },
    ],
    schemaExample: { threshold_days: 14 },
  },
  {
    name: "knox_scan_staged",
    category: "Scanner",
    description:
      "Analyzes currently git-staged changes against Knox knowledge to detect architectural conflicts.",
    parameters: [
      {
        name: "repo_path",
        type: "string",
        required: false,
        description: "Path to git repository (default: current working directory)",
      },
    ],
    schemaExample: { repo_path: "." },
  },
  {
    name: "knox_start_session",
    category: "Lifecycle",
    description:
      "Explicitly initiates an agent interaction session with optional topic tags and metadata.",
    parameters: [
      {
        name: "actor",
        type: "string",
        required: true,
        description: "Agent or tool initiating the session",
      },
      {
        name: "topic",
        type: "string",
        required: false,
        description: "High-level goal or task description",
      },
    ],
    schemaExample: { actor: "gemini", topic: "Implement MCP tool provider" },
  },
  {
    name: "knox_end_session",
    category: "Lifecycle",
    description: "Closes an active agent session and calculates duration metrics.",
    parameters: [
      {
        name: "session_id",
        type: "string",
        required: true,
        description: "Session identifier to finalize",
      },
    ],
    schemaExample: { session_id: "s-001" },
  },
];

export const MCP_RESOURCES: McpResourceItem[] = [
  {
    uri: "knox://project/overview",
    name: "Project Overview Digest",
    mimeType: "text/markdown",
    description: "Real-time architectural snapshot: active session, total decisions, un-superseded actions, and stale files count.",
    example: `# Knox Project Overview
- **Active Session:** sess_01H... (antigravity)
- **Active Decisions:** 14 ADRs
- **Total Actions Recorded:** 132
- **Stale Files:** 0`,
  },
  {
    uri: "knox://decisions/active",
    name: "Active Architectural Decisions",
    mimeType: "text/markdown",
    description: "All active Architectural Decision Records (ADRs) with rationale and rejected alternatives.",
    example: `## Active Architectural Decisions
- **[dec-001]** Use D3 force simulation for Cosmos knowledge graph
  - **Rationale:** Natural element circle clustering and real-time motion physics
  - **Rejected Alternatives:** Static canvas, Full canvas rerendering`,
  },
  {
    uri: "knox://context/{file}",
    name: "File Architectural Context",
    mimeType: "text/markdown",
    description: "Contextual digest for any specific source file: active decisions, last touched actions, and dependency relations.",
    example: `## Knox File Context: src/parser.rs
- **Active Decisions:** 2 impacting parser
- **Last Action:** Refactored AST token stream (antigravity)
- **Lineage:** depends_on src/lexer.rs`,
  },
];

export const MCP_PROMPTS: McpPromptItem[] = [
  {
    name: "architectural-audit",
    description: "Instructs agent to evaluate code or staged changes against active ADR constraints and guardrails.",
    arguments: [
      { name: "files", type: "string", description: "Comma-separated list of files to audit", required: false },
      { name: "focus", type: "string", description: "Specific architectural concern (e.g. security, performance, persistence)", required: false },
    ],
    templatePreview: "Perform an architectural consistency audit on the target files using Knox guardrails and active decisions. Check for any regression against recorded ADRs...",
  },
  {
    name: "session-handover",
    description: "Compiles a clean session handover digest summarizing touched files, recorded actions, and pending tasks for the next agent or teammate.",
    arguments: [
      { name: "session_id", type: "string", description: "Session ID to summarize (defaults to current active session)", required: false },
    ],
    templatePreview: "Generate a comprehensive session handover summary including all recorded actions, architectural decisions, modified files, and open context items...",
  },
];
