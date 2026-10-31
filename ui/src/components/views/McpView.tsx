import React, { useMemo, useState } from "react";
import {
  PlugZap,
  Terminal,
  ChevronDown,
  ChevronRight,
  Info,
} from "lucide-react";
import { useKnoxStore } from "@/store/useKnoxStore";
import { PageHeader } from "@/components/ui/PageHeader";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/Card";
import { Button } from "@/components/ui/Button";
import { Badge } from "@/components/ui/Badge";
import { CodeBlock } from "@/components/ui/CodeBlock";
import { cn } from "@/lib/cn";

interface McpTool {
  name: string;
  category: "Read" | "Write" | "Query" | "Scanner" | "Lifecycle";
  description: string;
  parameters: {
    name: string;
    type: string;
    required: boolean;
    description: string;
  }[];
  schemaExample: Record<string, unknown>;
}

const MCP_TOOLS: McpTool[] = [
  {
    name: "knox_get_context",
    category: "Read",
    description:
      "Returns active, non-superseded decisions, plans, and actions for a file or project.",
    parameters: [
      {
        name: "file_path",
        type: "string",
        required: false,
        description: "Specific repository file path",
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
    ],
    schemaExample: { file_path: "src/parser.rs", current_only: true },
  },
  {
    name: "knox_record_action",
    category: "Write",
    description:
      "Records an atomic agent action (write, debug, read, review) linked to the active session.",
    parameters: [
      {
        name: "session_id",
        type: "string",
        required: true,
        description: "ID of the enclosing session",
      },
      {
        name: "actor",
        type: "string",
        required: true,
        description: "Agent name (e.g. gemini, cursor)",
      },
      {
        name: "action_type",
        type: "string",
        required: true,
        description: "Action type: write, debug, etc.",
      },
      {
        name: "summary",
        type: "string",
        required: true,
        description: "Concise summary of action performed",
      },
      {
        name: "files",
        type: "string[]",
        required: false,
        description: "Touched file paths",
      },
      {
        name: "tags",
        type: "string[]",
        required: false,
        description: "Topic tags",
      },
      {
        name: "confidence",
        type: "number",
        required: false,
        description: "Confidence score (0.0 to 1.0)",
      },
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
    description:
      "Records an architectural or design decision, optionally superseding an earlier approach.",
    parameters: [
      {
        name: "session_id",
        type: "string",
        required: true,
        description: "ID of the enclosing session",
      },
      {
        name: "actor",
        type: "string",
        required: true,
        description: "Agent or human making decision",
      },
      {
        name: "summary",
        type: "string",
        required: true,
        description: "Decision rationale and outcome",
      },
      {
        name: "supersedes",
        type: "string",
        required: false,
        description: "Action ID of prior decision superseded",
      },
      {
        name: "files",
        type: "string[]",
        required: false,
        description: "Associated file paths",
      },
    ],
    schemaExample: {
      session_id: "s-001",
      actor: "cursor",
      summary:
        "Chose SQLite with WAL mode over flat files for zero-copy concurrency",
      supersedes: "a-002",
    },
  },
  {
    name: "knox_get_history",
    category: "Read",
    description:
      "Retrieves complete append-only history for a file or project, including superseded actions.",
    parameters: [
      {
        name: "file_path",
        type: "string",
        required: false,
        description: "File path to trace historical decisions",
      },
      {
        name: "project",
        type: "string",
        required: false,
        description: "Project identifier",
      },
      {
        name: "limit",
        type: "number",
        required: false,
        description: "Maximum history items to return",
      },
    ],
    schemaExample: { file_path: "src/main.rs", limit: 25 },
  },
  {
    name: "knox_search",
    category: "Query",
    description:
      "Executes SQLite FTS5 full-text keyword search across all action summaries, tags, and files.",
    parameters: [
      {
        name: "query",
        type: "string",
        required: true,
        description: "FTS5 search phrase or keywords",
      },
      {
        name: "limit",
        type: "number",
        required: false,
        description: "Maximum matching actions (default: 50)",
      },
    ],
    schemaExample: { query: "nom parser grammar" },
  },
  {
    name: "knox_list_stale",
    category: "Query",
    description:
      "Lists all superseded and outdated actions for inspecting previous architectural iterations.",
    parameters: [
      {
        name: "limit",
        type: "number",
        required: false,
        description: "Limit number of stale records",
      },
    ],
    schemaExample: { limit: 50 },
  },
  {
    name: "knox_scan_staged",
    category: "Scanner",
    description:
      "Scans git staged index changes to attribute unrecorded manual developer edits to human.",
    parameters: [
      {
        name: "project",
        type: "string",
        required: false,
        description: "Git repository root directory (default: .)",
      },
    ],
    schemaExample: { project: "." },
  },
  {
    name: "knox_start_session",
    category: "Lifecycle",
    description:
      "Starts a continuous agent work session and returns a new session_id.",
    parameters: [
      {
        name: "actor",
        type: "string",
        required: true,
        description: "Agent name (e.g. gemini, cursor)",
      },
      {
        name: "project",
        type: "string",
        required: false,
        description: "Project name or root path",
      },
    ],
    schemaExample: { actor: "gemini", project: "knox" },
  },
  {
    name: "knox_end_session",
    category: "Lifecycle",
    description:
      "Explicitly closes an active session and returns total actions recorded statistics.",
    parameters: [
      {
        name: "session_id",
        type: "string",
        required: true,
        description: "Session ID to terminate",
      },
    ],
    schemaExample: { session_id: "s-001" },
  },
];

type ConfigTarget = "cursor" | "claude" | "antigravity" | "cli";
type CategoryFilter = "All" | McpTool["category"];

const CATEGORY_BADGE: Record<McpTool["category"], BadgePropsVariant> = {
  Write: "success",
  Read: "default",
  Query: "plan",
  Scanner: "warning",
  Lifecycle: "secondary",
};

type BadgePropsVariant =
  | "default"
  | "secondary"
  | "success"
  | "warning"
  | "plan"
  | "outline"
  | "danger";

export const McpView: React.FC = () => {
  const { showToast } = useKnoxStore();
  const [activeTarget, setActiveTarget] = useState<ConfigTarget>("cursor");
  const [expandedTool, setExpandedTool] = useState<string | null>(
    "knox_get_context"
  );
  const [categoryFilter, setCategoryFilter] = useState<CategoryFilter>("All");

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

  const handleCopied = (label: string) => {
    showToast(`Copied ${label} to clipboard`);
  };

  const filteredTools = useMemo(() => {
    if (categoryFilter === "All") return MCP_TOOLS;
    return MCP_TOOLS.filter((t) => t.category === categoryFilter);
  }, [categoryFilter]);

  const categories: CategoryFilter[] = [
    "All",
    "Read",
    "Write",
    "Query",
    "Scanner",
    "Lifecycle",
  ];

  return (
    <div className="p-5 sm:p-6 space-y-5 max-w-4xl mx-auto overflow-y-auto h-full">
      <PageHeader
        title="MCP Configuration"
        description="Copy-paste setup for Knox’s stdio Model Context Protocol server. This screen documents tools and client config — it does not start or manage a live MCP process."
        icon={<PlugZap />}
      />

      <Card>
        <CardHeader className="pb-0">
          <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-2">
            <CardTitle className="flex items-center gap-2">
              <Terminal className="size-4 text-primary" aria-hidden />
              Client setup
            </CardTitle>
            <div className="flex flex-wrap items-center gap-2">
              <Badge variant="secondary">{MCP_TOOLS.length} tools documented</Badge>
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
              <code className="font-mono text-foreground">knox serve --stdio</code>{" "}
              on demand. Status below reflects documented capability, not a
              process this GUI is monitoring.
            </p>
          </div>

          <CodeBlock
            code="knox serve --stdio"
            language="shell"
            onCopied={() => handleCopied("launch command")}
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
                      "px-3 py-1.5 rounded-md text-xs font-medium capitalize transition-colors",
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
              onCopied={() => handleCopied(`${activeTarget} config`)}
            />
          </div>
        </CardContent>
      </Card>

      <div className="space-y-3">
        <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-2">
          <h2 className="text-sm font-semibold text-foreground flex items-center gap-2">
            <PlugZap className="size-4 text-primary" aria-hidden />
            MCP tools ({filteredTools.length})
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
                  className="w-full p-3.5 flex items-center justify-between hover:bg-muted/30 transition-colors text-left focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-focus-ring"
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
                    className="uppercase shrink-0"
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
                        onCopied={() => handleCopied("payload")}
                      />
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
