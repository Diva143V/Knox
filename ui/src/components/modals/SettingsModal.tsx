import React, { useState, useMemo } from "react";
import {
  SlidersHorizontal,
  X,
  Palette,
  Database,
  BrainCircuit,
  Network,
  PlugZap,
  ShieldCheck,
  FileInput,
  Monitor,
  Keyboard,
  Info,
  Search,
} from "lucide-react";
import { useKnoxStore } from "@/store/useKnoxStore";
import { cn } from "@/lib/cn";
import { Button } from "@/components/ui/Button";
import type { SettingsTabId } from "@/types/knox";
import {
  AppearanceTab,
  StorageTab,
  ModelTab,
  GraphTab,
  McpTab,
  ScannerTab,
  IngestionTab,
  SystemTab,
  ShortcutsTab,
  AboutTab,
} from "./settings";

interface CategoryDef {
  id: SettingsTabId;
  label: string;
  desc: string;
  icon: React.ReactNode;
}

const CATEGORIES: CategoryDef[] = [
  {
    id: "general",
    label: "Appearance",
    desc: "Theme, density, animations and startup view",
    icon: <Palette className="size-4" />,
  },
  {
    id: "storage",
    label: "Storage & Database",
    desc: "SQLite path, timeouts, telemetry and maintenance",
    icon: <Database className="size-4" />,
  },
  {
    id: "model",
    label: "Model & AI",
    desc: "LLM, embeddings, endpoints and provider settings",
    icon: <BrainCircuit className="size-4" />,
  },
  {
    id: "graph",
    label: "Graph Canvas",
    desc: "Cards vs Cosmos, D3 physics and default layout",
    icon: <Network className="size-4" />,
  },
  {
    id: "mcp",
    label: "AI Agents & MCP",
    desc: "Command modes, guardrails, and detected clients",
    icon: <PlugZap className="size-4" />,
  },
  {
    id: "scanner",
    label: "Git & Scanner",
    desc: "Human author name, ignore rules and hook policy",
    icon: <ShieldCheck className="size-4" />,
  },
  {
    id: "ingestion",
    label: "Ingestion",
    desc: "Transcript deduplication and preview rules",
    icon: <FileInput className="size-4" />,
  },
  {
    id: "system",
    label: "System & Shell",
    desc: "Tray behavior, startup launch and notifications",
    icon: <Monitor className="size-4" />,
  },
  {
    id: "shortcuts",
    label: "Keyboard Shortcuts",
    desc: "Interactive keybindings reference and cheat sheet",
    icon: <Keyboard className="size-4" />,
  },
  {
    id: "about",
    label: "About & Info",
    desc: "App version, system diagnostics and quick tours",
    icon: <Info className="size-4" />,
  },
];

export const SettingsModal: React.FC = () => {
  const {
    isSettingsOpen,
    closeSettings,
    activeSettingsTab,
    openSettings,
    resetUserSettings,
    showToast,
  } = useKnoxStore();

  const [searchQuery, setSearchQuery] = useState("");

  const filteredCategories = useMemo(() => {
    if (!searchQuery.trim()) return CATEGORIES;
    const q = searchQuery.toLowerCase();
    return CATEGORIES.filter(
      (c) => c.label.toLowerCase().includes(q) || c.desc.toLowerCase().includes(q)
    );
  }, [searchQuery]);

  if (!isSettingsOpen) return null;

  return (
    <div
      role="dialog"
      aria-modal="true"
      aria-labelledby="settings-dialog-title"
      className="fixed inset-0 z-50 flex items-center justify-center p-3 sm:p-5 bg-background/80 backdrop-blur-md animate-in fade-in-0 duration-150"
      onClick={(e) => {
        if (e.target === e.currentTarget) closeSettings();
      }}
    >
      <div className="w-full max-w-4xl h-[90vh] max-h-[720px] rounded-xl border border-border bg-card shadow-2xl flex flex-col overflow-hidden text-foreground">
        {/* Top Header */}
        <div className="h-14 px-4 sm:px-6 border-b border-border flex items-center justify-between gap-3 bg-muted/30 shrink-0">
          <div className="flex items-center gap-2.5">
            <div className="size-8 rounded-lg bg-primary/15 border border-primary/30 flex items-center justify-center text-primary">
              <SlidersHorizontal className="size-4" />
            </div>
            <div>
              <h2 id="settings-dialog-title" className="text-sm font-bold text-foreground">
                Knox Preferences
              </h2>
              <p className="text-2xs text-muted-foreground hidden sm:block">
                Workspace, storage engine, agent protocol and visual defaults
              </p>
            </div>
          </div>

          <div className="flex items-center gap-2">
            <div className="relative w-44 sm:w-64">
              <Search className="size-3.5 absolute left-2.5 top-1/2 -translate-y-1/2 text-muted-foreground" />
              <input
                type="text"
                placeholder="Search settings…"
                value={searchQuery}
                onChange={(e) => setSearchQuery(e.target.value)}
                className="w-full h-8 pl-8 pr-2.5 rounded-md bg-background border border-border text-xs text-foreground placeholder:text-muted-foreground focus:outline-none focus:ring-1 focus:ring-primary"
              />
            </div>
            <Button
              type="button"
              variant="ghost"
              size="icon"
              onClick={closeSettings}
              title="Close (Esc)"
              aria-label="Close settings"
              className="size-8"
            >
              <X className="size-4" />
            </Button>
          </div>
        </div>

        {/* Body: Split Sidebar Navigation + Content Panel */}
        <div className="flex-1 flex min-h-0 overflow-hidden">
          {/* Left Category Sidebar */}
          <aside
            className="w-48 sm:w-56 border-r border-border bg-sidebar/50 p-2 overflow-y-auto space-y-1 shrink-0 select-none"
            aria-label="Settings categories"
          >
            {filteredCategories.map((cat) => {
              const isActive = activeSettingsTab === cat.id;
              return (
                <button
                  key={cat.id}
                  type="button"
                  onClick={() => openSettings(cat.id)}
                  className={cn(
                    "w-full flex items-center gap-2.5 px-3 py-2 rounded-lg text-xs font-medium transition-all text-left cursor-pointer",
                    isActive
                      ? "bg-primary text-primary-foreground shadow-xs font-semibold"
                      : "text-muted-foreground hover:text-foreground hover:bg-muted/70"
                  )}
                >
                  <span className={cn("shrink-0", isActive ? "text-primary-foreground" : "text-primary")}>
                    {cat.icon}
                  </span>
                  <span className="truncate">{cat.label}</span>
                </button>
              );
            })}
          </aside>

          {/* Right Scrollable Setting Area */}
          <main className="flex-1 min-w-0 overflow-y-auto p-4 sm:p-6 bg-background space-y-6">
            {activeSettingsTab === "general" && <AppearanceTab />}
            {activeSettingsTab === "storage" && <StorageTab />}
            {activeSettingsTab === "model" && <ModelTab />}
            {activeSettingsTab === "graph" && <GraphTab />}
            {activeSettingsTab === "mcp" && <McpTab />}
            {activeSettingsTab === "scanner" && <ScannerTab />}
            {activeSettingsTab === "ingestion" && <IngestionTab />}
            {activeSettingsTab === "system" && <SystemTab />}
            {activeSettingsTab === "shortcuts" && <ShortcutsTab />}
            {activeSettingsTab === "about" && <AboutTab />}
          </main>
        </div>

        {/* Bottom Actions Bar */}
        <div className="h-12 px-4 sm:px-6 border-t border-border flex items-center justify-between bg-muted/20 shrink-0">
          <Button
            type="button"
            variant="ghost"
            size="sm"
            onClick={() => {
              if (window.confirm("Restore all settings back to factory defaults?")) {
                resetUserSettings();
                showToast("All settings restored to default.");
              }
            }}
            className="text-xs text-muted-foreground hover:text-danger"
          >
            Restore Defaults
          </Button>

          <Button
            type="button"
            variant="primary"
            size="sm"
            onClick={closeSettings}
            className="px-5 text-xs font-semibold"
          >
            Done
          </Button>
        </div>
      </div>
    </div>
  );
};
