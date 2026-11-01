import React, { useState, useEffect } from "react";
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
  Globe,
  HardDrive,
  BookOpen,
  Compass,
  PlusCircle,
  SlidersHorizontal,
} from "lucide-react";
import { useKnoxStore } from "@/store/useKnoxStore";
import { VIEW_BY_ID, type NavItemDef } from "@/lib/navigation";
import { cn } from "@/lib/cn";
import { Button } from "@/components/ui/Button";
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

export const Header: React.FC = () => {
  const {
    activeTab,
    status,
    isLoading,
    refreshData,
    setOnboardingOpen,
    openTour,
    openSettings,
    activeProjectScope,
  } = useKnoxStore();
  const current = VIEW_BY_ID[activeTab] ?? VIEW_BY_ID.dashboard;
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

  const currentProjectName = activeProjectScope
    ? activeProjectScope.replace(/\\/g, "/").split("/").filter(Boolean).pop() || activeProjectScope
    : null;

  return (
    <header
      className={cn(
        "h-11 border-b border-border bg-card/50 px-4",
        "flex items-center justify-between shrink-0 select-none"
      )}
    >
      <nav
        className="flex items-center gap-1.5 min-w-0"
        aria-label="Breadcrumb"
      >
        <span className="flex items-center gap-1 text-xs font-medium text-muted-foreground">
          {activeProjectScope ? (
            <>
              <HardDrive className="size-3 text-primary shrink-0" />
              <span className="truncate max-w-[140px]">{currentProjectName}</span>
            </>
          ) : (
            <>
              <Globe className="size-3 text-primary shrink-0" />
              <span>Global KG</span>
            </>
          )}
        </span>
        <ChevronRight
          className="size-3.5 text-muted-foreground/50 shrink-0"
          aria-hidden
        />
        <div className="flex items-center gap-1.5 min-w-0 text-primary">
          {ICONS[current.icon]}
          <h1 className="text-xs font-semibold text-foreground tracking-tight truncate">
            {current.fullLabel}
          </h1>
        </div>
        <span className="hidden xl:inline-block text-sm text-muted-foreground ml-2 truncate border-l border-border pl-2.5 max-w-md">
          {current.description}
        </span>
      </nav>

      <div className="flex items-center gap-1.5 shrink-0">
        {status && (
          <div
            className="hidden sm:flex items-center gap-1.5 px-2 py-1 rounded-md border border-border bg-background text-sm text-muted-foreground font-mono"
            title="Indexed actions in knowledge graph"
          >
            <Database className="size-3 text-success" aria-hidden />
            <span>
              <span className="text-foreground font-semibold">
                {status.total_actions}
              </span>{" "}
              actions
            </span>
          </div>
        )}

        <Button
          type="button"
          variant="primary"
          size="sm"
          onClick={() => setIsThoughtModalOpen(true)}
          title="Quickly record a technical doubt, minor decision, or note (Ctrl+Alt+N)"
          aria-label="Record thought or doubt"
          className="gap-1.5"
        >
          <PlusCircle className="size-3.5" />
          <span className="hidden sm:inline">+ Thought</span>
        </Button>

        <Button
          type="button"
          variant="outline"
          size="sm"
          onClick={() => openTour()}
          title="Open Welcome & Getting Started Tour"
          aria-label="Open getting started tour"
        >
          <Compass className="size-3.5 text-primary" />
          <span className="hidden sm:inline">Tour</span>
        </Button>

        <Button
          type="button"
          variant="outline"
          size="sm"
          onClick={() => setOnboardingOpen(true)}
          title="Open Setup & Onboarding Wizard"
          aria-label="Open setup guide"
        >
          <BookOpen className="size-3.5 text-primary" />
          <span className="hidden sm:inline">Setup</span>
        </Button>

        <Button
          type="button"
          variant="outline"
          size="sm"
          onClick={() => openSettings()}
          title="Open Overall Settings (Ctrl+,)"
          aria-label="Open overall settings"
        >
          <SlidersHorizontal className="size-3.5 text-primary" />
          <span className="hidden sm:inline">Settings</span>
        </Button>

        <Button
          type="button"
          variant="ghost"
          size="sm"
          onClick={() => refreshData()}
          disabled={isLoading}
          title="Refresh view data (F5)"
          aria-label="Refresh data"
          loading={isLoading}
        >
          {!isLoading && <RefreshCw className="size-3.5" />}
          <span className="hidden sm:inline">Refresh</span>
        </Button>
      </div>

      <QuickThoughtModal
        isOpen={isThoughtModalOpen}
        onClose={() => setIsThoughtModalOpen(false)}
      />
    </header>
  );
};
