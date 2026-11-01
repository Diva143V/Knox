import React, { useState, useEffect, useMemo } from "react";
import {
  SlidersHorizontal,
  X,
  Palette,
  Database,
  Network,
  PlugZap,
  ShieldCheck,
  FileInput,
  Monitor,
  Keyboard,
  Info,
  FolderOpen,
  ExternalLink,
  RefreshCw,
  Plus,
  Compass,
  BookOpen,
  Search,
  HardDrive,
  Layers,
  Activity,
  CheckCircle2,
} from "lucide-react";
import { useKnoxStore } from "@/store/useKnoxStore";
import { KnoxAPI } from "@/services/api";
import { APP_VERSION } from "@/lib/navigation";
import { cn } from "@/lib/cn";
import { Button } from "@/components/ui/Button";
import { Badge } from "@/components/ui/Badge";
import { Input } from "@/components/ui/Input";
import type { SettingsTabId, DbStats, ViewTab } from "@/types/knox";

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
    userSettings,
    updateUserSettings,
    resetUserSettings,
    config,
    updateConfig,
    showToast,
    refreshData,
    openTour,
    setOnboardingOpen,
    setActiveTab,
  } = useKnoxStore();

  const [searchQuery, setSearchQuery] = useState("");
  const [dbStats, setDbStats] = useState<DbStats | null>(null);
  const [isLoadingStats, setIsLoadingStats] = useState(false);
  const [integrityStatus, setIntegrityStatus] = useState<string | null>(null);
  const [isCheckingIntegrity, setIsCheckingIntegrity] = useState(false);
  const [isVacuuming, setIsVacuuming] = useState(false);
  const [newIgnoreInput, setNewIgnoreInput] = useState("");

  // Load live DB stats when Storage tab opens or modal opens
  useEffect(() => {
    if (isSettingsOpen && activeSettingsTab === "storage") {
      setIsLoadingStats(true);
      KnoxAPI.getDbStats()
        .then(setDbStats)
        .catch((e) => console.warn("Failed to load db stats:", e))
        .finally(() => setIsLoadingStats(false));
    }
  }, [isSettingsOpen, activeSettingsTab]);

  const filteredCategories = useMemo(() => {
    if (!searchQuery.trim()) return CATEGORIES;
    const q = searchQuery.toLowerCase();
    return CATEGORIES.filter(
      (c) => c.label.toLowerCase().includes(q) || c.desc.toLowerCase().includes(q)
    );
  }, [searchQuery]);

  if (!isSettingsOpen) return null;

  const handleBrowseDbLocation = async () => {
    try {
      const file = await KnoxAPI.openFileDialog({
        name: "SQLite Database",
        extensions: ["db", "sqlite", "sqlite3"],
      });
      if (file) {
        await updateConfig({ kgPath: file });
        showToast(`Knowledge graph database path updated to: ${file}`);
        await refreshData();
      }
    } catch (err) {
      showToast(String(err), true);
    }
  };

  const handleRevealDbFolder = async () => {
    try {
      await KnoxAPI.revealDbFolder();
    } catch (err) {
      showToast(String(err), true);
    }
  };

  const handleRunIntegrityCheck = async () => {
    try {
      setIsCheckingIntegrity(true);
      const res = await KnoxAPI.runDbIntegrity();
      setIntegrityStatus(res);
      showToast(
        res.toLowerCase() === "ok"
          ? "SQLite integrity check passed with zero corruption!"
          : `Integrity check result: ${res}`,
        res.toLowerCase() !== "ok"
      );
    } catch (err) {
      setIntegrityStatus(String(err));
      showToast(String(err), true);
    } finally {
      setIsCheckingIntegrity(false);
    }
  };

  const handleRunVacuum = async () => {
    try {
      setIsVacuuming(true);
      const res = await KnoxAPI.runDbVacuum();
      showToast(res);
      // Refresh DB stats after vacuuming
      const updatedStats = await KnoxAPI.getDbStats();
      setDbStats(updatedStats);
    } catch (err) {
      showToast(String(err), true);
    } finally {
      setIsVacuuming(false);
    }
  };

  const handleAddIgnorePattern = () => {
    const val = newIgnoreInput.trim();
    if (!val) return;
    if (userSettings.scannerIgnorePatterns.includes(val)) {
      showToast("Pattern already exists", true);
      return;
    }
    updateUserSettings({
      scannerIgnorePatterns: [...userSettings.scannerIgnorePatterns, val],
    });
    setNewIgnoreInput("");
    showToast(`Added pattern: ${val}`);
  };

  const handleRemoveIgnorePattern = (pattern: string) => {
    updateUserSettings({
      scannerIgnorePatterns: userSettings.scannerIgnorePatterns.filter(
        (p) => p !== pattern
      ),
    });
  };

  const formatBytes = (bytes: number) => {
    if (!bytes || bytes <= 0 || isNaN(bytes)) return "0 B";
    const k = 1024;
    const sizes = ["B", "KB", "MB", "GB", "TB"];
    const i = Math.min(Math.floor(Math.log(bytes) / Math.log(k)), sizes.length - 1);
    return `${parseFloat((bytes / Math.pow(k, i)).toFixed(2))} ${sizes[i]}`;
  };

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
              <h2
                id="settings-dialog-title"
                className="text-sm font-semibold tracking-tight text-foreground flex items-center gap-2"
              >
                <span>Settings</span>
                <Badge variant="outline" className="text-3xs uppercase font-mono">
                  v{APP_VERSION}
                </Badge>
              </h2>
              <p className="text-xs text-muted-foreground hidden sm:block">
                Manage global Knox preferences, SQLite database, and agent integrations
              </p>
            </div>
          </div>

          {/* Quick Search in Settings */}
          <div className="flex items-center gap-2">
            <div className="relative w-44 sm:w-60">
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
            {/* 1. GENERAL & APPEARANCE */}
            {activeSettingsTab === "general" && (
              <div className="space-y-6 animate-in fade-in-50 duration-150">
                <div>
                  <h3 className="text-sm font-semibold text-foreground">Appearance & Theme</h3>
                  <p className="text-xs text-muted-foreground">Customize how Knox looks and behaves across your desktop</p>
                </div>

                <div className="space-y-4">
                  {/* Theme Mode */}
                  <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-3 p-3.5 rounded-lg border border-border bg-card/60">
                    <div>
                      <label className="text-xs font-medium text-foreground block">Color Theme</label>
                      <span className="text-xs text-muted-foreground">
                        Select between Graphite Vault (Dark), Graphite Light, or match OS theme
                      </span>
                    </div>
                    <div className="flex items-center gap-1.5 shrink-0">
                      {(["dark", "light", "system"] as const).map((mode) => (
                        <button
                          key={mode}
                          type="button"
                          onClick={() => updateUserSettings({ theme: mode })}
                          className={cn(
                            "px-3 py-1.5 rounded-md text-xs font-medium capitalize border transition-all cursor-pointer",
                            userSettings.theme === mode
                              ? "bg-primary text-primary-foreground border-primary shadow-xs font-semibold"
                              : "bg-muted/70 text-muted-foreground border-border hover:text-foreground hover:bg-muted"
                          )}
                        >
                          {mode}
                        </button>
                      ))}
                    </div>
                  </div>

                  {/* UI Density */}
                  <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-3 p-3.5 rounded-lg border border-border bg-card/60">
                    <div>
                      <label className="text-xs font-medium text-foreground block">Interface Density</label>
                      <span className="text-xs text-muted-foreground">
                        Compact mode condenses table padding and action card sizes
                      </span>
                    </div>
                    <div className="flex items-center gap-1.5 shrink-0">
                      {(["comfortable", "compact"] as const).map((d) => (
                        <button
                          key={d}
                          type="button"
                          onClick={() => updateUserSettings({ uiDensity: d })}
                          className={cn(
                            "px-3 py-1.5 rounded-md text-xs font-medium capitalize border transition-all cursor-pointer",
                            userSettings.uiDensity === d
                              ? "bg-primary text-primary-foreground border-primary shadow-xs font-semibold"
                              : "bg-muted/70 text-muted-foreground border-border hover:text-foreground hover:bg-muted"
                          )}
                        >
                          {d}
                        </button>
                      ))}
                    </div>
                  </div>

                  {/* Glow & Bloom FX */}
                  <div className="flex items-center justify-between p-3.5 rounded-lg border border-border bg-card/60">
                    <div>
                      <label className="text-xs font-medium text-foreground block">Glow & Radial Bloom FX</label>
                      <span className="text-xs text-muted-foreground">
                        Enable glowing Cosmos orbs and smooth neon bloom auras (disable to boost battery life)
                      </span>
                    </div>
                    <input
                      type="checkbox"
                      checked={userSettings.enableGlowFx}
                      onChange={(e) => updateUserSettings({ enableGlowFx: e.target.checked })}
                      className="size-4 rounded border-border text-primary focus:ring-primary cursor-pointer"
                    />
                  </div>

                  {/* Default Startup Tab */}
                  <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-3 p-3.5 rounded-lg border border-border bg-card/60">
                    <div>
                      <label className="text-xs font-medium text-foreground block">Default Startup View</label>
                      <span className="text-xs text-muted-foreground">
                        Which view should open automatically when launching Knox
                      </span>
                    </div>
                    <select
                      value={userSettings.defaultStartupView}
                      onChange={(e) => updateUserSettings({ defaultStartupView: e.target.value as ViewTab })}
                      className="h-8 px-2.5 rounded-md border border-border bg-background text-xs font-mono text-foreground focus:outline-none focus:ring-1 focus:ring-primary cursor-pointer"
                    >
                      <option value="dashboard">Dashboard</option>
                      <option value="graph">Knowledge Graph</option>
                      <option value="query">Query Console</option>
                      <option value="stats">Metrics & Stats</option>
                      <option value="ingest">Ingest Transcripts</option>
                      <option value="scanner">Git Edit Scanner</option>
                      <option value="mcp">MCP Studio</option>
                      <option value="workspace">Workspace</option>
                    </select>
                  </div>
                </div>
              </div>
            )}

            {/* 2. STORAGE & DATABASE */}
            {activeSettingsTab === "storage" && (
              <div className="space-y-6 animate-in fade-in-50 duration-150">
                <div>
                  <h3 className="text-sm font-semibold text-foreground">Storage & Database</h3>
                  <p className="text-xs text-muted-foreground">
                    SQLite WAL persistence, session lifecycle timeouts, and database health maintenance
                  </p>
                </div>

                <div className="space-y-4">
                  {/* Database Path */}
                  <div className="p-4 rounded-lg border border-border bg-card/60 space-y-3">
                    <div className="flex items-center justify-between">
                      <div className="flex items-center gap-2">
                        <HardDrive className="size-4 text-primary" />
                        <span className="text-xs font-medium text-foreground">SQLite Database Location</span>
                      </div>
                      <Badge variant="outline" className="font-mono text-2xs">
                        SQLite 3.48+ (WAL)
                      </Badge>
                    </div>

                    <div className="p-2.5 rounded-md bg-muted/60 border border-border font-mono text-xs text-foreground break-all select-all">
                      {config?.kg_path || "~/.knox/knox.db"}
                    </div>

                    <div className="flex flex-wrap items-center gap-2">
                      <Button
                        type="button"
                        variant="primary"
                        size="sm"
                        onClick={handleBrowseDbLocation}
                        className="gap-1.5"
                      >
                        <FolderOpen className="size-3.5" />
                        <span>Relocate / Browse DB…</span>
                      </Button>

                      <Button
                        type="button"
                        variant="outline"
                        size="sm"
                        onClick={handleRevealDbFolder}
                        className="gap-1.5"
                      >
                        <ExternalLink className="size-3.5" />
                        <span>Reveal in Explorer</span>
                      </Button>
                    </div>
                  </div>

                  {/* Storage Scope Mode */}
                  <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-3 p-3.5 rounded-lg border border-border bg-card/60">
                    <div>
                      <label className="text-xs font-medium text-foreground block">Global Shared Storage Mode</label>
                      <span className="text-xs text-muted-foreground">
                        Single global database across all repositories (recommended) vs isolated .knox folders
                      </span>
                    </div>
                    <div className="flex items-center gap-1.5 shrink-0">
                      <button
                        type="button"
                        onClick={() => updateConfig({ globalMode: true })}
                        className={cn(
                          "px-3 py-1.5 rounded-md text-xs font-medium border transition-all cursor-pointer",
                          config?.global_mode
                            ? "bg-primary text-primary-foreground border-primary shadow-xs font-semibold"
                            : "bg-muted/70 text-muted-foreground border-border hover:text-foreground hover:bg-muted"
                        )}
                      >
                        Global Mode
                      </button>
                      <button
                        type="button"
                        onClick={() => updateConfig({ globalMode: false })}
                        className={cn(
                          "px-3 py-1.5 rounded-md text-xs font-medium border transition-all cursor-pointer",
                          !config?.global_mode
                            ? "bg-primary text-primary-foreground border-primary shadow-xs font-semibold"
                            : "bg-muted/70 text-muted-foreground border-border hover:text-foreground hover:bg-muted"
                        )}
                      >
                        Project-Local
                      </button>
                    </div>
                  </div>

                  {/* Session Idle Timeout */}
                  <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-3 p-3.5 rounded-lg border border-border bg-card/60">
                    <div>
                      <label className="text-xs font-medium text-foreground block">Session Idle Timeout</label>
                      <span className="text-xs text-muted-foreground">
                        Inactivity duration before open AI agent sessions auto-close
                      </span>
                    </div>
                    <div className="flex items-center gap-1.5 shrink-0">
                      {[
                        { label: "15m", val: 900 },
                        { label: "30m", val: 1800 },
                        { label: "1h", val: 3600 },
                        { label: "2h", val: 7200 },
                      ].map((item) => (
                        <button
                          key={item.val}
                          type="button"
                          onClick={() => updateConfig({ sessionTimeoutSeconds: item.val })}
                          className={cn(
                            "px-2.5 py-1 rounded-md text-xs font-mono font-medium border transition-all cursor-pointer",
                            config?.session_timeout_seconds === item.val
                              ? "bg-primary text-primary-foreground border-primary shadow-xs font-semibold"
                              : "bg-muted/70 text-muted-foreground border-border hover:text-foreground hover:bg-muted"
                          )}
                        >
                          {item.label}
                        </button>
                      ))}
                    </div>
                  </div>

                  {/* Live Database Telemetry Card */}
                  <div className="p-4 rounded-lg border border-border bg-card/60 space-y-3">
                    <div className="flex items-center justify-between">
                      <div className="flex items-center gap-2">
                        <Activity className="size-4 text-success" />
                        <span className="text-xs font-medium text-foreground">Live Telemetry & Diagnostics</span>
                      </div>
                      <Button
                        type="button"
                        variant="ghost"
                        size="sm"
                        onClick={() => {
                          setIsLoadingStats(true);
                          KnoxAPI.getDbStats()
                            .then(setDbStats)
                            .finally(() => setIsLoadingStats(false));
                        }}
                        loading={isLoadingStats}
                        className="h-6 text-2xs"
                      >
                        <RefreshCw className="size-3" />
                        <span>Refresh</span>
                      </Button>
                    </div>

                    {dbStats ? (
                      <div className="grid grid-cols-2 sm:grid-cols-4 gap-2.5">
                        <div className="p-2.5 rounded-md bg-muted/40 border border-border">
                          <p className="text-2xs text-muted-foreground uppercase font-mono">DB File Size</p>
                          <p className="text-xs font-semibold text-foreground font-mono mt-0.5">
                            {formatBytes(dbStats.db_size_bytes)}
                          </p>
                        </div>
                        <div className="p-2.5 rounded-md bg-muted/40 border border-border">
                          <p className="text-2xs text-muted-foreground uppercase font-mono">WAL Size</p>
                          <p className="text-xs font-semibold text-foreground font-mono mt-0.5">
                            {formatBytes(dbStats.wal_size_bytes)}
                          </p>
                        </div>
                        <div className="p-2.5 rounded-md bg-muted/40 border border-border">
                          <p className="text-2xs text-muted-foreground uppercase font-mono">Total Actions</p>
                          <p className="text-xs font-semibold text-primary font-mono mt-0.5">
                            {dbStats.total_actions}
                          </p>
                        </div>
                        <div className="p-2.5 rounded-md bg-muted/40 border border-border">
                          <p className="text-2xs text-muted-foreground uppercase font-mono">Relations</p>
                          <p className="text-xs font-semibold text-foreground font-mono mt-0.5">
                            {dbStats.total_relations}
                          </p>
                        </div>
                      </div>
                    ) : (
                      <div className="p-3 text-center text-xs text-muted-foreground">Loading storage stats…</div>
                    )}

                    {/* Maintenance Actions */}
                    <div className="pt-2 border-t border-border flex flex-wrap items-center gap-2">
                      <Button
                        type="button"
                        variant="outline"
                        size="sm"
                        onClick={handleRunIntegrityCheck}
                        loading={isCheckingIntegrity}
                        className="gap-1.5"
                      >
                        <CheckCircle2 className="size-3.5 text-success" />
                        <span>Check SQLite Integrity</span>
                      </Button>

                      <Button
                        type="button"
                        variant="outline"
                        size="sm"
                        onClick={handleRunVacuum}
                        loading={isVacuuming}
                        className="gap-1.5"
                      >
                        <Layers className="size-3.5 text-primary" />
                        <span>Vacuum & Defrag DB</span>
                      </Button>
                    </div>

                    {integrityStatus && (
                      <div className="p-2 rounded-md bg-muted text-xs font-mono text-foreground border border-border">
                        SQLite PRAGMA Result: {integrityStatus}
                      </div>
                    )}
                  </div>
                </div>
              </div>
            )}

            {/* 3. GRAPH CANVAS */}
            {activeSettingsTab === "graph" && (
              <div className="space-y-6 animate-in fade-in-50 duration-150">
                <div>
                  <h3 className="text-sm font-semibold text-foreground">Knowledge Graph Canvas</h3>
                  <p className="text-xs text-muted-foreground">
                    Customize the default visual presentation, continuous D3 physics, and layout parameters
                  </p>
                </div>

                <div className="space-y-4">
                  {/* Default View Mode */}
                  <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-3 p-3.5 rounded-lg border border-border bg-card/60">
                    <div>
                      <label className="text-xs font-medium text-foreground block">Default Node Mode</label>
                      <span className="text-xs text-muted-foreground">
                        Cards mode shows structured rectangular summaries; Cosmos mode renders glowing spherical nodes
                      </span>
                    </div>
                    <div className="flex items-center gap-1.5 shrink-0">
                      {(["cards", "cosmos"] as const).map((mode) => (
                        <button
                          key={mode}
                          type="button"
                          onClick={() => updateUserSettings({ defaultGraphMode: mode })}
                          className={cn(
                            "px-3 py-1.5 rounded-md text-xs font-medium capitalize border transition-all cursor-pointer",
                            userSettings.defaultGraphMode === mode
                              ? "bg-primary text-primary-foreground border-primary shadow-xs font-semibold"
                              : "bg-muted/70 text-muted-foreground border-border hover:text-foreground hover:bg-muted"
                          )}
                        >
                          {mode === "cards" ? "Cards (Structured)" : "Cosmos (Orbs)"}
                        </button>
                      ))}
                    </div>
                  </div>

                  {/* Continuous Live Physics Toggle */}
                  <div className="flex items-center justify-between p-3.5 rounded-lg border border-border bg-card/60">
                    <div>
                      <label className="text-xs font-medium text-foreground block">Continuous D3 Force Physics</label>
                      <span className="text-xs text-muted-foreground">
                        Keep live force simulation active continuously for spring-like organic node interaction
                      </span>
                    </div>
                    <input
                      type="checkbox"
                      checked={userSettings.defaultLivePhysics}
                      onChange={(e) => updateUserSettings({ defaultLivePhysics: e.target.checked })}
                      className="size-4 rounded border-border text-primary focus:ring-primary cursor-pointer"
                    />
                  </div>

                  {/* Default Layout Engine */}
                  <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-3 p-3.5 rounded-lg border border-border bg-card/60">
                    <div>
                      <label className="text-xs font-medium text-foreground block">Default Layout Engine</label>
                      <span className="text-xs text-muted-foreground">
                        Initial graph layout algorithm applied when rendering new subgraphs
                      </span>
                    </div>
                    <select
                      value={userSettings.defaultLayoutEngine}
                      onChange={(e) =>
                        updateUserSettings({
                          defaultLayoutEngine: e.target.value as "hierarchical" | "dagre" | "force" | "grid",
                        })
                      }
                      className="h-8 px-2.5 rounded-md border border-border bg-background text-xs font-mono text-foreground focus:outline-none focus:ring-1 focus:ring-primary cursor-pointer"
                    >
                      <option value="hierarchical">Hierarchical (ELK Layered)</option>
                      <option value="dagre">Dagre (Fast DAG)</option>
                      <option value="force">D3 Force Directed</option>
                      <option value="grid">Compact Grid</option>
                    </select>
                  </div>

                  {/* Overlays: File Nodes, Legend, Edge Labels */}
                  <div className="p-3.5 rounded-lg border border-border bg-card/60 space-y-3">
                    <span className="text-xs font-medium text-foreground block">Canvas Elements & Overlays</span>
                    <div className="space-y-2">
                      <label className="flex items-center gap-2.5 text-xs text-foreground cursor-pointer">
                        <input
                          type="checkbox"
                          checked={userSettings.showFileNodes}
                          onChange={(e) => updateUserSettings({ showFileNodes: e.target.checked })}
                          className="size-4 rounded border-border text-primary focus:ring-primary"
                        />
                        <span>Display touched File nodes alongside Action nodes</span>
                      </label>
                      <label className="flex items-center gap-2.5 text-xs text-foreground cursor-pointer">
                        <input
                          type="checkbox"
                          checked={userSettings.showLegend}
                          onChange={(e) => updateUserSettings({ showLegend: e.target.checked })}
                          className="size-4 rounded border-border text-primary focus:ring-primary"
                        />
                        <span>Show interactive floating color & role legend</span>
                      </label>
                      <label className="flex items-center gap-2.5 text-xs text-foreground cursor-pointer">
                        <input
                          type="checkbox"
                          checked={userSettings.showEdgeLabels}
                          onChange={(e) => updateUserSettings({ showEdgeLabels: e.target.checked })}
                          className="size-4 rounded border-border text-primary focus:ring-primary"
                        />
                        <span>Show directional relationship badges on causal edges (implements, depends_on)</span>
                      </label>
                    </div>
                  </div>
                </div>
              </div>
            )}

            {/* 4. AI AGENTS & MCP */}
            {activeSettingsTab === "mcp" && (
              <div className="space-y-6 animate-in fade-in-50 duration-150">
                <div>
                  <h3 className="text-sm font-semibold text-foreground">AI Agents & MCP Configuration</h3>
                  <p className="text-xs text-muted-foreground">
                    Model Context Protocol server settings and integrations with Antigravity, Cursor, Claude Code, and Gemini
                  </p>
                </div>

                <div className="space-y-4">
                  {/* MCP Command Mode */}
                  <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-3 p-3.5 rounded-lg border border-border bg-card/60">
                    <div>
                      <label className="text-xs font-medium text-foreground block">Command Invocation Mode</label>
                      <span className="text-xs text-muted-foreground">
                        Use "knox" if installed to system PATH, or full absolute executable path for portable installs
                      </span>
                    </div>
                    <div className="flex items-center gap-1.5 shrink-0">
                      <button
                        type="button"
                        onClick={() => updateUserSettings({ mcpCommandMode: "system" })}
                        className={cn(
                          "px-3 py-1.5 rounded-md text-xs font-medium border transition-all cursor-pointer font-mono",
                          userSettings.mcpCommandMode === "system"
                            ? "bg-primary text-primary-foreground border-primary shadow-xs font-semibold"
                            : "bg-muted/70 text-muted-foreground border-border hover:text-foreground hover:bg-muted"
                        )}
                      >
                        knox (System PATH)
                      </button>
                      <button
                        type="button"
                        onClick={() => updateUserSettings({ mcpCommandMode: "absolute" })}
                        className={cn(
                          "px-3 py-1.5 rounded-md text-xs font-medium border transition-all cursor-pointer font-mono",
                          userSettings.mcpCommandMode === "absolute"
                            ? "bg-primary text-primary-foreground border-primary shadow-xs font-semibold"
                            : "bg-muted/70 text-muted-foreground border-border hover:text-foreground hover:bg-muted"
                        )}
                      >
                        Absolute Path
                      </button>
                    </div>
                  </div>

                  {/* Pre-Flight Guardrails Enforcement */}
                  <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-3 p-3.5 rounded-lg border border-border bg-card/60">
                    <div>
                      <label className="text-xs font-medium text-foreground block">Guardrails Enforcement</label>
                      <span className="text-xs text-muted-foreground">
                        Prevent AI coding agents from executing prohibited actions or rejected alternatives
                      </span>
                    </div>
                    <select
                      value={userSettings.guardrailsMode}
                      onChange={(e) =>
                        updateUserSettings({
                          guardrailsMode: e.target.value as "strict" | "advisory" | "off",
                        })
                      }
                      className="h-8 px-2.5 rounded-md border border-border bg-background text-xs font-mono text-foreground focus:outline-none focus:ring-1 focus:ring-primary cursor-pointer"
                    >
                      <option value="advisory">Advisory (Flag warning & allow)</option>
                      <option value="strict">Strict (Fail recording on conflict)</option>
                      <option value="off">Off (Disable checks)</option>
                    </select>
                  </div>

                  {/* Markdown Token Digest Compression */}
                  <div className="flex items-center justify-between p-3.5 rounded-lg border border-border bg-card/60">
                    <div>
                      <label className="text-xs font-medium text-foreground block">Token-Compact Markdown Digest</label>
                      <span className="text-xs text-muted-foreground">
                        Automatically compress MCP context queries into dense Markdown digests (saves &gt;70% context window tokens)
                      </span>
                    </div>
                    <input
                      type="checkbox"
                      checked={userSettings.enableMarkdownDigest}
                      onChange={(e) => updateUserSettings({ enableMarkdownDigest: e.target.checked })}
                      className="size-4 rounded border-border text-primary focus:ring-primary cursor-pointer"
                    />
                  </div>

                  {/* Link to MCP View */}
                  <div className="p-3.5 rounded-lg border border-border bg-card/40 flex items-center justify-between">
                    <div>
                      <p className="text-xs font-medium text-foreground">Need to manage individual agent JSON snippets?</p>
                      <p className="text-2xs text-muted-foreground">
                        Open the full MCP Configuration Studio to inspect and inject custom configs
                      </p>
                    </div>
                    <Button
                      type="button"
                      variant="primary"
                      size="sm"
                      onClick={() => {
                        closeSettings();
                        setActiveTab("mcp");
                      }}
                      className="gap-1 text-xs"
                    >
                      <span>Open MCP Studio</span>
                      <ExternalLink className="size-3" />
                    </Button>
                  </div>
                </div>
              </div>
            )}

            {/* 5. GIT & SCANNER */}
            {activeSettingsTab === "scanner" && (
              <div className="space-y-6 animate-in fade-in-50 duration-150">
                <div>
                  <h3 className="text-sm font-semibold text-foreground">Git & Human Edit Scanner</h3>
                  <p className="text-xs text-muted-foreground">
                    Preferences for detecting manual code changes made outside AI sessions
                  </p>
                </div>

                <div className="space-y-4">
                  {/* Default Author Attribution */}
                  <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-3 p-3.5 rounded-lg border border-border bg-card/60">
                    <div>
                      <label className="text-xs font-medium text-foreground block">Default Human Actor Handle</label>
                      <span className="text-xs text-muted-foreground">
                        Actor tag assigned to manual human edits detected via Git staged changes
                      </span>
                    </div>
                    <Input
                      type="text"
                      value={userSettings.defaultAuthorName}
                      onChange={(e) => updateUserSettings({ defaultAuthorName: e.target.value })}
                      className="w-48 font-mono text-xs"
                    />
                  </div>

                  {/* Pre-commit hook policy */}
                  <div className="flex items-center justify-between p-3.5 rounded-lg border border-border bg-card/60">
                    <div>
                      <label className="text-xs font-medium text-foreground block">Strict Pre-Commit Hook</label>
                      <span className="text-xs text-muted-foreground">
                        Abort git commit if Knox database is locked, rather than proceeding silently
                      </span>
                    </div>
                    <input
                      type="checkbox"
                      checked={userSettings.strictGitHook}
                      onChange={(e) => updateUserSettings({ strictGitHook: e.target.checked })}
                      className="size-4 rounded border-border text-primary focus:ring-primary cursor-pointer"
                    />
                  </div>

                  {/* Diff Scan Depth */}
                  <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-3 p-3.5 rounded-lg border border-border bg-card/60">
                    <div>
                      <label className="text-xs font-medium text-foreground block">Diff Scan Depth Limit</label>
                      <span className="text-xs text-muted-foreground">
                        Maximum staged diff lines analyzed to summarize human commits
                      </span>
                    </div>
                    <div className="flex items-center gap-2">
                      <Input
                        type="number"
                        value={userSettings.diffScanDepth}
                        onChange={(e) =>
                          updateUserSettings({
                            diffScanDepth: parseInt(e.target.value, 10) || 1000,
                          })
                        }
                        className="w-28 font-mono text-xs"
                      />
                      <span className="text-xs text-muted-foreground">lines</span>
                    </div>
                  </div>

                  {/* File Ignore Patterns */}
                  <div className="p-3.5 rounded-lg border border-border bg-card/60 space-y-3">
                    <div>
                      <label className="text-xs font-medium text-foreground block">Scan Excluded Patterns</label>
                      <span className="text-xs text-muted-foreground">
                        File paths matching these glob patterns will be ignored during staged diff inspection
                      </span>
                    </div>

                    <div className="flex flex-wrap gap-1.5 max-h-36 overflow-y-auto p-1.5 rounded-md bg-muted/40 border border-border">
                      {userSettings.scannerIgnorePatterns.map((pat) => (
                        <span
                          key={pat}
                          className="inline-flex items-center gap-1.5 px-2 py-0.5 rounded bg-card border border-border text-2xs font-mono text-foreground"
                        >
                          <span>{pat}</span>
                          <button
                            type="button"
                            onClick={() => handleRemoveIgnorePattern(pat)}
                            className="hover:text-danger cursor-pointer"
                            title={`Remove ${pat}`}
                          >
                            <X className="size-3" />
                          </button>
                        </span>
                      ))}
                    </div>

                    <div className="flex items-center gap-2">
                      <Input
                        type="text"
                        placeholder="e.g. *.lock, build/**"
                        value={newIgnoreInput}
                        onChange={(e) => setNewIgnoreInput(e.target.value)}
                        onKeyDown={(e) => {
                          if (e.key === "Enter") {
                            e.preventDefault();
                            handleAddIgnorePattern();
                          }
                        }}
                        className="font-mono text-xs"
                      />
                      <Button
                        type="button"
                        variant="outline"
                        size="sm"
                        onClick={handleAddIgnorePattern}
                        className="shrink-0"
                      >
                        <Plus className="size-3.5" />
                        <span>Add Pattern</span>
                      </Button>
                    </div>
                  </div>
                </div>
              </div>
            )}

            {/* 6. INGESTION & TRANSCRIPTS */}
            {activeSettingsTab === "ingestion" && (
              <div className="space-y-6 animate-in fade-in-50 duration-150">
                <div>
                  <h3 className="text-sm font-semibold text-foreground">Ingestion & Transcripts</h3>
                  <p className="text-xs text-muted-foreground">
                    Rules for importing IDE conversations and conversation transcripts into the knowledge graph
                  </p>
                </div>

                <div className="space-y-4">
                  {/* Deduplication Strategy */}
                  <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-3 p-3.5 rounded-lg border border-border bg-card/60">
                    <div>
                      <label className="text-xs font-medium text-foreground block">Deduplication Strategy</label>
                      <span className="text-xs text-muted-foreground">
                        How to handle action items with matching SHA-256 fingerprints
                      </span>
                    </div>
                    <div className="flex items-center gap-1.5 shrink-0">
                      <button
                        type="button"
                        onClick={() => updateUserSettings({ deduplicationMode: "skip" })}
                        className={cn(
                          "px-3 py-1.5 rounded-md text-xs font-medium border transition-all cursor-pointer",
                          userSettings.deduplicationMode === "skip"
                            ? "bg-primary text-primary-foreground border-primary shadow-xs font-semibold"
                            : "bg-muted/70 text-muted-foreground border-border hover:text-foreground hover:bg-muted"
                        )}
                      >
                        Skip Existing
                      </button>
                      <button
                        type="button"
                        onClick={() => updateUserSettings({ deduplicationMode: "supersede" })}
                        className={cn(
                          "px-3 py-1.5 rounded-md text-xs font-medium border transition-all cursor-pointer",
                          userSettings.deduplicationMode === "supersede"
                            ? "bg-primary text-primary-foreground border-primary shadow-xs font-semibold"
                            : "bg-muted/70 text-muted-foreground border-border hover:text-foreground hover:bg-muted"
                        )}
                      >
                        Record as Revision
                      </button>
                    </div>
                  </div>

                  {/* Require Ingestion Preview */}
                  <div className="flex items-center justify-between p-3.5 rounded-lg border border-border bg-card/60">
                    <div>
                      <label className="text-xs font-medium text-foreground block">Require Interactive Preview</label>
                      <span className="text-xs text-muted-foreground">
                        Always display preview modal with categorized action count breakdown before committing writes
                      </span>
                    </div>
                    <input
                      type="checkbox"
                      checked={userSettings.requireIngestPreview}
                      onChange={(e) => updateUserSettings({ requireIngestPreview: e.target.checked })}
                      className="size-4 rounded border-border text-primary focus:ring-primary cursor-pointer"
                    />
                  </div>
                </div>
              </div>
            )}

            {/* 7. SYSTEM & SHELL */}
            {activeSettingsTab === "system" && (
              <div className="space-y-6 animate-in fade-in-50 duration-150">
                <div>
                  <h3 className="text-sm font-semibold text-foreground">Desktop & System Shell</h3>
                  <p className="text-xs text-muted-foreground">
                    Native Windows desktop integration, system tray, and background daemon execution
                  </p>
                </div>

                <div className="space-y-4">
                  {/* Minimize to Tray */}
                  <div className="flex items-center justify-between p-3.5 rounded-lg border border-border bg-card/60">
                    <div>
                      <label className="text-xs font-medium text-foreground block">Minimize to System Tray on Close</label>
                      <span className="text-xs text-muted-foreground">
                        Clicking the titlebar &times; hides Knox to the Windows notification tray so AI agents can still query
                      </span>
                    </div>
                    <input
                      type="checkbox"
                      checked={userSettings.closeToTray}
                      onChange={(e) => updateUserSettings({ closeToTray: e.target.checked })}
                      className="size-4 rounded border-border text-primary focus:ring-primary cursor-pointer"
                    />
                  </div>

                  {/* Notifications */}
                  <div className="flex items-center justify-between p-3.5 rounded-lg border border-border bg-card/60">
                    <div>
                      <label className="text-xs font-medium text-foreground block">Desktop Toast Notifications</label>
                      <span className="text-xs text-muted-foreground">
                        Display in-app toast alerts when actions are recorded, doubts resolved, or settings updated
                      </span>
                    </div>
                    <input
                      type="checkbox"
                      checked={userSettings.showDesktopNotifications}
                      onChange={(e) => updateUserSettings({ showDesktopNotifications: e.target.checked })}
                      className="size-4 rounded border-border text-primary focus:ring-primary cursor-pointer"
                    />
                  </div>
                </div>
              </div>
            )}

            {/* 8. KEYBOARD SHORTCUTS */}
            {activeSettingsTab === "shortcuts" && (
              <div className="space-y-6 animate-in fade-in-50 duration-150">
                <div>
                  <h3 className="text-sm font-semibold text-foreground">Keyboard Shortcuts & Cheatsheet</h3>
                  <p className="text-xs text-muted-foreground">
                    Universal keyboard shortcuts for fast power-user navigation in Knox
                  </p>
                </div>

                <div className="grid grid-cols-1 sm:grid-cols-2 gap-2.5">
                  {[
                    { keys: ["Ctrl", ","], label: "Toggle Settings", desc: "Open or dismiss this settings window" },
                    { keys: ["Ctrl", "K"], label: "Command Palette", desc: "Search actions, files, tools and views" },
                    { keys: ["Ctrl", "Alt", "N"], label: "Quick Thought", desc: "Instantly record a doubt, decision, or note" },
                    { keys: ["1"], label: "Dashboard View", desc: "KPIs, trends and recent actions" },
                    { keys: ["2"], label: "Knowledge Graph", desc: "Interactive causal graph canvas" },
                    { keys: ["3"], label: "Search & Query", desc: "FTS5 query and multi-filter console" },
                    { keys: ["4"], label: "Metrics & Stats", desc: "Actor distributions and 7-day volume" },
                    { keys: ["5"], label: "Ingest Studio", desc: "Transcript import and conversation mapper" },
                    { keys: ["6"], label: "Git Edit Scan", desc: "Detect and stage human code commits" },
                    { keys: ["7"], label: "MCP Studio", desc: "Configure AI agent stdio JSON snippets" },
                    { keys: ["8"], label: "Workspace Setup", desc: "Initialize knox.md and repository hooks" },
                    { keys: ["F5"], label: "Refresh Data", desc: "Re-query SQLite database from disk" },
                    { keys: ["P"], label: "Peek Window", desc: "Dock guided tour into a bottom pill" },
                    { keys: ["Esc"], label: "Dismiss Dialog", desc: "Close active modal or inspector pane" },
                  ].map((sc) => (
                    <div
                      key={sc.label}
                      className="p-3 rounded-lg border border-border bg-card/60 flex items-center justify-between gap-3"
                    >
                      <div className="min-w-0">
                        <p className="text-xs font-semibold text-foreground">{sc.label}</p>
                        <p className="text-2xs text-muted-foreground truncate">{sc.desc}</p>
                      </div>
                      <div className="flex items-center gap-1 shrink-0 font-mono text-xs">
                        {sc.keys.map((k) => (
                          <kbd
                            key={k}
                            className="px-1.5 py-0.5 rounded-sm bg-muted border border-border text-foreground text-3xs font-semibold shadow-2xs"
                          >
                            {k}
                          </kbd>
                        ))}
                      </div>
                    </div>
                  ))}
                </div>
              </div>
            )}

            {/* 9. ABOUT & DIAGNOSTICS */}
            {activeSettingsTab === "about" && (
              <div className="space-y-6 animate-in fade-in-50 duration-150">
                <div>
                  <h3 className="text-sm font-semibold text-foreground">About Knox</h3>
                  <p className="text-xs text-muted-foreground">
                    Architectural knowledge graph for AI-assisted software development
                  </p>
                </div>

                <div className="p-4 rounded-xl border border-border bg-card/60 space-y-4">
                  <div className="flex items-center gap-3">
                    <div className="size-12 rounded-xl bg-primary/15 border border-primary/30 flex items-center justify-center text-primary text-xl font-bold">
                      🛡️
                    </div>
                    <div>
                      <h4 className="text-base font-bold text-foreground tracking-tight">Knox Desktop</h4>
                      <p className="text-xs text-muted-foreground font-mono">
                        v{APP_VERSION} &bull; Rust 1.85+ (Edition 2024) &bull; Tauri v2.2
                      </p>
                    </div>
                  </div>

                  <p className="text-xs text-muted-foreground leading-relaxed">
                    Knox acts as persistent, external long-term memory for AI coding agents (Antigravity, Cursor, Claude Code, Gemini CLI).
                    All architectural decisions, implementation actions, and human git commits are indexed into a local-first SQLite WAL knowledge graph with zero telemetry tracking.
                  </p>

                  <div className="grid grid-cols-2 sm:grid-cols-4 gap-2 pt-2 border-t border-border font-mono text-2xs">
                    <div>
                      <span className="text-muted-foreground block uppercase">Subsystem</span>
                      <span className="text-foreground font-semibold">Windows GUI</span>
                    </div>
                    <div>
                      <span className="text-muted-foreground block uppercase">Database</span>
                      <span className="text-foreground font-semibold">SQLite 3.48 (WAL)</span>
                    </div>
                    <div>
                      <span className="text-muted-foreground block uppercase">Frontend</span>
                      <span className="text-foreground font-semibold">React 19 + Tailwind v4</span>
                    </div>
                    <div>
                      <span className="text-muted-foreground block uppercase">License</span>
                      <span className="text-foreground font-semibold">MIT License</span>
                    </div>
                  </div>

                  {/* Quick Action Guides */}
                  <div className="pt-2 border-t border-border flex flex-wrap items-center gap-2">
                    <Button
                      type="button"
                      variant="outline"
                      size="sm"
                      onClick={() => {
                        closeSettings();
                        openTour();
                      }}
                      className="gap-1.5"
                    >
                      <Compass className="size-3.5 text-primary" />
                      <span>Getting Started Tour</span>
                    </Button>

                    <Button
                      type="button"
                      variant="outline"
                      size="sm"
                      onClick={() => {
                        closeSettings();
                        setOnboardingOpen(true);
                      }}
                      className="gap-1.5"
                    >
                      <BookOpen className="size-3.5 text-primary" />
                      <span>Setup & MCP Wizard</span>
                    </Button>
                  </div>
                </div>
              </div>
            )}
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
