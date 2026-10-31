import React, { useState, useEffect, useCallback } from "react";
import {
  Shield,
  Database,
  PlugZap,
  CheckCircle2,
  Check,
  ChevronRight,
  ChevronLeft,
  X,
  FolderGit2,
  FileInput,
  FolderOpen,
  GitCommit,
} from "lucide-react";
import { useKnoxStore } from "@/store/useKnoxStore";
import { KnoxAPI } from "@/services/api";
import type { KnoxConfig } from "@/types/knox";
import { Button } from "@/components/ui/Button";
import { Badge } from "@/components/ui/Badge";
import { Input } from "@/components/ui/Input";
import { CodeBlock } from "@/components/ui/CodeBlock";
import { cn } from "@/lib/cn";

const AI_CONFIGS = [
  {
    id: "antigravity",
    name: "Antigravity",
    badge: "Official",
    desc: "Google DeepMind agentic coding assistant",
    snippet: `{
  "mcpServers": {
    "knox": {
      "command": "knox",
      "args": ["serve", "--stdio"]
    }
  }
}`,
  },
  {
    id: "cursor",
    name: "Cursor",
    badge: "IDE",
    desc: "AI code editor with MCP server support",
    snippet: `{
  "mcpServers": {
    "knox": {
      "command": "knox",
      "args": ["serve", "--stdio"]
    }
  }
}`,
  },
  {
    id: "claude",
    name: "Claude Code / Desktop",
    badge: "Anthropic",
    desc: "Claude Desktop and Claude CLI workflows",
    snippet: `{
  "mcpServers": {
    "knox": {
      "command": "knox",
      "args": ["serve", "--stdio"]
    }
  }
}`,
  },
  {
    id: "gemini",
    name: "Gemini CLI",
    badge: "CLI",
    desc: "Gemini terminal agent with MCP integration",
    snippet: `{
  "mcpServers": {
    "knox": {
      "command": "knox",
      "args": ["serve", "--stdio"]
    }
  }
}`,
  },
];

const STEPS = [
  { title: "Welcome", icon: Shield },
  { title: "Workspace", icon: Database },
  { title: "MCP", icon: PlugZap },
  { title: "Ready", icon: CheckCircle2 },
] as const;

export const OnboardingWizard: React.FC = () => {
  const {
    onboardingOpen,
    setOnboardingOpen,
    status,
    showToast,
    setActiveTab,
    refreshData,
  } = useKnoxStore();

  const [currentStep, setCurrentStep] = useState(0);
  const [selectedAgent, setSelectedAgent] = useState("antigravity");
  const [config, setConfig] = useState<KnoxConfig | null>(null);
  const [workspacePath, setWorkspacePath] = useState(".");
  const [hookInstalled, setHookInstalled] = useState(false);
  const [hasKnoxMd, setHasKnoxMd] = useState(false);
  const [isInitializing, setIsInitializing] = useState(false);
  const [isInstallingHook, setIsInstallingHook] = useState(false);
  const [setupError, setSetupError] = useState<string | null>(null);

  // Hooks must run unconditionally — early return after hooks (fixes blank root).
  useEffect(() => {
    if (!onboardingOpen) return;
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        setOnboardingOpen(false);
      }
    };
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [onboardingOpen, setOnboardingOpen]);

  useEffect(() => {
    if (!onboardingOpen) return;
    let cancelled = false;
    KnoxAPI.getConfig()
      .then((cfg) => {
        if (!cancelled) setConfig(cfg);
      })
      .catch(() => {
        if (!cancelled) setConfig(null);
      });
    return () => {
      cancelled = true;
    };
  }, [onboardingOpen]);

  const refreshWorkspaceStatus = useCallback(async (path: string) => {
    try {
      const [md, hook] = await Promise.all([
        KnoxAPI.getKnoxMd(path),
        KnoxAPI.getHookStatus(path),
      ]);
      setHasKnoxMd(md !== null);
      setHookInstalled(hook);
    } catch {
      setHasKnoxMd(false);
      setHookInstalled(false);
    }
  }, []);

  useEffect(() => {
    if (!onboardingOpen || currentStep !== 1) return;
    refreshWorkspaceStatus(workspacePath);
  }, [onboardingOpen, currentStep, workspacePath, refreshWorkspaceStatus]);

  const handleDismiss = useCallback(() => {
    setOnboardingOpen(false);
  }, [setOnboardingOpen]);

  const handleComplete = useCallback(() => {
    try {
      localStorage.setItem("knox_onboarding_completed", "true");
    } catch {
      /* storage unavailable */
    }
    setOnboardingOpen(false);
    showToast("Welcome to Knox");
  }, [setOnboardingOpen, showToast]);

  const handleBrowse = async () => {
    try {
      const folder = await KnoxAPI.openFolderDialog();
      if (folder) {
        setWorkspacePath(folder);
        setSetupError(null);
      }
    } catch (err) {
      showToast(String(err), true);
    }
  };

  const handleInit = async () => {
    try {
      setIsInitializing(true);
      setSetupError(null);
      const res = await KnoxAPI.initProject(workspacePath);
      showToast(res);
      await refreshWorkspaceStatus(workspacePath);
      await refreshData();
    } catch (err) {
      const msg = String(err);
      setSetupError(msg);
      showToast(msg, true);
    } finally {
      setIsInitializing(false);
    }
  };

  const handleInstallHook = async () => {
    try {
      setIsInstallingHook(true);
      setSetupError(null);
      const res = await KnoxAPI.installHook(workspacePath);
      showToast(res);
      await refreshWorkspaceStatus(workspacePath);
    } catch (err) {
      const msg = String(err);
      setSetupError(msg);
      showToast(msg, true);
    } finally {
      setIsInstallingHook(false);
    }
  };

  const goToTab = (tab: "workspace" | "ingest" | "mcp") => {
    handleComplete();
    setActiveTab(tab);
  };

  if (!onboardingOpen) return null;

  const currentConfig =
    AI_CONFIGS.find((a) => a.id === selectedAgent) || AI_CONFIGS[0];

  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center p-4 bg-overlay motion-safe:animate-in motion-safe:fade-in duration-200"
      role="dialog"
      aria-modal="true"
      aria-labelledby="onboarding-title"
    >
      <div className="w-full max-w-2xl bg-card border border-border rounded-[var(--radius-xl,10px)] shadow-2xl overflow-hidden flex flex-col max-h-[90vh]">
        <div className="px-5 py-4 border-b border-border flex items-center justify-between bg-card">
          <div className="flex items-center gap-2.5 min-w-0">
            <div className="p-1.5 rounded-md bg-primary/10 text-primary border border-primary/20 shrink-0">
              <Shield className="size-5" aria-hidden />
            </div>
            <div className="min-w-0">
              <h2
                id="onboarding-title"
                className="text-sm font-semibold text-foreground flex items-center gap-2"
              >
                Knox setup
                <Badge variant="secondary">v1.2.1</Badge>
              </h2>
              <p className="text-xs text-muted-foreground">
                Step {currentStep + 1} of {STEPS.length} —{" "}
                {STEPS[currentStep].title}
              </p>
            </div>
          </div>
          <Button
            type="button"
            variant="ghost"
            size="icon"
            onClick={handleDismiss}
            aria-label="Close setup wizard"
            title="Close (Esc). You can reopen from Setup Guide."
          >
            <X className="size-4" />
          </Button>
        </div>

        <div
          className="grid grid-cols-4 border-b border-border bg-muted/20"
          role="tablist"
          aria-label="Onboarding steps"
        >
          {STEPS.map((step, idx) => {
            const Icon = step.icon;
            const done = idx < currentStep;
            const active = idx === currentStep;
            return (
              <button
                key={step.title}
                type="button"
                role="tab"
                aria-selected={active}
                onClick={() => setCurrentStep(idx)}
                className={cn(
                  "flex items-center justify-center gap-2 py-2.5 text-xs font-medium border-b-2 transition-colors",
                  "focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-focus-ring",
                  active
                    ? "border-primary text-primary bg-primary/5"
                    : done
                      ? "border-success/60 text-success bg-success/5"
                      : "border-transparent text-muted-foreground hover:text-foreground"
                )}
              >
                {done ? (
                  <Check className="size-3.5" aria-hidden />
                ) : (
                  <Icon className="size-3.5" aria-hidden />
                )}
                <span className="hidden sm:inline">{step.title}</span>
              </button>
            );
          })}
        </div>

        <div className="p-5 sm:p-6 overflow-y-auto flex-1 min-h-[300px]">
          {currentStep === 0 && (
            <div className="space-y-5">
              <div className="text-center space-y-2 py-2">
                <div className="inline-flex p-3 rounded-lg bg-primary/10 border border-primary/25 text-primary">
                  <Shield className="size-8" aria-hidden />
                </div>
                <h3 className="text-lg font-semibold text-foreground tracking-tight">
                  Local knowledge graph for AI agents
                </h3>
                <p className="text-xs text-muted-foreground max-w-md mx-auto leading-relaxed">
                  Knox records coding decisions, human edits, and architectural
                  lineage in SQLite on your machine. Agents query the graph
                  instead of rediscovering context every session.
                </p>
              </div>

              <div className="grid grid-cols-1 md:grid-cols-3 gap-3">
                {[
                  {
                    icon: Database,
                    title: "Persistent memory",
                    body: "Decisions and actions survive context resets.",
                  },
                  {
                    icon: Shield,
                    title: "Private & local",
                    body: "Rust core + SQLite with FTS5 — data stays on disk.",
                  },
                  {
                    icon: PlugZap,
                    title: "MCP bridge",
                    body: "Stdio Model Context Protocol for compatible agents.",
                  },
                ].map((item) => (
                  <div
                    key={item.title}
                    className="p-3.5 rounded-lg border border-border bg-background space-y-1.5"
                  >
                    <div className="text-primary font-semibold text-xs flex items-center gap-1.5">
                      <item.icon className="size-3.5" aria-hidden />
                      {item.title}
                    </div>
                    <p className="text-sm text-muted-foreground leading-normal">
                      {item.body}
                    </p>
                  </div>
                ))}
              </div>
            </div>
          )}

          {currentStep === 1 && (
            <div className="space-y-4">
              <div className="space-y-1">
                <h3 className="text-sm font-semibold text-foreground">
                  Workspace & database
                </h3>
                <p className="text-xs text-muted-foreground">
                  Optional setup. You can skip and finish later from Workspace.
                </p>
              </div>

              <div className="p-3.5 rounded-lg border border-border bg-background space-y-2">
                <div className="flex items-center justify-between gap-2">
                  <span className="text-xs font-medium text-muted-foreground">
                    Database path
                  </span>
                  {config ? (
                    <Badge variant="secondary">Configured</Badge>
                  ) : (
                    <Badge variant="outline">Unavailable</Badge>
                  )}
                </div>
                <p
                  className="font-mono text-xs text-foreground break-all"
                  title={config?.kg_path}
                >
                  {config?.kg_path ?? "Could not load config (preview mode or IPC)."}
                </p>
                {config && (
                  <p className="text-sm text-muted-foreground">
                    Session timeout {config.session_timeout_seconds}s ·{" "}
                    {config.global_mode ? "Global mode" : "Project mode"}
                  </p>
                )}
              </div>

              {status && (
                <div className="grid grid-cols-2 gap-3">
                  <div className="p-3 rounded-lg border border-border bg-card text-center">
                    <div className="text-base font-semibold text-foreground font-mono">
                      {status.total_actions}
                    </div>
                    <div className="text-sm text-muted-foreground">
                      Actions logged
                    </div>
                  </div>
                  <div className="p-3 rounded-lg border border-border bg-card text-center">
                    <div className="text-base font-semibold text-foreground font-mono">
                      {status.total_sessions}
                    </div>
                    <div className="text-sm text-muted-foreground">
                      Sessions tracked
                    </div>
                  </div>
                </div>
              )}

              <div className="space-y-2 pt-1">
                <label htmlFor="onboard-path" className="text-overline block">
                  Project path (optional)
                </label>
                <div className="flex flex-col sm:flex-row gap-2">
                  <Input
                    id="onboard-path"
                    value={workspacePath}
                    onChange={(e) => setWorkspacePath(e.target.value)}
                    placeholder="Repository root"
                    className="flex-1"
                  />
                  <Button type="button" variant="outline" onClick={handleBrowse}>
                    <FolderOpen className="size-3.5" />
                    Browse
                  </Button>
                </div>
              </div>

              {setupError && (
                <p role="alert" className="text-xs text-danger">
                  {setupError}
                </p>
              )}

              <div className="flex flex-wrap gap-2">
                <Button
                  type="button"
                  variant="outline"
                  size="sm"
                  onClick={handleInit}
                  loading={isInitializing}
                  disabled={isInitializing}
                >
                  {!isInitializing && <FolderGit2 className="size-3.5" />}
                  {hasKnoxMd ? "Re-init knox.md" : "Initialize project"}
                </Button>
                <Button
                  type="button"
                  variant="outline"
                  size="sm"
                  onClick={handleInstallHook}
                  loading={isInstallingHook}
                  disabled={isInstallingHook}
                >
                  {!isInstallingHook && <GitCommit className="size-3.5" />}
                  {hookInstalled ? "Hook installed" : "Install git hook"}
                </Button>
                {hasKnoxMd && <Badge variant="success">knox.md present</Badge>}
                {hookInstalled && <Badge variant="success">Hook installed</Badge>}
              </div>
            </div>
          )}

          {currentStep === 2 && (
            <div className="space-y-4">
              <div className="space-y-1">
                <h3 className="text-sm font-semibold text-foreground">
                  Connect via MCP
                </h3>
                <p className="text-xs text-muted-foreground">
                  Copy configuration into your IDE. Knox does not start a server
                  from this wizard — clients spawn{" "}
                  <code className="font-mono text-foreground">
                    knox serve --stdio
                  </code>
                  .
                </p>
              </div>

              <div
                role="listbox"
                aria-label="AI client"
                className="flex gap-1 p-1 bg-muted/30 border border-border rounded-lg overflow-x-auto"
              >
                {AI_CONFIGS.map((agent) => (
                  <button
                    key={agent.id}
                    type="button"
                    role="option"
                    aria-selected={selectedAgent === agent.id}
                    onClick={() => setSelectedAgent(agent.id)}
                    className={cn(
                      "flex-1 py-1.5 px-3 rounded-md text-xs font-medium transition-colors whitespace-nowrap",
                      "focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-focus-ring",
                      selectedAgent === agent.id
                        ? "bg-card text-foreground border border-border"
                        : "text-muted-foreground hover:text-foreground"
                    )}
                  >
                    {agent.name}
                  </button>
                ))}
              </div>

              <p className="text-sm text-muted-foreground">
                {currentConfig.desc}
              </p>

              <CodeBlock
                code={currentConfig.snippet}
                language="json"
                onCopied={() => showToast("Copied MCP config")}
              />
            </div>
          )}

          {currentStep === 3 && (
            <div className="space-y-5">
              <div className="text-center space-y-2 py-2">
                <div className="inline-flex p-3 rounded-lg bg-success/10 text-success border border-success/25">
                  <CheckCircle2 className="size-8" aria-hidden />
                </div>
                <h3 className="text-lg font-semibold text-foreground">
                  You&apos;re ready
                </h3>
                <p className="text-xs text-muted-foreground max-w-sm mx-auto">
                  Open Workspace to finish project setup, ingest an Antigravity
                  transcript, or tweak MCP snippets anytime from Setup Guide.
                </p>
              </div>

              <div className="grid grid-cols-1 sm:grid-cols-3 gap-2">
                <Button
                  type="button"
                  variant="outline"
                  className="h-auto py-3 flex-col gap-1"
                  onClick={() => goToTab("workspace")}
                >
                  <FolderGit2 className="size-4 text-primary" />
                  <span>Workspace</span>
                </Button>
                <Button
                  type="button"
                  variant="outline"
                  className="h-auto py-3 flex-col gap-1"
                  onClick={() => goToTab("ingest")}
                >
                  <FileInput className="size-4 text-primary" />
                  <span>Ingest</span>
                </Button>
                <Button
                  type="button"
                  variant="outline"
                  className="h-auto py-3 flex-col gap-1"
                  onClick={() => goToTab("mcp")}
                >
                  <PlugZap className="size-4 text-primary" />
                  <span>MCP</span>
                </Button>
              </div>

              <div className="p-3.5 rounded-lg border border-border bg-background space-y-2">
                <p className="text-xs font-semibold text-foreground">
                  Desktop shortcuts
                </p>
                <div className="grid grid-cols-2 gap-2 text-xs">
                  {[
                    ["Command palette", "Ctrl + K"],
                    ["Refresh", "F5"],
                    ["Tab navigation", "1 – 8"],
                    ["Close inspector", "Esc"],
                  ].map(([label, keys]) => (
                    <div
                      key={label}
                      className="flex items-center justify-between p-2 rounded-md bg-card border border-border"
                    >
                      <span className="text-muted-foreground text-sm">
                        {label}
                      </span>
                      <kbd className="px-1.5 py-0.5 rounded bg-muted text-xs font-mono">
                        {keys}
                      </kbd>
                    </div>
                  ))}
                </div>
              </div>
            </div>
          )}
        </div>

        <div className="px-5 py-4 border-t border-border flex items-center justify-between bg-card">
          <div>
            {currentStep > 0 && (
              <Button
                type="button"
                variant="ghost"
                onClick={() => setCurrentStep((s) => s - 1)}
              >
                <ChevronLeft className="size-4" />
                Back
              </Button>
            )}
          </div>

          <div className="flex items-center gap-2">
            {currentStep < STEPS.length - 1 ? (
              <Button
                type="button"
                variant="primary"
                onClick={() => setCurrentStep((s) => s + 1)}
              >
                Continue
                <ChevronRight className="size-4" />
              </Button>
            ) : (
              <Button type="button" variant="primary" onClick={handleComplete}>
                <Check className="size-4" />
                Get started
              </Button>
            )}
          </div>
        </div>
      </div>
    </div>
  );
};
