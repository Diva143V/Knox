import React from "react";
import { ExternalLink } from "lucide-react";
import { useKnoxStore } from "@/store/useKnoxStore";
import { Button } from "@/components/ui/Button";
import { cn } from "@/lib/cn";

export const McpTab: React.FC = () => {
  const { userSettings, updateUserSettings, closeSettings, setActiveTab } = useKnoxStore();

  return (
    <div className="space-y-6 animate-in fade-in-50 duration-150">
      <div>
        <h3 className="text-sm font-semibold text-foreground">AI Agents & MCP Configuration</h3>
        <p className="text-xs text-muted-foreground">
          Model Context Protocol server settings and integrations with Antigravity, Cursor, Claude Code, and Gemini
        </p>
      </div>

      <div className="space-y-4">
        {/* MCP Command Mode */}
        <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-3 p-3.5 rounded-lg border border-border bg-card/60">
          <div>
            <label className="text-xs font-medium text-foreground block">Command Invocation Mode</label>
            <span className="text-xs text-muted-foreground">
              Use "knox" if installed to system PATH, or full absolute executable path for portable installs
            </span>
          </div>
          <div className="flex items-center gap-1.5 shrink-0">
            <button
              type="button"
              onClick={() => updateUserSettings({ mcpCommandMode: "system" })}
              className={cn(
                "px-3 py-1.5 rounded-md text-xs font-medium border transition-all cursor-pointer font-mono",
                userSettings.mcpCommandMode === "system"
                  ? "bg-primary text-primary-foreground border-primary shadow-xs font-semibold"
                  : "bg-muted/70 text-muted-foreground border-border hover:text-foreground hover:bg-muted"
              )}
            >
              knox (System PATH)
            </button>
            <button
              type="button"
              onClick={() => updateUserSettings({ mcpCommandMode: "absolute" })}
              className={cn(
                "px-3 py-1.5 rounded-md text-xs font-medium border transition-all cursor-pointer font-mono",
                userSettings.mcpCommandMode === "absolute"
                  ? "bg-primary text-primary-foreground border-primary shadow-xs font-semibold"
                  : "bg-muted/70 text-muted-foreground border-border hover:text-foreground hover:bg-muted"
              )}
            >
              Absolute Path
            </button>
          </div>
        </div>

        {/* Pre-Flight Guardrails Enforcement */}
        <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-3 p-3.5 rounded-lg border border-border bg-card/60">
          <div>
            <label className="text-xs font-medium text-foreground block">Guardrails Enforcement</label>
            <span className="text-xs text-muted-foreground">
              Prevent AI coding agents from executing prohibited actions or rejected alternatives
            </span>
          </div>
          <select
            value={userSettings.guardrailsMode}
            onChange={(e) =>
              updateUserSettings({
                guardrailsMode: e.target.value as "strict" | "advisory" | "off",
              })
            }
            className="h-8 px-2.5 rounded-md border border-border bg-background text-xs font-mono text-foreground focus:outline-none focus:ring-1 focus:ring-primary cursor-pointer"
          >
            <option value="advisory">Advisory (Flag warning & allow)</option>
            <option value="strict">Strict (Fail recording on conflict)</option>
            <option value="off">Off (Disable checks)</option>
          </select>
        </div>

        {/* Markdown Token Digest Compression */}
        <div className="flex items-center justify-between p-3.5 rounded-lg border border-border bg-card/60">
          <div>
            <label className="text-xs font-medium text-foreground block">Token-Compact Markdown Digest</label>
            <span className="text-xs text-muted-foreground">
              Automatically compress MCP context queries into dense Markdown digests (saves &gt;70% context window tokens)
            </span>
          </div>
          <input
            type="checkbox"
            checked={userSettings.enableMarkdownDigest}
            onChange={(e) => updateUserSettings({ enableMarkdownDigest: e.target.checked })}
            className="size-4 rounded border-border text-primary focus:ring-primary cursor-pointer"
          />
        </div>

        {/* Link to MCP View */}
        <div className="p-3.5 rounded-lg border border-border bg-card/40 flex items-center justify-between">
          <div>
            <p className="text-xs font-medium text-foreground">Need to manage individual agent JSON snippets?</p>
            <p className="text-2xs text-muted-foreground">
              Open the full MCP Configuration Studio to inspect and inject custom configs
            </p>
          </div>
          <Button
            type="button"
            variant="primary"
            size="sm"
            onClick={() => {
              closeSettings();
              setActiveTab("mcp");
            }}
            className="gap-1 text-xs"
          >
            <span>Open MCP Studio</span>
            <ExternalLink className="size-3" />
          </Button>
        </div>
      </div>
    </div>
  );
};

