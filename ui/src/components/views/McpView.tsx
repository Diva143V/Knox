import React, { useState } from "react";
import {
  PlugZap,
  Copy,
  Check,
  Terminal,
  ChevronDown,
  ChevronRight,
} from "lucide-react";
import { useKnoxStore } from "@/store/useKnoxStore";

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

export const McpView: React.FC = () => {
  const { showToast } = useKnoxStore();
  const [activeTarget, setActiveTarget] = useState<ConfigTarget>("cursor");
  const [copied, setCopied] = useState(false);
  const [expandedTool, setExpandedTool] = useState<string | null>("knox_get_context");

  const getConfigSnippet = (target: ConfigTarget): string => {
    switch (target) {
      case "cursor":
        return JSON.stringify(
          {
            mcpServers: {
              knox: {
                command: "knox",
                args: ["serve", "--stdio"],
              },
            },
          },
          null,
          2
        );
      case "claude":
        return JSON.stringify(
          {
            mcpServers: {
              knox: {
                command: "knox",
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
                  command: "knox",
                  args: ["serve", "--stdio"],
                },
              },
            },
          },
          null,
          2
        );
      case "cli":
        return "knox serve --stdio";
    }
  };

  const getTargetLocation = (target: ConfigTarget): string => {
    switch (target) {
      case "cursor":
        return "~/.cursor/mcp.json";
      case "claude":
        return "%APPDATA%\\Claude\\claude_desktop_config.json";
      case "antigravity":
        return "~/.gemini/antigravity/mcp_servers.json";
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

  return (
    <div className="p-6 space-y-6 max-w-4xl mx-auto overflow-y-auto h-full select-none">
      {/* Header */}
      <div>
        <h1 className="text-xl font-bold tracking-tight text-foreground flex items-center gap-2">
          <PlugZap className="w-5 h-5 text-primary" />
          <span>Model Context Protocol (MCP) Server</span>
        </h1>
        <p className="text-xs text-muted-foreground mt-1">
          Knox exposes a universal stdio MCP server for Cursor, Antigravity, Claude Desktop, and
          Copilot, providing agents direct access to the causal knowledge graph.
        </p>
      </div>

      {/* MCP Status & Transport Card */}
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
              9 Tools Registered
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

        {/* Integration Preset Tabs */}
        <div className="space-y-3 pt-2">
          <div className="flex items-center justify-between border-b border-border/60 pb-2">
            <div className="flex items-center gap-1">
              {(["cursor", "claude", "antigravity", "cli"] as ConfigTarget[]).map((target) => (
                <button
                  key={target}
                  onClick={() => setActiveTarget(target)}
                  className={`px-3 py-1 rounded-lg text-xs font-medium transition cursor-pointer capitalize ${
                    activeTarget === target
                      ? "bg-primary text-primary-foreground shadow-sm"
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

      {/* 9 Registered MCP Tools Accordion */}
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
    </div>
  );
};
