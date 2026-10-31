import React from "react";
import {
  ChevronRight,
  Database,
  RefreshCw,
  LayoutDashboard,
  Network,
  Search,
  BarChart2,
  FileInput,
  ShieldCheck,
  PlugZap,
  FolderGit2,
  Sparkles,
} from "lucide-react";
import { useKnoxStore } from "@/store/useKnoxStore";
import type { ViewTab } from "@/types/knox";

const VIEW_METADATA: Record<
  ViewTab,
  { label: string; description: string; icon: React.ReactNode }
> = {
  dashboard: {
    label: "Dashboard",
    description: "System overview, KPIs, and recent activity feed",
    icon: <LayoutDashboard className="w-4 h-4 text-primary" />,
  },
  graph: {
    label: "Knowledge Graph",
    description: "Interactive causal graph canvas of agent decisions & writes",
    icon: <Network className="w-4 h-4 text-primary" />,
  },
  query: {
    label: "Query Console",
    description: "Full-text search (FTS5) and structured multi-filter lookup",
    icon: <Search className="w-4 h-4 text-primary" />,
  },
  stats: {
    label: "Metrics & Stats",
    description: "Action type and actor distributions with 7-day trend",
    icon: <BarChart2 className="w-4 h-4 text-primary" />,
  },
  ingest: {
    label: "Ingest Transcripts",
    description: "Import Antigravity & AI tool session transcripts (.jsonl)",
    icon: <FileInput className="w-4 h-4 text-primary" />,
  },
  scanner: {
    label: "Human Edit Scanner",
    description: "Git staged diff analyzer and pre-commit hook guard",
    icon: <ShieldCheck className="w-4 h-4 text-primary" />,
  },
  mcp: {
    label: "MCP Server Config",
    description: "Model Context Protocol tools & editor connection snippets",
    icon: <PlugZap className="w-4 h-4 text-primary" />,
  },
  workspace: {
    label: "Workspace & Hook",
    description: "Initialize knox.md memory and register repository hooks",
    icon: <FolderGit2 className="w-4 h-4 text-primary" />,
  },
};

export const Header: React.FC = () => {
  const { activeTab, status, isLoading, refreshData, setOnboardingOpen } = useKnoxStore();
  const currentView = VIEW_METADATA[activeTab] || VIEW_METADATA.dashboard;

  return (
    <header className="h-11 border-b border-border/60 bg-card/40 backdrop-blur px-4 flex items-center justify-between shrink-0 select-none">
      {/* Left: Breadcrumbs & View Name */}
      <div className="flex items-center gap-2 min-w-0">
        <span className="text-xs font-semibold text-muted-foreground hover:text-foreground transition">
          Knox
        </span>
        <ChevronRight className="w-3.5 h-3.5 text-muted-foreground/60 shrink-0" />
        <div className="flex items-center gap-1.5 min-w-0">
          {currentView.icon}
          <span className="text-xs font-semibold text-foreground tracking-tight truncate">
            {currentView.label}
          </span>
        </div>
        <span className="hidden md:inline-block text-[11px] text-muted-foreground ml-2 truncate border-l border-border/50 pl-2">
          {currentView.description}
        </span>
      </div>

      {/* Right: Status Pill, Setup Guide & Refresh Action */}
      <div className="flex items-center gap-2 shrink-0">
        {status && (
          <div className="hidden sm:flex items-center gap-1.5 px-2 py-0.5 rounded-md bg-card/80 border border-border/60 text-[11px] text-muted-foreground font-mono">
            <Database className="w-3 h-3 text-emerald-400" />
            <span>{status.total_actions} actions</span>
          </div>
        )}

        <button
          onClick={() => setOnboardingOpen(true)}
          className="flex items-center gap-1.5 px-2.5 py-1 rounded-md text-xs font-medium text-primary hover:text-primary-foreground hover:bg-primary/20 border border-primary/30 transition cursor-pointer"
          title="Open Setup & Onboarding Wizard"
        >
          <Sparkles className="w-3.5 h-3.5 text-primary" />
          <span className="hidden sm:inline">Setup Guide</span>
        </button>

        <button
          onClick={() => refreshData()}
          disabled={isLoading}
          className="flex items-center gap-1.5 px-2.5 py-1 rounded-md text-xs font-medium text-muted-foreground hover:text-foreground hover:bg-card border border-transparent hover:border-border transition cursor-pointer"
          title="Refresh view data (F5)"
        >
          <RefreshCw className={`w-3.5 h-3.5 ${isLoading ? "animate-spin text-primary" : ""}`} />
          <span className="hidden sm:inline">Refresh</span>
        </button>
      </div>
    </header>
  );
};
