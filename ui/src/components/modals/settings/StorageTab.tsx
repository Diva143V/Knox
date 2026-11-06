import React, { useState, useEffect } from "react";
import { HardDrive, FolderOpen, ExternalLink, Activity, RefreshCw, CheckCircle2, Layers } from "lucide-react";
import { useKnoxStore } from "@/store/useKnoxStore";
import { KnoxAPI } from "@/services/api";
import { Button } from "@/components/ui/Button";
import { Badge } from "@/components/ui/Badge";
import { cn } from "@/lib/cn";
import type { DbStats } from "@/types/knox";

export const StorageTab: React.FC = () => {
  const { config, updateConfig, showToast, refreshData } = useKnoxStore();
  const [dbStats, setDbStats] = useState<DbStats | null>(null);
  const [isLoadingStats, setIsLoadingStats] = useState(false);
  const [integrityStatus, setIntegrityStatus] = useState<string | null>(null);
  const [isCheckingIntegrity, setIsCheckingIntegrity] = useState(false);
  const [isVacuuming, setIsVacuuming] = useState(false);

  useEffect(() => {
    let cancelled = false;
    setIsLoadingStats(true);
    KnoxAPI.getDbStats()
      .then((stats) => {
        if (!cancelled) setDbStats(stats);
      })
      .catch((e) => console.warn("Failed to load db stats:", e))
      .finally(() => {
        if (!cancelled) setIsLoadingStats(false);
      });
    return () => {
      cancelled = true;
    };
  }, []);

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
      const updatedStats = await KnoxAPI.getDbStats();
      setDbStats(updatedStats);
    } catch (err) {
      showToast(String(err), true);
    } finally {
      setIsVacuuming(false);
    }
  };

  const formatBytes = (bytes: number) => {
    if (!bytes || bytes <= 0 || isNaN(bytes)) return "0 B";
    const k = 1024;
    const sizes = ["B", "KB", "MB", "GB", "TB"];
    const i = Math.min(Math.floor(Math.log(bytes) / Math.log(k)), sizes.length - 1);
    return `${parseFloat((bytes / Math.pow(k, i)).toFixed(2))} ${sizes[i]}`;
  };

  return (
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
  );
};
