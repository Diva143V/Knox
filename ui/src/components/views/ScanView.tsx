import React, { useState, useEffect, useCallback } from "react";
import {
  ShieldCheck,
  GitCommit,
  FileCode,
  CheckCircle2,
  FolderOpen,
  AlertCircle,
  GitPullRequest,
  Check,
  Trash2,
  AlertTriangle,
} from "lucide-react";
import { KnoxAPI } from "@/services/api";
import { useKnoxStore } from "@/store/useKnoxStore";
import type { ScanReport } from "@/types/knox";
import { PageHeader } from "@/components/ui/PageHeader";
import { EmptyState } from "@/components/ui/EmptyState";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/Card";
import { Button } from "@/components/ui/Button";
import { Input } from "@/components/ui/Input";
import { Badge } from "@/components/ui/Badge";

export const ScanView: React.FC = () => {
  const { showToast, refreshData } = useKnoxStore();
  const [projectPath, setProjectPath] = useState(".");
  const [isScanning, setIsScanning] = useState(false);
  const [scanReport, setScanReport] = useState<ScanReport | null>(null);
  const [scanError, setScanError] = useState<string | null>(null);
  const [hookInstalled, setHookInstalled] = useState<boolean | null>(null);
  const [isCheckingHook, setIsCheckingHook] = useState(false);

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
        setScanError(null);
      }
    } catch (err) {
      showToast(String(err), true);
    }
  };

  const handleScan = async () => {
    try {
      setIsScanning(true);
      setScanError(null);
      const res = await KnoxAPI.scanStaged(projectPath);
      setScanReport(res);
      showToast(
        `Scan complete. Detected ${res.human_edits_detected} human edits in staged changes.`
      );
      await refreshData();
      await checkHook(projectPath);
    } catch (err) {
      const msg = String(err);
      setScanError(msg);
      showToast(msg, true);
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
    const ok = window.confirm(
      "Uninstall the Knox pre-commit hook from this repository?"
    );
    if (!ok) return;
    try {
      const res = await KnoxAPI.uninstallHook(projectPath);
      showToast(res);
      await checkHook(projectPath);
    } catch (err) {
      showToast(String(err), true);
    }
  };

  return (
    <div className="p-5 sm:p-6 space-y-5 max-w-4xl mx-auto overflow-y-auto h-full">
      <PageHeader
        title="Human Edit Scan"
        description="Compare staged git changes against recent agent actions and record untracked manual edits as human actions."
        icon={<ShieldCheck />}
      />

      <Card>
        <CardContent className="p-4 space-y-4">
          <div className="space-y-1.5">
            <label
              htmlFor="scan-repo-path"
              className="text-overline block"
            >
              Git repository path
            </label>
            <div className="flex flex-col gap-2">
              <Input
                id="scan-repo-path"
                type="text"
                value={projectPath}
                onChange={(e) => setProjectPath(e.target.value)}
                placeholder="e.g. D:\Projects\MyRepo or ."
                className="w-full min-w-0"
                aria-describedby="scan-path-hint"
              />
              <div className="flex flex-wrap gap-2">
                <Button
                  type="button"
                  variant="outline"
                  onClick={handleBrowseFolder}
                  title="Browse repository directory"
                >
                  <FolderOpen className="size-3.5" />
                  Browse
                </Button>
                <Button
                  type="button"
                  variant="primary"
                  onClick={handleScan}
                  disabled={isScanning}
                  loading={isScanning}
                >
                  {!isScanning && <GitPullRequest className="size-3.5" />}
                  {isScanning ? "Scanning…" : "Scan staged"}
                </Button>
              </div>
            </div>
            <p id="scan-path-hint" className="text-sm text-muted-foreground">
              Requires a valid git repository. Failures often mean path is not a repo
              or git is unavailable.
            </p>
          </div>

          {scanError && (
            <div
              role="alert"
              className="px-3 py-2 rounded-md border border-danger/40 bg-danger/10 text-xs text-danger flex gap-2"
            >
              <AlertTriangle className="size-3.5 shrink-0 mt-0.5" aria-hidden />
              <span>{scanError}</span>
            </div>
          )}

          <div className="pt-3 border-t border-border-subtle flex flex-col sm:flex-row sm:items-center justify-between gap-3 text-xs">
            <div>
              <div className="flex flex-wrap items-center gap-2">
                <GitCommit className="size-4 text-primary" aria-hidden />
                <span className="font-semibold text-foreground">
                  Git pre-commit hook
                </span>
                {isCheckingHook ? (
                  <span className="text-xs text-muted-foreground font-mono">
                    Checking…
                  </span>
                ) : hookInstalled ? (
                  <Badge variant="success" className="gap-1">
                    <Check className="size-3" aria-hidden />
                    Installed
                  </Badge>
                ) : (
                  <Badge variant="warning">Not installed</Badge>
                )}
              </div>
              <p className="text-muted-foreground text-sm mt-1">
                On commit, scans staged changes and records human actions into Knox.
              </p>
            </div>
            <div className="flex items-center gap-2 shrink-0">
              {!hookInstalled ? (
                <Button
                  type="button"
                  variant="outline"
                  size="sm"
                  onClick={handleInstallHook}
                >
                  <Check className="size-3" />
                  Install hook
                </Button>
              ) : (
                <Button
                  type="button"
                  variant="danger"
                  size="sm"
                  onClick={handleUninstallHook}
                >
                  <Trash2 className="size-3" />
                  Uninstall
                </Button>
              )}
            </div>
          </div>
        </CardContent>
      </Card>

      {scanReport ? (
        <Card>
          <CardHeader className="flex-row items-center justify-between space-y-0 pb-0">
            <CardTitle className="flex items-center gap-2">
              <CheckCircle2 className="size-4 text-success" aria-hidden />
              Scan results
            </CardTitle>
            <div className="flex items-center gap-2">
              <Badge variant="secondary" className="font-mono">
                <span className="text-primary font-semibold">
                  {scanReport.human_edits_detected}
                </span>
                &nbsp;human edits
              </Badge>
              <Badge variant="secondary" className="font-mono">
                <span className="text-success font-semibold">
                  {scanReport.actions_recorded}
                </span>
                &nbsp;recorded
              </Badge>
            </div>
          </CardHeader>
          <CardContent className="pt-4 space-y-4">
            {scanReport.diff_stat && (
              <div className="space-y-1.5">
                <span className="text-overline">Git numstat</span>
                <pre className="p-3 rounded-md border border-border bg-background font-mono text-sm text-muted-foreground overflow-x-auto leading-relaxed">
                  {scanReport.diff_stat}
                </pre>
              </div>
            )}

            {scanReport.files_scanned.length === 0 ? (
              <EmptyState
                compact
                icon={<AlertCircle />}
                title="No staged changes detected"
                description={
                  <>
                    Stage modified files with{" "}
                    <code className="text-primary font-mono">git add &lt;file&gt;</code>{" "}
                    then scan again.
                  </>
                }
              />
            ) : (
              <div className="space-y-2">
                <span className="text-overline">
                  Staged files ({scanReport.files_scanned.length})
                </span>
                <div className="border border-border rounded-md overflow-hidden overflow-x-auto">
                  <table className="w-full text-left text-xs font-mono">
                    <thead className="bg-card text-xs text-muted-foreground uppercase border-b border-border">
                      <tr>
                        <th className="px-3 py-2 font-medium">Path</th>
                        <th className="px-3 py-2 font-medium">Diff</th>
                        <th className="px-3 py-2 font-medium">Action</th>
                        <th className="px-3 py-2 font-medium text-right">
                          Attribution
                        </th>
                      </tr>
                    </thead>
                    <tbody className="divide-y divide-border-subtle bg-background/50">
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
                          <tr key={idx} className="hover:bg-card/40">
                            <td className="px-3 py-2 text-foreground">
                              <span className="inline-flex items-center gap-2 max-w-xs">
                                <FileCode className="size-3.5 text-primary shrink-0" aria-hidden />
                                <span className="truncate" title={file}>
                                  {file}
                                </span>
                              </span>
                            </td>
                            <td className="px-3 py-2 text-plan">{stat}</td>
                            <td className="px-3 py-2 text-muted-foreground">
                              {scanReport.actions_recorded > 0
                                ? "human_write"
                                : "already_tracked"}
                            </td>
                            <td className="px-3 py-2 text-right">
                              <Badge variant="success">human</Badge>
                            </td>
                          </tr>
                        );
                      })}
                    </tbody>
                  </table>
                </div>
              </div>
            )}
          </CardContent>
        </Card>
      ) : (
        <EmptyState
          icon={<AlertCircle />}
          title="No scan results yet"
          description="Run Scan staged above to compare the git index against recent agent actions and attribute human edits."
        />
      )}
    </div>
  );
};
