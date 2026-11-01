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
  Compass,
} from "lucide-react";
import { useKnoxStore } from "@/store/useKnoxStore";
import { ActionBadge } from "@/components/ui/ActionBadge";
import { ActorBadge } from "@/components/ui/ActorBadge";
import { ALL_NAV_ITEMS, APP_VERSION, type NavItemDef } from "@/lib/navigation";
import { cn } from "@/lib/cn";
import type { ViewTab, KnoxAction } from "@/types/knox";

const ICONS: Record<NavItemDef["icon"], React.ReactNode> = {
  dashboard: <LayoutDashboard className="size-4 text-primary" />,
  graph: <Network className="size-4 text-primary" />,
  query: <Search className="size-4 text-primary" />,
  stats: <BarChart2 className="size-4 text-primary" />,
  ingest: <FileInput className="size-4 text-primary" />,
  scanner: <ShieldCheck className="size-4 text-primary" />,
  mcp: <PlugZap className="size-4 text-primary" />,
  workspace: <FolderGit2 className="size-4 text-primary" />,
};

const itemClass = cn(
  "flex items-center justify-between px-3 py-2 rounded-md text-foreground cursor-pointer transition-colors my-0.5",
  "data-[selected=true]:bg-primary-muted data-[selected=true]:text-foreground",
  "hover:bg-muted/60"
);

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
    openTour,
    openSettings,
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

  useEffect(() => {
    if (!commandPaletteOpen) setSearch("");
  }, [commandPaletteOpen]);

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
    showToast(
      `Filter: ${next ? "Current actions only" : "All historical actions"}`,
      "info"
    );
    setCommandPaletteOpen(false);
  };

  const handleRefresh = async () => {
    await refreshData();
    showToast("Knowledge graph data refreshed");
    setCommandPaletteOpen(false);
  };

  return (
    <div
      role="dialog"
      aria-modal="true"
      aria-label="Command palette"
      onClick={(e) => {
        if (e.target === e.currentTarget) setCommandPaletteOpen(false);
      }}
      className="fixed inset-0 z-50 flex items-start justify-center pt-16 sm:pt-20 bg-background/75 p-4"
    >
      <div className="w-full max-w-xl rounded-lg border border-border bg-popover shadow-md overflow-hidden flex flex-col motion-safe:animate-in motion-safe:fade-in motion-safe:zoom-in-95 motion-safe:duration-150">
        <Command
          className="w-full flex flex-col"
          value={search}
          onValueChange={setSearch}
          label="Command palette"
        >
          <div className="flex items-center px-3 border-b border-border bg-card">
            <Search className="size-4 text-primary mr-2.5 shrink-0" aria-hidden />
            <Command.Input
              autoFocus
              placeholder="Search views, actions, commands, or files…"
              className="w-full py-3 bg-transparent text-xs text-foreground placeholder:text-muted-foreground focus:outline-none font-mono"
            />
            <button
              type="button"
              onClick={() => setCommandPaletteOpen(false)}
              className="p-1 text-muted-foreground hover:text-foreground rounded-sm cursor-pointer transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-focus-ring"
              title="Close (Esc)"
              aria-label="Close command palette"
            >
              <X className="size-4" />
            </button>
          </div>

          <Command.List className="max-h-96 overflow-y-auto p-2 space-y-1 text-xs">
            <Command.Empty className="py-10 text-center px-4">
              <p className="text-xs font-medium text-foreground">No results</p>
              <p className="text-sm text-muted-foreground mt-1">
                Nothing matched &ldquo;{search}&rdquo;. Try a view name, action
                summary, or command.
              </p>
            </Command.Empty>

            <Command.Group
              heading="Views"
              className="text-overline px-2 py-1.5 [&_[cmdk-group-heading]]:text-overline [&_[cmdk-group-heading]]:px-1 [&_[cmdk-group-heading]]:py-1"
            >
              {ALL_NAV_ITEMS.map((view) => (
                <Command.Item
                  key={view.id}
                  value={`${view.fullLabel} ${view.label} ${view.description}`}
                  onSelect={() => navigateTo(view.id)}
                  className={itemClass}
                >
                  <div className="flex items-center gap-2.5 truncate min-w-0">
                    {ICONS[view.icon]}
                    <div className="truncate">
                      <span className="font-medium text-foreground">
                        {view.fullLabel}
                      </span>
                      <span className="text-muted-foreground text-sm ml-2 truncate hidden sm:inline">
                        — {view.description}
                      </span>
                    </div>
                  </div>
                  <kbd className="font-mono text-xs px-1.5 py-0.5 rounded-sm bg-background border border-border text-muted-foreground shrink-0 ml-2">
                    {view.shortcut}
                  </kbd>
                </Command.Item>
              ))}
            </Command.Group>

            <Command.Group
              heading="Commands"
              className="text-overline px-2 py-1.5 [&_[cmdk-group-heading]]:text-overline [&_[cmdk-group-heading]]:px-1 [&_[cmdk-group-heading]]:py-1"
            >
              <Command.Item
                value="Refresh Knowledge Graph F5"
                onSelect={handleRefresh}
                className={itemClass}
              >
                <div className="flex items-center gap-2.5">
                  <RefreshCw className="size-4 text-success" aria-hidden />
                  <span className="font-medium text-foreground">
                    Refresh Knowledge Graph
                  </span>
                </div>
                <kbd className="font-mono text-xs px-1.5 py-0.5 rounded-sm bg-background border border-border text-muted-foreground">
                  F5
                </kbd>
              </Command.Item>

              <Command.Item
                value="Toggle Current-Only Filter"
                onSelect={handleToggleCurrentOnly}
                className={itemClass}
              >
                <div className="flex items-center gap-2.5">
                  <SlidersHorizontal
                    className="size-4 text-warning"
                    aria-hidden
                  />
                  <span className="font-medium text-foreground">
                    Toggle Current-Only Filter
                  </span>
                </div>
                <span className="text-xs font-mono text-muted-foreground">
                  {currentOnly ? "Current" : "All"}
                </span>
              </Command.Item>

              <Command.Item
                value="Scan Staged Git Changes"
                onSelect={() => navigateTo("scanner")}
                className={itemClass}
              >
                <div className="flex items-center gap-2.5">
                  <GitPullRequest className="size-4 text-primary" aria-hidden />
                  <span className="font-medium text-foreground">
                    Scan Staged Git Changes
                  </span>
                </div>
                <kbd className="font-mono text-xs px-1.5 py-0.5 rounded-sm bg-background border border-border text-muted-foreground">
                  G C
                </kbd>
              </Command.Item>

              <Command.Item
                value="Initialize New Workspace"
                onSelect={() => navigateTo("workspace")}
                className={itemClass}
              >
                <div className="flex items-center gap-2.5">
                  <FolderPlus className="size-4 text-plan" aria-hidden />
                  <span className="font-medium text-foreground">
                    Initialize New Workspace
                  </span>
                </div>
                <kbd className="font-mono text-xs px-1.5 py-0.5 rounded-sm bg-background border border-border text-muted-foreground">
                  G W
                </kbd>
              </Command.Item>

              <Command.Item
                value="Welcome & Getting Started Tour"
                onSelect={() => {
                  setCommandPaletteOpen(false);
                  openTour();
                }}
                className={itemClass}
              >
                <div className="flex items-center gap-2.5">
                  <Compass className="size-4 text-primary" aria-hidden />
                  <span className="font-medium text-foreground">
                    Welcome & Getting Started Tour
                  </span>
                </div>
                <kbd className="font-mono text-xs px-1.5 py-0.5 rounded-sm bg-background border border-border text-muted-foreground">
                  Tour
                </kbd>
              </Command.Item>

              <Command.Item
                value="Settings: Open Overall Settings"
                onSelect={() => {
                  setCommandPaletteOpen(false);
                  openSettings("general");
                }}
                className={itemClass}
              >
                <div className="flex items-center gap-2.5">
                  <SlidersHorizontal className="size-4 text-primary" aria-hidden />
                  <span className="font-medium text-foreground">
                    Settings: Open Overall Settings
                  </span>
                </div>
                <kbd className="font-mono text-xs px-1.5 py-0.5 rounded-sm bg-background border border-border text-muted-foreground">
                  Ctrl+,
                </kbd>
              </Command.Item>

              <Command.Item
                value="Settings: Storage & Database Maintenance"
                onSelect={() => {
                  setCommandPaletteOpen(false);
                  openSettings("storage");
                }}
                className={itemClass}
              >
                <div className="flex items-center gap-2.5">
                  <SlidersHorizontal className="size-4 text-primary" aria-hidden />
                  <span className="font-medium text-foreground">
                    Settings: Storage & Database Maintenance
                  </span>
                </div>
                <kbd className="font-mono text-xs px-1.5 py-0.5 rounded-sm bg-background border border-border text-muted-foreground">
                  DB
                </kbd>
              </Command.Item>
            </Command.Group>

            {actions.length > 0 && (
              <Command.Group
                heading="Recent Actions"
                className="text-overline px-2 py-1.5 [&_[cmdk-group-heading]]:text-overline [&_[cmdk-group-heading]]:px-1 [&_[cmdk-group-heading]]:py-1"
              >
                {actions.slice(0, 8).map((action) => (
                  <Command.Item
                    key={action.id}
                    value={`${action.summary} ${action.actor} ${action.action_type} ${action.files?.join(" ") ?? ""}`}
                    onSelect={() => handleSelectAction(action)}
                    className={itemClass}
                  >
                    <div className="flex items-center gap-2 truncate min-w-0">
                      <ActionBadge
                        type={action.action_type}
                        className="text-2xs px-1.5 py-0 shrink-0"
                      />
                      <span className="truncate text-foreground font-medium">
                        {action.summary}
                      </span>
                    </div>
                    <div className="flex items-center gap-2 shrink-0 ml-2">
                      <ActorBadge
                        actor={action.actor}
                        className="text-2xs px-1.5 py-0"
                      />
                      <span className="font-mono text-2xs text-muted-foreground">
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

          <div className="px-3 py-2 border-t border-border bg-card flex items-center justify-between text-sm text-muted-foreground">
            <div className="flex items-center gap-3">
              <span className="flex items-center gap-1">
                <kbd className="font-mono text-xs px-1 rounded-sm bg-background border border-border">
                  ↑↓
                </kbd>
                <span>Navigate</span>
              </span>
              <span className="flex items-center gap-1">
                <kbd className="font-mono text-xs px-1 rounded-sm bg-background border border-border">
                  ↵
                </kbd>
                <span>Select</span>
              </span>
              <span className="flex items-center gap-1">
                <kbd className="font-mono text-xs px-1 rounded-sm bg-background border border-border">
                  esc
                </kbd>
                <span>Close</span>
              </span>
            </div>
            <span className="font-mono text-xs text-primary">
              Knox v{APP_VERSION}
            </span>
          </div>
        </Command>
      </div>
    </div>
  );
};
