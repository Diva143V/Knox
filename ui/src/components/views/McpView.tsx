import React, { useState, useEffect, useCallback, useMemo, useRef } from "react";
import {
  PlugZap,
  Terminal,
  ChevronDown,
  ChevronUp,
  ChevronRight,
  Search,
  HardDrive,
  Plus,
  RefreshCw,
  FolderOpen,
  Trash2,
  FileCode,
  Sliders,
  X,
  Copy,
  Check,
  Info,
  GitFork,
  ShieldCheck,
  Layers,
  Sparkles,
  BookOpen,
} from "lucide-react";
import { useKnoxStore } from "@/store/useKnoxStore";
import { KnoxAPI } from "@/services/api";
import type { DetectedMcpApp, McpCommandMode, McpScanLog } from "@/types/knox";
import { PageHeader } from "@/components/ui/PageHeader";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/Card";
import { Button } from "@/components/ui/Button";
import { Badge } from "@/components/ui/Badge";
import { CodeBlock } from "@/components/ui/CodeBlock";
import { cn } from "@/lib/cn";

interface McpTool {
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

const MCP_TOOLS: McpTool[] = [
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

const MCP_RESOURCES: McpResourceItem[] = [
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

const MCP_PROMPTS: McpPromptItem[] = [
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

type ConfigTarget = "cursor" | "claude" | "antigravity" | "cli";
type CategoryFilter = "All" | McpTool["category"];

const STORAGE_KEY_CUSTOM_APPS = "knox_custom_mcp_apps";
const STORAGE_KEY_COMMAND_MODE = "knox_mcp_command_mode";

const CATEGORY_BADGE: Record<
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

export const McpView: React.FC = () => {
  const { showToast } = useKnoxStore();

  // Command mode: Absolute path vs system command
  const [commandMode, setCommandMode] = useState<McpCommandMode>(() => {
    try {
      const saved = localStorage.getItem(STORAGE_KEY_COMMAND_MODE);
      if (saved === "system" || saved === "absolute") return saved;
    } catch {
      // ignore
    }
    return "absolute";
  });

  const [apps, setApps] = useState<DetectedMcpApp[]>([]);
  const [isScanning, setIsScanning] = useState(false);
  const [isDeepScanning, setIsDeepScanning] = useState(false);
  const [actionLoadingId, setActionLoadingId] = useState<string | null>(null);

  // Collapse/Expand state for detected apps list
  const [showAllApps, setShowAllApps] = useState(false);

  // Live Terminal state
  const [showTerminal, setShowTerminal] = useState(false);
  const [logs, setLogs] = useState<McpScanLog[]>([]);
  const [autoScroll, setAutoScroll] = useState(true);
  const logContainerRef = useRef<HTMLDivElement>(null);

  // Custom app modal state
  const [showCustomModal, setShowCustomModal] = useState(false);
  const [customName, setCustomName] = useState("");
  const [customPath, setCustomPath] = useState("");
  const [customLoading, setCustomLoading] = useState(false);

  // Snippets & documentation
  const [activeTarget, setActiveTarget] = useState<ConfigTarget>("cursor");
  const [categoryFilter, setCategoryFilter] = useState<CategoryFilter>("All");
  const [expandedTool, setExpandedTool] = useState<string | null>("knox_get_context");

  // Setup live event listener for scan logs
  useEffect(() => {
    let unlisten: (() => void) | undefined;
    KnoxAPI.listenMcpScanLogs((log) => {
      setLogs((prev) => [...prev, log]);
    }).then((fn) => {
      unlisten = fn;
    });
    return () => {
      if (unlisten) unlisten();
    };
  }, []);

  // Auto-scroll terminal log window
  useEffect(() => {
    if (autoScroll && logContainerRef.current) {
      logContainerRef.current.scrollTop = logContainerRef.current.scrollHeight;
    }
  }, [logs, autoScroll]);

  const handleClearLogs = () => {
    setLogs([]);
    showToast("Cleared scan activity console.");
  };

  const handleCopyLogs = () => {
    if (logs.length === 0) {
      showToast("No logs to copy", true);
      return;
    }
    const text = logs
      .map((l) => `[${l.timestamp}] [${l.level.toUpperCase()}] ${l.message}`)
      .join("\n");
    navigator.clipboard.writeText(text);
    showToast("Copied scan terminal logs to clipboard!");
  };

  const renderLogLevelBadge = (level: McpScanLog["level"]) => {
    switch (level) {
      case "info":
        return (
          <span className="text-2xs font-bold px-1 py-0.5 rounded-sm bg-warning/20 text-warning border border-warning/30 shrink-0">
            INFO
          </span>
        );
      case "scan":
        return (
          <span className="text-2xs font-bold px-1 py-0.5 rounded-sm bg-primary/20 text-primary border border-primary/30 shrink-0">
            SCAN
          </span>
        );
      case "found":
        return (
          <span className="text-2xs font-bold px-1 py-0.5 rounded-sm bg-success/20 text-success border border-success/30 shrink-0">
            FOUND
          </span>
        );
      case "config":
        return (
          <span className="text-2xs font-bold px-1 py-0.5 rounded-sm bg-primary/20 text-primary border border-primary/30 shrink-0">
            CONFIG
          </span>
        );
      case "skip":
        return (
          <span className="text-2xs font-bold px-1 py-0.5 rounded-sm bg-muted text-muted-foreground border border-border shrink-0">
            SKIP
          </span>
        );
      case "error":
        return (
          <span className="text-2xs font-bold px-1 py-0.5 rounded-sm bg-danger/20 text-danger border border-danger/30 shrink-0">
            ERROR
          </span>
        );
      default:
        return null;
    }
  };

  const saveCommandMode = (mode: McpCommandMode) => {
    setCommandMode(mode);
    try {
      localStorage.setItem(STORAGE_KEY_COMMAND_MODE, mode);
    } catch {
      // ignore
    }
    showToast(
      `Switched MCP Command Mode to ${
        mode === "absolute" ? "Absolute Executable Path" : "System PATH ('knox')"
      }`
    );
  };

  const getSavedCustomApps = (): DetectedMcpApp[] => {
    try {
      const raw = localStorage.getItem(STORAGE_KEY_CUSTOM_APPS);
      if (raw) return JSON.parse(raw);
    } catch {
      // ignore
    }
    return [];
  };

  const saveCustomApps = (customList: DetectedMcpApp[]) => {
    try {
      localStorage.setItem(STORAGE_KEY_CUSTOM_APPS, JSON.stringify(customList));
    } catch {
      // ignore
    }
  };

  const refreshConfigs = useCallback(
    async (deep = false, openTerminal = false) => {
      if (openTerminal) {
        setShowTerminal(true);
      }
      if (deep) {
        setIsDeepScanning(true);
      } else {
        setIsScanning(true);
      }

      try {
        const detected = deep
          ? await KnoxAPI.deepScanMcpConfigs()
          : await KnoxAPI.detectMcpConfigs();

        const customSaved = getSavedCustomApps();

        // Refresh status for custom saved apps
        const inspectedCustom: DetectedMcpApp[] = [];
        for (const custom of customSaved) {
          try {
            const inspected = await KnoxAPI.inspectCustomMcpConfig(custom.file_path);
            inspectedCustom.push({
              ...inspected,
              id: custom.id,
              name: custom.name || inspected.name,
              is_custom: true,
            });
          } catch {
            inspectedCustom.push(custom);
          }
        }

        // Merge avoiding duplicates by file_path
        const pathSet = new Set<string>();
        const combined: DetectedMcpApp[] = [];

        for (const item of detected) {
          if (!pathSet.has(item.file_path.toLowerCase())) {
            pathSet.add(item.file_path.toLowerCase());
            combined.push(item);
          }
        }

        for (const item of inspectedCustom) {
          if (!pathSet.has(item.file_path.toLowerCase())) {
            pathSet.add(item.file_path.toLowerCase());
            combined.push(item);
          }
        }

        setApps(combined);
        if (deep) {
          showToast(
            `Deep PC Scan complete! Found ${combined.length} configuration paths.`
          );
        }
      } catch (e) {
        console.error("Failed to scan MCP configs:", e);
        showToast(String(e), true);
      } finally {
        setIsScanning(false);
        setIsDeepScanning(false);
      }
    },
    [showToast]
  );

  useEffect(() => {
    refreshConfigs(false);
  }, [refreshConfigs]);

  const handleInject = async (app: DetectedMcpApp) => {
    setActionLoadingId(app.id);
    try {
      const useSystem = commandMode === "system";
      await KnoxAPI.injectMcpConfig(app.file_path, useSystem);
      showToast(
        `Added Knox to ${app.name} (${useSystem ? "knox" : "absolute path"})!`
      );
      setApps((prev) =>
        prev.map((a) =>
          a.id === app.id ? { ...a, is_configured: true, exists: true } : a
        )
      );
    } catch (e) {
      showToast(`Failed to configure ${app.name}: ${e}`, true);
    } finally {
      setActionLoadingId(null);
    }
  };

  const handleRemove = async (app: DetectedMcpApp) => {
    setActionLoadingId(app.id);
    try {
      await KnoxAPI.removeMcpConfig(app.file_path);
      showToast(`Removed Knox from ${app.name}. Backup saved to .bak.`);
      setApps((prev) =>
        prev.map((a) =>
          a.id === app.id ? { ...a, is_configured: false } : a
        )
      );
    } catch (e) {
      showToast(`Failed to remove Knox from ${app.name}: ${e}`, true);
    } finally {
      setActionLoadingId(null);
    }
  };

  const handleBrowseCustomFile = async () => {
    try {
      const selected = await KnoxAPI.openFileDialog({
        name: "MCP Config JSON",
        extensions: ["json"],
      });
      if (selected) {
        setCustomPath(selected);
        if (!customName) {
          const parts = selected.replace(/\\/g, "/").split("/");
          const fileName = parts[parts.length - 1];
          const folderName = parts.length > 1 ? parts[parts.length - 2] : "";
          if (folderName && folderName.toLowerCase() !== "user") {
            setCustomName(`${folderName} App`);
          } else {
            setCustomName(fileName.replace(".json", ""));
          }
        }
      }
    } catch (e) {
      showToast(`Failed to pick file: ${e}`, true);
    }
  };

  const handleSaveCustomApp = async () => {
    if (!customPath.trim()) {
      showToast("Please provide or browse a configuration file path", true);
      return;
    }

    setCustomLoading(true);
    try {
      const inspected = await KnoxAPI.inspectCustomMcpConfig(customPath.trim());
      const newCustomApp: DetectedMcpApp = {
        ...inspected,
        id: `custom-${Date.now()}`,
        name: customName.trim() || inspected.name,
        is_custom: true,
      };

      const currentCustom = getSavedCustomApps();
      const updatedCustom = [
        ...currentCustom.filter(
          (c) =>
            c.file_path.toLowerCase() !== newCustomApp.file_path.toLowerCase()
        ),
        newCustomApp,
      ];
      saveCustomApps(updatedCustom);

      await KnoxAPI.injectMcpConfig(
        newCustomApp.file_path,
        commandMode === "system"
      );
      newCustomApp.is_configured = true;

      setApps((prev) => [
        newCustomApp,
        ...prev.filter(
          (a) =>
            a.file_path.toLowerCase() !== newCustomApp.file_path.toLowerCase()
        ),
      ]);

      showToast(`Custom app "${newCustomApp.name}" added and Knox configured!`);
      setShowCustomModal(false);
      setCustomName("");
      setCustomPath("");
    } catch (e) {
      showToast(`Failed to add custom app: ${e}`, true);
    } finally {
      setCustomLoading(false);
    }
  };

  const handleDeleteCustomApp = (id: string, e: React.MouseEvent) => {
    e.stopPropagation();
    const currentCustom = getSavedCustomApps();
    const updated = currentCustom.filter((c) => c.id !== id);
    saveCustomApps(updated);
    setApps((prev) => prev.filter((a) => a.id !== id));
    showToast("Removed custom app from list.");
  };

  const getConfigSnippet = (target: ConfigTarget): string => {
    const cmd =
      commandMode === "system" ? "knox" : "D:\\Knox\\target\\release\\knox.exe";
    switch (target) {
      case "cursor":
      case "claude":
        return JSON.stringify(
          {
            mcpServers: {
              knox: {
                command: cmd,
                args: ["serve", "--stdio"],
              },
            },
          },
          null,
          2
        );
      case "antigravity":
        return JSON.stringify(
          {
            mcp: {
              servers: {
                knox: {
                  command: cmd,
                  args: ["serve", "--stdio"],
                },
              },
            },
          },
          null,
          2
        );
      case "cli":
        return `${cmd} serve --stdio`;
    }
  };

  const getTargetLocation = (target: ConfigTarget): string => {
    switch (target) {
      case "cursor":
        return "~/.cursor/mcp.json";
      case "claude":
        return "%APPDATA%\\Claude\\claude_desktop_config.json";
      case "antigravity":
        return "~/.gemini/config/mcp_config.json";
      case "cli":
        return "Terminal / Shell command";
    }
  };

  const currentSnippet = getConfigSnippet(activeTarget);

  const filteredTools = useMemo(() => {
    if (categoryFilter === "All") return MCP_TOOLS;
    return MCP_TOOLS.filter((t) => t.category === categoryFilter);
  }, [categoryFilter]);

  const categories: CategoryFilter[] = [
    "All",
    "Read",
    "Write",
    "Query",
    "Graph",
    "Safety",
    "Scanner",
    "Lifecycle",
  ];

  const MAX_COLLAPSED_APPS = 4;
  const visibleApps = showAllApps ? apps : apps.slice(0, MAX_COLLAPSED_APPS);
  const hiddenAppsCount = Math.max(0, apps.length - MAX_COLLAPSED_APPS);

  return (
    <div
      className={cn(
        "p-5 sm:p-6 space-y-5 mx-auto overflow-y-auto h-full transition-all duration-300",
        showTerminal ? "max-w-7xl" : "max-w-4xl"
      )}
    >
      <PageHeader
        title="Model Context Protocol (MCP) Server"
        description="Auto-detect, directly inject, and manage the Knox stdio MCP bridge across all your IDEs, coding tools, and private applications."
        icon={<PlugZap />}
      />

      <div
        className={cn(
          "flex flex-col",
          showTerminal && "xl:flex-row items-start gap-6"
        )}
      >
        {/* Left Column */}
        <div
          className={cn(
            "space-y-5 min-w-0",
            showTerminal ? "w-full xl:flex-1" : "w-full"
          )}
        >
          {/* SECTION 1: AUTO-CONFIGURATOR */}
          <Card>
            <CardHeader className="pb-0">
              <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-3 border-b border-border-subtle pb-4">
                <div>
                  <CardTitle className="flex items-center gap-2">
                    <Sliders className="size-4 text-primary" aria-hidden />
                    IDE & Application Auto-Configurator
                  </CardTitle>
                  <p className="text-sm text-muted-foreground mt-0.5">
                    1-Click injection of Knox stdio server into detected configuration files.
                  </p>
                </div>

                <div className="flex flex-wrap items-center gap-2">
                  <Button
                    type="button"
                    variant="outline"
                    size="sm"
                    onClick={() => refreshConfigs(false, true)}
                    disabled={isScanning || isDeepScanning}
                    title="Quick scan of standard IDE directories"
                  >
                    <RefreshCw
                      className={cn(
                        "size-3.5",
                        isScanning && "animate-spin text-primary"
                      )}
                    />
                    <span>{isScanning ? "Scanning..." : "Quick Scan"}</span>
                  </Button>

                  <Button
                    type="button"
                    variant="outline"
                    size="sm"
                    onClick={() => refreshConfigs(true, true)}
                    disabled={isScanning || isDeepScanning}
                    title="Deep scan across all drive letters (C:\, D:\, etc.)"
                  >
                    <HardDrive
                      className={cn(
                        "size-3.5",
                        isDeepScanning && "animate-pulse text-primary"
                      )}
                    />
                    <span>
                      {isDeepScanning ? "Searching Drives..." : "Deep Search PC"}
                    </span>
                  </Button>

                  <Button
                    type="button"
                    variant={showTerminal ? "primary" : "outline"}
                    size="sm"
                    onClick={() => setShowTerminal((prev) => !prev)}
                    title={
                      showTerminal
                        ? "Hide Live Scan Terminal"
                        : "Open Live Scan Terminal"
                    }
                  >
                    <Terminal className="size-3.5" />
                    <span>Live Logs</span>
                    <ChevronDown
                      className={cn(
                        "size-3 transition-transform duration-200",
                        showTerminal && "rotate-180"
                      )}
                    />
                  </Button>

                  <Button
                    type="button"
                    variant="primary"
                    size="sm"
                    onClick={() => setShowCustomModal(true)}
                  >
                    <Plus className="size-3.5" />
                    <span>Add Custom App</span>
                  </Button>
                </div>
              </div>
            </CardHeader>

            <CardContent className="pt-4 space-y-4">
              {/* Command Path Mode Toggle */}
              <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-3 p-3 rounded-md bg-muted/40 border border-border text-xs">
                <div className="space-y-0.5">
                  <span className="font-semibold text-foreground flex items-center gap-1.5">
                    Command Path Injection Mode:
                  </span>
                  <p className="text-2xs text-muted-foreground">
                    {commandMode === "absolute"
                      ? "Writes full executable path (100% fail-proof, no Windows PATH setup required)."
                      : "Writes system command 'knox' (ideal if syncing configs across multiple PCs via Git)."}
                  </p>
                </div>

                <div className="inline-flex rounded-md border border-border bg-card p-0.5 shrink-0">
                  <button
                    type="button"
                    onClick={() => saveCommandMode("absolute")}
                    className={cn(
                      "px-2.5 py-1 rounded-sm text-2xs font-medium transition-colors cursor-pointer",
                      commandMode === "absolute"
                        ? "bg-primary text-primary-foreground"
                        : "text-muted-foreground hover:text-foreground"
                    )}
                  >
                    Absolute Path (Recommended)
                  </button>
                  <button
                    type="button"
                    onClick={() => saveCommandMode("system")}
                    className={cn(
                      "px-2.5 py-1 rounded-sm text-2xs font-medium transition-colors cursor-pointer",
                      commandMode === "system"
                        ? "bg-primary text-primary-foreground"
                        : "text-muted-foreground hover:text-foreground"
                    )}
                  >
                    System PATH ('knox')
                  </button>
                </div>
              </div>

              {/* Detected Applications List */}
              <div className="space-y-2">
                {apps.length === 0 ? (
                  <div className="py-8 text-center space-y-2 border border-dashed border-border rounded-lg p-4">
                    <Search className="size-6 text-muted-foreground/40 mx-auto" />
                    <p className="text-xs font-medium text-foreground">
                      No applications detected yet
                    </p>
                    <p className="text-2xs text-muted-foreground">
                      Click Quick Scan or Deep Search PC to scan your system.
                    </p>
                  </div>
                ) : (
                  <>
                    {visibleApps.map((app) => {
                      const isLoading = actionLoadingId === app.id;
                      return (
                        <div
                          key={app.id}
                          className="flex flex-col sm:flex-row sm:items-center justify-between p-3 rounded-md border border-border bg-card hover:bg-muted/30 transition-colors gap-3"
                        >
                          <div className="min-w-0 flex-1 space-y-1">
                            <div className="flex items-center gap-2 flex-wrap">
                              <span className="font-semibold text-xs text-foreground">
                                {app.name}
                              </span>
                              <Badge variant="secondary" className="uppercase font-mono text-2xs">
                                {app.category}
                              </Badge>
                              {app.is_custom && (
                                <Badge variant="outline" className="text-2xs">
                                  Custom App
                                </Badge>
                              )}

                              {app.is_configured ? (
                                <span className="inline-flex items-center gap-1 text-2xs font-mono px-2 py-0.5 rounded-sm bg-success/15 text-success border border-success/30 font-medium">
                                  <Check className="size-3" />
                                  Configured
                                </span>
                              ) : app.exists ? (
                                <span className="inline-flex items-center gap-1 text-2xs font-mono px-2 py-0.5 rounded-sm bg-warning/15 text-warning border border-warning/30">
                                  Detected
                                </span>
                              ) : (
                                <span className="text-2xs font-mono px-2 py-0.5 rounded-sm bg-muted text-muted-foreground border border-border">
                                  Not Installed
                                </span>
                              )}
                            </div>

                            <p
                              className="font-mono text-2xs text-muted-foreground truncate"
                              title={app.file_path}
                            >
                              {app.file_path}
                            </p>
                          </div>

                          <div className="flex items-center gap-2 shrink-0">
                            {app.is_configured ? (
                              <Button
                                type="button"
                                variant="outline"
                                size="sm"
                                onClick={() => handleRemove(app)}
                                disabled={isLoading}
                                className="text-danger hover:bg-danger/10 border-danger/30"
                              >
                                {isLoading ? "Removing..." : "Remove Knox"}
                              </Button>
                            ) : (
                              <Button
                                type="button"
                                variant="primary"
                                size="sm"
                                onClick={() => handleInject(app)}
                                disabled={isLoading}
                              >
                                {isLoading ? "Adding..." : "Add Knox"}
                              </Button>
                            )}

                            {app.is_custom && (
                              <Button
                                type="button"
                                variant="ghost"
                                size="icon"
                                onClick={(e) => handleDeleteCustomApp(app.id, e)}
                                title="Remove custom application from list"
                                className="size-7 text-muted-foreground hover:text-danger"
                              >
                                <Trash2 className="size-3.5" />
                              </Button>
                            )}
                          </div>
                        </div>
                      );
                    })}

                    {hiddenAppsCount > 0 && (
                      <button
                        type="button"
                        onClick={() => setShowAllApps((prev) => !prev)}
                        className="w-full py-2 px-3 rounded-md border border-dashed border-border bg-background hover:bg-muted text-xs font-medium text-foreground transition-colors cursor-pointer flex items-center justify-center gap-1.5 mt-2"
                      >
                        {showAllApps ? (
                          <>
                            <ChevronUp className="size-3.5 text-primary" />
                            <span>
                              Show Less (Displaying all {apps.length} applications)
                            </span>
                          </>
                        ) : (
                          <>
                            <ChevronDown className="size-3.5 text-primary" />
                            <span>
                              Show More ({hiddenAppsCount} more detected application
                              {hiddenAppsCount > 1 ? "s" : ""})
                            </span>
                          </>
                        )}
                      </button>
                    )}
                  </>
                )}
              </div>
            </CardContent>
          </Card>

          {/* SECTION 2: MANUAL REFERENCE & SNIPPETS */}
          <Card>
            <CardHeader className="pb-0">
              <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-2">
                <CardTitle className="flex items-center gap-2">
                  <Terminal className="size-4 text-primary" aria-hidden />
                  Client setup
                </CardTitle>
                <div className="flex flex-wrap items-center gap-2">
                  <Badge variant="secondary">
                    {MCP_TOOLS.length} tools documented
                  </Badge>
                  <Badge variant="outline">JSON-RPC 2.0</Badge>
                  <Badge variant="outline">stdio transport</Badge>
                </div>
              </div>
            </CardHeader>
            <CardContent className="pt-4 space-y-4">
              <div
                role="note"
                className="flex gap-2 px-3 py-2 rounded-md border border-border bg-background text-sm text-muted-foreground leading-relaxed"
              >
                <Info className="size-3.5 shrink-0 mt-0.5 text-primary" aria-hidden />
                <p>
                  IDE clients spawn{" "}
                  <code className="font-mono text-foreground">
                    knox serve --stdio
                  </code>{" "}
                  on demand. Status below reflects documented capability, not a
                  process this GUI is monitoring.
                </p>
              </div>

              <CodeBlock
                code="knox serve --stdio"
                language="shell"
                onCopied={() => showToast("Copied launch command to clipboard")}
              />

              <div className="space-y-3">
                <div
                  role="tablist"
                  aria-label="MCP client target"
                  className="flex flex-wrap items-center gap-1 border-b border-border-subtle pb-2"
                >
                  {(["cursor", "claude", "antigravity", "cli"] as ConfigTarget[]).map(
                    (target) => (
                      <button
                        key={target}
                        type="button"
                        role="tab"
                        aria-selected={activeTarget === target}
                        onClick={() => setActiveTarget(target)}
                        className={cn(
                          "px-3 py-1.5 rounded-md text-xs font-medium capitalize transition-colors cursor-pointer",
                          "focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-focus-ring",
                          activeTarget === target
                            ? "bg-primary text-primary-foreground"
                            : "text-muted-foreground hover:text-foreground hover:bg-muted/60"
                        )}
                      >
                        {target === "cli" ? "CLI" : target}
                      </button>
                    )
                  )}
                </div>

                <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-1 text-sm text-muted-foreground">
                  <span>Config file location</span>
                  <span className="font-mono text-foreground text-xs break-all">
                    {getTargetLocation(activeTarget)}
                  </span>
                </div>

                <CodeBlock
                  code={currentSnippet}
                  language={activeTarget === "cli" ? "shell" : "json"}
                  onCopied={() => showToast(`Copied ${activeTarget} config to clipboard`)}
                />
              </div>
            </CardContent>
          </Card>

          {/* SECTION 3: 9 REGISTERED MCP TOOLS ACCORDION */}
          <div className="space-y-3">
            <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-2">
              <h2 className="text-sm font-semibold text-foreground flex items-center gap-2">
                <PlugZap className="size-4 text-primary" aria-hidden />
                Registered MCP Tools ({filteredTools.length})
              </h2>
              <div
                role="toolbar"
                aria-label="Filter tools by category"
                className="flex flex-wrap gap-1"
              >
                {categories.map((cat) => (
                  <Button
                    key={cat}
                    type="button"
                    size="sm"
                    variant={categoryFilter === cat ? "primary" : "ghost"}
                    onClick={() => setCategoryFilter(cat)}
                    aria-pressed={categoryFilter === cat}
                  >
                    {cat}
                  </Button>
                ))}
              </div>
            </div>

            <div className="space-y-2" role="list">
              {filteredTools.map((tool) => {
                const isExpanded = expandedTool === tool.name;
                return (
                  <div
                    key={tool.name}
                    role="listitem"
                    className="rounded-lg border border-border bg-card overflow-hidden"
                  >
                    <button
                      type="button"
                      onClick={() =>
                        setExpandedTool(isExpanded ? null : tool.name)
                      }
                      aria-expanded={isExpanded}
                      className="w-full p-3.5 flex items-center justify-between hover:bg-muted/30 transition-colors text-left cursor-pointer focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-focus-ring"
                    >
                      <div className="flex items-center gap-2.5 min-w-0">
                        {isExpanded ? (
                          <ChevronDown
                            className="size-4 text-primary shrink-0"
                            aria-hidden
                          />
                        ) : (
                          <ChevronRight
                            className="size-4 text-muted-foreground shrink-0"
                            aria-hidden
                          />
                        )}
                        <span className="font-mono font-semibold text-foreground text-xs">
                          {tool.name}
                        </span>
                        <span className="text-xs text-muted-foreground truncate hidden sm:inline">
                          — {tool.description}
                        </span>
                      </div>
                      <Badge
                        variant={CATEGORY_BADGE[tool.category]}
                        className="uppercase shrink-0 font-mono text-2xs"
                      >
                        {tool.category}
                      </Badge>
                    </button>

                    {isExpanded && (
                      <div className="px-4 pb-4 border-t border-border-subtle bg-background/40 space-y-3 text-xs">
                        <p className="pt-3 text-foreground leading-relaxed">
                          {tool.description}
                        </p>

                        <div className="space-y-1.5">
                          <span className="text-overline">Parameters</span>
                          <div className="border border-border rounded-md overflow-hidden overflow-x-auto">
                            <table className="w-full text-left text-xs">
                              <thead className="bg-card text-xs font-mono text-muted-foreground uppercase border-b border-border">
                                <tr>
                                  <th className="px-3 py-1.5 font-medium">Name</th>
                                  <th className="px-3 py-1.5 font-medium">Type</th>
                                  <th className="px-3 py-1.5 font-medium">
                                    Required
                                  </th>
                                  <th className="px-3 py-1.5 font-medium">
                                    Description
                                  </th>
                                </tr>
                              </thead>
                              <tbody className="divide-y divide-border-subtle font-mono text-sm">
                                {tool.parameters.map((param) => (
                                  <tr key={param.name}>
                                    <td className="px-3 py-1.5 text-primary font-semibold">
                                      {param.name}
                                    </td>
                                    <td className="px-3 py-1.5 text-foreground/80">
                                      {param.type}
                                    </td>
                                    <td className="px-3 py-1.5">
                                      {param.required ? (
                                        <span className="text-danger text-xs">
                                          required
                                        </span>
                                      ) : (
                                        <span className="text-muted-foreground text-xs">
                                          optional
                                        </span>
                                      )}
                                    </td>
                                    <td className="px-3 py-1.5 text-muted-foreground font-sans text-xs">
                                      {param.description}
                                    </td>
                                  </tr>
                                ))}
                              </tbody>
                            </table>
                          </div>
                        </div>

                        <div className="space-y-1.5">
                          <span className="text-overline">
                            Sample invocation payload
                          </span>
                          <CodeBlock
                            code={JSON.stringify(tool.schemaExample, null, 2)}
                            language="json"
                            onCopied={() => showToast("Copied sample payload to clipboard")}
                          />
                        </div>
                      </div>
                    )}
                  </div>
                );
              })}
            </div>
          </div>

          {/* SECTION 4: NATIVE MCP RESOURCES */}
          <Card>
            <CardHeader className="pb-3">
              <div className="flex items-center justify-between">
                <CardTitle className="flex items-center gap-2">
                  <Layers className="size-4 text-primary" aria-hidden />
                  Native MCP Resources ({MCP_RESOURCES.length})
                </CardTitle>
                <Badge variant="outline" className="font-mono text-2xs uppercase">
                  Read-Only URIs
                </Badge>
              </div>
              <p className="text-sm text-muted-foreground mt-1">
                Directly readable by Claude, Cursor, and Antigravity agents via standard MCP resource templates.
              </p>
            </CardHeader>
            <CardContent className="space-y-3 pt-0">
              <div className="space-y-2.5">
                {MCP_RESOURCES.map((res) => (
                  <div
                    key={res.uri}
                    className="rounded-lg border border-border bg-card p-3.5 space-y-2 hover:border-border-focus transition-colors"
                  >
                    <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-2">
                      <div className="flex items-center gap-2 min-w-0">
                        <span className="font-mono font-semibold text-primary text-xs truncate">
                          {res.uri}
                        </span>
                        <span className="text-2xs text-muted-foreground font-mono">
                          ({res.mimeType})
                        </span>
                      </div>
                      <Button
                        type="button"
                        variant="outline"
                        size="sm"
                        className="h-7 text-xs shrink-0 self-start sm:self-auto"
                        onClick={() => {
                          navigator.clipboard.writeText(res.uri);
                          showToast(`Copied ${res.uri} to clipboard`);
                        }}
                      >
                        <Copy className="size-3 text-muted-foreground mr-1" />
                        <span>Copy URI</span>
                      </Button>
                    </div>
                    <p className="text-xs text-foreground/80 leading-relaxed">
                      {res.description}
                    </p>
                    <div className="pt-1">
                      <span className="text-overline block mb-1">Payload Sample</span>
                      <CodeBlock
                        code={res.example}
                        language="markdown"
                        onCopied={() => showToast(`Copied ${res.name} sample to clipboard`)}
                      />
                    </div>
                  </div>
                ))}
              </div>
            </CardContent>
          </Card>

          {/* SECTION 5: NATIVE MCP PROMPTS */}
          <Card>
            <CardHeader className="pb-3">
              <div className="flex items-center justify-between">
                <CardTitle className="flex items-center gap-2">
                  <Sparkles className="size-4 text-warning" aria-hidden />
                  Native MCP Prompts ({MCP_PROMPTS.length})
                </CardTitle>
                <Badge variant="warning" className="font-mono text-2xs uppercase">
                  Standard Workflows
                </Badge>
              </div>
              <p className="text-sm text-muted-foreground mt-1">
                Reusable prompt templates registered natively in MCP clients for architectural verification and agent handovers.
              </p>
            </CardHeader>
            <CardContent className="space-y-3 pt-0">
              <div className="grid grid-cols-1 md:grid-cols-2 gap-3">
                {MCP_PROMPTS.map((prompt) => (
                  <div
                    key={prompt.name}
                    className="rounded-lg border border-border bg-card p-3.5 space-y-2 flex flex-col justify-between"
                  >
                    <div className="space-y-1.5">
                      <div className="flex items-center justify-between gap-2">
                        <span className="font-mono font-bold text-foreground text-xs">
                          /{prompt.name}
                        </span>
                        <Badge variant="outline" className="text-2xs">
                          {prompt.arguments.length} args
                        </Badge>
                      </div>
                      <p className="text-xs text-muted-foreground leading-relaxed">
                        {prompt.description}
                      </p>
                    </div>
                    <div className="pt-2 border-t border-border-subtle space-y-1.5">
                      <span className="text-overline block">Arguments</span>
                      <div className="space-y-1 font-mono text-2xs">
                        {prompt.arguments.map((arg) => (
                          <div key={arg.name} className="flex items-center justify-between text-muted-foreground">
                            <span className="text-primary font-semibold">{arg.name}</span>
                            <span>{arg.required ? "required" : "optional"}</span>
                          </div>
                        ))}
                      </div>
                    </div>
                  </div>
                ))}
              </div>
            </CardContent>
          </Card>

          {/* SECTION 6: SMART AUTO-SESSIONS & CONTEXT OPTIMIZATION */}
          <Card className="border-primary/30 bg-primary/5">
            <CardHeader className="pb-3">
              <CardTitle className="flex items-center gap-2 text-primary">
                <ShieldCheck className="size-4 text-primary" aria-hidden />
                Autonomous Agent Optimization & Guardrails
              </CardTitle>
              <p className="text-sm text-muted-foreground mt-1">
                Knox eliminates friction and cuts context token usage so autonomous agents work faster and make zero contradictory changes.
              </p>
            </CardHeader>
            <CardContent className="pt-0">
              <div className="grid grid-cols-1 sm:grid-cols-3 gap-3 text-xs">
                <div className="p-3 rounded-lg border border-border/80 bg-background/60 space-y-1.5">
                  <div className="flex items-center gap-1.5 font-semibold text-foreground">
                    <Sparkles className="size-3.5 text-primary" />
                    <span>Implicit Auto-Sessions</span>
                  </div>
                  <p className="text-muted-foreground text-xs leading-relaxed">
                    Agents never need to call <code className="text-primary font-mono">knox_start_session</code> first. Recording an action automatically attaches to or creates an active session.
                  </p>
                </div>

                <div className="p-3 rounded-lg border border-border/80 bg-background/60 space-y-1.5">
                  <div className="flex items-center gap-1.5 font-semibold text-foreground">
                    <BookOpen className="size-3.5 text-plan" />
                    <span>Token-Compact Markdown</span>
                  </div>
                  <p className="text-muted-foreground text-xs leading-relaxed">
                    Context tools return compact, high-density bullet tables instead of raw bloated JSON, cutting agent token consumption by over 60%.
                  </p>
                </div>

                <div className="p-3 rounded-lg border border-border/80 bg-background/60 space-y-1.5">
                  <div className="flex items-center gap-1.5 font-semibold text-foreground">
                    <GitFork className="size-3.5 text-warning" />
                    <span>Blast Radius & Lineage</span>
                  </div>
                  <p className="text-muted-foreground text-xs leading-relaxed">
                    <code className="text-warning font-mono">knox_get_impact</code> and <code className="text-warning font-mono">knox_check_guardrails</code> alert agents to breaking architectural violations before code edits happen.
                  </p>
                </div>
              </div>
            </CardContent>
          </Card>
        </div>

        {/* Right Column: Live Scan Terminal */}
        {showTerminal && (
          <div className="w-full xl:w-[460px] shrink-0 sticky top-4 rounded-xl border border-border bg-card shadow-xl p-4 flex flex-col h-[740px] motion-safe:animate-in motion-safe:fade-in motion-safe:slide-in-from-right-4 motion-safe:duration-200">
            {/* Header */}
            <div className="flex items-center justify-between pb-3 border-b border-border shrink-0">
              <div className="flex items-center gap-2">
                <div
                  className={cn(
                    "size-2.5 rounded-full",
                    isScanning || isDeepScanning
                      ? "bg-warning animate-ping"
                      : "bg-success"
                  )}
                />
                <Terminal className="size-4 text-primary" />
                <span className="text-xs font-bold text-foreground font-mono tracking-tight">
                  Scan Activity Log
                </span>
              </div>

              <div className="flex items-center gap-1.5">
                <button
                  type="button"
                  onClick={() => setAutoScroll(!autoScroll)}
                  className={cn(
                    "px-2 py-0.5 rounded-sm text-2xs font-mono border transition-colors cursor-pointer",
                    autoScroll
                      ? "bg-primary/20 text-primary border-primary/40 font-semibold"
                      : "bg-muted text-muted-foreground border-border"
                  )}
                  title="Toggle auto-scrolling to bottom"
                >
                  Scroll: {autoScroll ? "ON" : "OFF"}
                </button>

                <Button
                  type="button"
                  variant="ghost"
                  size="icon"
                  className="size-7"
                  onClick={handleCopyLogs}
                  title="Copy logs to clipboard"
                  aria-label="Copy logs"
                >
                  <Copy className="size-3.5" />
                </Button>

                <Button
                  type="button"
                  variant="ghost"
                  size="icon"
                  className="size-7"
                  onClick={handleClearLogs}
                  title="Clear console"
                  aria-label="Clear console"
                >
                  <Trash2 className="size-3.5" />
                </Button>

                <Button
                  type="button"
                  variant="ghost"
                  size="icon"
                  className="size-7 text-muted-foreground hover:text-danger"
                  onClick={() => setShowTerminal(false)}
                  title="Close terminal panel"
                  aria-label="Close terminal panel"
                >
                  <X className="size-3.5" />
                </Button>
              </div>
            </div>

            {/* Console Output */}
            <div
              ref={logContainerRef}
              className="flex-1 overflow-y-auto font-mono text-2xs p-2 space-y-1.5 scrollbar-thin select-text my-2 bg-background rounded-md border border-border"
            >
              {logs.length === 0 ? (
                <div className="h-full flex flex-col items-center justify-center text-center p-6 text-muted-foreground space-y-2">
                  <Terminal className="size-8 opacity-40 mx-auto" />
                  <p className="text-xs font-medium text-foreground">
                    Terminal Ready
                  </p>
                  <p className="text-2xs text-muted-foreground max-w-[240px]">
                    Click Quick Scan or Deep Search PC to stream real-time filesystem discovery events here.
                  </p>
                </div>
              ) : (
                logs.map((log, i) => (
                  <div
                    key={i}
                    className="leading-tight flex items-start gap-1.5 text-foreground break-all"
                  >
                    <span className="text-muted-foreground shrink-0 text-2xs">
                      {log.timestamp}
                    </span>
                    {renderLogLevelBadge(log.level)}
                    <span className="flex-1 text-foreground">{log.message}</span>
                  </div>
                ))
              )}
            </div>

            {/* Status Footer */}
            <div className="pt-2 border-t border-border flex items-center justify-between text-2xs text-muted-foreground font-mono shrink-0">
              <span>{logs.length} events logged</span>
              <span className="truncate max-w-[220px]">
                {isScanning
                  ? "Quick scan in progress..."
                  : isDeepScanning
                  ? "Deep scanning drives..."
                  : "Idle / Complete"}
              </span>
            </div>
          </div>
        )}
      </div>

      {/* Add Custom / Private App Modal */}
      {showCustomModal && (
        <div className="fixed inset-0 z-50 flex items-center justify-center p-4 bg-background/80 backdrop-blur-sm motion-safe:animate-in motion-safe:fade-in motion-safe:duration-150">
          <div className="w-full max-w-lg bg-card border border-border rounded-xl shadow-2xl p-6 space-y-4">
            <div className="flex items-center justify-between border-b border-border-subtle pb-3">
              <h3 className="text-sm font-semibold text-foreground flex items-center gap-2">
                <FileCode className="size-4 text-primary" />
                <span>Configure Custom or Private Application</span>
              </h3>
              <Button
                type="button"
                variant="ghost"
                size="icon"
                className="size-7"
                onClick={() => setShowCustomModal(false)}
                aria-label="Close modal"
              >
                <X className="size-4" />
              </Button>
            </div>

            <p className="text-xs text-muted-foreground leading-relaxed">
              Knox will auto-detect the MCP schema (`mcpServers`, `mcp.servers`, or `context_servers`), create a safe `.bak` backup, and inject the Knox server entry directly.
            </p>

            <div className="space-y-3 text-xs">
              <div>
                <label className="block text-2xs font-semibold text-foreground mb-1">
                  Application Name
                </label>
                <input
                  type="text"
                  placeholder="e.g. My Studio Tool, Roo Code, Custom IDE"
                  value={customName}
                  onChange={(e) => setCustomName(e.target.value)}
                  className="w-full px-3 py-2 rounded-md bg-background border border-border focus:border-primary focus:outline-hidden text-foreground text-xs"
                />
              </div>

              <div>
                <label className="block text-2xs font-semibold text-foreground mb-1">
                  Configuration File Path (.json)
                </label>
                <div className="flex gap-2">
                  <input
                    type="text"
                    placeholder="e.g. C:\Users\...\settings.json"
                    value={customPath}
                    onChange={(e) => setCustomPath(e.target.value)}
                    className="flex-1 px-3 py-2 rounded-md bg-background border border-border focus:border-primary focus:outline-hidden text-foreground text-xs font-mono"
                  />
                  <Button
                    type="button"
                    variant="outline"
                    onClick={handleBrowseCustomFile}
                    className="shrink-0"
                  >
                    <FolderOpen className="size-3.5 text-primary" />
                    <span>Browse...</span>
                  </Button>
                </div>
              </div>
            </div>

            <div className="pt-2 flex items-center justify-end gap-2 border-t border-border-subtle">
              <Button
                type="button"
                variant="ghost"
                onClick={() => setShowCustomModal(false)}
              >
                Cancel
              </Button>
              <Button
                type="button"
                variant="primary"
                onClick={handleSaveCustomApp}
                disabled={customLoading || !customPath.trim()}
              >
                <Check className="size-3.5" />
                <span>
                  {customLoading ? "Saving..." : "Save & Configure Knox"}
                </span>
              </Button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
};
