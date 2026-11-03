import React, { useEffect, useState } from "react";
import {
  LayoutDashboard,
  Network,
  Search,
  BarChart2,
  FileInput,
  ShieldCheck,
  PlugZap,
  FolderGit2,
  RefreshCw,
  Compass,
  SlidersHorizontal,
  BookOpen,
  PlusCircle,
} from "lucide-react";
import { useKnoxStore } from "@/store/useKnoxStore";
import { cn } from "@/lib/cn";
import { NAV_GROUPS, type NavItemDef } from "@/lib/navigation";
import { QuickThoughtModal } from "@/components/modals/QuickThoughtModal";

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

const RAIL_ITEM =
  "group relative flex h-full flex-1 md:h-9 md:w-9 md:flex-none cursor-pointer select-none items-center justify-center gap-1 px-0.5 md:px-0 transition-colors duration-100";

/**
 * Icon rail on desktop, labelled bottom bar under 768px. Selection is a 2px
 * accent line on the rail's border (top edge in the bottom bar) — never a fill.
 */
export const Sidebar: React.FC = () => {
  const {
    activeTab,
    setActiveTab,
    status,
    isLoading,
    refreshData,
    openTour,
    openSettings,
    setOnboardingOpen,
  } = useKnoxStore();

  const [isThoughtModalOpen, setIsThoughtModalOpen] = useState(false);

  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if ((e.ctrlKey || e.metaKey) && e.altKey && (e.key === "n" || e.key === "N")) {
        e.preventDefault();
        setIsThoughtModalOpen((prev) => !prev);
      }
    };
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, []);

  const utilities = [
    {
      id: "refresh",
      label: "Refresh graph",
      icon: (
        <RefreshCw
          className={cn("size-3.5", isLoading && "animate-spin text-primary")}
        />
      ),
      onClick: () => refreshData(),
      disabled: isLoading,
    },
    {
      id: "thought",
      label: "Record thought (Ctrl+Alt+N)",
      icon: <PlusCircle className="size-3.5" />,
      onClick: () => setIsThoughtModalOpen(true),
    },
    {
      id: "tour",
      label: "Getting started tour",
      icon: <Compass className="size-3.5" />,
      onClick: () => openTour(),
    },
    {
      id: "setup",
      label: "Setup guide",
      icon: <BookOpen className="size-3.5" />,
      onClick: () => setOnboardingOpen(true),
    },
    {
      id: "settings",
      label: "Settings (Ctrl+,)",
      icon: <SlidersHorizontal className="size-3.5" />,
      onClick: () => openSettings(),
    },
  ];

  return (
    <aside
      className="z-20 flex h-14 w-full shrink-0 select-none flex-row border-t border-border bg-sidebar pb-[env(safe-area-inset-bottom)] md:h-auto md:w-14 md:flex-col md:border-t-0 md:border-r md:pb-0"
      aria-label="Primary navigation"
    >
      <nav
        className="flex min-w-0 flex-1 flex-row items-stretch gap-0 overflow-x-auto px-1 [scrollbar-width:none] md:flex-col md:items-center md:gap-0.5 md:overflow-visible md:px-0 md:py-2 [&::-webkit-scrollbar]:hidden"
        aria-label="Views"
      >
        {NAV_GROUPS.map((group, groupIdx) => (
          <React.Fragment key={group.id}>
            {groupIdx > 0 && (
              <div
                className="my-1 h-5 w-px shrink-0 self-center bg-border md:my-1.5 md:h-px md:w-5"
                aria-hidden
              />
            )}
            {group.items.map((item) => {
              const isActive = activeTab === item.id;
              return (
                <button
                  key={item.id}
                  type="button"
                  onClick={() => setActiveTab(item.id)}
                  title={`${item.fullLabel} (${item.shortcut})`}
                  aria-current={isActive ? "page" : undefined}
                  aria-label={item.fullLabel}
                  className={cn(
                    RAIL_ITEM,
                    "flex-col md:flex-row",
                    "focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-focus-ring",
                    isActive
                      ? "text-foreground"
                      : "text-muted-foreground hover:bg-white/[0.03] hover:text-foreground"
                  )}
                >
                  <span
                    className={cn(
                      "shrink-0 transition-colors",
                      isActive && "text-primary"
                    )}
                  >
                    {ICONS[item.icon]}
                  </span>
                  <span
                    className={cn(
                      "max-w-full truncate text-[10px] leading-none md:hidden",
                      isActive ? "text-foreground" : "text-muted-foreground"
                    )}
                  >
                    {item.label}
                  </span>

                  {isActive && (
                    <>
                      {/* Desktop: 2px line on the rail's border */}
                      <span
                        aria-hidden
                        className="absolute -right-[11px] top-1.5 bottom-1.5 hidden w-0.5 rounded-full bg-primary md:block"
                      />
                      {/* Narrow: line under the label */}
                      <span
                        aria-hidden
                        className="absolute left-1/2 top-0 h-0.5 w-6 -translate-x-1/2 rounded-full bg-primary md:hidden"
                      />
                    </>
                  )}
                </button>
              );
            })}
          </React.Fragment>
        ))}
      </nav>

      {/* Utilities: rail bottom on desktop, compact row appended on narrow */}
      <div className="flex shrink-0 items-center gap-0.5 border-l border-border px-1 md:w-full md:flex-col md:gap-1 md:border-l-0 md:border-t md:px-0 md:py-2">
        {utilities.map((util) => (
          <button
            key={util.id}
            type="button"
            onClick={util.onClick}
            disabled={util.disabled}
            aria-label={util.label}
            title={
              util.id === "refresh" && status
                ? `Refresh graph (${status.total_actions} actions)`
                : util.label
            }
            className={cn(
              "flex size-9 shrink-0 cursor-pointer items-center justify-center rounded-[var(--radius-card)] text-muted-foreground transition-colors",
              "hover:bg-white/[0.04] hover:text-foreground",
              "focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-focus-ring",
              "disabled:cursor-progress disabled:opacity-60"
            )}
          >
            {util.icon}
          </button>
        ))}
      </div>

      <QuickThoughtModal
        isOpen={isThoughtModalOpen}
        onClose={() => setIsThoughtModalOpen(false)}
      />
    </aside>
  );
};
