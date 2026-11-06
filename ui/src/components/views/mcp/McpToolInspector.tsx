import React from "react";
import {
  PlugZap,
  ChevronDown,
  ChevronRight,
  Layers,
  Copy,
  ShieldCheck,
  Sparkles,
  BookOpen,
  GitFork,
} from "lucide-react";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/Card";
import { Button } from "@/components/ui/Button";
import { Badge } from "@/components/ui/Badge";
import { CodeBlock } from "@/components/ui/CodeBlock";
import {
  type McpTool,
  type CategoryFilter,
  CATEGORY_BADGE,
  MCP_RESOURCES,
  MCP_PROMPTS,
} from "./mcpData";

interface McpToolInspectorProps {
  filteredTools: McpTool[];
  categories: CategoryFilter[];
  categoryFilter: CategoryFilter;
  setCategoryFilter: (cat: CategoryFilter) => void;
  expandedTool: string | null;
  setExpandedTool: (name: string | null) => void;
  showToast: (msg: string, isError?: boolean) => void;
}

export const McpToolInspector: React.FC<McpToolInspectorProps> = ({
  filteredTools,
  categories,
  categoryFilter,
  setCategoryFilter,
  expandedTool,
  setExpandedTool,
  showToast,
}) => {
  return (
    <div className="space-y-6">
      {/* SECTION 1: REGISTERED MCP TOOLS ACCORDION */}
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
                              <th className="px-3 py-1.5 font-medium">Required</th>
                              <th className="px-3 py-1.5 font-medium">Description</th>
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
                                    <span className="text-danger text-xs">required</span>
                                  ) : (
                                    <span className="text-muted-foreground text-xs">optional</span>
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
                      <span className="text-overline">Sample invocation payload</span>
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

      {/* SECTION 2: NATIVE MCP RESOURCES */}
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
              </div>
            ))}
          </div>
        </CardContent>
      </Card>

      {/* SECTION 3: NATIVE MCP PROMPTS */}
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

      {/* SECTION 4: SMART AUTO-SESSIONS & CONTEXT OPTIMIZATION */}
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
  );
};
