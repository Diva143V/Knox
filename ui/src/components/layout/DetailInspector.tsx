import React, { useMemo, useState } from "react";
import {
  X,
  FolderGit2,
  Copy,
  Check,
  Target,
  FileCode,
  Sparkles,
  GitBranch,
  PanelRightClose,
} from "lucide-react";
import { useKnoxStore } from "@/store/useKnoxStore";
import { ActionDetail } from "@/components/graph/ActionDetail";
import { Button } from "@/components/ui/Button";
import { Badge } from "@/components/ui/Badge";
import { cn } from "@/lib/cn";

export const DetailInspector: React.FC = () => {
  const {
    selectedAction,
    setSelectedAction,
    selectedProject,
    setSelectedProject,
    activeProjectScope,
    setProjectScope,
    actions,
    showToast,
  } = useKnoxStore();

  const [copiedPath, setCopiedPath] = useState(false);

  const projectStats = useMemo(() => {
    if (!selectedProject) return { actionCount: 0, files: [] as string[] };
    const normProjPath = selectedProject.path.toLowerCase().replace(/\\/g, "/");
    const projName = selectedProject.name.toLowerCase();

    const matchingActions = actions.filter((a) => {
      const hasMatchingFile =
        a.files &&
        a.files.some((f) => {
          const normF = f.toLowerCase().replace(/\\/g, "/");
          return normF.startsWith(normProjPath) || normF.includes(projName);
        });
      const matchesTag =
        a.tags && a.tags.some((t) => t.toLowerCase() === projName);
      return hasMatchingFile || matchesTag || a.summary.toLowerCase().includes(projName);
    });

    const fileSet = new Set<string>();
    matchingActions.forEach((a) => {
      a.files?.forEach((f) => fileSet.add(f));
    });

    return {
      actionCount: matchingActions.length,
      files: Array.from(fileSet),
    };
  }, [selectedProject, actions]);

  const handleCopyPath = (path: string) => {
    navigator.clipboard.writeText(path);
    setCopiedPath(true);
    showToast("Copied project path to clipboard");
    setTimeout(() => setCopiedPath(false), 2000);
  };

  const handleClose = () => {
    setSelectedAction(null);
    setSelectedProject(null);
  };

  if (!selectedAction && !selectedProject) return null;

  return (
    <>
      {/* Narrow-viewport scrim — tap to close */}
      <button
        type="button"
        aria-label="Close inspector overlay"
        className="fixed inset-0 top-9 z-30 bg-background/60 xl:hidden cursor-pointer"
        onClick={handleClose}
      />

      <aside
        className={cn(
          "shrink-0 border-l border-border bg-sidebar flex flex-col z-40 overflow-hidden",
          "w-[min(100vw-3.5rem,24rem)] xl:w-96 2xl:w-[420px]",
          "fixed right-0 top-9 bottom-0 xl:static xl:top-auto xl:bottom-auto xl:h-full",
          "motion-safe:animate-in motion-safe:slide-in-from-right-4 motion-safe:duration-150"
        )}
        aria-label="Detail inspector"
      >
        {/* Header */}
        <div className="h-11 border-b border-border px-3 flex items-center justify-between shrink-0 gap-2">
          <div className="flex items-center gap-2 min-w-0">
            {selectedAction ? (
              <>
                <PanelRightClose className="size-4 text-primary shrink-0" aria-hidden />
                <span className="text-xs font-semibold text-foreground truncate">
                  Action Inspector
                </span>
              </>
            ) : selectedProject ? (
              <>
                <FolderGit2 className="size-4 text-primary shrink-0" aria-hidden />
                <div className="min-w-0">
                  <div className="text-overline leading-none mb-1">Project Summary</div>
                  <div className="flex items-center gap-1.5 min-w-0">
                    <span className="text-xs font-semibold text-foreground truncate">
                      {selectedProject.name}
                    </span>
                  </div>
                </div>
              </>
            ) : null}
          </div>

          <Button
            type="button"
            variant="ghost"
            size="icon"
            onClick={handleClose}
            title="Close Inspector (Esc)"
            aria-label="Close inspector"
            className="size-7 shrink-0"
          >
            <X className="size-4" />
          </Button>
        </div>

        {/* Scrollable Body */}
        <div className="flex-1 overflow-y-auto p-4">
          {selectedAction ? (
            <ActionDetail action={selectedAction} />
          ) : selectedProject ? (
            <div className="space-y-4 text-xs text-foreground">
              {/* Project Header Card */}
              <div className="p-3.5 rounded-lg border border-border bg-card space-y-3">
                <div className="flex items-center justify-between">
                  <div className="flex items-center gap-2">
                    <div className="size-8 rounded-md bg-primary/15 border border-primary/30 flex items-center justify-center text-primary">
                      <FolderGit2 className="size-4" />
                    </div>
                    <div>
                      <h3 className="text-sm font-bold text-foreground font-sans tracking-tight">
                        {selectedProject.name}
                      </h3>
                      <Badge variant="outline" className="text-2xs">
                        PROJECT BRANCH
                      </Badge>
                    </div>
                  </div>
                  <Sparkles className="size-4 text-primary" />
                </div>

                {/* Physical Path */}
                <div className="space-y-1 pt-1 border-t border-border">
                  <div className="flex items-center justify-between text-muted-foreground text-2xs">
                    <span>Root Filesystem Path:</span>
                    <button
                      type="button"
                      onClick={() => handleCopyPath(selectedProject.path)}
                      className="text-primary hover:underline flex items-center gap-0.5 cursor-pointer font-mono"
                    >
                      {copiedPath ? (
                        <Check className="size-2.5 text-success" />
                      ) : (
                        <Copy className="size-2.5" />
                      )}
                      <span>{copiedPath ? "Copied" : "Copy"}</span>
                    </button>
                  </div>
                  <p
                    className="font-mono text-xs text-foreground bg-background p-2 rounded-md border border-border break-all select-all leading-tight"
                    title={selectedProject.path}
                  >
                    {selectedProject.path}
                  </p>
                </div>

                {/* Metric Counters */}
                <div className="grid grid-cols-2 gap-2 pt-1 font-mono text-xs">
                  <div className="p-2 rounded-md bg-background border border-border flex flex-col items-center">
                    <span className="text-xs font-bold text-success">
                      {projectStats.actionCount}
                    </span>
                    <span className="text-2xs text-muted-foreground uppercase">Actions</span>
                  </div>
                  <div className="p-2 rounded-md bg-background border border-border flex flex-col items-center">
                    <span className="text-xs font-bold text-primary">
                      {projectStats.files.length}
                    </span>
                    <span className="text-2xs text-muted-foreground uppercase">Files</span>
                  </div>
                </div>
              </div>

              {/* Quick Action: Focus Project */}
              <div>
                {activeProjectScope?.toLowerCase() ===
                selectedProject.path.toLowerCase() ? (
                  <Button
                    type="button"
                    variant="outline"
                    className="w-full"
                    onClick={() => setProjectScope(null)}
                  >
                    <GitBranch className="size-3.5" />
                    <span>Clear Focus (Show All Projects)</span>
                  </Button>
                ) : (
                  <Button
                    type="button"
                    variant="primary"
                    className="w-full"
                    onClick={() => setProjectScope(selectedProject.path)}
                  >
                    <Target className="size-3.5" />
                    <span>Focus this Project</span>
                  </Button>
                )}
              </div>

              {/* Codebase Files */}
              <div className="space-y-2 pt-2 border-t border-border">
                <div className="flex items-center justify-between">
                  <span className="text-2xs font-semibold text-muted-foreground uppercase tracking-wider flex items-center gap-1.5">
                    <FileCode className="size-3.5 text-primary" />
                    <span>Project Files</span>
                  </span>
                  <span className="text-2xs font-mono text-muted-foreground bg-card px-1.5 py-0.5 rounded-sm border border-border">
                    {projectStats.files.length}
                  </span>
                </div>

                {projectStats.files.length > 0 ? (
                  <div className="space-y-1 max-h-56 overflow-y-auto pr-0.5">
                    {projectStats.files.map((file, idx) => (
                      <div
                        key={idx}
                        className="p-1.5 rounded-md bg-card hover:bg-muted border border-border text-xs font-mono text-foreground truncate select-all flex items-center gap-1.5"
                      >
                        <FileCode className="size-3 text-muted-foreground shrink-0" />
                        <span className="truncate">{file}</span>
                      </div>
                    ))}
                  </div>
                ) : (
                  <p className="text-sm text-muted-foreground italic">
                    No files recorded under this project branch yet.
                  </p>
                )}
              </div>
            </div>
          ) : null}
        </div>
      </aside>
    </>
  );
};
