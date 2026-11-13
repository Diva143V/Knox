import React, { useState, useEffect } from "react";
import { Database, ArrowRight, X, CheckCircle2, Settings } from "lucide-react";
import { KnoxAPI } from "@/services/api";
import { useKnoxStore } from "@/store/useKnoxStore";
import { Button } from "@/components/ui/Button";
import { Badge } from "@/components/ui/Badge";
import type { StrayDatabaseDto } from "@/types/knox";

export const StrayDbPromptModal: React.FC = () => {
  const { config, updateConfig, refreshData, showToast, openSettings } = useKnoxStore();
  const [isOpen, setIsOpen] = useState(false);
  const [strays, setStrays] = useState<StrayDatabaseDto[]>([]);
  const [isConsolidating, setIsConsolidating] = useState(false);

  useEffect(() => {
    // Only check if config is loaded and user has not been prompted yet
    if (!config || config.stray_db_prompted) return;

    let isMounted = true;
    KnoxAPI.scanStrayDatabases()
      .then((detected) => {
        if (!isMounted) return;
        const totalActions = detected.reduce((acc, s) => acc + s.action_count, 0);
        if (detected.length > 0 && totalActions > 0) {
          setStrays(detected);
          setIsOpen(true);
        } else {
          // No stray databases with pending actions detected; mark as prompted
          updateConfig({ strayDbPrompted: true }).catch(() => {});
        }
      })
      .catch((err) => {
        console.warn("Failed initial stray database scan:", err);
      });

    return () => {
      isMounted = false;
    };
  }, [config?.stray_db_prompted, config, updateConfig]);

  if (!isOpen || strays.length === 0) return null;

  const handleDismiss = async () => {
    try {
      await updateConfig({ strayDbPrompted: true });
    } catch {
      // ignore
    } finally {
      setIsOpen(false);
    }
  };

  const handleGoToSettings = async () => {
    try {
      await updateConfig({ strayDbPrompted: true });
    } catch {
      // ignore
    } finally {
      setIsOpen(false);
      openSettings("storage");
    }
  };

  const handleConsolidateAll = async () => {
    setIsConsolidating(true);
    let totalActions = 0;
    let totalEdges = 0;
    let hadError = false;

    try {
      for (const stray of strays) {
        try {
          const report = await KnoxAPI.consolidateStrayDatabase(stray.db_path);
          totalActions += report.actions_moved;
          totalEdges += report.edges_moved;
        } catch (strayErr) {
          console.warn(`Failed consolidating stray ${stray.db_path}:`, strayErr);
          hadError = true;
        }
      }

      await updateConfig({ strayDbPrompted: true });
      if (hadError && totalActions === 0) {
        showToast("Consolidation encountered issues. You can review them in Settings > Storage.", true);
      } else {
        showToast(
          totalActions > 0 || totalEdges > 0
            ? `Consolidated ${totalActions} action(s) and ${totalEdges} edge(s) into global database!`
            : "No pending actions to import."
        );
      }
      await refreshData();
      setIsOpen(false);
    } catch (err) {
      console.error("Consolidation error:", err);
      showToast(`Consolidation error: ${String(err)}`, true);
      try {
        await updateConfig({ strayDbPrompted: true });
      } catch {
        // ignore
      }
      setIsOpen(false);
    } finally {
      setIsConsolidating(false);
    }
  };

  const totalActionsDetected = strays.reduce((acc, s) => acc + s.action_count, 0);

  return (
    <div
      role="dialog"
      aria-modal="true"
      aria-labelledby="stray-prompt-title"
      className="fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/60 backdrop-blur-sm animate-in fade-in duration-150"
    >
      <div className="relative w-full max-w-lg bg-card border border-border rounded-[var(--radius-card)] shadow-2xl overflow-hidden flex flex-col">
        {/* Header */}
        <div className="flex items-center justify-between p-4 border-b border-border bg-muted/20">
          <div className="flex items-center gap-2.5">
            <div className="p-1.5 rounded-md bg-warning/10 text-warning border border-warning/20">
              <Database className="size-4" />
            </div>
            <div>
              <h2 id="stray-prompt-title" className="text-sm font-semibold text-foreground">
                Legacy Databases Detected
              </h2>
              <p className="text-2xs text-muted-foreground">
                Found {strays.length} project-local database{strays.length > 1 ? "s" : ""}
              </p>
            </div>
          </div>
          <Button
            variant="ghost"
            size="icon"
            onClick={handleDismiss}
            aria-label="Dismiss consolidation prompt"
            className="size-7 text-muted-foreground hover:text-foreground"
          >
            <X className="size-4" />
          </Button>
        </div>

        {/* Content */}
        <div className="p-4 space-y-3.5 text-xs">
          <p className="text-muted-foreground leading-relaxed">
            Knox uses a unified database at <code className="font-mono text-foreground font-semibold">~/.knox/knox.db</code> for all projects.
            We found historical memory in project-local directories that can be safely consolidated into your global database:
          </p>

          <div className="max-h-48 overflow-y-auto space-y-2 pr-1">
            {strays.map((s) => {
              const projectName =
                s.project_path.split(/[/\\]/).filter(Boolean).pop() || s.project_path;
              return (
                <div
                  key={s.db_path}
                  className="p-2.5 rounded-md border border-border bg-muted/30 flex items-center justify-between gap-3 text-2xs"
                >
                  <div className="min-w-0 flex-1">
                    <div className="font-medium text-foreground truncate">{projectName}</div>
                    <div className="font-mono text-muted-foreground truncate" title={s.db_path}>
                      {s.db_path}
                    </div>
                  </div>
                  <Badge variant="outline" className="font-mono shrink-0">
                    {s.action_count} actions
                  </Badge>
                </div>
              );
            })}
          </div>

          <div className="p-2.5 rounded-md bg-primary/5 border border-primary/20 text-2xs text-muted-foreground flex items-center gap-2">
            <CheckCircle2 className="size-3.5 text-primary shrink-0" />
            <span>
              Total: <strong className="text-foreground">{totalActionsDetected} actions</strong> ready to import. Deduplication prevents repeated entries.
            </span>
          </div>
        </div>

        {/* Footer */}
        <div className="flex items-center justify-between p-3.5 border-t border-border bg-muted/10 gap-2">
          <Button
            variant="ghost"
            size="sm"
            onClick={handleGoToSettings}
            className="text-2xs text-muted-foreground hover:text-foreground gap-1.5"
          >
            <Settings className="size-3" />
            <span>Review in Settings</span>
          </Button>

          <div className="flex items-center gap-2">
            <Button
              variant="outline"
              size="sm"
              onClick={handleDismiss}
              className="text-2xs"
            >
              Later
            </Button>
            <Button
              variant="primary"
              size="sm"
              loading={isConsolidating}
              onClick={handleConsolidateAll}
              className="text-2xs gap-1.5"
            >
              <span>Consolidate All</span>
              <ArrowRight className="size-3" />
            </Button>
          </div>
        </div>
      </div>
    </div>
  );
};
