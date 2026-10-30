import React, { useState, useEffect, useCallback } from "react";
import {
  ShieldCheck,
  GitCommit,
  FileCode,
  CheckCircle2,
  FolderOpen,
  AlertCircle,
  RefreshCw,
  GitPullRequest,
  Check,
  Trash2,
} from "lucide-react";
import { KnoxAPI } from "@/services/api";
import { useKnoxStore } from "@/store/useKnoxStore";
import type { ScanReport } from "@/types/knox";

export const ScanView: React.FC = () => {
  const { showToast, refreshData } = useKnoxStore();
  const [projectPath, setProjectPath] = useState(".");
  const [isScanning, setIsScanning] = useState(false);
  const [scanReport, setScanReport] = useState<ScanReport | null>(null);
  const [hookInstalled, setHookInstalled] = useState<boolean | null>(null);
  const [isCheckingHook, setIsCheckingHook] = useState(false);

  // Check backend hook status
  const checkHook = useCallback(async (path: string) => {
    setIsCheckingHook(true);
    try {
      const isInstalled = await KnoxAPI.getHookStatus(path);
      setHookInstalled(isInstalled);
    } catch {
      setHookInstalled(false);
    } finally {
      setIsCheckingHook(false);
    }
  }, []);

  useEffect(() => {
    checkHook(projectPath);
  }, [projectPath, checkHook]);

  const handleBrowseFolder = async () => {
    try {
      const folder = await KnoxAPI.openFolderDialog();
      if (folder) {
        setProjectPath(folder);
      }
    } catch (err) {
      showToast(String(err), true);
    }
  };

  const handleScan = async () => {
    try {
      setIsScanning(true);
      const res = await KnoxAPI.scanStaged(projectPath);
      setScanReport(res);
      showToast(
        `Scan complete. Detected ${res.human_edits_detected} human edits in staged changes.`
      );
      await refreshData();
      await checkHook(projectPath);
    } catch (err) {
      showToast(String(err), true);
    } finally {
      setIsScanning(false);
    }
  };

  const handleInstallHook = async () => {
    try {
      const res = await KnoxAPI.installHook(projectPath);
      showToast(res);
      await checkHook(projectPath);
    } catch (err) {
      showToast(String(err), true);
    }
  };

  const handleUninstallHook = async () => {
    try {
      const res = await KnoxAPI.uninstallHook(projectPath);
      showToast(res);
      await checkHook(projectPath);
    } catch (err) {
      showToast(String(err), true);
    }
  };

  return (
    <div className="p-6 space-y-6 max-w-4xl mx-auto overflow-y-auto h-full select-none">
      {/* Header */}
      <div>
        <h1 className="text-xl font-bold tracking-tight text-foreground flex items-center gap-2">
          <ShieldCheck className="w-5 h-5 text-primary" />
          <span>Human Edit Scanner</span>
        </h1>
        <p className="text-xs text-muted-foreground mt-1">
          Detects manual developer edits in staged git changes that were not executed by an AI agent,
          recording them as <span className="font-mono text-foreground font-semibold">human</span> actions in the knowledge graph.
        </p>
      </div>

      {/* Repository Path & Scan Bar */}
      <div className="p-5 rounded-xl border border-border bg-card space-y-4">
        <div className="space-y-1.5">
          <label className="text-xs font-semibold text-muted-foreground uppercase tracking-wider">
            Project Git Repository Path
          </label>
          <div className="flex gap-2">
            <input
              type="text"
              value={projectPath}
              onChange={(e) => setProjectPath(e.target.value)}
              placeholder="e.g. D:\Projects\MyRepo or ."
              className="flex-1 bg-background border border-border rounded-lg px-3 py-2 text-xs font-mono text-foreground focus:outline-none focus:border-primary"
            />
            <button
              onClick={handleBrowseFolder}
              className="flex items-center gap-1.5 px-3 py-2 rounded-lg border border-border bg-card hover:bg-background text-xs font-medium text-foreground transition cursor-pointer"
              title="Browse repository directory"
            >
              <FolderOpen className="w-3.5 h-3.5 text-muted-foreground" />
              <span>Browse…</span>
            </button>
            <button
              onClick={handleScan}
              disabled={isScanning}
              className="flex items-center gap-1.5 px-4 py-2 rounded-lg bg-primary text-primary-foreground text-xs font-semibold hover:bg-primary/90 transition cursor-pointer shadow-sm shadow-primary/25 disabled:opacity-50"
            >
              {isScanning ? (
                <>
                  <RefreshCw className="w-3.5 h-3.5 animate-spin" />
                  <span>Scanning Staged Changes…</span>
                </>
              ) : (
                <>
                  <GitPullRequest className="w-3.5 h-3.5" />
                  <span>Scan Staged Changes</span>
                </>
              )}
            </button>
          </div>
        </div>

        {/* Hook Management Card */}
        <div className="pt-3 border-t border-border/60 flex items-center justify-between text-xs">
          <div>
            <div className="flex items-center gap-2">
              <GitCommit className="w-4 h-4 text-primary" />
              <span className="font-semibold text-foreground">Git Pre-Commit Hook</span>
              {isCheckingHook ? (
                <span className="text-[10px] text-muted-foreground animate-pulse font-mono">
                  Checking…
                </span>
              ) : hookInstalled ? (
                <span className="flex items-center gap-1 text-[10px] font-mono px-2 py-0.5 rounded bg-emerald-500/10 text-emerald-400 border border-emerald-500/30">
                  <Check className="w-3 h-3" />
                  <span>Active & Installed</span>
                </span>
              ) : (
                <span className="text-[10px] font-mono px-2 py-0.5 rounded bg-amber-500/10 text-amber-400 border border-amber-500/30">
                  Not Installed
                </span>
              )}
            </div>
            <p className="text-muted-foreground text-[11px] mt-0.5">
              Automatically scans staged git changes on commit and writes human actions into Knox.
            </p>
          </div>

          <div className="flex items-center gap-2">
            {!hookInstalled ? (
              <button
                onClick={handleInstallHook}
                className="flex items-center gap-1.5 px-3 py-1.5 rounded-lg bg-primary/10 border border-primary/30 hover:bg-primary/20 text-xs font-medium text-primary transition cursor-pointer"
              >
                <Check className="w-3 h-3" />
                <span>Install Hook</span>
              </button>
            ) : (
              <button
                onClick={handleUninstallHook}
                className="flex items-center gap-1.5 px-3 py-1.5 rounded-lg border border-rose-500/30 bg-rose-500/10 hover:bg-rose-500/20 text-xs font-medium text-rose-400 transition cursor-pointer"
              >
                <Trash2 className="w-3 h-3" />
                <span>Uninstall Hook</span>
              </button>
            )}
          </div>
        </div>
      </div>

      {/* Scan Results / Report */}
      {scanReport ? (
        <div className="p-5 rounded-xl border border-border bg-card space-y-4">
          <div className="flex items-center justify-between">
            <span className="text-xs font-semibold text-foreground flex items-center gap-2">
              <CheckCircle2 className="w-4 h-4 text-emerald-400" />
              <span>Scan Results</span>
            </span>
            <div className="flex items-center gap-2">
              <span className="text-xs font-mono px-2 py-0.5 rounded bg-background border border-border text-foreground">
                <span className="text-primary font-semibold">{scanReport.human_edits_detected}</span> human edits
              </span>
              <span className="text-xs font-mono px-2 py-0.5 rounded bg-background border border-border text-foreground">
                <span className="text-emerald-400 font-semibold">{scanReport.actions_recorded}</span> actions recorded
              </span>
            </div>
          </div>

          {/* Diff statistics */}
          {scanReport.diff_stat && (
            <div className="space-y-1">
              <span className="text-[10px] font-semibold text-muted-foreground uppercase tracking-wider">
                Git Numstat Diff
              </span>
              <pre className="p-3 rounded-lg border border-border/50 bg-background font-mono text-[11px] text-muted-foreground overflow-x-auto leading-relaxed">
                {scanReport.diff_stat}
              </pre>
            </div>
          )}

          {/* Files scanned table */}
          {scanReport.files_scanned.length === 0 ? (
            <div className="p-6 rounded-lg border border-dashed border-border text-center space-y-1">
              <p className="text-xs font-medium text-foreground">No staged changes detected.</p>
              <p className="text-[11px] text-muted-foreground">
                Stage your modified files with <code className="text-primary font-mono">git add &lt;file&gt;</code> to scan for human edits.
              </p>
            </div>
          ) : (
            <div className="space-y-2 pt-1">
              <span className="text-[10px] font-semibold text-muted-foreground uppercase tracking-wider">
                Detected Edits Table ({scanReport.files_scanned.length})
              </span>
              <div className="border border-border/60 rounded-lg overflow-hidden">
                <table className="w-full text-left text-xs divide-y divide-border/60 font-mono">
                  <thead className="bg-card/80 text-[10px] text-muted-foreground uppercase">
                    <tr>
                      <th className="px-3 py-2 font-medium">Staged File Path</th>
                      <th className="px-3 py-2 font-medium">Diff Stats</th>
                      <th className="px-3 py-2 font-medium">Recorded Action</th>
                      <th className="px-3 py-2 font-medium text-right">Attribution</th>
                    </tr>
                  </thead>
                  <tbody className="divide-y divide-border/40 text-[11px] bg-background/50">
                    {scanReport.files_scanned.map((file, idx) => {
                      let stat = "staged";
                      if (scanReport.diff_stat) {
                        const line = scanReport.diff_stat
                          .split("\n")
                          .find((l) => l.includes(file));
                        if (line && line.includes("|")) {
                          stat = line.split("|")[1].trim();
                        }
                      }
                      return (
                        <tr key={idx} className="hover:bg-card/30">
                          <td className="px-3 py-2 font-medium text-foreground flex items-center gap-2 truncate max-w-xs">
                            <FileCode className="w-3.5 h-3.5 text-primary shrink-0" />
                            <span className="truncate">{file}</span>
                          </td>
                          <td className="px-3 py-2 text-indigo-400">{stat}</td>
                          <td className="px-3 py-2 text-muted-foreground">
                            {scanReport.actions_recorded > 0 ? "human_write" : "already_tracked"}
                          </td>
                          <td className="px-3 py-2 text-right">
                            <span className="text-[10px] px-1.5 py-0.5 rounded bg-emerald-500/10 text-emerald-400 border border-emerald-500/20">
                              human
                            </span>
                          </td>
                        </tr>
                      );
                    })}
                  </tbody>
                </table>
              </div>
            </div>
          )}
        </div>
      ) : (
        /* Intentional Empty State */
        <div className="p-8 rounded-xl border border-dashed border-border bg-card/40 text-center space-y-3">
          <div className="w-10 h-10 rounded-xl bg-primary/10 border border-primary/20 flex items-center justify-center mx-auto text-primary">
            <AlertCircle className="w-5 h-5" />
          </div>
          <div>
            <h3 className="text-sm font-semibold text-foreground">No Active Scan Results</h3>
            <p className="text-xs text-muted-foreground mt-1 max-w-sm mx-auto">
              Click &ldquo;Scan Staged Changes&rdquo; to compare your git index against recent agent actions and record human manual contributions.
            </p>
          </div>
        </div>
      )}
    </div>
  );
};
