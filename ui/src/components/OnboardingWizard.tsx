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
import { KnoxLogo } from "@/components/ui/KnoxLogo";
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

  const handleDismiss = () => {
    setOnboardingOpen(false);
  };

  const handleComplete = () => {
    try {
      localStorage.setItem("knox_onboarding_completed", "true");
    } catch {
      /* storage unavailable */
    }
    setOnboardingOpen(false);
    showToast("Welcome to Knox 🛡️!");
    refreshData();
  };

  const handleBrowseFolder = async () => {
    try {
      const selected = await KnoxAPI.openFolderDialog();
      if (selected) {
        setWorkspacePath(selected);
        setSetupError(null);
        await refreshWorkspaceStatus(selected);
      }
    } catch (e) {
      console.warn("Folder picker unavailable:", e);
    }
  };

  const handleInitWorkspace = async () => {
    setIsInitializing(true);
    setSetupError(null);
    try {
      const res = await KnoxAPI.initProject(workspacePath);
      showToast(res);
      await refreshWorkspaceStatus(workspacePath);
    } catch (err) {
      const msg = String(err);
      setSetupError(msg);
      showToast(msg, true);
    } finally {
      setIsInitializing(false);
    }
  };

  const handleToggleHook = async () => {
    setIsInstallingHook(true);
    setSetupError(null);
    try {
      const res = hookInstalled
        ? await KnoxAPI.uninstallHook(workspacePath)
        : await KnoxAPI.installHook(workspacePath);
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
          <div className="flex items-center gap-3 min-w-0">
            <KnoxLogo size={28} variant="card" />
            <div className="min-w-0">
              <h2
                id="onboarding-title"
                className="text-sm font-semibold text-foreground flex items-center gap-2"
              >
                Knox Setup & Onboarding Wizard
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
                <div className="inline-flex items-center justify-center">
                  <KnoxLogo size={56} variant="card" />
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
                    title: "Standard MCP",
                    body: "Connects to Antigravity, Cursor, Claude, and CLI tools.",
                  },
                ].map((item) => {
                  const ItemIcon = item.icon;
                  return (
                    <div
                      key={item.title}
                      className="p-3.5 rounded-lg border border-border bg-card space-y-1.5"
                    >
                      <div className="flex items-center gap-2 text-primary font-semibold text-xs">
                        <ItemIcon className="size-4 shrink-0" aria-hidden />
                        <span>{item.title}</span>
                      </div>
                      <p className="text-sm text-muted-foreground leading-relaxed">
                        {item.body}
                      </p>
                    </div>
                  );
                })}
              </div>

              {status && (
                <div className="p-3 rounded-lg bg-background border border-border flex items-center justify-between text-xs font-mono">
                  <span className="text-muted-foreground">Current graph</span>
                  <span className="text-foreground">
                    <span className="font-semibold text-primary">
                      {status.total_actions}
                    </span>{" "}
                    actions ·{" "}
                    <span className="font-semibold text-foreground">
                      {status.projects_tracked}
                    </span>{" "}
                    projects
                  </span>
                </div>
              )}
            </div>
          )}

          {currentStep === 1 && (
            <div className="space-y-4">
              <div>
                <h3 className="text-sm font-semibold text-foreground">
                  Workspace & git repository
                </h3>
                <p className="text-xs text-muted-foreground mt-0.5 leading-relaxed">
                  Point Knox at a repository to create a{" "}
                  <code className="font-mono text-foreground">knox.md</code>{" "}
                  context file and optionally install the pre-commit hook that
                  surfaces human edits to the knowledge graph.
                </p>
              </div>

              <div className="space-y-1.5">
                <label
                  htmlFor="onboarding-workspace-path"
                  className="text-sm font-medium text-foreground"
                >
                  Repository root
                </label>
                <div className="flex gap-2">
                  <Input
                    id="onboarding-workspace-path"
                    value={workspacePath}
                    onChange={(e) => {
                      setWorkspacePath(e.target.value);
                      setSetupError(null);
                    }}
                    placeholder="."
                    className="font-mono text-sm"
                  />
                  <Button
                    type="button"
                    variant="outline"
                    onClick={handleBrowseFolder}
                    title="Browse for folder"
                  >
                    <FolderOpen className="size-3.5" />
                    Browse
                  </Button>
                </div>
              </div>

              {setupError && (
                <p
                  role="alert"
                  className="text-xs text-danger bg-danger/10 border border-danger/30 p-2.5 rounded-md"
                >
                  {setupError}
                </p>
              )}

              <div className="grid grid-cols-1 sm:grid-cols-2 gap-3 pt-1">
                <div className="p-3.5 rounded-lg border border-border bg-card space-y-2">
                  <div className="flex items-center justify-between">
                    <span className="font-medium text-xs text-foreground flex items-center gap-1.5">
                      <FolderGit2 className="size-3.5 text-primary" />
                      knox.md file
                    </span>
                    <Badge variant={hasKnoxMd ? "success" : "secondary"}>
                      {hasKnoxMd ? "Present" : "Missing"}
                    </Badge>
                  </div>
                  <p className="text-sm text-muted-foreground">
                    Project-level context document read by AI agents.
                  </p>
                  <Button
                    type="button"
                    variant={hasKnoxMd ? "outline" : "primary"}
                    size="sm"
                    className="w-full"
                    onClick={handleInitWorkspace}
                    disabled={isInitializing}
                  >
                    {isInitializing
                      ? "Writing…"
                      : hasKnoxMd
                        ? "Re-initialize knox.md"
                        : "Create knox.md"}
                  </Button>
                </div>

                <div className="p-3.5 rounded-lg border border-border bg-card space-y-2">
                  <div className="flex items-center justify-between">
                    <span className="font-medium text-xs text-foreground flex items-center gap-1.5">
                      <GitCommit className="size-3.5 text-primary" />
                      Pre-commit hook
                    </span>
                    <Badge variant={hookInstalled ? "success" : "secondary"}>
                      {hookInstalled ? "Active" : "Inactive"}
                    </Badge>
                  </div>
                  <p className="text-sm text-muted-foreground">
                    Captures human git commits into the knowledge graph.
                  </p>
                  <Button
                    type="button"
                    variant={hookInstalled ? "outline" : "primary"}
                    size="sm"
                    className="w-full"
                    onClick={handleToggleHook}
                    disabled={isInstallingHook}
                  >
                    {isInstallingHook
                      ? "Updating…"
                      : hookInstalled
                        ? "Remove hook"
                        : "Install hook"}
                  </Button>
                </div>
              </div>

              <div className="p-3 rounded-lg bg-background border border-border font-mono text-xs space-y-1">
                <div className="flex justify-between text-muted-foreground">
                  <span>Global database</span>
                  <span
                    className="text-foreground truncate max-w-[280px]"
                    title={config?.kg_path}
                  >
                    {config?.kg_path || "~/.knox/knox.db"}
                  </span>
                </div>
                <div className="flex justify-between text-muted-foreground">
                  <span>Format</span>
                  <span className="text-foreground">
                    SQLite + FTS5 full-text index
                  </span>
                </div>
              </div>
            </div>
          )}

          {currentStep === 2 && (
            <div className="space-y-4">
              <div>
                <h3 className="text-sm font-semibold text-foreground">
                  Connect your AI assistant
                </h3>
                <p className="text-xs text-muted-foreground mt-0.5 leading-relaxed">
                  Knox exposes 14 MCP tools, resources, and prompts over stdio. Paste this configuration
                  into your tool’s MCP config file.
                </p>
              </div>

              <div className="grid grid-cols-2 sm:grid-cols-4 gap-2">
                {AI_CONFIGS.map((cfg) => (
                  <button
                    key={cfg.id}
                    type="button"
                    onClick={() => setSelectedAgent(cfg.id)}
                    className={cn(
                      "p-2.5 rounded-lg border text-left transition-colors cursor-pointer",
                      "focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-focus-ring",
                      selectedAgent === cfg.id
                        ? "bg-primary-muted border-primary text-foreground"
                        : "bg-card border-border hover:bg-muted/50 text-foreground"
                    )}
                  >
                    <div className="flex items-center justify-between mb-1">
                      <span className="font-semibold text-xs">{cfg.name}</span>
                      <span className="text-2xs font-mono uppercase text-muted-foreground">
                        {cfg.badge}
                      </span>
                    </div>
                    <p className="text-sm text-muted-foreground leading-snug line-clamp-2">
                      {cfg.desc}
                    </p>
                  </button>
                ))}
              </div>

              <CodeBlock
                code={currentConfig.snippet}
                language="json"
                onCopied={() =>
                  showToast(`Copied ${currentConfig.name} configuration!`)
                }
              />

              <div className="p-3 rounded-lg bg-background border border-border flex items-center justify-between text-xs">
                <span className="text-muted-foreground">
                  Need custom flags or manual command?
                </span>
                <Button
                  type="button"
                  variant="ghost"
                  size="sm"
                  onClick={() => goToTab("mcp")}
                >
                  Open MCP docs →
                </Button>
              </div>
            </div>
          )}

          {currentStep === 3 && (
            <div className="space-y-4">
              <div className="p-4 rounded-xl border border-success/30 bg-success/10 text-center space-y-1.5">
                <div className="size-9 rounded-full bg-success/20 border border-success/40 text-success flex items-center justify-center mx-auto">
                  <Check className="size-5" />
                </div>
                <h3 className="text-base font-semibold text-foreground">
                  Knox is ready
                </h3>
                <p className="text-xs text-muted-foreground max-w-sm mx-auto leading-relaxed">
                  Start an agent session or explore existing knowledge using the
                  navigation sidebar or keyboard shortcuts.
                </p>
              </div>

              <div className="grid grid-cols-3 gap-2">
                <Button
                  type="button"
                  variant="outline"
                  className="h-auto py-3 flex-col gap-1.5"
                  onClick={() => goToTab("workspace")}
                >
                  <FolderGit2 className="size-4 text-primary" />
                  <span>Workspace</span>
                </Button>

                <Button
                  type="button"
                  variant="outline"
                  className="h-auto py-3 flex-col gap-1.5"
                  onClick={() => goToTab("ingest")}
                >
                  <FileInput className="size-4 text-primary" />
                  <span>Ingest</span>
                </Button>

                <Button
                  type="button"
                  variant="outline"
                  className="h-auto py-3 flex-col gap-1.5"
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
