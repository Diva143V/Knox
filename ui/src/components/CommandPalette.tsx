import React, { useEffect, useState } from "react";
import { Command } from "cmdk";
import {
  LayoutDashboard,
  Network,
  Search,
  BarChart2,
  FileInput,
  ShieldCheck,
  PlugZap,
  FolderGit2,
  X,
  RefreshCw,
  SlidersHorizontal,
  FolderPlus,
  GitPullRequest,
} from "lucide-react";
import { useKnoxStore } from "@/store/useKnoxStore";
import { ActionBadge } from "@/components/ui/ActionBadge";
import { ActorBadge } from "@/components/ui/ActorBadge";
import type { ViewTab, KnoxAction } from "@/types/knox";

interface NavView {
  id: ViewTab;
  label: string;
  desc: string;
  icon: React.ReactNode;
  shortcut: string;
}

const VIEWS: NavView[] = [
  {
    id: "dashboard",
    label: "Dashboard",
    desc: "Overview KPIs, recent actions, and diagnostics",
    icon: <LayoutDashboard className="w-4 h-4 text-indigo-400" />,
    shortcut: "1",
  },
  {
    id: "graph",
    label: "Knowledge Graph Canvas",
    desc: "Interactive causal graph, causal edges, and layout engines",
    icon: <Network className="w-4 h-4 text-indigo-400" />,
    shortcut: "2",
  },
  {
    id: "query",
    label: "Query Console",
    desc: "FTS5 keyword search, file prefixes, and multi-filters",
    icon: <Search className="w-4 h-4 text-indigo-400" />,
    shortcut: "3",
  },
  {
    id: "stats",
    label: "Metrics & Stats",
    desc: "7-day activity sparkline and actor/type distribution charts",
    icon: <BarChart2 className="w-4 h-4 text-indigo-400" />,
    shortcut: "4",
  },
  {
    id: "ingest",
    label: "Ingest Transcripts",
    desc: "Drag-and-drop .jsonl transcripts with format auto-detection",
    icon: <FileInput className="w-4 h-4 text-indigo-400" />,
    shortcut: "5",
  },
  {
    id: "scanner",
    label: "Human Edit Scanner",
    desc: "Scan git staged changes and manage pre-commit hook",
    icon: <ShieldCheck className="w-4 h-4 text-indigo-400" />,
    shortcut: "6",
  },
  {
    id: "mcp",
    label: "MCP Server Config",
    desc: "Stdio Model Context Protocol status and 9-tool catalog",
    icon: <PlugZap className="w-4 h-4 text-indigo-400" />,
    shortcut: "7",
  },
  {
    id: "workspace",
    label: "Workspace Setup & knox.md",
    desc: "4-step wizard to initialize repository and instruction file",
    icon: <FolderGit2 className="w-4 h-4 text-indigo-400" />,
    shortcut: "8",
  },
];

export const CommandPalette: React.FC = () => {
  const {
    commandPaletteOpen,
    setCommandPaletteOpen,
    setActiveTab,
    actions,
    setSelectedAction,
    currentOnly,
    setCurrentOnly,
    refreshData,
    showToast,
  } = useKnoxStore();

  const [search, setSearch] = useState("");

  useEffect(() => {
    const down = (e: KeyboardEvent) => {
      if ((e.key === "k" || e.key === "K") && (e.metaKey || e.ctrlKey)) {
        e.preventDefault();
        setCommandPaletteOpen(!commandPaletteOpen);
      }
      if (e.key === "Escape" && commandPaletteOpen) {
        setCommandPaletteOpen(false);
      }
    };

    document.addEventListener("keydown", down);
    return () => document.removeEventListener("keydown", down);
  }, [commandPaletteOpen, setCommandPaletteOpen]);

  if (!commandPaletteOpen) return null;

  const navigateTo = (tab: ViewTab) => {
    setActiveTab(tab);
    setCommandPaletteOpen(false);
    setSearch("");
  };

  const handleSelectAction = (action: KnoxAction) => {
    setSelectedAction(action);
    setActiveTab("graph");
    setCommandPaletteOpen(false);
    setSearch("");
  };

  const handleToggleCurrentOnly = () => {
    const next = !currentOnly;
    setCurrentOnly(next);
    showToast(`Filter: ${next ? "Current actions only" : "All historical actions"}`);
    setCommandPaletteOpen(false);
  };

  const handleRefresh = async () => {
    await refreshData();
    showToast("Knowledge graph data refreshed");
    setCommandPaletteOpen(false);
  };

  return (
    <div
      onClick={(e) => {
        if (e.target === e.currentTarget) setCommandPaletteOpen(false);
      }}
      className="fixed inset-0 z-50 flex items-start justify-center pt-20 bg-background/80 backdrop-blur-md p-4 animate-in fade-in duration-150 select-none"
    >
      <div className="w-full max-w-xl rounded-2xl border border-border/80 bg-card shadow-2xl overflow-hidden flex flex-col">
        <Command
          className="w-full flex flex-col"
          value={search}
          onValueChange={setSearch}
        >
          {/* Search Header */}
          <div className="flex items-center px-4 border-b border-border/80 bg-background/40">
            <Search className="w-4 h-4 text-primary mr-3 shrink-0" />
            <Command.Input
              autoFocus
              placeholder="Search views, actions, commands, or files…"
              className="w-full py-3.5 bg-transparent text-xs text-foreground placeholder:text-muted-foreground focus:outline-none font-mono"
            />
            <button
              onClick={() => setCommandPaletteOpen(false)}
              className="p-1 text-muted-foreground hover:text-foreground rounded cursor-pointer transition"
              title="Close (Esc)"
            >
              <X className="w-4 h-4" />
            </button>
          </div>

          {/* List Results */}
          <Command.List className="max-h-96 overflow-y-auto p-2 space-y-2 text-xs">
            <Command.Empty className="py-10 text-center text-xs text-muted-foreground">
              No results found for &ldquo;{search}&rdquo;.
            </Command.Empty>

            {/* Navigation Group */}
            <Command.Group
              heading="Views & Navigation"
              className="text-muted-foreground font-semibold px-2 py-1 text-[10px] uppercase tracking-wider"
            >
              {VIEWS.map((view) => (
                <Command.Item
                  key={view.id}
                  onSelect={() => navigateTo(view.id)}
                  className="flex items-center justify-between px-3 py-2 rounded-lg text-foreground hover:bg-primary/20 hover:text-primary data-[selected=true]:bg-primary/20 data-[selected=true]:text-primary cursor-pointer transition my-0.5"
                >
                  <div className="flex items-center gap-2.5 truncate">
                    {view.icon}
                    <div className="truncate">
                      <span className="font-medium text-foreground">{view.label}</span>
                      <span className="text-muted-foreground text-[11px] ml-2 truncate hidden sm:inline">
                        — {view.desc}
                      </span>
                    </div>
                  </div>
                  <kbd className="font-mono text-[10px] px-1.5 py-0.5 rounded bg-background border border-border/60 text-muted-foreground shrink-0 ml-2">
                    {view.shortcut}
                  </kbd>
                </Command.Item>
              ))}
            </Command.Group>

            {/* Quick Actions Group */}
            <Command.Group
              heading="Quick Commands"
              className="text-muted-foreground font-semibold px-2 py-1 text-[10px] uppercase tracking-wider pt-1"
            >
              <Command.Item
                onSelect={handleRefresh}
                className="flex items-center justify-between px-3 py-2 rounded-lg text-foreground hover:bg-primary/20 hover:text-primary data-[selected=true]:bg-primary/20 data-[selected=true]:text-primary cursor-pointer transition my-0.5"
              >
                <div className="flex items-center gap-2.5">
                  <RefreshCw className="w-4 h-4 text-emerald-400" />
                  <span className="font-medium text-foreground">Refresh Knowledge Graph</span>
                </div>
                <kbd className="font-mono text-[10px] px-1.5 py-0.5 rounded bg-background border border-border/60 text-muted-foreground">
                  F5
                </kbd>
              </Command.Item>

              <Command.Item
                onSelect={handleToggleCurrentOnly}
                className="flex items-center justify-between px-3 py-2 rounded-lg text-foreground hover:bg-primary/20 hover:text-primary data-[selected=true]:bg-primary/20 data-[selected=true]:text-primary cursor-pointer transition my-0.5"
              >
                <div className="flex items-center gap-2.5">
                  <SlidersHorizontal className="w-4 h-4 text-amber-400" />
                  <span className="font-medium text-foreground">Toggle Current-Only Filter</span>
                </div>
                <span className="text-[10px] font-mono text-muted-foreground">
                  {currentOnly ? "Current" : "All"}
                </span>
              </Command.Item>

              <Command.Item
                onSelect={() => navigateTo("scanner")}
                className="flex items-center justify-between px-3 py-2 rounded-lg text-foreground hover:bg-primary/20 hover:text-primary data-[selected=true]:bg-primary/20 data-[selected=true]:text-primary cursor-pointer transition my-0.5"
              >
                <div className="flex items-center gap-2.5">
                  <GitPullRequest className="w-4 h-4 text-cyan-400" />
                  <span className="font-medium text-foreground">Scan Staged Git Changes</span>
                </div>
                <kbd className="font-mono text-[10px] px-1.5 py-0.5 rounded bg-background border border-border/60 text-muted-foreground">
                  G C
                </kbd>
              </Command.Item>

              <Command.Item
                onSelect={() => navigateTo("workspace")}
                className="flex items-center justify-between px-3 py-2 rounded-lg text-foreground hover:bg-primary/20 hover:text-primary data-[selected=true]:bg-primary/20 data-[selected=true]:text-primary cursor-pointer transition my-0.5"
              >
                <div className="flex items-center gap-2.5">
                  <FolderPlus className="w-4 h-4 text-purple-400" />
                  <span className="font-medium text-foreground">Initialize New Workspace</span>
                </div>
                <kbd className="font-mono text-[10px] px-1.5 py-0.5 rounded bg-background border border-border/60 text-muted-foreground">
                  G W
                </kbd>
              </Command.Item>
            </Command.Group>

            {/* Actions Group */}
            {actions.length > 0 && (
              <Command.Group
                heading="Recent Knowledge Graph Actions"
                className="text-muted-foreground font-semibold px-2 py-1 text-[10px] uppercase tracking-wider pt-1"
              >
                {actions.slice(0, 8).map((action) => (
                  <Command.Item
                    key={action.id}
                    onSelect={() => handleSelectAction(action)}
                    className="flex items-center justify-between px-3 py-2 rounded-lg text-foreground hover:bg-primary/20 hover:text-primary data-[selected=true]:bg-primary/20 data-[selected=true]:text-primary cursor-pointer transition text-xs my-0.5"
                  >
                    <div className="flex items-center gap-2 truncate min-w-0">
                      <ActionBadge type={action.action_type} className="text-[9px] px-1.5 py-0 shrink-0" />
                      <span className="truncate text-foreground font-medium">{action.summary}</span>
                    </div>
                    <div className="flex items-center gap-2 shrink-0 ml-2">
                      <ActorBadge actor={action.actor} className="text-[9px] px-1.5 py-0" />
                      <span className="font-mono text-[9px] text-muted-foreground">
                        {action.files && action.files.length > 0
                          ? `${action.files.length} file${action.files.length > 1 ? "s" : ""}`
                          : ""}
                      </span>
                    </div>
                  </Command.Item>
                ))}
              </Command.Group>
            )}
          </Command.List>

          {/* Palette Footer */}
          <div className="px-4 py-2 border-t border-border/60 bg-background/50 flex items-center justify-between text-[11px] text-muted-foreground">
            <div className="flex items-center gap-3">
              <span className="flex items-center gap-1">
                <kbd className="font-mono text-[10px] px-1 rounded bg-card border border-border">↑↓</kbd>
                <span>Navigate</span>
              </span>
              <span className="flex items-center gap-1">
                <kbd className="font-mono text-[10px] px-1 rounded bg-card border border-border">↵</kbd>
                <span>Select</span>
              </span>
              <span className="flex items-center gap-1">
                <kbd className="font-mono text-[10px] px-1 rounded bg-card border border-border">esc</kbd>
                <span>Close</span>
              </span>
            </div>
            <span className="font-mono text-[10px] text-primary">Knox v1.2.0</span>
          </div>
        </Command>
      </div>
    </div>
  );
};
