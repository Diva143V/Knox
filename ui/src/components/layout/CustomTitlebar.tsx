import React, { useState, useEffect } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { Minus, Square, X, Search, Shield } from "lucide-react";
import { useKnoxStore } from "@/store/useKnoxStore";

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
      // In Tauri v2 configured with hide-to-tray, close triggers WindowEvent::CloseRequested
      await getCurrentWindow().close();
    } catch (e) {
      console.error(e);
    }
  };

  return (
    <div
      data-tauri-drag-region
      className="h-10 w-full border-b border-border/60 bg-sidebar/90 backdrop-blur flex items-center justify-between px-3 select-none z-50 text-xs shrink-0"
    >
      {/* Left: Branding & Status */}
      <div className="flex items-center gap-2.5 no-drag">
        <div className="flex items-center gap-1.5 font-bold text-foreground tracking-tight">
          <Shield className="w-4 h-4 text-primary" />
          <span>KNOX</span>
          <span className="text-[10px] text-muted-foreground font-mono font-normal">
            v1.2.0
          </span>
        </div>

        <div className="h-3 w-px bg-border/80 mx-1" />

        <div className="flex items-center gap-1.5 text-[11px] text-muted-foreground">
          <span className="w-2 h-2 rounded-full bg-emerald-500 animate-pulse" />
          <span className="font-mono truncate max-w-[220px]">
            {config?.kg_path || "knox.db"}
          </span>
          {status && (
            <span className="text-[10px] bg-card px-1.5 py-0.5 rounded border border-border">
              {status.total_actions} actions
            </span>
          )}
        </div>
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

