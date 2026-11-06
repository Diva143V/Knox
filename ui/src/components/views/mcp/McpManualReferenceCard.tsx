import React from "react";
import { Terminal, Info } from "lucide-react";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/Card";
import { Badge } from "@/components/ui/Badge";
import { CodeBlock } from "@/components/ui/CodeBlock";
import { cn } from "@/lib/cn";
import { type ConfigTarget, MCP_TOOLS } from "./mcpData";

interface McpManualReferenceCardProps {
  activeTarget: ConfigTarget;
  setActiveTarget: (target: ConfigTarget) => void;
  getTargetLocation: (target: ConfigTarget) => string;
  currentSnippet: string;
  showToast: (msg: string, isError?: boolean) => void;
}

export const McpManualReferenceCard: React.FC<McpManualReferenceCardProps> = ({
  activeTarget,
  setActiveTarget,
  getTargetLocation,
  currentSnippet,
  showToast,
}) => {
  return (
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
  );
};
