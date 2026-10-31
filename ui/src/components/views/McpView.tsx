import React, { useState, useEffect, useCallback, useRef } from "react";
import {
  PlugZap,
  Copy,
  Check,
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
} from "lucide-react";
import { useKnoxStore } from "@/store/useKnoxStore";
import { KnoxAPI } from "@/services/api";
import type { DetectedMcpApp, McpCommandMode, McpScanLog } from "@/types/knox";

interface McpTool {
  name: string;
  category: "Read" | "Write" | "Query" | "Scanner" | "Lifecycle";
  description: string;
  parameters: { name: string; type: string; required: boolean; description: string }[];
  schemaExample: Record<string, unknown>;
}

const MCP_TOOLS: McpTool[] = [
  {
    name: "knox_get_context",
    category: "Read",
    description: "Returns active, non-superseded decisions, plans, and actions for a file or project.",
    parameters: [
      { name: "file_path", type: "string", required: false, description: "Specific repository file path" },
      { name: "project", type: "string", required: false, description: "Project name or root identifier" },
      { name: "current_only", type: "boolean", required: false, description: "Filter out superseded actions (default: true)" },
      { name: "limit", type: "number", required: false, description: "Max results to return (default: 50)" },
    ],
    schemaExample: { file_path: "src/parser.rs", current_only: true },
  },
  {
    name: "knox_record_action",
    category: "Write",
    description: "Records an atomic agent action (write, debug, read, review) linked to the active session.",
    parameters: [
      { name: "session_id", type: "string", required: true, description: "ID of the enclosing session" },
      { name: "actor", type: "string", required: true, description: "Agent name (e.g. gemini, cursor)" },
      { name: "action_type", type: "string", required: true, description: "Action type: write, debug, etc." },
      { name: "summary", type: "string", required: true, description: "Concise summary of action performed" },
      { name: "files", type: "string[]", required: false, description: "Touched file paths" },
      { name: "tags", type: "string[]", required: false, description: "Topic tags" },
      { name: "confidence", type: "number", required: false, description: "Confidence score (0.0 to 1.0)" },
    ],
    schemaExample: {
      session_id: "s-001",
      actor: "gemini",
      action_type: "write",
      summary: "Implemented Tauri v2 IPC commands in src-tauri",
      files: ["src-tauri/src/lib.rs"],
    },
  },
  {
    name: "knox_record_decision",
    category: "Write",
    description: "Records an architectural or design decision, optionally superseding an earlier approach.",
    parameters: [
      { name: "session_id", type: "string", required: true, description: "ID of the enclosing session" },
      { name: "actor", type: "string", required: true, description: "Agent or human making decision" },
      { name: "summary", type: "string", required: true, description: "Decision rationale and outcome" },
      { name: "supersedes", type: "string", required: false, description: "Action ID of prior decision superseded" },
      { name: "files", type: "string[]", required: false, description: "Associated file paths" },
    ],
    schemaExample: {
      session_id: "s-001",
      actor: "cursor",
      summary: "Chose SQLite with WAL mode over flat files for zero-copy concurrency",
      supersedes: "a-002",
    },
  },
  {
    name: "knox_get_history",
    category: "Read",
    description: "Retrieves complete append-only history for a file or project, including superseded actions.",
    parameters: [
      { name: "file_path", type: "string", required: false, description: "File path to trace historical decisions" },
      { name: "project", type: "string", required: false, description: "Project identifier" },
      { name: "limit", type: "number", required: false, description: "Maximum history items to return" },
    ],
    schemaExample: { file_path: "src/main.rs", limit: 25 },
  },
  {
    name: "knox_search",
    category: "Query",
    description: "Executes SQLite FTS5 full-text keyword search across all action summaries, tags, and files.",
    parameters: [
      { name: "query", type: "string", required: true, description: "FTS5 search phrase or keywords" },
      { name: "limit", type: "number", required: false, description: "Maximum matching actions (default: 50)" },
    ],
    schemaExample: { query: "nom parser grammar" },
  },
  {
    name: "knox_list_stale",
    category: "Query",
    description: "Lists all superseded and outdated actions for inspecting previous architectural iterations.",
    parameters: [
      { name: "limit", type: "number", required: false, description: "Limit number of stale records" },
    ],
    schemaExample: { limit: 50 },
  },
  {
    name: "knox_scan_staged",
    category: "Scanner",
    description: "Scans git staged index changes to attribute unrecorded manual developer edits to human.",
    parameters: [
      { name: "project", type: "string", required: false, description: "Git repository root directory (default: .)" },
    ],
    schemaExample: { project: "." },
  },
  {
    name: "knox_start_session",
    category: "Lifecycle",
    description: "Starts a continuous agent work session and returns a new session_id.",
    parameters: [
      { name: "actor", type: "string", required: true, description: "Agent name (e.g. gemini, cursor)" },
      { name: "project", type: "string", required: false, description: "Project name or root path" },
    ],
    schemaExample: { actor: "gemini", project: "knox" },
  },
  {
    name: "knox_end_session",
    category: "Lifecycle",
    description: "Explicitly closes an active session and returns total actions recorded statistics.",
    parameters: [
      { name: "session_id", type: "string", required: true, description: "Session ID to terminate" },
    ],
    schemaExample: { session_id: "s-001" },
  },
];

type ConfigTarget = "cursor" | "claude" | "antigravity" | "cli";

const STORAGE_KEY_CUSTOM_APPS = "knox_custom_mcp_apps";
const STORAGE_KEY_COMMAND_MODE = "knox_mcp_command_mode";

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
  const [copied, setCopied] = useState(false);
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
          <span className="text-[9px] font-bold px-1 py-0.2 rounded bg-amber-500/20 text-amber-300 border border-amber-500/30 shrink-0">
            INFO
          </span>
        );
      case "scan":
        return (
          <span className="text-[9px] font-bold px-1 py-0.2 rounded bg-sky-500/20 text-sky-300 border border-sky-500/30 shrink-0">
            SCAN
          </span>
        );
      case "found":
        return (
          <span className="text-[9px] font-bold px-1 py-0.2 rounded bg-emerald-500/20 text-emerald-300 border border-emerald-500/30 shrink-0">
            FOUND
          </span>
        );
      case "config":
        return (
          <span className="text-[9px] font-bold px-1 py-0.2 rounded bg-purple-500/20 text-purple-300 border border-purple-500/30 shrink-0">
            CONFIG
          </span>
        );
      case "skip":
        return (
          <span className="text-[9px] font-bold px-1 py-0.2 rounded bg-zinc-700/50 text-zinc-400 border border-zinc-700 shrink-0">
            SKIP
          </span>
        );
      case "error":
        return (
          <span className="text-[9px] font-bold px-1 py-0.2 rounded bg-rose-500/20 text-rose-300 border border-rose-500/30 shrink-0">
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
    showToast(`Switched MCP Command Mode to ${mode === "absolute" ? "Absolute Executable Path" : "System PATH ('knox')"}`);
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

  const refreshConfigs = useCallback(async (deep = false, openTerminal = false) => {
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
        showToast(`Deep PC Scan complete! Found ${combined.length} configuration paths.`);
      }
    } catch (e) {
      console.error("Failed to scan MCP configs:", e);
      showToast(String(e), true);
    } finally {
      setIsScanning(false);
      setIsDeepScanning(false);
    }
  }, [showToast]);

  useEffect(() => {
    refreshConfigs(false);
  }, [refreshConfigs]);

  const handleInject = async (app: DetectedMcpApp) => {
    setActionLoadingId(app.id);
    try {
      const useSystem = commandMode === "system";
      await KnoxAPI.injectMcpConfig(app.file_path, useSystem);
      showToast(`Added Knox to ${app.name} (${useSystem ? "knox" : "absolute path"})!`);
      // Update state locally
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
        // Attempt to extract suggested name from path
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
          (c) => c.file_path.toLowerCase() !== newCustomApp.file_path.toLowerCase()
        ),
        newCustomApp,
      ];
      saveCustomApps(updatedCustom);

      // Auto-inject Knox immediately
      await KnoxAPI.injectMcpConfig(newCustomApp.file_path, commandMode === "system");
      newCustomApp.is_configured = true;

      setApps((prev) => [
        newCustomApp,
        ...prev.filter(
          (a) => a.file_path.toLowerCase() !== newCustomApp.file_path.toLowerCase()
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
    const cmd = commandMode === "system" ? "knox" : "D:\\Knox\\target\\release\\knox.exe";
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

  const handleCopy = (text: string, label: string) => {
    navigator.clipboard.writeText(text);
    setCopied(true);
    showToast(`Copied ${label} to clipboard!`);
    setTimeout(() => setCopied(false), 2000);
  };

  const toggleTool = (toolName: string) => {
    setExpandedTool(expandedTool === toolName ? null : toolName);
  };

  const MAX_COLLAPSED_APPS = 4;
  const visibleApps = showAllApps ? apps : apps.slice(0, MAX_COLLAPSED_APPS);
  const hiddenAppsCount = Math.max(0, apps.length - MAX_COLLAPSED_APPS);

  return (
    <div className={`p-6 space-y-6 ${showTerminal ? "max-w-7xl" : "max-w-4xl"} mx-auto overflow-y-auto h-full select-none transition-all duration-300`}>
      {/* View Header */}
      <div>
        <h1 className="text-xl font-bold tracking-tight text-foreground flex items-center gap-2">
          <PlugZap className="w-5 h-5 text-primary" />
          <span>Model Context Protocol (MCP) Server</span>
        </h1>
        <p className="text-xs text-muted-foreground mt-1">
          Auto-detect, directly inject, and manage the Knox stdio MCP bridge across all your IDEs, coding tools, and private applications.
        </p>
      </div>

      <div className={`flex flex-col ${showTerminal ? "xl:flex-row items-start gap-6" : ""}`}>
        {/* Left Column: Configurator & Reference */}
        <div className={`space-y-6 min-w-0 ${showTerminal ? "w-full xl:flex-1" : "w-full"}`}>
          {/* ── SECTION 1: AUTOMATED DISCOVERY & 1-CLICK INJECTION RAIL ── */}
          <div className="p-5 rounded-xl border border-border bg-card space-y-4 shadow-xs">
            <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-3 border-b border-border/60 pb-4">
              <div>
                <h2 className="text-sm font-semibold text-foreground flex items-center gap-2">
                  <Sliders className="w-4 h-4 text-primary" />
                  <span>IDE & Application Auto-Configurator</span>
                </h2>
                <p className="text-xs text-muted-foreground mt-0.5">
                  1-Click injection of Knox stdio server into detected configuration files.
                </p>
              </div>

              {/* Action Bar */}
              <div className="flex flex-wrap items-center gap-2">
                <button
                  onClick={() => refreshConfigs(false, true)}
                  disabled={isScanning || isDeepScanning}
                  className="flex items-center gap-1.5 px-3 py-1.5 rounded-lg border border-border bg-background hover:bg-muted text-xs font-medium text-foreground transition cursor-pointer disabled:opacity-50"
                  title="Quick scan of standard IDE directories"
                >
                  <RefreshCw className={`w-3.5 h-3.5 ${isScanning ? "animate-spin text-primary" : ""}`} />
                  <span>{isScanning ? "Scanning..." : "Quick Scan"}</span>
                </button>

                <button
                  onClick={() => refreshConfigs(true, true)}
                  disabled={isScanning || isDeepScanning}
                  className="flex items-center gap-1.5 px-3 py-1.5 rounded-lg border border-border bg-background hover:bg-muted text-xs font-medium text-foreground transition cursor-pointer disabled:opacity-50"
                  title="Deep scan across all drive letters (C:\, D:\, etc.)"
                >
                  <HardDrive className={`w-3.5 h-3.5 ${isDeepScanning ? "animate-pulse text-indigo-400" : ""}`} />
                  <span>{isDeepScanning ? "Searching Drives..." : "Deep Search PC"}</span>
                </button>

                <button
                  onClick={() => setShowTerminal((prev) => !prev)}
                  className={`flex items-center gap-1.5 px-3 py-1.5 rounded-lg border text-xs font-medium transition cursor-pointer ${
                    showTerminal
                      ? "bg-primary/20 border-primary/50 text-primary shadow-xs font-semibold"
                      : "border-border bg-background hover:bg-muted text-foreground"
                  }`}
                  title={showTerminal ? "Hide Live Scan Terminal" : "Open Live Scan Terminal"}
                >
                  <Terminal className="w-3.5 h-3.5" />
                  <span>Live Logs</span>
                  <ChevronDown className={`w-3 h-3 transition-transform duration-200 ${showTerminal ? "rotate-180" : ""}`} />
                </button>

                <button
                  onClick={() => setShowCustomModal(true)}
                  className="flex items-center gap-1.5 px-3 py-1.5 rounded-lg bg-primary hover:bg-primary/90 text-primary-foreground text-xs font-semibold shadow-xs transition cursor-pointer"
                >
                  <Plus className="w-3.5 h-3.5" />
                  <span>Add Custom App</span>
                </button>
              </div>
            </div>

        {/* Command Path Mode Toggle */}
        <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-3 p-3 rounded-lg bg-background/60 border border-border/60 text-xs">
          <div className="space-y-0.5">
            <span className="font-semibold text-foreground flex items-center gap-1.5">
              Command Path Injection Mode:
            </span>
            <p className="text-[11px] text-muted-foreground">
              {commandMode === "absolute"
                ? "Writes full executable path (100% fail-proof, no Windows PATH setup required)."
                : "Writes system command 'knox' (ideal if syncing configs across multiple PCs via Git)."}
            </p>
          </div>

          <div className="inline-flex rounded-lg border border-border bg-card p-0.5 shrink-0">
            <button
              onClick={() => saveCommandMode("absolute")}
              className={`px-3 py-1 rounded-md text-[11px] font-medium transition cursor-pointer ${
                commandMode === "absolute"
                  ? "bg-primary text-primary-foreground shadow-xs"
                  : "text-muted-foreground hover:text-foreground"
              }`}
            >
              Absolute Path (Recommended)
            </button>
            <button
              onClick={() => saveCommandMode("system")}
              className={`px-3 py-1 rounded-md text-[11px] font-medium transition cursor-pointer ${
                commandMode === "system"
                  ? "bg-primary text-primary-foreground shadow-xs"
                  : "text-muted-foreground hover:text-foreground"
              }`}
            >
              System PATH ('knox')
            </button>
          </div>
        </div>

        {/* Detected Applications List */}
        <div className="space-y-2">
          {apps.length === 0 ? (
            <div className="py-8 text-center space-y-2 border border-dashed border-border/80 rounded-xl p-4">
              <Search className="w-6 h-6 text-muted-foreground/40 mx-auto" />
              <p className="text-xs font-medium text-foreground">No applications detected yet</p>
              <p className="text-[11px] text-muted-foreground">
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
                    className="flex flex-col sm:flex-row sm:items-center justify-between p-3 rounded-xl border border-border/70 bg-card hover:bg-card/80 transition gap-3"
                  >
                    {/* Left: App Metadata */}
                    <div className="min-w-0 flex-1 space-y-1">
                      <div className="flex items-center gap-2 flex-wrap">
                        <span className="font-semibold text-xs text-foreground">
                          {app.name}
                        </span>
                        <span className="text-[10px] font-mono px-1.5 py-0.2 rounded bg-muted/60 text-muted-foreground uppercase">
                          {app.category}
                        </span>
                        {app.is_custom && (
                          <span className="text-[10px] font-mono px-1.5 py-0.2 rounded bg-purple-500/10 text-purple-400 border border-purple-500/20">
                            Custom App
                          </span>
                        )}

                        {/* Status Badges */}
                        {app.is_configured ? (
                          <span className="inline-flex items-center gap-1 text-[10px] font-mono px-2 py-0.5 rounded-full bg-emerald-500/15 text-emerald-400 border border-emerald-500/30 font-medium">
                            <Check className="w-3 h-3" />
                            Configured
                          </span>
                        ) : app.exists ? (
                          <span className="inline-flex items-center gap-1 text-[10px] font-mono px-2 py-0.5 rounded-full bg-amber-500/15 text-amber-400 border border-amber-500/30">
                            Detected
                          </span>
                        ) : (
                          <span className="text-[10px] font-mono px-2 py-0.5 rounded-full bg-muted/40 text-muted-foreground border border-border/40">
                            Not Installed
                          </span>
                        )}
                      </div>

                      <p className="font-mono text-[11px] text-muted-foreground truncate" title={app.file_path}>
                        {app.file_path}
                      </p>
                    </div>

                    {/* Right: Actions */}
                    <div className="flex items-center gap-2 shrink-0">
                      {app.is_configured ? (
                        <button
                          onClick={() => handleRemove(app)}
                          disabled={isLoading}
                          className="px-3 py-1.5 rounded-lg border border-rose-500/30 bg-rose-500/10 hover:bg-rose-500/20 text-rose-400 text-xs font-medium transition cursor-pointer disabled:opacity-50"
                        >
                          {isLoading ? "Removing..." : "Remove Knox"}
                        </button>
                      ) : (
                        <button
                          onClick={() => handleInject(app)}
                          disabled={isLoading}
                          className="px-3 py-1.5 rounded-lg bg-emerald-600 hover:bg-emerald-500 text-white text-xs font-semibold shadow-xs transition cursor-pointer disabled:opacity-50"
                        >
                          {isLoading ? "Adding..." : "Add Knox"}
                        </button>
                      )}

                      {app.is_custom && (
                        <button
                          onClick={(e) => handleDeleteCustomApp(app.id, e)}
                          className="p-1.5 rounded-lg text-muted-foreground hover:text-rose-400 hover:bg-muted/40 transition cursor-pointer"
                          title="Remove custom application from list"
                        >
                          <Trash2 className="w-3.5 h-3.5" />
                        </button>
                      )}
                    </div>
                  </div>
                );
              })}

              {/* "Show More" / "Show Less" Down Arrow Button */}
              {hiddenAppsCount > 0 && (
                <button
                  onClick={() => setShowAllApps((prev) => !prev)}
                  className="w-full py-2 px-3 rounded-xl border border-dashed border-border/80 bg-background/50 hover:bg-muted/50 text-xs font-medium text-foreground transition cursor-pointer flex items-center justify-center gap-1.5 mt-2"
                >
                  {showAllApps ? (
                    <>
                      <ChevronUp className="w-3.5 h-3.5 text-primary" />
                      <span>Show Less (Displaying all {apps.length} applications)</span>
                    </>
                  ) : (
                    <>
                      <ChevronDown className="w-3.5 h-3.5 text-primary" />
                      <span>Show More ({hiddenAppsCount} more detected application{hiddenAppsCount > 1 ? "s" : ""})</span>
                    </>
                  )}
                </button>
              )}
            </>
          )}
        </div>
      </div>

      {/* ── CUSTOM / PRIVATE APP MODAL ── */}
      {showCustomModal && (
        <div className="fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/75 backdrop-blur-xs animate-in fade-in duration-150">
          <div className="w-full max-w-lg bg-card border border-border rounded-2xl shadow-2xl p-6 space-y-4">
            <div className="flex items-center justify-between border-b border-border/60 pb-3">
              <h3 className="text-sm font-semibold text-foreground flex items-center gap-2">
                <FileCode className="w-4 h-4 text-primary" />
                <span>Configure Custom or Private Application</span>
              </h3>
              <button
                onClick={() => setShowCustomModal(false)}
                className="text-muted-foreground hover:text-foreground text-xs cursor-pointer p-1"
              >
                ✕
              </button>
            </div>

            <p className="text-xs text-muted-foreground leading-relaxed">
              Knox will auto-detect the MCP schema (`mcpServers`, `mcp.servers`, or `context_servers`), create a safe `.bak` backup, and inject the Knox server entry directly.
            </p>

            <div className="space-y-3 text-xs">
              <div>
                <label className="block text-[11px] font-semibold text-foreground mb-1">
                  Application Name
                </label>
                <input
                  type="text"
                  placeholder="e.g. My Studio Tool, Roo Code, Custom IDE"
                  value={customName}
                  onChange={(e) => setCustomName(e.target.value)}
                  className="w-full px-3 py-2 rounded-lg bg-background border border-border focus:border-primary focus:outline-hidden text-foreground text-xs"
                />
              </div>

              <div>
                <label className="block text-[11px] font-semibold text-foreground mb-1">
                  Configuration File Path (.json)
                </label>
                <div className="flex gap-2">
                  <input
                    type="text"
                    placeholder="e.g. C:\Users\...\settings.json"
                    value={customPath}
                    onChange={(e) => setCustomPath(e.target.value)}
                    className="flex-1 px-3 py-2 rounded-lg bg-background border border-border focus:border-primary focus:outline-hidden text-foreground text-xs font-mono"
                  />
                  <button
                    onClick={handleBrowseCustomFile}
                    className="flex items-center gap-1.5 px-3 py-2 rounded-lg border border-border bg-background hover:bg-muted text-xs font-medium text-foreground cursor-pointer transition shrink-0"
                  >
                    <FolderOpen className="w-3.5 h-3.5 text-primary" />
                    <span>Browse...</span>
                  </button>
                </div>
              </div>
            </div>

            <div className="pt-2 flex items-center justify-end gap-2 border-t border-border/60">
              <button
                onClick={() => setShowCustomModal(false)}
                className="px-3.5 py-1.5 rounded-lg text-xs font-medium text-muted-foreground hover:text-foreground hover:bg-muted/40 cursor-pointer transition"
              >
                Cancel
              </button>
              <button
                onClick={handleSaveCustomApp}
                disabled={customLoading || !customPath.trim()}
                className="flex items-center gap-1.5 px-4 py-2 rounded-xl text-xs font-semibold bg-primary hover:bg-primary/90 text-primary-foreground shadow-xs cursor-pointer transition disabled:opacity-50"
              >
                <Check className="w-3.5 h-3.5" />
                <span>{customLoading ? "Saving..." : "Save & Configure Knox"}</span>
              </button>
            </div>
          </div>
        </div>
      )}

      {/* ── SECTION 2: PROTOCOL STATUS & MANUAL REFERENCE SNIPPETS ── */}
      <div className="p-5 rounded-xl border border-border bg-card space-y-4">
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-2">
            <div className="w-2.5 h-2.5 rounded-full bg-emerald-400 animate-pulse" />
            <span className="text-xs font-semibold text-foreground">
              Server Transport: Standard I/O (stdio)
            </span>
          </div>
          <div className="flex items-center gap-2">
            <span className="text-[10px] font-mono px-2 py-0.5 rounded bg-primary/10 text-primary border border-primary/20">
              {MCP_TOOLS.length} Tools Registered
            </span>
            <span className="text-[10px] font-mono px-2 py-0.5 rounded bg-emerald-500/10 text-emerald-400 border border-emerald-500/20">
              JSON-RPC 2.0
            </span>
          </div>
        </div>

        {/* Command line invocation */}
        <div className="p-3 rounded-lg bg-background font-mono text-xs text-indigo-400 border border-border/80 flex items-center justify-between">
          <div className="flex items-center gap-2">
            <Terminal className="w-3.5 h-3.5 text-muted-foreground" />
            <code>knox serve --stdio</code>
          </div>
          <button
            onClick={() => handleCopy("knox serve --stdio", "launch command")}
            className="flex items-center gap-1 text-xs text-muted-foreground hover:text-foreground cursor-pointer transition"
            title="Copy command"
          >
            <Copy className="w-3.5 h-3.5" />
            <span className="text-[11px]">Copy</span>
          </button>
        </div>

        <p className="text-[11px] text-muted-foreground/90 bg-background/50 p-2.5 rounded-lg border border-border/50 leading-relaxed">
          <span className="font-semibold text-foreground">Client-Managed Stdio:</span> Standard I/O processes are spawned on-demand by your IDE or desktop agent client upon workspace activation and directly query Knox SQLite.
        </p>

        {/* Manual Copyable Snippet Tabs */}
        <div className="space-y-3 pt-2">
          <div className="flex items-center justify-between border-b border-border/60 pb-2">
            <div className="flex items-center gap-1">
              {(["cursor", "claude", "antigravity", "cli"] as ConfigTarget[]).map((target) => (
                <button
                  key={target}
                  onClick={() => setActiveTarget(target)}
                  className={`px-3 py-1 rounded-lg text-xs font-medium transition cursor-pointer capitalize ${
                    activeTarget === target
                      ? "bg-primary text-primary-foreground shadow-xs"
                      : "text-muted-foreground hover:text-foreground hover:bg-background"
                  }`}
                >
                  {target === "cli" ? "CLI Command" : target}
                </button>
              ))}
            </div>

            <button
              onClick={() => handleCopy(currentSnippet, `${activeTarget} config`)}
              className="flex items-center gap-1.5 px-3 py-1 rounded-lg bg-card hover:bg-background border border-border text-xs text-foreground cursor-pointer transition"
            >
              {copied ? (
                <>
                  <Check className="w-3 h-3 text-emerald-400" />
                  <span className="text-emerald-400">Copied</span>
                </>
              ) : (
                <>
                  <Copy className="w-3 h-3" />
                  <span>Copy Configuration</span>
                </>
              )}
            </button>
          </div>

          <div className="space-y-1">
            <div className="flex items-center justify-between text-[11px] text-muted-foreground">
              <span>Target File Location:</span>
              <span className="font-mono text-foreground text-[10px]">{getTargetLocation(activeTarget)}</span>
            </div>
            <pre className="p-3.5 rounded-lg bg-background font-mono text-xs text-foreground border border-border/80 overflow-x-auto leading-relaxed select-text">
              {currentSnippet}
            </pre>
          </div>
        </div>
      </div>

      {/* ── SECTION 3: 9 REGISTERED MCP TOOLS ACCORDION ── */}
      <div className="space-y-3">
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-2">
            <PlugZap className="w-4 h-4 text-primary" />
            <span className="text-sm font-semibold tracking-tight text-foreground">
              Registered MCP Tools ({MCP_TOOLS.length})
            </span>
          </div>
          <span className="text-xs text-muted-foreground">
            Click any tool to inspect parameter schemas
          </span>
        </div>

        <div className="space-y-2">
          {MCP_TOOLS.map((tool) => {
            const isExpanded = expandedTool === tool.name;
            return (
              <div
                key={tool.name}
                className="rounded-xl border border-border/70 bg-card overflow-hidden transition"
              >
                {/* Tool Row Header */}
                <button
                  onClick={() => toggleTool(tool.name)}
                  className="w-full p-3.5 flex items-center justify-between hover:bg-background/40 transition cursor-pointer text-left"
                >
                  <div className="flex items-center gap-2.5 min-w-0">
                    {isExpanded ? (
                      <ChevronDown className="w-4 h-4 text-primary shrink-0" />
                    ) : (
                      <ChevronRight className="w-4 h-4 text-muted-foreground shrink-0" />
                    )}
                    <span className="font-mono font-bold text-foreground text-xs">
                      {tool.name}
                    </span>
                    <span className="text-xs text-muted-foreground truncate hidden sm:inline">
                      — {tool.description}
                    </span>
                  </div>

                  <span
                    className={`text-[9px] px-2 py-0.5 rounded font-mono uppercase shrink-0 ${
                      tool.category === "Write"
                        ? "bg-emerald-500/10 text-emerald-400 border border-emerald-500/20"
                        : tool.category === "Read"
                        ? "bg-indigo-500/10 text-indigo-400 border border-indigo-500/20"
                        : tool.category === "Query"
                        ? "bg-purple-500/10 text-purple-400 border border-purple-500/20"
                        : tool.category === "Scanner"
                        ? "bg-amber-500/10 text-amber-400 border border-amber-500/20"
                        : "bg-cyan-500/10 text-cyan-400 border border-cyan-500/20"
                    }`}
                  >
                    {tool.category}
                  </span>
                </button>

                {/* Expanded Schema & Parameter Details */}
                {isExpanded && (
                  <div className="p-4 pt-1 border-t border-border/50 bg-background/40 space-y-3 text-xs">
                    <p className="text-foreground text-xs leading-relaxed">
                      {tool.description}
                    </p>

                    {/* Parameters Table */}
                    <div className="space-y-1.5">
                      <span className="text-[10px] font-semibold text-muted-foreground uppercase tracking-wider">
                        Parameters
                      </span>
                      <div className="border border-border/60 rounded-lg overflow-hidden">
                        <table className="w-full text-left text-xs divide-y divide-border/60">
                          <thead className="bg-card text-[10px] font-mono text-muted-foreground uppercase">
                            <tr>
                              <th className="px-3 py-1.5 font-medium">Name</th>
                              <th className="px-3 py-1.5 font-medium">Type</th>
                              <th className="px-3 py-1.5 font-medium">Required</th>
                              <th className="px-3 py-1.5 font-medium">Description</th>
                            </tr>
                          </thead>
                          <tbody className="divide-y divide-border/40 text-[11px] font-mono">
                            {tool.parameters.map((param) => (
                              <tr key={param.name} className="hover:bg-card/30">
                                <td className="px-3 py-1.5 text-primary font-bold">{param.name}</td>
                                <td className="px-3 py-1.5 text-foreground/80">{param.type}</td>
                                <td className="px-3 py-1.5">
                                  {param.required ? (
                                    <span className="text-rose-400 text-[10px]">required</span>
                                  ) : (
                                    <span className="text-muted-foreground text-[10px]">optional</span>
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

                    {/* Schema Example JSON */}
                    <div className="space-y-1 pt-1">
                      <div className="flex items-center justify-between">
                        <span className="text-[10px] font-semibold text-muted-foreground uppercase tracking-wider">
                          Sample Invocation Payload
                        </span>
                        <button
                          onClick={() =>
                            handleCopy(JSON.stringify(tool.schemaExample, null, 2), "payload")
                          }
                          className="text-[10px] text-primary hover:underline cursor-pointer flex items-center gap-1 font-mono"
                        >
                          <Copy className="w-3 h-3" />
                          <span>Copy Payload</span>
                        </button>
                      </div>
                      <pre className="p-2.5 rounded-lg bg-background border border-border/60 font-mono text-[11px] text-muted-foreground overflow-x-auto leading-relaxed select-text">
                        {JSON.stringify(tool.schemaExample, null, 2)}
                      </pre>
                    </div>
                  </div>
                )}
              </div>
            );
          })}
        </div>
      </div>
      {/* End of Section 3 */}
      </div>
      {/* End of Left Column */}

      {/* ── RIGHT COLUMN: LIVE SCAN ACTIVITY TERMINAL ── */}
      {showTerminal && (
        <div className="w-full xl:w-[460px] shrink-0 sticky top-4 rounded-2xl border border-border/80 bg-zinc-950/95 backdrop-blur-md shadow-2xl p-4 flex flex-col h-[740px] animate-in fade-in slide-in-from-right-4 duration-200">
          {/* Terminal Header */}
          <div className="flex items-center justify-between pb-3 border-b border-zinc-800 shrink-0">
            <div className="flex items-center gap-2">
              <div
                className={`w-2.5 h-2.5 rounded-full ${
                  isScanning || isDeepScanning
                    ? "bg-amber-400 animate-ping"
                    : "bg-emerald-400"
                }`}
              />
              <Terminal className="w-4 h-4 text-primary" />
              <span className="text-xs font-bold text-zinc-100 font-mono tracking-tight">
                Scan Activity Log
              </span>
            </div>

            <div className="flex items-center gap-1.5">
              <button
                onClick={() => setAutoScroll(!autoScroll)}
                className={`px-2 py-0.5 rounded text-[10px] font-mono border transition cursor-pointer ${
                  autoScroll
                    ? "bg-primary/20 text-primary border-primary/40 font-semibold"
                    : "bg-zinc-800 text-zinc-400 border-zinc-700"
                }`}
                title="Toggle auto-scrolling to bottom"
              >
                Scroll: {autoScroll ? "ON" : "OFF"}
              </button>

              <button
                onClick={handleCopyLogs}
                className="p-1.5 rounded-md text-zinc-400 hover:text-zinc-100 hover:bg-zinc-800 transition cursor-pointer"
                title="Copy logs to clipboard"
              >
                <Copy className="w-3.5 h-3.5" />
              </button>

              <button
                onClick={handleClearLogs}
                className="p-1.5 rounded-md text-zinc-400 hover:text-zinc-100 hover:bg-zinc-800 transition cursor-pointer"
                title="Clear console"
              >
                <Trash2 className="w-3.5 h-3.5" />
              </button>

              <button
                onClick={() => setShowTerminal(false)}
                className="p-1.5 rounded-md text-zinc-400 hover:text-rose-400 hover:bg-zinc-800 transition cursor-pointer ml-1"
                title="Close terminal panel"
              >
                <X className="w-3.5 h-3.5" />
              </button>
            </div>
          </div>

          {/* Terminal Console Output */}
          <div
            ref={logContainerRef}
            className="flex-1 overflow-y-auto font-mono text-[11px] p-2 space-y-1.5 scrollbar-thin select-text my-2"
          >
            {logs.length === 0 ? (
              <div className="h-full flex flex-col items-center justify-center text-center p-6 text-zinc-500 space-y-2">
                <Terminal className="w-8 h-8 opacity-40 mx-auto" />
                <p className="text-xs font-medium text-zinc-400">Terminal Ready</p>
                <p className="text-[10px] text-zinc-500 max-w-[240px]">
                  Click Quick Scan or Deep Search PC to stream real-time filesystem discovery events here.
                </p>
              </div>
            ) : (
              logs.map((log, i) => (
                <div key={i} className="leading-tight flex items-start gap-1.5 text-zinc-300 break-all">
                  <span className="text-zinc-500 shrink-0 text-[10px]">{log.timestamp}</span>
                  {renderLogLevelBadge(log.level)}
                  <span className="flex-1 text-zinc-300">{log.message}</span>
                </div>
              ))
            )}
          </div>

          {/* Terminal Status Footer */}
          <div className="pt-2 border-t border-zinc-800/80 flex items-center justify-between text-[10px] text-zinc-400 font-mono shrink-0">
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
    {/* End of 2-Column Split */}
    </div>
  );
};
