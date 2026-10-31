import React, { useState, useEffect } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { Minus, Square, X, Search, Shield } from "lucide-react";
import { useKnoxStore } from "@/store/useKnoxStore";
import { APP_VERSION } from "@/lib/navigation";
import { cn } from "@/lib/cn";
import { ThemeSwitcher } from "@/components/ui/ThemeSwitcher";

const isTauri =
  typeof window !== "undefined" &&
  ("__TAURI_INTERNALS__" in window || "__TAURI__" in window);

export const CustomTitlebar: React.FC = () => {
  const { toggleCommandPalette, config, status } = useKnoxStore();
  const [isMaximized, setIsMaximized] = useState(false);

  useEffect(() => {
    if (!isTauri) return;
    try {
      const win = getCurrentWindow();
      win.isMaximized().then(setIsMaximized);
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

  const kgPath = config?.kg_path || "knox.db";

  return (
    <div
      data-tauri-drag-region
      className="h-9 w-full border-b border-border bg-sidebar flex items-center justify-between px-2.5 select-none z-50 text-sm shrink-0"
    >
      {/* Left: Brand + DB path */}
      <div className="flex items-center gap-2 min-w-0 no-drag flex-1">
        <div className="flex items-center gap-1.5 font-semibold text-foreground tracking-tight shrink-0">
          <Shield className="size-3.5 text-primary" aria-hidden />
          <span>Knox</span>
          <span className="text-xs text-muted-foreground font-mono font-normal">
            v{APP_VERSION}
          </span>
        </div>

        <div className="h-3 w-px bg-border shrink-0 hidden sm:block" aria-hidden />

        <div
          className="hidden sm:flex items-center gap-1.5 text-xs text-muted-foreground min-w-0"
          title={kgPath}
        >
          <span
            className="size-1.5 rounded-full bg-success shrink-0"
            aria-label="Database online"
          />
          <span className="font-mono truncate max-w-[180px] xl:max-w-[280px]">
            {kgPath}
          </span>
          {status && (
            <span className="text-xs bg-card px-1.5 py-0.5 rounded-sm border border-border shrink-0 font-mono">
              {status.total_actions}
            </span>
          )}
        </div>
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

      {/* Compact command trigger below lg — keeps appearance control visible */}
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

      {/* Right: Appearance + window controls (shrink-0 so theme control never clips) */}
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
