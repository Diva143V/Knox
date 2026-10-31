import React, { useState } from "react";
import {
  Compass,
  LayoutDashboard,
  Network,
  Search,
  FileInput,
  ShieldCheck,
  ShieldAlert,
  PlugZap,
  FolderGit2,
  CheckCircle2,
  Check,
  ChevronRight,
  ChevronLeft,
  X,
  Copy,
  Zap,
  GitBranch,
  Activity,
  Orbit,
  Atom,
  Cpu,
  Clock,
  Sparkles,
  BookOpen,
  Terminal,
  Keyboard,
  ArrowRight,
  Layers,
  FileCode,
  Scale,
  Bug,
  Eye,
  Maximize2,
} from "lucide-react";
import { KnoxLogo } from "@/components/ui/KnoxLogo";
import { Button } from "@/components/ui/Button";
import { Badge } from "@/components/ui/Badge";
import { CodeBlock } from "@/components/ui/CodeBlock";
import { useKnoxStore } from "@/store/useKnoxStore";
import { useTour } from "@/hooks/useTour";
import type { TourStepDef } from "@/types/tour";
import { cn } from "@/lib/cn";

const TOUR_STEPS: TourStepDef[] = [
  {
    id: "welcome",
    tabId: "dashboard",
    stepNumber: 1,
    badge: "Architecture Memory",
    badgeVariant: "default",
    title: "Persistent Knowledge Graph for AI Coding Agents",
    subtitle: "Eliminate context drift and prevent token-wasting re-indexing across coding sessions.",
    description:
      "Knox acts as a local, deterministic brain for your AI agents (Antigravity, Cursor, Claude, Copilot). It automatically records architectural choices, touched code symbols, and causal file relationships so agents never forget past context or undo each other's work.",
    highlights: [
      {
        iconName: "Zap",
        label: "Zero Token Waste",
        detail: "Agents query high-density Markdown digests instead of burning thousands of context tokens re-reading files.",
      },
      {
        iconName: "GitBranch",
        label: "Causal Provenance",
        detail: "Every code change is linked to its rationale, rejected alternatives, and parent agent session.",
      },
      {
        iconName: "ShieldCheck",
        label: "Architectural Guardrails",
        detail: "Flag conflicting edits and breaking design regressions before commits touch your repository.",
      },
    ],
    visual: { type: "welcome-concept" },
  },
  {
    id: "dashboard",
    tabId: "dashboard",
    stepNumber: 2,
    badge: "Telemetry & KPIs",
    badgeVariant: "secondary",
    title: "Live Project Health & Activity Pulse",
    subtitle: "High-level index metrics, recent agent activity timeline, and quick-launch actions.",
    description:
      "The Dashboard gives you an immediate pulse on your repository's architectural health. Monitor indexed action counts, active agent sessions, file clusters, and recent human vs AI editing velocity in real time.",
    highlights: [
      {
        iconName: "LayoutDashboard",
        label: "Quick KPI Telemetry",
        detail: "Track total indexed actions, causal edges, active sessions, and stale file counters at a glance.",
      },
      {
        iconName: "Activity",
        label: "Activity Pulse Timeline",
        detail: "Chronological audit trail of edits performed across Antigravity, Cursor, Claude, and manual commits.",
      },
      {
        iconName: "FolderGit2",
        label: "Scoped Intelligence",
        detail: "Switch focus instantly between your global multi-project knowledge base and specific repository folders.",
      },
    ],
    visual: { type: "dashboard-kpi" },
  },
  {
    id: "graph",
    tabId: "graph",
    stepNumber: 3,
    badge: "Cosmos & Cards Canvas",
    badgeVariant: "plan",
    title: "Interactive Visual Knowledge Graph",
    subtitle: "Explore causal dependency trees with live dynamic D3 physics motion.",
    description:
      "Powered by @xyflow/react and continuous live D3 force simulation, the Graph Canvas renders project architecture in two views: radiant Cosmos Orbs and structured Cards.",
    highlights: [
      {
        iconName: "Orbit",
        label: "Cosmos Orb Mode",
        detail: "Radiant element circles with role-based glowing auras (Decide in gold, Plan in cyan, Write in emerald, Debug in rose).",
      },
      {
        iconName: "Atom",
        label: "Live Dynamic Physics",
        detail: "Continuous D3 physics simulation allows you to grab, drag, and shake nodes in satisfying real-time motion.",
      },
      {
        iconName: "Network",
        label: "Semantic Causal Edges",
        detail: "Visually trace depends_on, implements, verifies, and supersedes relations with cardinal routing.",
      },
    ],
    visual: { type: "graph-cosmos" },
  },
  {
    id: "query",
    tabId: "query",
    stepNumber: 4,
    badge: "FTS5 Search Engine",
    badgeVariant: "secondary",
    title: "Semantic Search Across Project Memory",
    subtitle: "Instant answers to 'Why was this changed?' and 'Who decided this trade-off?'.",
    description:
      "SQLite FTS5-powered search engine allows developers and AI agents to query past decisions, touched symbols, file histories, and architectural trade-offs in sub-millisecond time.",
    highlights: [
      {
        iconName: "Search",
        label: "Full-Text Search",
        detail: "Deeply queries action summaries, ADR rationales, rejected alternatives, and code symbols.",
      },
      {
        iconName: "Filter",
        label: "Multi-Filter Facets",
        detail: "Slice and filter by actor (gemini, cursor, claude), action intent (write, debug, decide), or file path.",
      },
      {
        iconName: "Layers",
        label: "Causal Deep-Inspection",
        detail: "Click any result to jump to the node, view parent decisions, and trace affected blast radius.",
      },
    ],
    visual: { type: "query-search" },
  },
  {
    id: "ingest",
    tabId: "ingest",
    stepNumber: 5,
    badge: "Retroactive Ingestion",
    badgeVariant: "default",
    title: "Bootstrap Project History in Seconds",
    subtitle: "Convert existing AI conversation logs into a persistent knowledge graph.",
    description:
      "Drag and drop or browse conversation transcripts from Antigravity (transcript.jsonl), Claude, or Cursor to automatically extract actions, code diffs, and architectural decisions retroactively.",
    highlights: [
      {
        iconName: "FileInput",
        label: "Drag & Drop Ingestion",
        detail: "Easily ingest single session transcripts or entire agent log directories with real-time parsing progress.",
      },
      {
        iconName: "Cpu",
        label: "Heuristic Action Grouping",
        detail: "Intelligently aggregates repetitive tool calls into structured human-readable intentions.",
      },
      {
        iconName: "CheckCircle2",
        label: "Atomic Persistence",
        detail: "Extracted graph nodes, file edges, and causal links are committed safely into your local Knox database.",
      },
    ],
    visual: { type: "ingest-dropzone" },
  },
  {
    id: "scanner",
    tabId: "scanner",
    stepNumber: 6,
    badge: "Pre-Commit Guardrails",
    badgeVariant: "warning",
    title: "Verify Git Changes Against Architectural Rules",
    subtitle: "Prevent accidental architectural regressions before you commit or merge.",
    description:
      "Inspects your currently git-staged changes (git diff --cached) against Knox's recorded ADRs and decisions, alerting you whenever code modifications violate established architectural constraints.",
    highlights: [
      {
        iconName: "ShieldAlert",
        label: "Architectural Conflict Checks",
        detail: "Flags when uncommitted edits contradict active architectural decisions or ADR rationale.",
      },
      {
        iconName: "Clock",
        label: "Stale Node Detection",
        detail: "Highlights files that have evolved in code without recorded context updates from recent agent sessions.",
      },
      {
        iconName: "GitBranch",
        label: "Pre-Commit Git Hook",
        detail: "Install with 'knox hook install' for automatic zero-friction pre-commit verification.",
      },
    ],
    visual: { type: "scanner-diff" },
  },
  {
    id: "mcp",
    tabId: "mcp",
    stepNumber: 7,
    badge: "14 Tools & Resources",
    badgeVariant: "outline",
    title: "Direct Bridge for AI Coding Assistants",
    subtitle: "Connect Antigravity, Cursor, Claude Desktop, and Gemini CLI in 1 click.",
    description:
      "Knox exposes a native Model Context Protocol (MCP) server over standard I/O (knox serve --stdio) equipped with 14 specialized tools, native resources (knox://...), and prompt templates.",
    highlights: [
      {
        iconName: "PlugZap",
        label: "1-Click Auto-Configuration",
        detail: "Automatic discovery and direct JSON config injection into Cursor, Claude, Antigravity, and Zed.",
      },
      {
        iconName: "Sparkles",
        label: "Smart Implicit Auto-Sessions",
        detail: "Agents record actions seamlessly without needing explicit session creation boilerplate.",
      },
      {
        iconName: "BookOpen",
        label: "Token-Compact Markdown",
        detail: "Context tools return compact bullet digests instead of raw JSON, saving over 60% of agent context window tokens.",
      },
    ],
    visual: { type: "mcp-bridge" },
  },
  {
    id: "workspace",
    tabId: "workspace",
    stepNumber: 8,
    badge: "Power-User Workflows",
    badgeVariant: "secondary",
    title: "Multi-Project Management & Fast Navigation",
    subtitle: "Fly across your codebase and knowledge graph with rapid hotkeys.",
    description:
      "Manage multiple project repositories from a single unified global vault, and navigate any action, file, or view instantly using the universal Command Palette (Ctrl+K).",
    highlights: [
      {
        iconName: "FolderGit2",
        label: "Multi-Workspace Vault",
        detail: "Switch scopes seamlessly between different repositories or keep an umbrella multi-repo overview.",
      },
      {
        iconName: "Terminal",
        label: "Universal Command Palette",
        detail: "Press Ctrl+K anywhere to search indexed actions, jump to files, or toggle views instantly.",
      },
      {
        iconName: "Keyboard",
        label: "Keyboard-First Ergonomics",
        detail: "Jump to views instantly: G D (Dashboard), G G (Graph), G Q (Query), G M (MCP), G W (Workspace).",
      },
    ],
    visual: { type: "workspace-shortcuts" },
  },
  {
    id: "ready",
    stepNumber: 9,
    badge: "Ready to Explore",
    badgeVariant: "success",
    title: "Knox is Ready to Supercharge Your AI Workflows",
    subtitle: "Choose how you want to begin exploring your knowledge graph.",
    description:
      "Your local Knox engine is armed and ready. Start an agent session in your IDE, inspect your repository dependencies, or ingest existing transcripts to bootstrap your project history.",
    highlights: [
      {
        iconName: "Orbit",
        label: "Explore Knowledge Graph",
        detail: "View your project nodes, causal branches, and live physics simulation on canvas.",
      },
      {
        iconName: "FileInput",
        label: "Bootstrap Project History",
        detail: "Drag and drop your past Antigravity or Cursor conversation logs.",
      },
      {
        iconName: "PlugZap",
        label: "Connect AI IDE Agents",
        detail: "Copy the standard MCP snippet or auto-configure your IDE in the MCP view.",
      },
    ],
    visual: { type: "ready-completion" },
  },
];

const MCP_SAMPLE_SNIPPET = `{
  "mcpServers": {
    "knox": {
      "command": "knox",
      "args": ["serve", "--stdio"]
    }
  }
}`;

export const OnboardingTourModal: React.FC = () => {
  const {
    isTourOpen,
    setActiveTab,
    setOnboardingOpen,
    showToast,
  } = useKnoxStore();

  const {
    currentStepIndex,
    currentStep,
    totalSteps,
    isFirstStep,
    isLastStep,
    dontShowAgain,
    setDontShowAgain,
    goToNextStep,
    goToPrevStep,
    goToStep,
    skipTour,
    completeTour,
  } = useTour(TOUR_STEPS);

  const [copiedSnippet, setCopiedSnippet] = useState(false);
  const [isPeekMode, setIsPeekMode] = useState(false);

  // Toggle peek mode with 'P' key
  React.useEffect(() => {
    if (!isTourOpen) return;
    const handleKeyDown = (e: KeyboardEvent) => {
      const target = e.target as HTMLElement;
      if (target.tagName === "INPUT" || target.tagName === "TEXTAREA") return;
      if (e.key === "p" || e.key === "P") {
        e.preventDefault();
        setIsPeekMode((prev) => !prev);
      }
    };
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [isTourOpen]);

  if (!isTourOpen) return null;

  const handleCopyMcp = () => {
    try {
      navigator.clipboard.writeText(MCP_SAMPLE_SNIPPET);
      setCopiedSnippet(true);
      showToast("Copied standard Knox MCP config to clipboard", "success");
      setTimeout(() => setCopiedSnippet(false), 2000);
    } catch {
      showToast("Failed to copy snippet to clipboard", true);
    }
  };

  const handleActionExploreGraph = () => {
    setActiveTab("graph");
    completeTour();
  };

  const handleActionIngestTranscript = () => {
    setActiveTab("ingest");
    completeTour();
  };

  const handleActionSetupWorkspace = () => {
    completeTour();
    setOnboardingOpen(true);
  };

  const renderIcon = (name: string, className?: string) => {
    switch (name) {
      case "Zap":
        return <Zap className={className} />;
      case "GitBranch":
        return <GitBranch className={className} />;
      case "ShieldCheck":
        return <ShieldCheck className={className} />;
      case "LayoutDashboard":
        return <LayoutDashboard className={className} />;
      case "Activity":
        return <Activity className={className} />;
      case "FolderGit2":
        return <FolderGit2 className={className} />;
      case "Orbit":
        return <Orbit className={className} />;
      case "Atom":
        return <Atom className={className} />;
      case "Network":
        return <Network className={className} />;
      case "Search":
        return <Search className={className} />;
      case "Filter":
        return <Search className={className} />;
      case "Layers":
        return <Layers className={className} />;
      case "FileInput":
        return <FileInput className={className} />;
      case "Cpu":
        return <Cpu className={className} />;
      case "ShieldAlert":
        return <ShieldAlert className={className} />;
      case "Clock":
        return <Clock className={className} />;
      case "PlugZap":
        return <PlugZap className={className} />;
      case "Sparkles":
        return <Sparkles className={className} />;
      case "BookOpen":
        return <BookOpen className={className} />;
      case "Terminal":
        return <Terminal className={className} />;
      case "Keyboard":
        return <Keyboard className={className} />;
      default:
        return <CheckCircle2 className={className} />;
    }
  };

  const renderVisualCard = (step: TourStepDef) => {
    switch (step.visual.type) {
      case "welcome-concept":
        return (
          <div className="h-full flex flex-col justify-center items-center p-6 bg-gradient-to-b from-primary/10 via-background/40 to-background/80 rounded-xl border border-primary/20 relative overflow-hidden">
            <div className="absolute -top-12 -right-12 size-40 bg-primary/20 rounded-full blur-3xl pointer-events-none" />
            <div className="absolute -bottom-12 -left-12 size-40 bg-cyan-500/15 rounded-full blur-3xl pointer-events-none" />

            <div className="relative flex items-center justify-center gap-6 my-auto">
              {/* Agent Node */}
              <div className="flex flex-col items-center gap-1.5 animate-pulse">
                <div className="size-14 rounded-full border-2 border-primary/60 bg-primary/15 shadow-[0_0_20px_rgba(16,185,129,0.35)] flex items-center justify-center">
                  <Cpu className="size-6 text-primary" />
                </div>
                <span className="text-2xs font-mono text-primary font-semibold">AI Agent</span>
              </div>

              {/* Connecting Causal Line */}
              <div className="w-16 h-0.5 bg-gradient-to-r from-primary via-warning to-cyan-400 relative">
                <span className="absolute -top-3 left-1/2 -translate-x-1/2 text-[9px] font-mono text-muted-foreground uppercase tracking-widest">
                  Memory
                </span>
              </div>

              {/* Knox Knowledge Vault */}
              <div className="flex flex-col items-center gap-1.5">
                <div className="size-16 rounded-full border-2 border-warning/70 bg-warning/20 shadow-[0_0_25px_rgba(245,158,11,0.35)] flex items-center justify-center">
                  <Scale className="size-7 text-warning" />
                </div>
                <span className="text-2xs font-mono text-warning font-semibold">ADR Decision</span>
              </div>

              {/* Connecting Line */}
              <div className="w-16 h-0.5 bg-gradient-to-r from-warning to-cyan-400 relative">
                <span className="absolute -top-3 left-1/2 -translate-x-1/2 text-[9px] font-mono text-muted-foreground uppercase tracking-widest">
                  Shapes
                </span>
              </div>

              {/* Code Files Node */}
              <div className="flex flex-col items-center gap-1.5">
                <div className="size-14 rounded-full border-2 border-cyan-400/60 bg-cyan-500/15 shadow-[0_0_20px_rgba(6,182,212,0.35)] flex items-center justify-center">
                  <FileCode className="size-6 text-cyan-300" />
                </div>
                <span className="text-2xs font-mono text-cyan-300 font-semibold">Source Files</span>
              </div>
            </div>

            <div className="mt-4 px-3 py-1.5 rounded-full bg-card/80 border border-border text-2xs font-mono text-muted-foreground text-center">
              Deterministic Memory Bridge • 0% Context Drift • 100% Provenance
            </div>
          </div>
        );

      case "dashboard-kpi":
        return (
          <div className="h-full flex flex-col justify-center p-5 bg-card/60 rounded-xl border border-border space-y-3">
            <span className="text-overline text-muted-foreground">Live Telemetry Preview</span>
            <div className="grid grid-cols-2 gap-2.5">
              <div className="p-3 rounded-lg border border-border bg-background/80 space-y-1">
                <div className="flex items-center justify-between text-muted-foreground text-2xs">
                  <span>Total Actions</span>
                  <Activity className="size-3 text-primary" />
                </div>
                <div className="text-xl font-bold font-mono text-foreground">132</div>
                <div className="text-3xs text-success font-mono">+18 in active session</div>
              </div>

              <div className="p-3 rounded-lg border border-border bg-background/80 space-y-1">
                <div className="flex items-center justify-between text-muted-foreground text-2xs">
                  <span>Active Decisions</span>
                  <Scale className="size-3 text-warning" />
                </div>
                <div className="text-xl font-bold font-mono text-warning">28</div>
                <div className="text-3xs text-muted-foreground font-mono">100% current</div>
              </div>

              <div className="p-3 rounded-lg border border-border bg-background/80 space-y-1">
                <div className="flex items-center justify-between text-muted-foreground text-2xs">
                  <span>Causal Edges</span>
                  <Network className="size-3 text-plan" />
                </div>
                <div className="text-xl font-bold font-mono text-foreground">246</div>
                <div className="text-3xs text-muted-foreground font-mono">DAG provenance</div>
              </div>

              <div className="p-3 rounded-lg border border-border bg-background/80 space-y-1">
                <div className="flex items-center justify-between text-muted-foreground text-2xs">
                  <span>Stale Conflicts</span>
                  <ShieldCheck className="size-3 text-success" />
                </div>
                <div className="text-xl font-bold font-mono text-success">0</div>
                <div className="text-3xs text-success font-mono">Architecture aligned</div>
              </div>
            </div>
          </div>
        );

      case "graph-cosmos":
        return (
          <div className="h-full flex flex-col justify-center items-center p-6 bg-gradient-to-br from-card via-background to-card rounded-xl border border-border relative overflow-hidden">
            <div className="flex items-center justify-center gap-5 my-auto">
              <div className="size-16 rounded-full border-2 border-warning/80 bg-warning/20 shadow-[0_0_25px_rgba(245,158,11,0.4)] flex flex-col items-center justify-center text-center animate-bounce">
                <Scale className="size-5 text-warning" />
                <span className="text-[9px] font-mono font-bold text-warning">ADR</span>
              </div>
              <div className="size-14 rounded-full border-2 border-emerald-400/80 bg-emerald-500/20 shadow-[0_0_20px_rgba(16,185,129,0.35)] flex flex-col items-center justify-center text-center">
                <Zap className="size-5 text-emerald-300" />
                <span className="text-[9px] font-mono font-bold text-emerald-300">Write</span>
              </div>
              <div className="size-14 rounded-full border-2 border-cyan-400/80 bg-cyan-500/20 shadow-[0_0_20px_rgba(6,182,212,0.35)] flex flex-col items-center justify-center text-center">
                <Compass className="size-5 text-cyan-300" />
                <span className="text-[9px] font-mono font-bold text-cyan-300">Plan</span>
              </div>
              <div className="size-12 rounded-full border-2 border-rose-400/80 bg-rose-500/20 shadow-[0_0_18px_rgba(239,68,68,0.35)] flex flex-col items-center justify-center text-center">
                <Bug className="size-4 text-rose-300" />
                <span className="text-[8px] font-mono font-bold text-rose-300">Debug</span>
              </div>
            </div>
            <div className="flex items-center gap-2 mt-4">
              <Badge variant="plan" className="text-2xs font-mono">
                Cosmos Orb Mode
              </Badge>
              <Badge variant="warning" className="text-2xs font-mono">
                Live D3 Physics Active
              </Badge>
            </div>
          </div>
        );

      case "query-search":
        return (
          <div className="h-full flex flex-col justify-center p-5 bg-card/60 rounded-xl border border-border space-y-3">
            <span className="text-overline text-muted-foreground">FTS5 Search Bar Preview</span>
            <div className="p-2.5 rounded-lg border border-primary/50 bg-background/90 flex items-center gap-2 shadow-xs">
              <Search className="size-4 text-primary" />
              <span className="font-mono text-xs text-foreground font-medium">
                sqlite migration force simulation
              </span>
            </div>

            <div className="space-y-1.5">
              <span className="text-overline text-muted-foreground">Active Facet Filters</span>
              <div className="flex flex-wrap gap-1.5 font-mono text-2xs">
                <span className="px-2 py-0.5 rounded-sm bg-primary/15 text-primary border border-primary/30">
                  actor: antigravity
                </span>
                <span className="px-2 py-0.5 rounded-sm bg-warning/15 text-warning border border-warning/30">
                  type: decide
                </span>
                <span className="px-2 py-0.5 rounded-sm bg-muted text-muted-foreground border border-border">
                  current: true
                </span>
              </div>
            </div>

            <div className="p-2.5 rounded-md border border-border-subtle bg-muted/40 font-mono text-2xs text-foreground/80 space-y-1">
              <div className="flex items-center justify-between text-muted-foreground">
                <span className="text-primary font-semibold">act-042 [decide]</span>
                <span>2 mins ago</span>
              </div>
              <div className="truncate">Use D3 force simulation for Cosmos knowledge graph</div>
            </div>
          </div>
        );

      case "ingest-dropzone":
        return (
          <div className="h-full flex flex-col justify-center items-center p-6 border-2 border-dashed border-primary/40 rounded-xl bg-primary/5 text-center space-y-3">
            <div className="size-12 rounded-full bg-primary/15 border border-primary/30 flex items-center justify-center text-primary">
              <FileInput className="size-6" />
            </div>
            <div>
              <p className="text-xs font-semibold text-foreground">
                Drag & Drop Conversation Transcripts
              </p>
              <p className="text-2xs text-muted-foreground font-mono mt-0.5">
                Supports Antigravity transcript.jsonl, Cursor, Claude
              </p>
            </div>
            <div className="flex items-center gap-2 pt-1">
              <span className="px-2 py-0.5 rounded-sm bg-card border border-border text-2xs font-mono text-muted-foreground">
                .jsonl
              </span>
              <span className="px-2 py-0.5 rounded-sm bg-card border border-border text-2xs font-mono text-muted-foreground">
                .json
              </span>
              <span className="px-2 py-0.5 rounded-sm bg-card border border-border text-2xs font-mono text-muted-foreground">
                Auto-Parsed
              </span>
            </div>
          </div>
        );

      case "scanner-diff":
        return (
          <div className="h-full flex flex-col justify-center p-5 bg-card/60 rounded-xl border border-border space-y-3">
            <div className="flex items-center justify-between">
              <span className="text-overline text-muted-foreground">Git Staged Inspector</span>
              <Badge variant="success" className="text-2xs font-mono">
                PASSING
              </Badge>
            </div>

            <div className="space-y-2 font-mono text-2xs">
              <div className="p-2.5 rounded-md border border-border bg-background space-y-1">
                <div className="flex items-center justify-between text-foreground font-semibold">
                  <span>src/mcp/format.rs</span>
                  <span className="text-success">+142 -18</span>
                </div>
                <p className="text-muted-foreground text-3xs">
                  Implements token-compact Markdown digest engine
                </p>
              </div>

              <div className="p-2.5 rounded-md border border-border bg-background space-y-1">
                <div className="flex items-center justify-between text-foreground font-semibold">
                  <span>ui/src/hooks/useLivePhysics.ts</span>
                  <span className="text-success">+210 -0</span>
                </div>
                <p className="text-muted-foreground text-3xs">
                  Complies with ADR-003: Continuous D3 force simulation
                </p>
              </div>
            </div>

            <div className="flex items-center gap-2 text-2xs text-success font-mono pt-1">
              <ShieldCheck className="size-3.5" />
              <span>0 architectural violations detected across staged diffs</span>
            </div>
          </div>
        );

      case "mcp-bridge":
        return (
          <div className="h-full flex flex-col justify-center p-4 bg-card/60 rounded-xl border border-border space-y-2.5">
            <div className="flex items-center justify-between">
              <span className="text-overline text-muted-foreground">MCP Stdio Configuration</span>
              <Button
                type="button"
                variant="outline"
                size="sm"
                className="h-6 text-2xs font-mono"
                onClick={handleCopyMcp}
              >
                {copiedSnippet ? (
                  <>
                    <Check className="size-3 text-success mr-1" />
                    <span>Copied!</span>
                  </>
                ) : (
                  <>
                    <Copy className="size-3 mr-1" />
                    <span>Copy JSON</span>
                  </>
                )}
              </Button>
            </div>

            <CodeBlock code={MCP_SAMPLE_SNIPPET} language="json" />

            <div className="flex items-center justify-between text-3xs font-mono text-muted-foreground pt-1">
              <span>14 Registered Tools</span>
              <span>Native Resources & Prompts</span>
            </div>
          </div>
        );

      case "workspace-shortcuts":
        return (
          <div className="h-full flex flex-col justify-center p-5 bg-card/60 rounded-xl border border-border space-y-3">
            <span className="text-overline text-muted-foreground">Power-User Keyboard Shortcuts</span>
            <div className="space-y-2 font-mono text-xs">
              <div className="flex items-center justify-between p-2 rounded-md bg-background border border-border">
                <span className="text-muted-foreground font-sans text-2xs">Universal Command Palette</span>
                <kbd className="px-1.5 py-0.5 rounded-sm bg-card border border-border text-primary font-bold text-2xs">
                  Ctrl + K
                </kbd>
              </div>

              <div className="flex items-center justify-between p-2 rounded-md bg-background border border-border">
                <span className="text-muted-foreground font-sans text-2xs">Jump to Knowledge Graph</span>
                <kbd className="px-1.5 py-0.5 rounded-sm bg-card border border-border text-foreground text-2xs">
                  G then G
                </kbd>
              </div>

              <div className="flex items-center justify-between p-2 rounded-md bg-background border border-border">
                <span className="text-muted-foreground font-sans text-2xs">Jump to Query Search</span>
                <kbd className="px-1.5 py-0.5 rounded-sm bg-card border border-border text-foreground text-2xs">
                  G then Q
                </kbd>
              </div>

              <div className="flex items-center justify-between p-2 rounded-md bg-background border border-border">
                <span className="text-muted-foreground font-sans text-2xs">Jump to MCP Server View</span>
                <kbd className="px-1.5 py-0.5 rounded-sm bg-card border border-border text-foreground text-2xs">
                  G then M
                </kbd>
              </div>
            </div>
          </div>
        );

      case "ready-completion":
        return (
          <div className="h-full flex flex-col justify-center p-5 bg-gradient-to-b from-primary/10 via-card to-background rounded-xl border border-primary/30 space-y-3 text-center">
            <div className="size-14 rounded-full bg-primary/20 border-2 border-primary/60 text-primary mx-auto flex items-center justify-center shadow-[0_0_20px_rgba(16,185,129,0.35)]">
              <CheckCircle2 className="size-8" />
            </div>

            <div>
              <h4 className="text-sm font-bold text-foreground font-mono">
                Knox Engine Active
              </h4>
              <p className="text-2xs text-muted-foreground mt-0.5">
                Select your starting action below to dive right in
              </p>
            </div>

            <div className="space-y-1.5 pt-1">
              <Button
                type="button"
                variant="primary"
                className="w-full justify-between h-9 text-xs font-semibold"
                onClick={handleActionExploreGraph}
              >
                <span className="flex items-center gap-2">
                  <Orbit className="size-4" />
                  <span>Explore Knowledge Graph Canvas</span>
                </span>
                <ArrowRight className="size-3.5" />
              </Button>

              <Button
                type="button"
                variant="outline"
                className="w-full justify-between h-8 text-xs font-medium"
                onClick={handleActionIngestTranscript}
              >
                <span className="flex items-center gap-2">
                  <FileInput className="size-3.5 text-plan" />
                  <span>Ingest Past Transcript Logs</span>
                </span>
                <ArrowRight className="size-3.5" />
              </Button>

              <Button
                type="button"
                variant="ghost"
                className="w-full justify-between h-8 text-xs font-medium"
                onClick={handleActionSetupWorkspace}
              >
                <span className="flex items-center gap-2">
                  <FolderGit2 className="size-3.5 text-warning" />
                  <span>Configure Workspace Database</span>
                </span>
                <ArrowRight className="size-3.5" />
              </Button>
            </div>
          </div>
        );

      default:
        return null;
    }
  };

  if (isPeekMode) {
    return (
      <div
        role="dialog"
        aria-modal="false"
        aria-label="Welcome and Getting Started Tour (Peek Mode)"
        className="fixed bottom-6 left-1/2 -translate-x-1/2 z-50 pointer-events-auto animate-in slide-in-from-bottom-5 duration-200"
      >
        <div className="bg-card/95 border border-primary/40 shadow-2xl rounded-2xl px-5 py-3 flex items-center gap-4 text-foreground ring-1 ring-white/10 backdrop-blur-none">
          {/* Logo & Step Info */}
          <div className="flex items-center gap-2.5 pr-3 border-r border-border">
            <KnoxLogo className="h-5 w-auto" />
            <div className="flex flex-col">
              <div className="flex items-center gap-1.5">
                <span className="text-2xs font-mono font-bold text-primary">
                  Step {currentStepIndex + 1}/{totalSteps}
                </span>
                <span className="size-1 rounded-full bg-muted-foreground/40" />
                <span className="text-2xs font-mono text-muted-foreground uppercase">
                  #{currentStep.tabId || "dashboard"}
                </span>
              </div>
              <span
                className="text-xs font-bold text-foreground font-sans truncate max-w-[200px]"
                title={currentStep.title}
              >
                {currentStep.title}
              </span>
            </div>
          </div>

          {/* Jump Dots */}
          <div className="hidden md:flex items-center gap-1.5">
            {TOUR_STEPS.map((step, idx) => (
              <button
                key={step.id}
                type="button"
                onClick={() => goToStep(idx)}
                className={cn(
                  "size-2 rounded-full transition-all duration-200 cursor-pointer",
                  idx === currentStepIndex
                    ? "w-5 bg-primary"
                    : "bg-muted-foreground/30 hover:bg-muted-foreground/60"
                )}
                title={step.title}
                aria-label={`Jump to step ${idx + 1}`}
              />
            ))}
          </div>

          {/* Prev / Next Controls */}
          <div className="flex items-center gap-1.5">
            {!isFirstStep && (
              <Button
                type="button"
                variant="outline"
                size="sm"
                onClick={goToPrevStep}
                className="h-8 text-xs font-medium px-2.5"
              >
                <ChevronLeft className="size-3.5 mr-0.5" />
                <span>Back</span>
              </Button>
            )}

            <Button
              type="button"
              variant="primary"
              size="sm"
              onClick={goToNextStep}
              className="h-8 text-xs font-semibold px-3"
            >
              <span>{isLastStep ? "Get Started" : "Next"}</span>
              <ChevronRight className="size-3.5 ml-0.5" />
            </Button>
          </div>

          {/* Restore / Expand Modal */}
          <Button
            type="button"
            variant="outline"
            size="sm"
            onClick={() => setIsPeekMode(false)}
            className="h-8 text-2xs gap-1.5 px-2.5 border-primary/30 text-primary hover:bg-primary/10"
            title="Expand full tour view (or press 'P')"
          >
            <Maximize2 className="size-3.5" />
            <span className="hidden sm:inline">Expand Tour</span>
          </Button>

          {/* Close */}
          <Button
            type="button"
            variant="ghost"
            size="icon"
            className="size-8 text-muted-foreground hover:text-foreground"
            onClick={skipTour}
            aria-label="Close tour"
          >
            <X className="size-4" />
          </Button>
        </div>
      </div>
    );
  }

  return (
    <div
      role="dialog"
      aria-modal="true"
      aria-label="Welcome and Getting Started Tour"
      className="fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/20 animate-in fade-in duration-150"
    >
      <div className="w-full max-w-4xl bg-card border border-primary/30 rounded-2xl shadow-2xl overflow-hidden flex flex-col max-h-[90vh] text-foreground">
        {/* Modal Header */}
        <div className="px-6 py-4 border-b border-border-subtle flex items-center justify-between bg-background/50">
          <div className="flex items-center gap-3">
            <KnoxLogo className="h-6 w-auto" />
            <div className="flex items-center gap-2">
              <span className="text-xs font-bold tracking-tight text-foreground font-mono">
                Getting Started Tour
              </span>
              <Badge variant="outline" className="font-mono text-2xs">
                Step {currentStepIndex + 1} of {totalSteps}
              </Badge>
            </div>
          </div>

          <div className="flex items-center gap-2">
            {/* Live background window indicator */}
            <div className="hidden sm:flex items-center gap-1.5 px-2.5 py-1 rounded-full bg-primary/10 border border-primary/25 text-primary text-2xs font-mono font-medium">
              <span className="size-1.5 rounded-full bg-primary animate-pulse" />
              <span>Live Window: #{currentStep.tabId || "dashboard"}</span>
            </div>

            {/* Peek Window Toggle */}
            <Button
              type="button"
              variant="outline"
              size="sm"
              onClick={() => setIsPeekMode(true)}
              className="h-8 text-2xs gap-1.5 border-border hover:border-primary/40 text-muted-foreground hover:text-foreground"
              title="Minimize modal into floating bottom bar to view full background window (press 'P')"
            >
              <Eye className="size-3.5 text-primary" />
              <span className="hidden sm:inline">Peek Window</span>
            </Button>

            <Button
              type="button"
              variant="ghost"
              size="icon"
              className="size-8 text-muted-foreground hover:text-foreground"
              onClick={skipTour}
              aria-label="Close tour"
            >
              <X className="size-4" />
            </Button>
          </div>
        </div>

        {/* Modal Body: Two-Column Responsive Layout */}
        <div className="flex-1 overflow-y-auto p-6 grid grid-cols-1 md:grid-cols-12 gap-6 min-h-[380px]">
          {/* Left Column: Explanation & Highlights */}
          <div className="md:col-span-7 flex flex-col justify-between space-y-4">
            <div className="space-y-3">
              <div className="flex items-center gap-2">
                <Badge
                  variant={currentStep.badgeVariant || "default"}
                  className="uppercase font-mono text-2xs"
                >
                  {currentStep.badge}
                </Badge>
              </div>

              <div>
                <h3 className="text-lg font-bold text-foreground leading-snug tracking-tight">
                  {currentStep.title}
                </h3>
                <p className="text-xs text-primary font-medium mt-1">
                  {currentStep.subtitle}
                </p>
              </div>

              <p className="text-xs text-muted-foreground leading-relaxed">
                {currentStep.description}
              </p>
            </div>

            {/* Highlights List */}
            <div className="space-y-2.5 pt-2 border-t border-border-subtle">
              {currentStep.highlights.map((h, i) => (
                <div key={i} className="flex items-start gap-2.5 text-xs">
                  <div className="p-1 rounded-md bg-primary/10 text-primary shrink-0 mt-0.5">
                    {renderIcon(h.iconName, "size-3.5")}
                  </div>
                  <div className="min-w-0">
                    <span className="font-semibold text-foreground mr-1.5">
                      {h.label}:
                    </span>
                    <span className="text-muted-foreground text-xs leading-relaxed">
                      {h.detail}
                    </span>
                  </div>
                </div>
              ))}
            </div>
          </div>

          {/* Right Column: Visual Preview Card */}
          <div className="md:col-span-5 h-[340px] md:h-auto">
            {renderVisualCard(currentStep)}
          </div>
        </div>

        {/* Modal Footer */}
        <div className="px-6 py-4 border-t border-border-subtle bg-background/50 flex flex-col sm:flex-row items-center justify-between gap-3">
          {/* Don't show again toggle */}
          <label className="flex items-center gap-2 text-xs text-muted-foreground cursor-pointer hover:text-foreground select-none">
            <input
              type="checkbox"
              checked={dontShowAgain}
              onChange={(e) => setDontShowAgain(e.target.checked)}
              className="rounded border-border accent-primary cursor-pointer size-3.5"
            />
            <span>Don't show this again</span>
          </label>

          {/* Step Pill Indicators */}
          <div className="flex items-center gap-1.5">
            {TOUR_STEPS.map((step, idx) => (
              <button
                key={step.id}
                type="button"
                onClick={() => goToStep(idx)}
                className={cn(
                  "size-2 rounded-full transition-all duration-200 cursor-pointer",
                  idx === currentStepIndex
                    ? "w-6 bg-primary"
                    : "bg-muted-foreground/30 hover:bg-muted-foreground/60"
                )}
                aria-label={`Jump to step ${idx + 1}`}
                title={step.title}
              />
            ))}
          </div>

          {/* Navigation Controls */}
          <div className="flex items-center gap-2">
            {!isLastStep && (
              <Button
                type="button"
                variant="ghost"
                size="sm"
                onClick={skipTour}
                className="text-muted-foreground hover:text-foreground text-xs"
              >
                Skip Tour
              </Button>
            )}

            {!isFirstStep && (
              <Button
                type="button"
                variant="outline"
                size="sm"
                onClick={goToPrevStep}
                className="text-xs font-medium"
              >
                <ChevronLeft className="size-3.5 mr-1" />
                <span>Back</span>
              </Button>
            )}

            <Button
              type="button"
              variant="primary"
              size="sm"
              onClick={goToNextStep}
              className="text-xs font-semibold"
            >
              <span>{isLastStep ? "Get Started" : "Next"}</span>
              <ChevronRight className="size-3.5 ml-1" />
            </Button>
          </div>
        </div>
      </div>
    </div>
  );
};

