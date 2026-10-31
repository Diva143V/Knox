import React, { useState, useEffect } from "react";
import {
  Shield,
  Database,
  PlugZap,
  CheckCircle2,
  Copy,
  Check,
  ChevronRight,
  ChevronLeft,
  X,
  Sparkles,
  Command,
} from "lucide-react";
import { useKnoxStore } from "@/store/useKnoxStore";

const AI_CONFIGS = [
  {
    id: "antigravity",
    name: "Antigravity",
    badge: "Official",
    desc: "Google DeepMind's advanced agentic coding assistant",
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
    desc: "AI code editor with custom MCP server support",
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
    desc: "Claude Desktop and Claude CLI agent workflows",
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
    desc: "Gemini agent terminal interface with MCP integration",
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

export const OnboardingWizard: React.FC = () => {
  const { onboardingOpen, setOnboardingOpen, status, showToast } = useKnoxStore();
  const [currentStep, setCurrentStep] = useState<number>(0);
  const [selectedAgent, setSelectedAgent] = useState<string>("antigravity");
  const [copied, setCopied] = useState<boolean>(false);

  if (!onboardingOpen) return null;

  const steps = [
    { title: "Welcome", icon: <Shield className="w-4 h-4" /> },
    { title: "Workspace & DB", icon: <Database className="w-4 h-4" /> },
    { title: "AI Integration", icon: <PlugZap className="w-4 h-4" /> },
    { title: "Ready", icon: <CheckCircle2 className="w-4 h-4" /> },
  ];

  const handleComplete = () => {
    try {
      localStorage.setItem("knox_onboarding_completed", "true");
    } catch {
      // Storage unavailable or restricted
    }
    setOnboardingOpen(false);
    showToast("Welcome to Knox 🛡️ v1.2.1!");
  };

  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        setOnboardingOpen(false);
      }
    };
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [setOnboardingOpen]);

  const handleCopy = (text: string) => {
    try {
      navigator.clipboard
        .writeText(text)
        .then(() => {
          setCopied(true);
          setTimeout(() => setCopied(false), 2000);
        })
        .catch(() => {});
    } catch {
      // Clipboard write failed or blocked
    }
  };

  const currentConfig = AI_CONFIGS.find((a) => a.id === selectedAgent) || AI_CONFIGS[0];

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/75 backdrop-blur-md animate-in fade-in duration-200">
      <div className="w-full max-w-2xl bg-card border border-border/80 rounded-2xl shadow-2xl overflow-hidden flex flex-col max-h-[90vh]">
        {/* Wizard Header */}
        <div className="px-6 py-4 border-b border-border/60 flex items-center justify-between bg-card/50">
          <div className="flex items-center gap-2.5">
            <div className="p-1.5 rounded-lg bg-primary/10 text-primary border border-primary/20">
              <Shield className="w-5 h-5 text-primary" />
            </div>
            <div>
              <h2 className="text-sm font-semibold text-foreground flex items-center gap-2">
                Knox Setup & Onboarding Wizard
                <span className="text-[10px] px-1.5 py-0.5 rounded-full bg-primary/20 text-primary font-mono font-medium">
                  v1.2.1
                </span>
              </h2>
              <p className="text-xs text-muted-foreground">
                Step {currentStep + 1} of {steps.length} — {steps[currentStep].title}
              </p>
            </div>
          </div>
          <button
            onClick={() => setOnboardingOpen(false)}
            className="p-1 rounded-lg text-muted-foreground hover:text-foreground hover:bg-muted/50 transition cursor-pointer"
            title="Close Setup Wizard"
          >
            <X className="w-4 h-4" />
          </button>
        </div>

        {/* Step Indicator Progress Bar */}
        <div className="grid grid-cols-4 border-b border-border/40 bg-muted/20">
          {steps.map((step, idx) => (
            <button
              key={step.title}
              onClick={() => setCurrentStep(idx)}
              className={`flex items-center justify-center gap-2 py-2.5 text-xs font-medium border-b-2 transition cursor-pointer ${
                idx === currentStep
                  ? "border-primary text-primary bg-primary/5"
                  : idx < currentStep
                  ? "border-emerald-500/60 text-emerald-400 bg-emerald-500/5"
                  : "border-transparent text-muted-foreground hover:text-foreground"
              }`}
            >
              {idx < currentStep ? (
                <Check className="w-3.5 h-3.5 text-emerald-400" />
              ) : (
                step.icon
              )}
              <span className="hidden sm:inline">{step.title}</span>
            </button>
          ))}
        </div>

        {/* Wizard Body Viewport */}
        <div className="p-6 overflow-y-auto flex-1 min-h-[320px]">
          {/* STEP 0: WELCOME */}
          {currentStep === 0 && (
            <div className="space-y-4 animate-in fade-in-50 duration-200">
              <div className="text-center py-4 space-y-2">
                <div className="inline-flex p-3 rounded-2xl bg-gradient-to-tr from-primary/20 via-primary/10 to-transparent border border-primary/30 shadow-lg shadow-primary/10">
                  <Sparkles className="w-8 h-8 text-primary" />
                </div>
                <h3 className="text-lg font-bold text-foreground">
                  The Local-First Knowledge Graph for AI Agents
                </h3>
                <p className="text-xs text-muted-foreground max-w-md mx-auto leading-relaxed">
                  Knox automatically captures coding decisions, human edits, and architectural
                  lineage across sessions. AI assistants query your knowledge graph instead of
                  wasting tokens rediscovering context.
                </p>
              </div>

              <div className="grid grid-cols-1 md:grid-cols-3 gap-3 pt-2">
                <div className="p-3.5 rounded-xl border border-border/60 bg-muted/20 space-y-1.5">
                  <div className="text-primary font-semibold text-xs flex items-center gap-1.5">
                    <Database className="w-3.5 h-3.5" />
                    Zero Context Loss
                  </div>
                  <p className="text-[11px] text-muted-foreground leading-normal">
                    Preserves decisions and architectural intent between resets.
                  </p>
                </div>

                <div className="p-3.5 rounded-xl border border-border/60 bg-muted/20 space-y-1.5">
                  <div className="text-emerald-400 font-semibold text-xs flex items-center gap-1.5">
                    <Shield className="w-3.5 h-3.5" />
                    100% Private & Local
                  </div>
                  <p className="text-[11px] text-muted-foreground leading-normal">
                    Pure Rust core + SQLite database on your machine with FTS5 search.
                  </p>
                </div>

                <div className="p-3.5 rounded-xl border border-border/60 bg-muted/20 space-y-1.5">
                  <div className="text-violet-400 font-semibold text-xs flex items-center gap-1.5">
                    <PlugZap className="w-3.5 h-3.5" />
                    Universal MCP Bridge
                  </div>
                  <p className="text-[11px] text-muted-foreground leading-normal">
                    Model Context Protocol stdio transport for any compatible agent.
                  </p>
                </div>
              </div>
            </div>
          )}

          {/* STEP 1: WORKSPACE & DATABASE */}
          {currentStep === 1 && (
            <div className="space-y-4 animate-in fade-in-50 duration-200">
              <div className="space-y-1">
                <h3 className="text-sm font-semibold text-foreground">
                  Knowledge Graph Storage & Status
                </h3>
                <p className="text-xs text-muted-foreground">
                  Knox connects directly to your local workspace and manages SQLite state.
                </p>
              </div>

              <div className="space-y-3 pt-2">
                <div className="p-4 rounded-xl border border-border/60 bg-muted/20 space-y-2">
                  <div className="flex items-center justify-between">
                    <span className="text-xs font-medium text-muted-foreground">Database Path</span>
                    <span className="text-[11px] px-2 py-0.5 rounded-full bg-emerald-500/10 text-emerald-400 border border-emerald-500/20 font-mono">
                      Active
                    </span>
                  </div>
                  <div className="p-2.5 rounded-lg bg-card border border-border/40 font-mono text-xs text-foreground truncate">
                    ~/.knox/knox.db
                  </div>
                </div>

                <div className="p-4 rounded-xl border border-border/60 bg-muted/20 space-y-2">
                  <div className="flex items-center justify-between">
                    <span className="text-xs font-medium text-muted-foreground">Workspace Memory</span>
                    <span className="text-[11px] px-2 py-0.5 rounded-full bg-primary/10 text-primary border border-primary/20 font-mono">
                      knox.md
                    </span>
                  </div>
                  <p className="text-xs text-muted-foreground">
                    Knox maintains an active project summary in <code className="text-primary font-mono">knox.md</code> in your project root, which AI coding tools can automatically ingest.
                  </p>
                </div>

                {status && (
                  <div className="grid grid-cols-2 gap-3 pt-1">
                    <div className="p-3 rounded-lg border border-border/60 bg-card/60 text-center">
                      <div className="text-base font-bold text-foreground font-mono">
                        {status.total_actions}
                      </div>
                      <div className="text-[11px] text-muted-foreground">Total Actions Logged</div>
                    </div>
                    <div className="p-3 rounded-lg border border-border/60 bg-card/60 text-center">
                      <div className="text-base font-bold text-foreground font-mono">
                        {status.total_sessions}
                      </div>
                      <div className="text-[11px] text-muted-foreground">Sessions Tracked</div>
                    </div>
                  </div>
                )}
              </div>
            </div>
          )}

          {/* STEP 2: AI ASSISTANT MCP INTEGRATION */}
          {currentStep === 2 && (
            <div className="space-y-4 animate-in fade-in-50 duration-200">
              <div className="space-y-1">
                <h3 className="text-sm font-semibold text-foreground">
                  Connect AI Coding Assistants via MCP
                </h3>
                <p className="text-xs text-muted-foreground">
                  Add Knox as a Model Context Protocol tool in your favorite AI environment.
                </p>
              </div>

              {/* Agent Selector Tabs */}
              <div className="flex gap-2 p-1 bg-muted/30 border border-border/40 rounded-xl overflow-x-auto">
                {AI_CONFIGS.map((agent) => (
                  <button
                    key={agent.id}
                    onClick={() => setSelectedAgent(agent.id)}
                    className={`flex-1 py-1.5 px-3 rounded-lg text-xs font-medium transition cursor-pointer whitespace-nowrap ${
                      selectedAgent === agent.id
                        ? "bg-card text-foreground shadow-sm border border-border/60"
                        : "text-muted-foreground hover:text-foreground"
                    }`}
                  >
                    {agent.name}
                  </button>
                ))}
              </div>

              {/* Code Snippet Block */}
              <div className="relative rounded-xl border border-border/80 bg-zinc-950 p-4 font-mono text-xs text-zinc-100 shadow-inner">
                <div className="flex items-center justify-between mb-2 pb-2 border-b border-zinc-800 text-[11px] text-zinc-400">
                  <span>{currentConfig.name} Configuration Snippet</span>
                  <button
                    onClick={() => handleCopy(currentConfig.snippet)}
                    className="flex items-center gap-1.5 px-2 py-1 rounded bg-zinc-800 hover:bg-zinc-700 text-zinc-200 transition cursor-pointer"
                  >
                    {copied ? (
                      <>
                        <Check className="w-3.5 h-3.5 text-emerald-400" />
                        <span className="text-emerald-400">Copied</span>
                      </>
                    ) : (
                      <>
                        <Copy className="w-3.5 h-3.5" />
                        <span>Copy JSON</span>
                      </>
                    )}
                  </button>
                </div>
                <pre className="overflow-x-auto text-[11.5px] leading-relaxed text-emerald-400">
                  {currentConfig.snippet}
                </pre>
              </div>

              <div className="text-[11px] text-muted-foreground bg-muted/20 p-3 rounded-xl border border-border/50">
                💡 <strong>Protocol Note:</strong> Knox operates its MCP server via{" "}
                <code className="text-primary font-mono">knox serve --stdio</code>. It runs seamlessly as a sub-process for any MCP-compliant client.
              </div>
            </div>
          )}

          {/* STEP 3: READY & LAUNCH */}
          {currentStep === 3 && (
            <div className="space-y-4 animate-in fade-in-50 duration-200">
              <div className="text-center py-4 space-y-2">
                <div className="inline-flex p-3 rounded-2xl bg-emerald-500/10 text-emerald-400 border border-emerald-500/20">
                  <CheckCircle2 className="w-8 h-8" />
                </div>
                <h3 className="text-lg font-bold text-foreground">You are Ready to Use Knox!</h3>
                <p className="text-xs text-muted-foreground max-w-sm mx-auto">
                  Your local knowledge graph is online and ready to empower your AI workflows.
                </p>
              </div>

              {/* Quick Navigation Cheat Sheet */}
              <div className="p-4 rounded-xl border border-border/60 bg-muted/20 space-y-2.5">
                <div className="text-xs font-semibold text-foreground flex items-center gap-2">
                  <Command className="w-3.5 h-3.5 text-primary" />
                  Key Desktop Shortcuts
                </div>
                <div className="grid grid-cols-2 gap-2 text-xs">
                  <div className="flex items-center justify-between p-2 rounded-lg bg-card border border-border/40">
                    <span className="text-muted-foreground text-[11px]">Command Palette</span>
                    <kbd className="px-1.5 py-0.5 rounded bg-muted text-[10px] font-mono">Ctrl + K</kbd>
                  </div>
                  <div className="flex items-center justify-between p-2 rounded-lg bg-card border border-border/40">
                    <span className="text-muted-foreground text-[11px]">Live Refresh</span>
                    <kbd className="px-1.5 py-0.5 rounded bg-muted text-[10px] font-mono">F5</kbd>
                  </div>
                  <div className="flex items-center justify-between p-2 rounded-lg bg-card border border-border/40">
                    <span className="text-muted-foreground text-[11px]">Tab Navigation</span>
                    <kbd className="px-1.5 py-0.5 rounded bg-muted text-[10px] font-mono">1 – 8</kbd>
                  </div>
                  <div className="flex items-center justify-between p-2 rounded-lg bg-card border border-border/40">
                    <span className="text-muted-foreground text-[11px]">Close Inspector</span>
                    <kbd className="px-1.5 py-0.5 rounded bg-muted text-[10px] font-mono">Esc</kbd>
                  </div>
                </div>
              </div>
            </div>
          )}
        </div>

        {/* Wizard Footer Controls */}
        <div className="px-6 py-4 border-t border-border/60 flex items-center justify-between bg-card/50">
          <div>
            {currentStep > 0 && (
              <button
                onClick={() => setCurrentStep((s) => s - 1)}
                className="flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-xs font-medium text-muted-foreground hover:text-foreground hover:bg-muted/50 transition cursor-pointer"
              >
                <ChevronLeft className="w-4 h-4" />
                Back
              </button>
            )}
          </div>

          <div className="flex items-center gap-2">
            {currentStep < steps.length - 1 ? (
              <button
                onClick={() => setCurrentStep((s) => s + 1)}
                className="flex items-center gap-1.5 px-4 py-2 rounded-xl text-xs font-semibold bg-primary text-primary-foreground hover:opacity-90 shadow-md transition cursor-pointer"
              >
                Continue
                <ChevronRight className="w-4 h-4" />
              </button>
            ) : (
              <button
                onClick={handleComplete}
                className="flex items-center gap-1.5 px-5 py-2 rounded-xl text-xs font-semibold bg-emerald-600 text-white hover:bg-emerald-500 shadow-md shadow-emerald-600/20 transition cursor-pointer"
              >
                <Check className="w-4 h-4" />
                Get Started
              </button>
            )}
          </div>
        </div>
      </div>
    </div>
  );
};
