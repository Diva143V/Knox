import React, { useMemo, useState } from "react";
import {
  X,
  Layers,
  FolderGit2,
  Copy,
  Check,
  Target,
  FileCode,
  Sparkles,
  GitBranch,
} from "lucide-react";
import { useKnoxStore } from "@/store/useKnoxStore";
import { ActionDetail } from "@/components/graph/ActionDetail";

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

  if (!selectedAction && !selectedProject) return null;

  return (
    <aside className="w-80 shrink-0 border-l border-border/80 bg-sidebar/40 flex flex-col h-full z-20 select-none overflow-hidden animate-in slide-in-from-right-4 duration-150">
      {/* Inspector Header */}
      <div className="h-12 border-b border-border/60 px-4 flex items-center justify-between shrink-0">
        <div className="flex items-center gap-2">
          {selectedAction ? (
            <>
              <Layers className="w-4 h-4 text-primary" />
              <span className="text-xs font-semibold uppercase tracking-wider text-muted-foreground">
                Action Detail
              </span>
            </>
          ) : (
            <>
              <FolderGit2 className="w-4 h-4 text-sky-400" />
              <span className="text-xs font-semibold uppercase tracking-wider text-muted-foreground">
                Project Summary
              </span>
            </>
          )}
        </div>
        <button
          onClick={() => {
            setSelectedAction(null);
            setSelectedProject(null);
          }}
          className="p-1 rounded-md text-muted-foreground hover:text-foreground hover:bg-card transition cursor-pointer"
          title="Close Inspector (Esc)"
        >
          <X className="w-4 h-4" />
        </button>
      </div>

      {/* Inspector Scrollable Body */}
      <div className="flex-1 overflow-y-auto p-4">
        {selectedAction ? (
          <ActionDetail action={selectedAction} />
        ) : selectedProject ? (
          <div className="space-y-4 text-xs text-foreground">
            {/* Project Header Card */}
            <div className="p-3.5 rounded-xl border border-sky-500/40 bg-gradient-to-br from-sky-950/40 via-zinc-950 to-zinc-900 space-y-3">
              <div className="flex items-center justify-between">
                <div className="flex items-center gap-2">
                  <div className="w-8 h-8 rounded-lg bg-sky-500/20 border border-sky-500/40 flex items-center justify-center text-sky-400">
                    <FolderGit2 className="w-4 h-4" />
                  </div>
                  <div>
                    <h3 className="text-sm font-bold text-zinc-100 font-sans tracking-tight">
                      {selectedProject.name}
                    </h3>
                    <span className="text-[9px] font-mono px-1.5 py-0.2 rounded bg-sky-500/20 text-sky-300 border border-sky-500/30 uppercase font-semibold">
                      PROJECT BRANCH
                    </span>
                  </div>
                </div>
                <Sparkles className="w-4 h-4 text-sky-400" />
              </div>

              {/* Physical Path */}
              <div className="space-y-1 pt-1 border-t border-zinc-800">
                <div className="flex items-center justify-between text-zinc-400 text-[10px]">
                  <span>Root Filesystem Path:</span>
                  <button
                    onClick={() => handleCopyPath(selectedProject.path)}
                    className="text-sky-400 hover:underline flex items-center gap-0.5 cursor-pointer font-mono"
                  >
                    {copiedPath ? (
                      <Check className="w-2.5 h-2.5 text-emerald-400" />
                    ) : (
                      <Copy className="w-2.5 h-2.5" />
                    )}
                    <span>{copiedPath ? "Copied" : "Copy"}</span>
                  </button>
                </div>
                <p
                  className="font-mono text-[11px] text-zinc-300 bg-zinc-900/90 p-2 rounded border border-zinc-800 truncate select-all"
                  title={selectedProject.path}
                >
                  {selectedProject.path}
                </p>
              </div>

              {/* Project Metric Counters */}
              <div className="grid grid-cols-2 gap-2 pt-1 font-mono text-[11px]">
                <div className="p-2 rounded-lg bg-zinc-900/80 border border-zinc-800 flex flex-col items-center">
                  <span className="text-xs font-bold text-emerald-400">
                    {projectStats.actionCount}
                  </span>
                  <span className="text-[9px] text-zinc-500 uppercase">Actions</span>
                </div>
                <div className="p-2 rounded-lg bg-zinc-900/80 border border-zinc-800 flex flex-col items-center">
                  <span className="text-xs font-bold text-cyan-400">
                    {projectStats.files.length}
                  </span>
                  <span className="text-[9px] text-zinc-500 uppercase">Files</span>
                </div>
              </div>
            </div>

            {/* Quick Action: Focus Project */}
            <div>
              {activeProjectScope?.toLowerCase() ===
              selectedProject.path.toLowerCase() ? (
                <button
                  onClick={() => setProjectScope(null)}
                  className="w-full flex items-center justify-center gap-2 py-2 px-3 rounded-lg bg-zinc-800 hover:bg-zinc-700 text-zinc-200 border border-zinc-700 font-medium text-xs transition cursor-pointer"
                >
                  <GitBranch className="w-3.5 h-3.5 text-zinc-400" />
                  <span>Clear Focus (Show All Projects)</span>
                </button>
              ) : (
                <button
                  onClick={() => setProjectScope(selectedProject.path)}
                  className="w-full flex items-center justify-center gap-2 py-2 px-3 rounded-lg bg-sky-500/20 hover:bg-sky-500/30 text-sky-300 border border-sky-500/40 font-medium text-xs transition cursor-pointer shadow-xs"
                >
                  <Target className="w-3.5 h-3.5 text-sky-400" />
                  <span>Focus this Project</span>
                </button>
              )}
            </div>

            {/* Codebase Files */}
            <div className="space-y-2 pt-2 border-t border-zinc-800">
              <div className="flex items-center justify-between">
                <span className="text-[10px] font-semibold text-muted-foreground uppercase tracking-wider flex items-center gap-1.5">
                  <FileCode className="w-3.5 h-3.5 text-cyan-400" />
                  <span>Project Files</span>
                </span>
                <span className="text-[10px] font-mono text-muted-foreground bg-card px-1.5 py-0.2 rounded border border-border/50">
                  {projectStats.files.length}
                </span>
              </div>

              {projectStats.files.length > 0 ? (
                <div className="space-y-1 max-h-56 overflow-y-auto pr-1">
                  {projectStats.files.map((file, idx) => (
                    <div
                      key={idx}
                      className="p-1.5 rounded bg-card/50 hover:bg-card border border-border/50 text-[11px] font-mono text-foreground truncate select-all flex items-center gap-1.5"
                    >
                      <FileCode className="w-3 h-3 text-zinc-500 shrink-0" />
                      <span className="truncate">{file}</span>
                    </div>
                  ))}
                </div>
              ) : (
                <p className="text-[11px] text-muted-foreground italic">
                  No files recorded under this project branch yet.
                </p>
              )}
            </div>
          </div>
        ) : null}
      </div>
    </aside>
  );
};
