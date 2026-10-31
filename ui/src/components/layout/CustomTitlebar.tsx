import React, { useState, useEffect, useRef } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import {
  Minus,
  Square,
  X,
  Search,
  Globe,
  HardDrive,
  FolderOpen,
  ChevronDown,
  Check,
  Trash2,
} from "lucide-react";
import { KnoxLogo } from "@/components/ui/KnoxLogo";
import { useKnoxStore } from "@/store/useKnoxStore";
import { KnoxAPI } from "@/services/api";
import { APP_VERSION } from "@/lib/navigation";
import { cn } from "@/lib/cn";
import { ThemeSwitcher } from "@/components/ui/ThemeSwitcher";

const isTauri =
  typeof window !== "undefined" &&
  ("__TAURI_INTERNALS__" in window || "__TAURI__" in window);

export const CustomTitlebar: React.FC = () => {
  const {
    toggleCommandPalette,
    config,
    status,
    activeProjectScope,
    setProjectScope,
    recentProjects,
    addRecentProject,
    removeRecentProject,
  } = useKnoxStore();
  const [isMaximized, setIsMaximized] = useState(false);
  const [dropdownOpen, setDropdownOpen] = useState(false);
  const dropdownRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const handleClickOutside = (event: MouseEvent) => {
      if (dropdownRef.current && !dropdownRef.current.contains(event.target as Node)) {
        setDropdownOpen(false);
      }
    };
    if (dropdownOpen) {
      document.addEventListener("mousedown", handleClickOutside);
    }
    return () => {
      document.removeEventListener("mousedown", handleClickOutside);
    };
  }, [dropdownOpen]);

  useEffect(() => {
    if (!isTauri) return;
    try {
      const win = getCurrentWindow();
      win
        .isMaximized()
        .then(setIsMaximized)
        .catch((e) => console.warn("Could not query isMaximized", e));
    } catch (e) {
      console.warn("Could not query isMaximized", e);
    }
  }, []);

  const handleMinimize = async () => {
    if (!isTauri) return;
    try {
      await getCurrentWindow().minimize();
    } catch (e) {
      console.error(e);
    }
  };

  const handleToggleMaximize = async () => {
    if (!isTauri) return;
    try {
      const win = getCurrentWindow();
      await win.toggleMaximize();
      setIsMaximized(await win.isMaximized());
    } catch (e) {
      console.error(e);
    }
  };

  const handleClose = async () => {
    if (!isTauri) return;
    try {
      // Hide-to-tray: CloseRequested handled by Tauri backend
      await getCurrentWindow().close();
    } catch (e) {
      console.error(e);
    }
  };

  const handleSelectFolder = async () => {
    try {
      const folder = await KnoxAPI.openFolderDialog();
      if (folder) {
        addRecentProject(folder);
        setDropdownOpen(false);
      }
    } catch (err) {
      console.error("Failed to select folder dialog:", err);
    }
  };

  const currentProjectName = activeProjectScope
    ? activeProjectScope.replace(/\\/g, "/").split("/").filter(Boolean).pop() || activeProjectScope
    : null;

  return (
    <div
      data-tauri-drag-region
      className="h-9 w-full border-b border-border bg-sidebar flex items-center justify-between px-2.5 select-none z-50 text-sm shrink-0"
    >
      {/* Left: Branding & Project / Global Scope Dropdown */}
      <div className="flex items-center gap-2 min-w-0 no-drag flex-1">
        <div className="flex items-center gap-1.5 font-semibold text-foreground tracking-tight shrink-0">
          <KnoxLogo size={18} variant="mark" />
          <span>Knox</span>
          <span className="text-xs text-muted-foreground font-mono font-normal">
            v{APP_VERSION}
          </span>
        </div>

        <div className="h-3 w-px bg-border shrink-0 hidden sm:block" aria-hidden />

        {/* Interactive Scope Switcher Pill */}
        <div className="relative" ref={dropdownRef}>
          <button
            type="button"
            onClick={() => setDropdownOpen(!dropdownOpen)}
            className="flex items-center gap-1.5 px-2 py-0.5 rounded-md bg-card hover:bg-muted border border-border text-foreground transition-colors cursor-pointer text-xs"
            title="Switch Knowledge Graph Scope (Global vs Project Drive)"
          >
            {activeProjectScope ? (
              <HardDrive className="size-3 text-primary shrink-0" />
            ) : (
              <Globe className="size-3 text-primary shrink-0" />
            )}
            <span className="font-medium truncate max-w-[140px] lg:max-w-[180px]">
              {activeProjectScope ? currentProjectName : "Global KG"}
            </span>
            <ChevronDown
              className={cn(
                "size-3 text-muted-foreground transition-transform duration-200",
                dropdownOpen && "rotate-180"
              )}
            />
          </button>

          {/* Scope Dropdown Menu */}
          {dropdownOpen && (
            <div className="absolute top-full left-0 mt-1.5 w-80 rounded-lg border border-border bg-card/95 backdrop-blur-xl shadow-xl p-2 z-50 space-y-1 text-xs">
              <div className="px-2 py-1 text-2xs font-mono uppercase tracking-wider text-muted-foreground flex items-center justify-between">
                <span>Knowledge Graph Scope</span>
                <span className="text-2xs bg-primary/10 text-primary border border-primary/20 px-1 py-0.5 rounded-sm">
                  Single DB
                </span>
              </div>

              {/* Global Option */}
              <button
                type="button"
                onClick={() => {
                  setProjectScope(null);
                  setDropdownOpen(false);
                }}
                className={cn(
                  "w-full flex items-center justify-between p-2 rounded-md transition-colors text-left cursor-pointer",
                  !activeProjectScope
                    ? "bg-primary/15 border border-primary/40 text-foreground"
                    : "hover:bg-muted border border-transparent text-muted-foreground hover:text-foreground"
                )}
              >
                <div className="flex items-center gap-2.5 min-w-0">
                  <div className="size-6 rounded-md bg-primary/15 border border-primary/30 flex items-center justify-center shrink-0">
                    <Globe className="size-3.5 text-primary" />
                  </div>
                  <div className="min-w-0">
                    <p className="text-xs font-semibold leading-tight text-foreground">Global Knowledge Graph</p>
                    <p className="text-2xs font-mono text-muted-foreground truncate max-w-[190px]">
                      {config?.kg_path || "~/.knox/knox.db"}
                    </p>
                  </div>
                </div>
                {!activeProjectScope && <Check className="size-4 text-primary shrink-0" />}
              </button>

              <div className="h-px bg-border my-1" />

              <div className="px-2 py-0.5 text-2xs font-mono uppercase tracking-wider text-muted-foreground">
                Project Folders (Choice KG)
              </div>

              {/* Recent Projects List */}
              <div className="max-h-44 overflow-y-auto space-y-0.5 pr-0.5">
                {recentProjects.map((proj) => {
                  const isSelected =
                    activeProjectScope?.toLowerCase() === proj.path.toLowerCase();
                  return (
                    <div
                      key={proj.path}
                      className={cn(
                        "group flex items-center justify-between p-2 rounded-md transition-colors",
                        isSelected
                          ? "bg-primary/15 border border-primary/40 text-foreground"
                          : "hover:bg-muted border border-transparent text-muted-foreground hover:text-foreground"
                      )}
                    >
                      <button
                        type="button"
                        onClick={() => {
                          setProjectScope(proj.path);
                          setDropdownOpen(false);
                        }}
                        className="flex items-center gap-2.5 min-w-0 text-left flex-1 cursor-pointer"
                      >
                        <div className="size-6 rounded-md bg-primary/15 border border-primary/30 flex items-center justify-center shrink-0">
                          <HardDrive className="size-3.5 text-primary" />
                        </div>
                        <div className="min-w-0">
                          <p className="text-xs font-semibold leading-tight text-foreground">{proj.name}</p>
                          <p
                            className="text-2xs font-mono text-muted-foreground truncate max-w-[180px]"
                            title={proj.path}
                          >
                            {proj.path}
                          </p>
                        </div>
                      </button>

                      <div className="flex items-center gap-1 shrink-0">
                        {isSelected && <Check className="size-3.5 text-primary mr-1" />}
                        <button
                          type="button"
                          onClick={(e) => {
                            e.stopPropagation();
                            removeRecentProject(proj.path);
                          }}
                          className="opacity-0 group-hover:opacity-100 hover:text-danger p-1 rounded-sm hover:bg-muted transition-opacity cursor-pointer"
                          title="Remove from recent"
                          aria-label={`Remove ${proj.name} from recent`}
                        >
                          <Trash2 className="size-3" />
                        </button>
                      </div>
                    </div>
                  );
                })}
              </div>

              {/* Add Folder Action */}
              <button
                type="button"
                onClick={handleSelectFolder}
                className="w-full flex items-center gap-2 p-2 rounded-md text-primary hover:bg-primary/10 border border-dashed border-primary/30 hover:border-primary/50 transition-colors cursor-pointer font-medium text-xs mt-1"
              >
                <FolderOpen className="size-3.5" />
                <span>+ Open Project Folder…</span>
              </button>

              {/* Informative Footer */}
              <div className="p-2 rounded-md bg-muted/50 border border-border text-2xs text-muted-foreground leading-relaxed">
                <span className="font-semibold text-foreground">Single Global Storage:</span> All
                actions persist in <span className="font-mono text-primary">~/.knox/knox.db</span>.
                Switching filters causal branches & files without DB fragmentation.
              </div>
            </div>
          )}
        </div>

        {status && (
          <span className="hidden sm:inline-flex text-xs bg-card px-1.5 py-0.5 rounded-sm border border-border text-muted-foreground font-mono shrink-0">
            {status.total_actions} actions
          </span>
        )}
      </div>

      {/* Center: Command palette trigger */}
      <button
        type="button"
        onClick={toggleCommandPalette}
        aria-label="Open command palette"
        className={cn(
          "no-drag hidden lg:flex items-center gap-2 px-3 py-1 rounded-md max-w-[280px]",
          "bg-card border border-border text-muted-foreground hover:text-foreground",
          "text-xs transition-colors duration-100 cursor-pointer",
          "focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-focus-ring"
        )}
      >
        <Search className="size-3.5 shrink-0" aria-hidden />
        <span className="truncate">Search actions, files, views…</span>
        <kbd className="ml-1 font-mono text-2xs bg-muted px-1 py-0.5 rounded-sm border border-border text-muted-foreground">
          Ctrl+K
        </kbd>
      </button>

      {/* Compact command trigger below lg */}
      <button
        type="button"
        onClick={toggleCommandPalette}
        aria-label="Open command palette"
        className={cn(
          "no-drag lg:hidden flex items-center justify-center size-7 rounded-md",
          "text-muted-foreground hover:text-foreground hover:bg-card",
          "focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-focus-ring cursor-pointer"
        )}
      >
        <Search className="size-3.5" />
      </button>

      {/* Right: Appearance + window controls */}
      <div className="flex items-center gap-1 no-drag flex-1 justify-end min-w-0">
        <div className="shrink-0 mr-1">
          <ThemeSwitcher />
        </div>
        <button
          type="button"
          onClick={handleMinimize}
          className="w-10 h-7 shrink-0 flex items-center justify-center text-muted-foreground hover:text-foreground hover:bg-muted rounded-sm transition-colors cursor-pointer focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-focus-ring"
          title="Minimize"
          aria-label="Minimize window"
        >
          <Minus className="size-3.5" />
        </button>
        <button
          type="button"
          onClick={handleToggleMaximize}
          className="w-10 h-7 shrink-0 flex items-center justify-center text-muted-foreground hover:text-foreground hover:bg-muted rounded-sm transition-colors cursor-pointer focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-focus-ring"
          title={isMaximized ? "Restore" : "Maximize"}
          aria-label={isMaximized ? "Restore window" : "Maximize window"}
        >
          <Square className="size-3" aria-hidden />
        </button>
        <button
          type="button"
          onClick={handleClose}
          className="w-10 h-7 shrink-0 flex items-center justify-center text-muted-foreground hover:text-[var(--close-fg-hover)] hover:bg-danger rounded-sm transition-colors cursor-pointer focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-focus-ring"
          title="Hide to Tray"
          aria-label="Close window (hide to tray)"
        >
          <X className="size-3.5" />
        </button>
      </div>
    </div>
  );
};
