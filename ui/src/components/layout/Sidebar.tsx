import React from "react";
import {
  LayoutDashboard,
  Network,
  Search,
  BarChart2,
  FileInput,
  ShieldCheck,
  PlugZap,
  FolderGit2,
  Database,
  RefreshCw,
} from "lucide-react";
import { useKnoxStore } from "@/store/useKnoxStore";
import { cn } from "@/lib/cn";
import { NAV_GROUPS, type NavItemDef } from "@/lib/navigation";
import { Button } from "@/components/ui/Button";

const ICONS: Record<NavItemDef["icon"], React.ReactNode> = {
  dashboard: <LayoutDashboard className="size-4" />,
  graph: <Network className="size-4" />,
  query: <Search className="size-4" />,
  stats: <BarChart2 className="size-4" />,
  ingest: <FileInput className="size-4" />,
  scanner: <ShieldCheck className="size-4" />,
  mcp: <PlugZap className="size-4" />,
  workspace: <FolderGit2 className="size-4" />,
};

export const Sidebar: React.FC = () => {
  const { activeTab, setActiveTab, status, isLoading, refreshData } =
    useKnoxStore();

  return (
    <aside
      className={cn(
        "shrink-0 border-r border-border bg-sidebar flex flex-col select-none",
        "w-14 lg:w-56 transition-[width] duration-150"
      )}
      aria-label="Primary navigation"
    >
      <nav className="flex-1 overflow-y-auto p-2 space-y-4">
        {NAV_GROUPS.map((group) => (
          <div key={group.id} className="space-y-1">
            <div className="px-2.5 py-1 text-overline hidden lg:block">
              {group.label}
            </div>
            <div className="space-y-0.5" role="list">
              {group.items.map((item) => {
                const isActive = activeTab === item.id;
                return (
                  <button
                    key={item.id}
                    type="button"
                    role="listitem"
                    onClick={() => setActiveTab(item.id)}
                    title={`${item.fullLabel} (${item.shortcut})`}
                    aria-current={isActive ? "page" : undefined}
                    aria-label={item.fullLabel}
                    className={cn(
                      "group relative w-full flex items-center gap-2.5 px-2.5 py-2 rounded-md text-sm font-medium transition-colors duration-100 cursor-pointer",
                      "focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-focus-ring",
                      isActive
                        ? "bg-primary-muted text-foreground"
                        : "text-muted-foreground hover:text-foreground hover:bg-muted/60"
                    )}
                  >
                    {/* Teal active rail */}
                    <span
                      aria-hidden
                      className={cn(
                        "absolute left-0 top-1.5 bottom-1.5 w-0.5 rounded-full transition-opacity",
                        isActive ? "bg-primary opacity-100" : "opacity-0"
                      )}
                    />
                    <span
                      className={cn(
                        "shrink-0",
                        isActive ? "text-primary" : "text-muted-foreground group-hover:text-foreground"
                      )}
                    >
                      {ICONS[item.icon]}
                    </span>
                    <span className="hidden lg:inline truncate flex-1 text-left">
                      {item.label}
                    </span>
                    <kbd
                      className={cn(
                        "hidden lg:inline font-mono text-xs px-1 py-0.5 rounded-sm shrink-0",
                        isActive
                          ? "text-primary/80 bg-primary/10"
                          : "text-muted-foreground/50"
                      )}
                    >
                      {item.shortcut}
                    </kbd>
                  </button>
                );
              })}
            </div>
          </div>
        ))}
      </nav>

      {/* Status footer */}
      <div className="p-2.5 border-t border-border space-y-2">
        <div className="flex items-center justify-between text-sm text-muted-foreground">
          <div className="flex items-center gap-1.5 min-w-0">
            <Database className="size-3.5 text-success shrink-0" aria-hidden />
            <span className="hidden lg:inline truncate">SQLite</span>
          </div>
          <Button
            type="button"
            variant="ghost"
            size="icon"
            onClick={() => refreshData()}
            disabled={isLoading}
            aria-label="Refresh knowledge graph"
            title="Refresh knowledge graph"
            className="size-7"
          >
            <RefreshCw
              className={cn("size-3", isLoading && "animate-spin text-primary")}
            />
          </Button>
        </div>

        {status && (
          <div className="hidden lg:block text-xs text-muted-foreground font-mono space-y-0.5 px-0.5">
            <div className="flex justify-between gap-2">
              <span>Actions</span>
              <span className="text-foreground font-semibold">
                {status.total_actions}
              </span>
            </div>
            <div className="flex justify-between gap-2">
              <span>Sessions</span>
              <span className="text-foreground">{status.total_sessions}</span>
            </div>
            <div className="flex justify-between gap-2">
              <span>Projects</span>
              <span className="text-foreground">{status.projects_tracked}</span>
            </div>
          </div>
        )}
      </div>
    </aside>
  );
};
