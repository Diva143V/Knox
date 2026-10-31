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
      // In Tauri v2 configured with hide-to-tray, close triggers WindowEvent::CloseRequested
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
      className="h-10 w-full border-b border-border/60 bg-sidebar/90 backdrop-blur flex items-center justify-between px-3 select-none z-50 text-xs shrink-0 relative"
    >
      {/* Left: Branding & Project / Global Scope Dropdown */}
      <div className="flex items-center gap-2.5 no-drag">
        <div className="flex items-center gap-1.5 font-bold text-foreground tracking-tight">
          <KnoxLogo size={18} variant="mark" />
          <span>KNOX</span>
          <span className="text-[10px] text-muted-foreground font-mono font-normal">
            v1.2.1
          </span>
        </div>

        <div className="h-3 w-px bg-border/80 mx-1" />

        {/* Interactive Scope Switcher Pill */}
        <div className="relative" ref={dropdownRef}>
          <button
            onClick={() => setDropdownOpen(!dropdownOpen)}
            className="flex items-center gap-1.5 px-2.5 py-1 rounded-md bg-card/70 hover:bg-card border border-border/70 hover:border-border text-foreground transition cursor-pointer text-[11px]"
            title="Switch Knowledge Graph Scope (Global vs Project Drive)"
          >
            {activeProjectScope ? (
              <HardDrive className="w-3.5 h-3.5 text-primary shrink-0" />
            ) : (
              <Globe className="w-3.5 h-3.5 text-indigo-400 shrink-0" />
            )}
            <span className="font-medium truncate max-w-[170px]">
              {activeProjectScope ? currentProjectName : "Global KG"}
            </span>
            <ChevronDown
              className={`w-3 h-3 text-muted-foreground transition-transform duration-200 ${
                dropdownOpen ? "rotate-180" : ""
              }`}
            />
          </button>

          {/* Scope Dropdown Menu */}
          {dropdownOpen && (
            <div className="absolute top-full left-0 mt-1.5 w-84 rounded-xl border border-border/90 bg-zinc-950/95 backdrop-blur-xl shadow-2xl p-2 z-50 space-y-1 text-xs">
              <div className="px-2 py-1 text-[10px] font-mono uppercase tracking-wider text-muted-foreground flex items-center justify-between">
                <span>Knowledge Graph Scope</span>
                <span className="text-[9px] bg-primary/10 text-primary border border-primary/20 px-1 rounded">
                  Single DB
                </span>
              </div>

              {/* Global Option */}
              <button
                onClick={() => {
                  setProjectScope(null);
                  setDropdownOpen(false);
                }}
                className={`w-full flex items-center justify-between p-2 rounded-lg transition text-left cursor-pointer ${
                  !activeProjectScope
                    ? "bg-primary/15 border border-primary/40 text-foreground"
                    : "hover:bg-zinc-900 border border-transparent text-muted-foreground hover:text-foreground"
                }`}
              >
                <div className="flex items-center gap-2.5 min-w-0">
                  <div className="w-6 h-6 rounded-md bg-indigo-500/20 border border-indigo-500/30 flex items-center justify-center shrink-0">
                    <Globe className="w-3.5 h-3.5 text-indigo-400" />
                  </div>
                  <div className="min-w-0">
                    <p className="text-xs font-semibold leading-tight">Global Knowledge Graph</p>
                    <p className="text-[10px] font-mono text-muted-foreground truncate max-w-[210px]">
                      {config?.kg_path || "~/.knox/knox.db"}
                    </p>
                  </div>
                </div>
                {!activeProjectScope && <Check className="w-4 h-4 text-primary shrink-0" />}
              </button>

              <div className="h-px bg-border/60 my-1" />

              <div className="px-2 py-0.5 text-[10px] font-mono uppercase tracking-wider text-muted-foreground">
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
                      className={`group flex items-center justify-between p-2 rounded-lg transition ${
                        isSelected
                          ? "bg-primary/15 border border-primary/40 text-foreground"
                          : "hover:bg-zinc-900 border border-transparent text-muted-foreground hover:text-foreground"
                      }`}
                    >
                      <button
                        onClick={() => {
                          setProjectScope(proj.path);
                          setDropdownOpen(false);
                        }}
                        className="flex items-center gap-2.5 min-w-0 text-left flex-1 cursor-pointer"
                      >
                        <div className="w-6 h-6 rounded-md bg-primary/20 border border-primary/30 flex items-center justify-center shrink-0">
                          <HardDrive className="w-3.5 h-3.5 text-primary" />
                        </div>
                        <div className="min-w-0">
                          <p className="text-xs font-semibold leading-tight">{proj.name}</p>
                          <p
                            className="text-[10px] font-mono text-muted-foreground truncate max-w-[190px]"
                            title={proj.path}
                          >
                            {proj.path}
                          </p>
                        </div>
                      </button>

                      <div className="flex items-center gap-1 shrink-0">
                        {isSelected && <Check className="w-3.5 h-3.5 text-primary mr-1" />}
                        <button
                          onClick={(e) => {
                            e.stopPropagation();
                            removeRecentProject(proj.path);
                          }}
                          className="opacity-0 group-hover:opacity-100 hover:text-rose-400 p-1 rounded hover:bg-zinc-800 transition cursor-pointer"
                          title="Remove from recent"
                        >
                          <Trash2 className="w-3 h-3" />
                        </button>
                      </div>
                    </div>
                  );
                })}
              </div>

              {/* Add Folder Action */}
              <button
                onClick={handleSelectFolder}
                className="w-full flex items-center gap-2 p-2 rounded-lg text-primary hover:bg-primary/10 border border-dashed border-primary/30 hover:border-primary/50 transition cursor-pointer font-medium text-xs mt-1"
              >
                <FolderOpen className="w-3.5 h-3.5" />
                <span>+ Open Project Folder…</span>
              </button>

              {/* Informative Footer */}
              <div className="p-2 rounded-lg bg-zinc-900/60 border border-zinc-800/80 text-[10px] text-muted-foreground leading-relaxed">
                <span className="font-semibold text-zinc-300">Single Global Storage:</span> All
                actions persist in <span className="font-mono text-primary">~/.knox/knox.db</span>.
                Switching filters causal branches & files without DB fragmentation.
              </div>
            </div>
          )}
        </div>

        {status && (
          <span className="text-[10px] bg-card px-1.5 py-0.5 rounded border border-border text-muted-foreground font-mono">
            {status.total_actions} actions
          </span>
        )}
      </div>

      {/* Center: Command Palette Trigger Button */}
      <button
        onClick={toggleCommandPalette}
        className="no-drag flex items-center gap-2 px-3 py-1 rounded-md bg-card/80 hover:bg-card border border-border/80 text-muted-foreground hover:text-foreground text-[11px] transition-all cursor-pointer"
      >
        <Search className="w-3.5 h-3.5" />
        <span>Search actions, files, views…</span>
        <kbd className="ml-2 font-mono text-[10px] bg-muted/60 px-1 py-0.5 rounded border border-border/50 text-muted-foreground">
          Ctrl+K
        </kbd>
      </button>

      {/* Right: Window Controls */}
      <div className="flex items-center no-drag">
        <button
          onClick={handleMinimize}
          className="w-8 h-7 flex items-center justify-center text-muted-foreground hover:text-foreground hover:bg-card rounded transition cursor-pointer"
          title="Minimize"
        >
          <Minus className="w-3.5 h-3.5" />
        </button>
        <button
          onClick={handleToggleMaximize}
          className="w-8 h-7 flex items-center justify-center text-muted-foreground hover:text-foreground hover:bg-card rounded transition cursor-pointer"
          title={isMaximized ? "Restore" : "Maximize"}
        >
          <Square className="w-3 h-3" />
        </button>
        <button
          onClick={handleClose}
          className="w-8 h-7 flex items-center justify-center text-muted-foreground hover:text-white hover:bg-rose-600 rounded transition cursor-pointer"
          title="Hide to Tray"
        >
          <X className="w-3.5 h-3.5" />
        </button>
      </div>
    </div>
  );
};

