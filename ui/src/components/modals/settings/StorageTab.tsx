import React, { useState, useEffect } from "react";
import {
  HardDrive,
  FolderOpen,
  ExternalLink,
  Activity,
  RefreshCw,
  CheckCircle2,
  Layers,
  AlertTriangle,
  ArrowRight,
  GitBranch,
} from "lucide-react";
import { useKnoxStore } from "@/store/useKnoxStore";
import { KnoxAPI } from "@/services/api";
import { Button } from "@/components/ui/Button";
import { Badge } from "@/components/ui/Badge";
import { cn } from "@/lib/cn";
import type { DbStats, StrayDatabaseDto, ConsolidationReport } from "@/types/knox";

export const StorageTab: React.FC = () => {
  const { config, updateConfig, showToast, refreshData, status } = useKnoxStore();
  const [dbStats, setDbStats] = useState<DbStats | null>(null);
  const [isLoadingStats, setIsLoadingStats] = useState(false);
  const [integrityStatus, setIntegrityStatus] = useState<string | null>(null);
  const [isCheckingIntegrity, setIsCheckingIntegrity] = useState(false);
  const [isVacuuming, setIsVacuuming] = useState(false);

  // Stray database consolidation state
  const [strays, setStrays] = useState<StrayDatabaseDto[]>([]);
  const [isScanningStrays, setIsScanningStrays] = useState(false);
  const [consolidatingPath, setConsolidatingPath] = useState<string | null>(null);
  const [consolidationReport, setConsolidationReport] = useState<ConsolidationReport | null>(null);

  const fetchStats = () => {
    setIsLoadingStats(true);
    KnoxAPI.getDbStats()
      .then(setDbStats)
      .catch((e) => console.warn("Failed to load db stats:", e))
      .finally(() => setIsLoadingStats(false));
  };

  const scanForStrays = () => {
    setIsScanningStrays(true);
    KnoxAPI.scanStrayDatabases()
      .then(setStrays)
      .catch((e) => console.warn("Failed to scan stray databases:", e))
      .finally(() => setIsScanningStrays(false));
  };

  useEffect(() => {
    fetchStats();
    scanForStrays();
  }, []);

  const handleRelocateDb = async () => {
    try {
      const folder = await KnoxAPI.openFolderDialog();
      if (folder) {
        const updatedConfig = await KnoxAPI.relocateDatabase(folder);
        await updateConfig({ kgPath: updatedConfig.kg_path });
        showToast(`Knowledge graph relocated to: ${updatedConfig.kg_path}`);
        fetchStats();
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

  const handleConsolidate = async (path: string) => {
    try {
      setConsolidatingPath(path);
      const rep = await KnoxAPI.consolidateStrayDatabase(path);
      setConsolidationReport(rep);
      showToast(
        `Consolidated ${rep.actions_moved} action(s) and ${rep.edges_moved} edge(s) into global database!`
      );
      scanForStrays();
      fetchStats();
      await refreshData();
    } catch (err) {
      showToast(String(err), true);
    } finally {
      setConsolidatingPath(null);
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

  // WAL ratio calculation
  const totalSizeBytes = (dbStats?.db_size_bytes || 0) + (dbStats?.wal_size_bytes || 0);
  const walRatio =
    totalSizeBytes > 0 ? ((dbStats?.wal_size_bytes || 0) / totalSizeBytes) * 100 : 0;
  const isHighWal =
    walRatio > 50 || (dbStats?.wal_size_bytes || 0) > 50 * 1024 * 1024;

  const totalActions = dbStats?.total_actions ?? status?.total_actions ?? 0;
  const totalEdges = dbStats?.total_edges ?? 0;
  const totalRelations = dbStats?.total_relations ?? 0;
  const showZeroEdgesHint = totalActions > 0 && totalEdges === 0;

  return (
    <div className="space-y-6 animate-in fade-in-50 duration-150">
      <div>
        <h3 className="text-sm font-semibold text-foreground">Storage & Database</h3>
        <p className="text-xs text-muted-foreground">
          Single unified SQLite knowledge graph, storage hygiene, and database maintenance
        </p>
      </div>

      <div className="space-y-4">
        {/* Global Database Identity Card */}
        <div className="p-4 rounded-lg border border-border bg-card/60 space-y-3">
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-2">
              <HardDrive className="size-4 text-primary" />
              <span className="text-xs font-semibold text-foreground">Global Knowledge Graph</span>
            </div>
            <Badge variant="outline" className="font-mono text-2xs uppercase">
              Global Shared Database
            </Badge>
          </div>

          <div className="p-2.5 rounded-md bg-muted/60 border border-border font-mono text-xs text-foreground break-all select-all">
            {config?.kg_path || "~/.knox/knox.db"}
          </div>

          <div className="grid grid-cols-2 sm:grid-cols-3 gap-2 text-xs font-mono">
            <div className="p-2 rounded-md bg-muted/30 border border-border">
              <span className="text-2xs text-muted-foreground uppercase block">Tracked Projects</span>
              <span className="font-semibold text-foreground mt-0.5 block">
                {status?.projects_tracked || 1}
              </span>
            </div>
            <div className="p-2 rounded-md bg-muted/30 border border-border">
              <span className="text-2xs text-muted-foreground uppercase block">Recorded Actions</span>
              <span className="font-semibold text-primary mt-0.5 block">{totalActions}</span>
            </div>
            <div className="p-2 rounded-md bg-muted/30 border border-border col-span-2 sm:col-span-1">
              <span className="text-2xs text-muted-foreground uppercase block">Schema Version</span>
              <span className="font-semibold text-foreground mt-0.5 block">
                v{dbStats?.schema_version || 11}
              </span>
            </div>
          </div>

          <div className="flex flex-wrap items-center gap-2 pt-1">
            <Button
              type="button"
              variant="primary"
              size="sm"
              onClick={handleRelocateDb}
              className="gap-1.5"
            >
              <FolderOpen className="size-3.5" />
              <span>Relocate Database…</span>
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

        {/* Stray Database Import / Consolidation Card */}
        {strays.length > 0 && (
          <div className="p-4 rounded-lg border border-warning/40 bg-warning/5 space-y-3">
            <div className="flex items-center justify-between">
              <div className="flex items-center gap-2">
                <AlertTriangle className="size-4 text-warning" />
                <span className="text-xs font-semibold text-foreground">
                  Legacy Project Databases Detected ({strays.length})
                </span>
              </div>
              <Button
                type="button"
                variant="ghost"
                size="sm"
                onClick={scanForStrays}
                loading={isScanningStrays}
                className="h-6 text-2xs"
              >
                <RefreshCw className="size-3" />
                <span>Re-scan</span>
              </Button>
            </div>

            <p className="text-xs text-muted-foreground leading-relaxed">
              Found legacy project-local databases. Knox now uses a unified global knowledge graph.
              Consolidate them to import actions, sessions, and relationships without duplication:
            </p>

            <div className="space-y-2">
              {strays.map((s) => (
                <div
                  key={s.db_path}
                  className="p-3 rounded-md border border-border bg-card/80 flex flex-col sm:flex-row sm:items-center justify-between gap-2.5"
                >
                  <div className="min-w-0">
                    <p className="text-xs font-medium text-foreground truncate">{s.project_path}</p>
                    <p className="text-2xs font-mono text-muted-foreground truncate">{s.db_path}</p>
                    <p className="text-2xs text-primary font-medium mt-0.5">
                      {s.action_count} action(s) available for consolidation
                    </p>
                  </div>
                  <Button
                    type="button"
                    variant="primary"
                    size="sm"
                    loading={consolidatingPath === s.db_path}
                    onClick={() => handleConsolidate(s.db_path)}
                    className="shrink-0 gap-1 text-xs"
                  >
                    <span>Consolidate into Global DB</span>
                    <ArrowRight className="size-3" />
                  </Button>
                </div>
              ))}
            </div>

            {consolidationReport && (
              <div className="p-3 rounded-md bg-success/10 border border-success/30 text-xs font-mono text-success space-y-1">
                <p className="font-semibold">✓ Consolidation Complete</p>
                <p className="text-2xs text-foreground">
                  Moved {consolidationReport.actions_moved} action(s), {consolidationReport.sessions_moved} session(s), and {consolidationReport.edges_moved} edge(s) ({consolidationReport.actions_skipped} skipped/deduped).
                </p>
              </div>
            )}
          </div>
        )}

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
              <span className="text-xs font-semibold text-foreground">
                Live Telemetry & Graph Observability
              </span>
            </div>
            <Button
              type="button"
              variant="ghost"
              size="sm"
              onClick={fetchStats}
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

              <div
                className={cn(
                  "p-2.5 rounded-md border",
                  isHighWal
                    ? "bg-warning/10 border-warning/40 text-warning"
                    : "bg-muted/40 border-border"
                )}
              >
                <div className="flex items-center justify-between">
                  <p className="text-2xs text-muted-foreground uppercase font-mono">WAL Size</p>
                  {isHighWal && <AlertTriangle className="size-3 text-warning shrink-0" />}
                </div>
                <p className="text-xs font-semibold font-mono mt-0.5 text-foreground">
                  {formatBytes(dbStats.wal_size_bytes)}{" "}
                  <span className="text-2xs text-muted-foreground font-normal">
                    ({walRatio.toFixed(1)}%)
                  </span>
                </p>
              </div>

              <div className="p-2.5 rounded-md bg-muted/40 border border-border">
                <p className="text-2xs text-muted-foreground uppercase font-mono">Graph Edges</p>
                <p className="text-xs font-semibold text-primary font-mono mt-0.5">{totalEdges}</p>
              </div>

              <div className="p-2.5 rounded-md bg-muted/40 border border-border">
                <p className="text-2xs text-muted-foreground uppercase font-mono">
                  Legacy Relations
                </p>
                <p className="text-xs font-semibold text-foreground font-mono mt-0.5">
                  {totalRelations}
                </p>
              </div>
            </div>
          ) : (
            <div className="p-3 text-center text-xs text-muted-foreground">Loading storage stats…</div>
          )}

          {/* High WAL Warning */}
          {isHighWal && (
            <div className="p-3 rounded-md bg-warning/10 border border-warning/30 flex items-start gap-2 text-xs text-warning">
              <AlertTriangle className="size-4 shrink-0 mt-0.5" />
              <div>
                <p className="font-semibold">Elevated WAL Volume ({walRatio.toFixed(1)}%)</p>
                <p className="text-2xs text-warning/90 mt-0.5">
                  The SQLite Write-Ahead Log exceeds recommended thresholds. Running a vacuum will
                  checkpoint transactions and truncate the WAL to 0 bytes.
                </p>
              </div>
            </div>
          )}

          {/* 0-edges Contextual Diagnostic Hint */}
          {showZeroEdgesHint && (
            <div className="p-3 rounded-md bg-primary/10 border border-primary/30 flex items-start gap-2.5 text-xs text-foreground">
              <GitBranch className="size-4 text-primary shrink-0 mt-0.5" />
              <div>
                <p className="font-semibold text-primary">
                  {totalActions} actions recorded, 0 relations
                </p>
                <p className="text-2xs text-muted-foreground mt-0.5 leading-relaxed">
                  knox_trace has no edges to walk. Verify agent relation emission by providing{" "}
                  <code className="font-mono text-primary">relations</code> in{" "}
                  <code className="font-mono text-primary">knox_record</code> calls as documented in{" "}
                  <a
                    href="https://github.com/the-knox-project/knox"
                    target="_blank"
                    rel="noreferrer"
                    className="underline text-primary inline-flex items-center gap-0.5"
                  >
                    knox.md
                  </a>
                  .
                </p>
              </div>
            </div>
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
              <span>Vacuum & Truncate WAL</span>
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
