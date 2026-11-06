import React, { useState, useEffect, useCallback, useMemo, useRef } from "react";
import {
  PlugZap,
  Terminal,
  ChevronDown,
  ChevronUp,
  HardDrive,
  Plus,
  RefreshCw,
  Sliders,
} from "lucide-react";
import { useKnoxStore } from "@/store/useKnoxStore";
import { KnoxAPI } from "@/services/api";
import type { DetectedMcpApp, McpCommandMode, McpScanLog } from "@/types/knox";
import { EmptyState } from "@/components/ui/EmptyState";
import { Skeleton } from "@/components/ui/Skeleton";
import { PageHeader } from "@/components/ui/PageHeader";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/Card";
import { Button } from "@/components/ui/Button";
import { cn } from "@/lib/cn";

import {
  McpClientCard,
  McpLogsTerminal,
  McpManualConfigModal,
  McpToolInspector,
  McpManualReferenceCard,
  MCP_TOOLS,
  type ConfigTarget,
  type CategoryFilter,
} from "./mcp";

const STORAGE_KEY_CUSTOM_APPS = "knox_custom_mcp_apps";
const STORAGE_KEY_COMMAND_MODE = "knox_mcp_command_mode";

export const McpView: React.FC = () => {
  const { showToast } = useKnoxStore();

  // Command mode: Absolute path vs system command
  const [commandMode, setCommandMode] = useState<McpCommandMode>(() => {
    try {
      const saved = localStorage.getItem(STORAGE_KEY_COMMAND_MODE);
      if (saved === "system" || saved === "absolute") return saved;
    } catch {
      // ignore
    }
    return "absolute";
  });

  const [apps, setApps] = useState<DetectedMcpApp[]>([]);
  const [isScanning, setIsScanning] = useState(false);
  const [isDeepScanning, setIsDeepScanning] = useState(false);
  const [actionLoadingId, setActionLoadingId] = useState<string | null>(null);

  // Collapse/Expand state for detected apps list
  const [showAllApps, setShowAllApps] = useState(false);

  // Live Terminal state
  const [showTerminal, setShowTerminal] = useState(false);
  const [logs, setLogs] = useState<McpScanLog[]>([]);
  const [autoScroll, setAutoScroll] = useState(true);
  const logContainerRef = useRef<HTMLDivElement>(null);

  // Custom app modal state
  const [showCustomModal, setShowCustomModal] = useState(false);
  const [customName, setCustomName] = useState("");
  const [customPath, setCustomPath] = useState("");
  const [customLoading, setCustomLoading] = useState(false);

  // Snippets & documentation
  const [activeTarget, setActiveTarget] = useState<ConfigTarget>("cursor");
  const [categoryFilter, setCategoryFilter] = useState<CategoryFilter>("All");
  const [expandedTool, setExpandedTool] = useState<string | null>("knox_get_context");

  // Setup live event listener for scan logs with cancellation teardown
  useEffect(() => {
    let unlisten: (() => void) | undefined;
    let isMounted = true;
    KnoxAPI.listenMcpScanLogs((log) => {
      if (isMounted) {
        setLogs((prev) => [...prev, log]);
      }
    }).then((fn) => {
      if (isMounted) {
        unlisten = fn;
      } else {
        fn();
      }
    });
    return () => {
      isMounted = false;
      if (unlisten) unlisten();
    };
  }, []);

  // Auto-scroll terminal log window
  useEffect(() => {
    if (autoScroll && logContainerRef.current) {
      logContainerRef.current.scrollTop = logContainerRef.current.scrollHeight;
    }
  }, [logs, autoScroll]);

  const handleClearLogs = () => {
    setLogs([]);
    showToast("Cleared scan activity console.");
  };

  const handleCopyLogs = () => {
    if (logs.length === 0) {
      showToast("No logs to copy", true);
      return;
    }
    const text = logs
      .map((l) => `[${l.timestamp}] [${l.level.toUpperCase()}] ${l.message}`)
      .join("\n");
    navigator.clipboard.writeText(text);
    showToast("Copied scan terminal logs to clipboard!");
  };

  const saveCommandMode = (mode: McpCommandMode) => {
    setCommandMode(mode);
    try {
      localStorage.setItem(STORAGE_KEY_COMMAND_MODE, mode);
    } catch {
      // ignore
    }
    showToast(
      `Switched MCP Command Mode to ${
        mode === "absolute" ? "Absolute Executable Path" : "System PATH ('knox')"
      }`
    );
  };

  const getSavedCustomApps = (): DetectedMcpApp[] => {
    try {
      const raw = localStorage.getItem(STORAGE_KEY_CUSTOM_APPS);
      if (raw) return JSON.parse(raw);
    } catch {
      // ignore
    }
    return [];
  };

  const saveCustomApps = (customList: DetectedMcpApp[]) => {
    try {
      localStorage.setItem(STORAGE_KEY_CUSTOM_APPS, JSON.stringify(customList));
    } catch {
      // ignore
    }
  };

  const refreshConfigs = useCallback(
    async (deep = false, openTerminal = false) => {
      if (openTerminal) {
        setShowTerminal(true);
      }
      if (deep) {
        setIsDeepScanning(true);
      } else {
        setIsScanning(true);
      }

      try {
        const detected = deep
          ? await KnoxAPI.deepScanMcpConfigs()
          : await KnoxAPI.detectMcpConfigs();

        const customSaved = getSavedCustomApps();

        // Refresh status for custom saved apps
        const inspectedCustom: DetectedMcpApp[] = [];
        for (const custom of customSaved) {
          try {
            const inspected = await KnoxAPI.inspectCustomMcpConfig(custom.file_path);
            inspectedCustom.push({
              ...inspected,
              id: custom.id,
              name: custom.name || inspected.name,
              is_custom: true,
            });
          } catch {
            inspectedCustom.push(custom);
          }
        }

        // Merge avoiding duplicates by file_path
        const pathSet = new Set<string>();
        const combined: DetectedMcpApp[] = [];

        for (const item of detected) {
          if (!pathSet.has(item.file_path.toLowerCase())) {
            pathSet.add(item.file_path.toLowerCase());
            combined.push(item);
          }
        }

        for (const item of inspectedCustom) {
          if (!pathSet.has(item.file_path.toLowerCase())) {
            pathSet.add(item.file_path.toLowerCase());
            combined.push(item);
          }
        }

        setApps(combined);
        if (deep) {
          showToast(
            `Deep PC Scan complete! Found ${combined.length} configuration paths.`
          );
        }
      } catch (e) {
        console.error("Failed to scan MCP configs:", e);
        showToast(String(e), true);
      } finally {
        setIsScanning(false);
        setIsDeepScanning(false);
      }
    },
    [showToast]
  );

  useEffect(() => {
    let cancelled = false;
    refreshConfigs(false).catch(() => {
      if (cancelled) return;
    });
    return () => {
      cancelled = true;
    };
  }, [refreshConfigs]);

  const handleInject = async (app: DetectedMcpApp) => {
    setActionLoadingId(app.id);
    try {
      const useSystem = commandMode === "system";
      await KnoxAPI.injectMcpConfig(app.file_path, useSystem);
      showToast(
        `Added Knox to ${app.name} (${useSystem ? "knox" : "absolute path"})!`
      );
      setApps((prev) =>
        prev.map((a) =>
          a.id === app.id ? { ...a, is_configured: true, exists: true } : a
        )
      );
    } catch (e) {
      showToast(`Failed to configure ${app.name}: ${e}`, true);
    } finally {
      setActionLoadingId(null);
    }
  };

  const handleRemove = async (app: DetectedMcpApp) => {
    setActionLoadingId(app.id);
    try {
      await KnoxAPI.removeMcpConfig(app.file_path);
      showToast(`Removed Knox from ${app.name}. Backup saved to .bak.`);
      setApps((prev) =>
        prev.map((a) =>
          a.id === app.id ? { ...a, is_configured: false } : a
        )
      );
    } catch (e) {
      showToast(`Failed to remove Knox from ${app.name}: ${e}`, true);
    } finally {
      setActionLoadingId(null);
    }
  };

  const handleBrowseCustomFile = async () => {
    try {
      const selected = await KnoxAPI.openFileDialog({
        name: "MCP Config JSON",
        extensions: ["json"],
      });
      if (selected) {
        setCustomPath(selected);
        if (!customName) {
          const parts = selected.replace(/\\/g, "/").split("/");
          const fileName = parts[parts.length - 1];
          const folderName = parts.length > 1 ? parts[parts.length - 2] : "";
          if (folderName && folderName.toLowerCase() !== "user") {
            setCustomName(`${folderName} App`);
          } else {
            setCustomName(fileName.replace(".json", ""));
          }
        }
      }
    } catch (e) {
      showToast(`Failed to pick file: ${e}`, true);
    }
  };

  const handleSaveCustomApp = async () => {
    if (!customPath.trim()) {
      showToast("Please provide or browse a configuration file path", true);
      return;
    }

    setCustomLoading(true);
    try {
      const inspected = await KnoxAPI.inspectCustomMcpConfig(customPath.trim());
      const newCustomApp: DetectedMcpApp = {
        ...inspected,
        id: `custom-${Date.now()}`,
        name: customName.trim() || inspected.name,
        is_custom: true,
      };

      const currentCustom = getSavedCustomApps();
      const updatedCustom = [
        ...currentCustom.filter(
          (c) =>
            c.file_path.toLowerCase() !== newCustomApp.file_path.toLowerCase()
        ),
        newCustomApp,
      ];
      saveCustomApps(updatedCustom);

      await KnoxAPI.injectMcpConfig(
        newCustomApp.file_path,
        commandMode === "system"
      );
      newCustomApp.is_configured = true;

      setApps((prev) => [
        newCustomApp,
        ...prev.filter(
          (a) =>
            a.file_path.toLowerCase() !== newCustomApp.file_path.toLowerCase()
        ),
      ]);

      showToast(`Custom app "${newCustomApp.name}" added and Knox configured!`);
      setShowCustomModal(false);
      setCustomName("");
      setCustomPath("");
    } catch (e) {
      showToast(`Failed to add custom app: ${e}`, true);
    } finally {
      setCustomLoading(false);
    }
  };

  const handleDeleteCustomApp = (id: string, e: React.MouseEvent) => {
    e.stopPropagation();
    const currentCustom = getSavedCustomApps();
    const updated = currentCustom.filter((c) => c.id !== id);
    saveCustomApps(updated);
    setApps((prev) => prev.filter((a) => a.id !== id));
    showToast("Removed custom app from list.");
  };

  const getConfigSnippet = (target: ConfigTarget): string => {
    const cmd =
      commandMode === "system" ? "knox" : "D:\\Knox\\target\\release\\knox.exe";
    switch (target) {
      case "cursor":
      case "claude":
        return JSON.stringify(
          {
            mcpServers: {
              knox: {
                command: cmd,
                args: ["serve", "--stdio"],
              },
            },
          },
          null,
          2
        );
      case "antigravity":
        return JSON.stringify(
          {
            mcp: {
              servers: {
                knox: {
                  command: cmd,
                  args: ["serve", "--stdio"],
                },
              },
            },
          },
          null,
          2
        );
      case "cli":
        return `${cmd} serve --stdio`;
    }
  };

  const getTargetLocation = (target: ConfigTarget): string => {
    switch (target) {
      case "cursor":
        return "~/.cursor/mcp.json";
      case "claude":
        return "%APPDATA%\\Claude\\claude_desktop_config.json";
      case "antigravity":
        return "~/.gemini/config/mcp_config.json";
      case "cli":
        return "Terminal / Shell command";
    }
  };

  const currentSnippet = getConfigSnippet(activeTarget);

  const filteredTools = useMemo(() => {
    if (categoryFilter === "All") return MCP_TOOLS;
    return MCP_TOOLS.filter((t) => t.category === categoryFilter);
  }, [categoryFilter]);

  const categories: CategoryFilter[] = [
    "All",
    "Read",
    "Write",
    "Query",
    "Graph",
    "Safety",
    "Scanner",
    "Lifecycle",
  ];

  const MAX_COLLAPSED_APPS = 4;
  const visibleApps = showAllApps ? apps : apps.slice(0, MAX_COLLAPSED_APPS);
  const hiddenAppsCount = Math.max(0, apps.length - MAX_COLLAPSED_APPS);

  return (
    <div
      className={cn(
        "p-5 sm:p-6 space-y-5 mx-auto overflow-y-auto h-full transition-all duration-300",
        showTerminal ? "max-w-7xl" : "max-w-4xl"
      )}
    >
      <PageHeader
        title="Model Context Protocol (MCP) Server"
        description="Auto-detect, directly inject, and manage the Knox stdio MCP bridge across all your IDEs, coding tools, and private applications."
        icon={<PlugZap />}
      />

      <div
        className={cn(
          "flex flex-col",
          showTerminal && "xl:flex-row items-start gap-6"
        )}
      >
        {/* Left Column */}
        <div
          className={cn(
            "space-y-5 min-w-0",
            showTerminal ? "w-full xl:flex-1" : "w-full"
          )}
        >
          {/* SECTION 1: AUTO-CONFIGURATOR */}
          <Card>
            <CardHeader className="pb-0">
              <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-3 border-b border-border-subtle pb-4">
                <div>
                  <CardTitle className="flex items-center gap-2">
                    <Sliders className="size-4 text-primary" aria-hidden />
                    IDE & Application Auto-Configurator
                  </CardTitle>
                  <p className="text-sm text-muted-foreground mt-0.5">
                    1-Click injection of Knox stdio server into detected configuration files.
                  </p>
                </div>

                <div className="flex flex-wrap items-center gap-2">
                  <Button
                    type="button"
                    variant="outline"
                    size="sm"
                    onClick={() => refreshConfigs(false, true)}
                    disabled={isScanning || isDeepScanning}
                    title="Quick scan of standard IDE directories"
                  >
                    <RefreshCw
                      className={cn(
                        "size-3.5",
                        isScanning && "animate-spin text-primary"
                      )}
                    />
                    <span>{isScanning ? "Scanning..." : "Quick Scan"}</span>
                  </Button>

                  <Button
                    type="button"
                    variant="outline"
                    size="sm"
                    onClick={() => refreshConfigs(true, true)}
                    disabled={isScanning || isDeepScanning}
                    title="Deep scan across system drives and storage volumes"
                  >
                    <HardDrive
                      className={cn(
                        "size-3.5",
                        isDeepScanning && "animate-pulse text-primary"
                      )}
                    />
                    <span>
                      {isDeepScanning ? "Searching System..." : "Deep Search"}
                    </span>
                  </Button>

                  <Button
                    type="button"
                    variant={showTerminal ? "primary" : "outline"}
                    size="sm"
                    onClick={() => setShowTerminal((prev) => !prev)}
                    title={
                      showTerminal
                        ? "Hide Live Scan Terminal"
                        : "Open Live Scan Terminal"
                    }
                  >
                    <Terminal className="size-3.5" />
                    <span>Live Logs</span>
                    <ChevronDown
                      className={cn(
                        "size-3 transition-transform duration-200",
                        showTerminal && "rotate-180"
                      )}
                    />
                  </Button>

                  <Button
                    type="button"
                    variant="primary"
                    size="sm"
                    onClick={() => setShowCustomModal(true)}
                  >
                    <Plus className="size-3.5" />
                    <span>Add Custom App</span>
                  </Button>
                </div>
              </div>
            </CardHeader>

            <CardContent className="pt-4 space-y-4">
              {/* Command Path Mode Toggle */}
              <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-3 p-3 rounded-md bg-muted/40 border border-border text-xs">
                <div className="space-y-0.5">
                  <span className="font-semibold text-foreground flex items-center gap-1.5">
                    Command Path Injection Mode:
                  </span>
                  <p className="text-2xs text-muted-foreground">
                    {commandMode === "absolute"
                      ? "Writes full executable path (100% fail-proof, no Windows PATH setup required)."
                      : "Writes system command 'knox' (ideal if syncing configs across multiple PCs via Git)."}
                  </p>
                </div>

                <div className="inline-flex rounded-md border border-border bg-card p-0.5 shrink-0">
                  <button
                    type="button"
                    onClick={() => saveCommandMode("absolute")}
                    className={cn(
                      "px-2.5 py-1 rounded-sm text-2xs font-medium transition-colors cursor-pointer",
                      commandMode === "absolute"
                        ? "bg-primary text-primary-foreground"
                        : "text-muted-foreground hover:text-foreground"
                    )}
                  >
                    Absolute Path (Recommended)
                  </button>
                  <button
                    type="button"
                    onClick={() => saveCommandMode("system")}
                    className={cn(
                      "px-2.5 py-1 rounded-sm text-2xs font-medium transition-colors cursor-pointer",
                      commandMode === "system"
                        ? "bg-primary text-primary-foreground"
                        : "text-muted-foreground hover:text-foreground"
                    )}
                  >
                    System PATH ('knox')
                  </button>
                </div>
              </div>

              {/* Detected Applications List */}
              <div className="space-y-2">
                {isScanning || isDeepScanning ? (
                  <div className="space-y-2 py-1" role="status" aria-label="Scanning for MCP applications">
                    {Array.from({ length: 3 }).map((_, i) => (
                      <div
                        key={i}
                        className="flex flex-col sm:flex-row sm:items-center justify-between p-3 rounded-md border border-border bg-card/60 gap-3"
                      >
                        <div className="space-y-2 min-w-0 flex-1">
                          <div className="flex items-center gap-2">
                            <Skeleton className="h-4 w-28 rounded-sm" />
                            <Skeleton className="h-4 w-16 rounded-sm" />
                          </div>
                          <Skeleton className="h-3 w-48 rounded-sm" />
                        </div>
                        <Skeleton className="h-8 w-24 rounded-md shrink-0" />
                      </div>
                    ))}
                  </div>
                ) : apps.length === 0 ? (
                  <EmptyState
                    icon={<HardDrive className="size-6 text-muted-foreground/50" />}
                    title="No applications detected"
                    description="Run Quick Scan or Deep Search to discover installed MCP clients across your system."
                    action={
                      <Button
                        variant="outline"
                        size="sm"
                        onClick={() => refreshConfigs(false, true)}
                        className="gap-1.5"
                      >
                        <RefreshCw className="size-3.5" />
                        Quick scan
                      </Button>
                    }
                    compact
                  />
                ) : (
                  <>
                    {visibleApps.map((app) => (
                      <McpClientCard
                        key={app.id}
                        app={app}
                        actionLoadingId={actionLoadingId}
                        onInject={handleInject}
                        onRemove={handleRemove}
                        onDeleteCustomApp={handleDeleteCustomApp}
                      />
                    ))}

                    {hiddenAppsCount > 0 && (
                      <button
                        type="button"
                        onClick={() => setShowAllApps((prev) => !prev)}
                        className="w-full py-2 px-3 rounded-md border border-dashed border-border bg-background hover:bg-muted text-xs font-medium text-foreground transition-colors cursor-pointer flex items-center justify-center gap-1.5 mt-2"
                      >
                        {showAllApps ? (
                          <>
                            <ChevronUp className="size-3.5 text-primary" />
                            <span>
                              Show Less (Displaying all {apps.length} applications)
                            </span>
                          </>
                        ) : (
                          <>
                            <ChevronDown className="size-3.5 text-primary" />
                            <span>
                              Show More ({hiddenAppsCount} more detected application
                              {hiddenAppsCount > 1 ? "s" : ""})
                            </span>
                          </>
                        )}
                      </button>
                    )}
                  </>
                )}
              </div>
            </CardContent>
          </Card>

          {/* SECTION 2: MANUAL REFERENCE & SNIPPETS */}
          <McpManualReferenceCard
            activeTarget={activeTarget}
            setActiveTarget={setActiveTarget}
            getTargetLocation={getTargetLocation}
            currentSnippet={currentSnippet}
            showToast={showToast}
          />

          {/* SECTION 3 & 4: TOOLS ACCORDION, RESOURCES, PROMPTS & GUARDRAILS */}
          <McpToolInspector
            filteredTools={filteredTools}
            categories={categories}
            categoryFilter={categoryFilter}
            setCategoryFilter={setCategoryFilter}
            expandedTool={expandedTool}
            setExpandedTool={setExpandedTool}
            showToast={showToast}
          />
        </div>

        {/* Right Column: Live Scan Terminal */}
        <McpLogsTerminal
          showTerminal={showTerminal}
          setShowTerminal={setShowTerminal}
          logs={logs}
          autoScroll={autoScroll}
          setAutoScroll={setAutoScroll}
          logContainerRef={logContainerRef}
          onClearLogs={handleClearLogs}
          onCopyLogs={handleCopyLogs}
          isScanning={isScanning}
          isDeepScanning={isDeepScanning}
        />
      </div>

      {/* Add Custom / Private App Modal */}
      <McpManualConfigModal
        showCustomModal={showCustomModal}
        setShowCustomModal={setShowCustomModal}
        customName={customName}
        setCustomName={setCustomName}
        customPath={customPath}
        setCustomPath={setCustomPath}
        customLoading={customLoading}
        onBrowseCustomFile={handleBrowseCustomFile}
        onSaveCustomApp={handleSaveCustomApp}
      />
    </div>
  );
};
