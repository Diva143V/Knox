import { memo } from "react";
import { Handle, Position, type NodeProps } from "@xyflow/react";
import { Globe, Sparkles, FolderGit2 } from "lucide-react";
import { useKnoxStore } from "@/store/useKnoxStore";
import type { Project } from "@/types/knox";

export interface ProjectRootNodeData {
  name: string;
  path: string;
  isGlobal: boolean;
  actionCount: number;
  fileCount: number;
  project?: Project;
  isSelected?: boolean;
}

export const ProjectRootNode = memo(({ data }: NodeProps) => {
  const nodeData = data as unknown as ProjectRootNodeData;
  const isGlobal = nodeData.isGlobal;
  const { setSelectedProject } = useKnoxStore();

  const handleClick = () => {
    if (!isGlobal && nodeData.project) {
      setSelectedProject(nodeData.project);
    }
  };

  return (
    <div
      onClick={handleClick}
      className={`relative group select-none ${
        !isGlobal ? "cursor-pointer" : ""
      }`}
    >
      {/* Top Target Handle for non-global project branch nodes */}
      {!isGlobal && (
        <Handle
          type="target"
          position={Position.Top}
          className="!w-3 !h-3 !bg-sky-500 !border-2 !border-background !rounded-full transition hover:!scale-125"
        />
      )}

      {/* Outer Glow */}
      <div
        className={`absolute -inset-1 rounded-2xl blur-sm opacity-40 dark:opacity-70 group-hover:opacity-75 dark:group-hover:opacity-100 transition duration-300 ${
          isGlobal
            ? "bg-gradient-to-r from-indigo-500/30 via-purple-500/20 to-indigo-500/30"
            : "bg-gradient-to-r from-sky-500/30 via-cyan-500/20 to-teal-500/30"
        }`}
      />

      {/* Main Card */}
      <div
        className={`relative w-80 rounded-2xl border-2 bg-card/95 backdrop-blur-md shadow-xl p-4 space-y-3 transition ${
          isGlobal
            ? "border-indigo-500/50 hover:border-indigo-500 dark:border-indigo-500/60 dark:hover:border-indigo-400"
            : nodeData.isSelected
            ? "border-sky-500 ring-2 ring-sky-400/50 shadow-sky-500/20 shadow-xl"
            : "border-sky-500/50 hover:border-sky-500 dark:border-sky-500/60 dark:hover:border-sky-400"
        }`}
      >
        {/* Top Header */}
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-2 min-w-0">
            <div
              className={`size-8 rounded-xl border flex items-center justify-center shrink-0 shadow-xs ${
                isGlobal
                  ? "bg-indigo-500/15 border-indigo-500/30 text-indigo-600 dark:text-indigo-400"
                  : "bg-sky-500/15 border-sky-500/30 text-sky-600 dark:text-sky-400"
              }`}
            >
              {isGlobal ? (
                <Globe className="size-4 text-indigo-600 dark:text-indigo-400 animate-pulse" />
              ) : (
                <FolderGit2 className="size-4 text-sky-600 dark:text-sky-400" />
              )}
            </div>
            <div className="min-w-0">
              <div className="flex items-center gap-1.5 flex-wrap">
                <span className="text-xs font-bold text-foreground font-sans tracking-tight truncate max-w-[150px]">
                  {nodeData.name || (isGlobal ? "Global KG" : "Project Root")}
                </span>
                <span
                  className={`text-2xs font-mono px-1.5 py-0.5 rounded-full border uppercase font-semibold shrink-0 ${
                    isGlobal
                      ? "bg-indigo-500/15 text-indigo-700 dark:text-indigo-300 border-indigo-500/30"
                      : "bg-sky-500/15 text-sky-700 dark:text-sky-300 border-sky-500/30"
                  }`}
                >
                  {isGlobal ? "Global KG" : "Project Branch"}
                </span>
              </div>
              <p
                className="text-2xs font-mono text-muted-foreground truncate max-w-[190px]"
                title={nodeData.path}
              >
                {nodeData.path}
              </p>
            </div>
          </div>

          <Sparkles
            className={`size-4 opacity-80 shrink-0 ${
              isGlobal ? "text-indigo-600 dark:text-indigo-400" : "text-sky-600 dark:text-sky-400"
            }`}
          />
        </div>

        {/* Counters & Metadata Bar */}
        <div className="grid grid-cols-2 gap-2 pt-2 border-t border-border text-2xs font-mono">
          <div className="p-2 rounded-lg bg-muted/50 border border-border flex flex-col items-center justify-center">
            <span
              className={`text-xs font-bold ${
                isGlobal ? "text-indigo-600 dark:text-indigo-400" : "text-emerald-600 dark:text-emerald-400"
              }`}
            >
              {nodeData.actionCount}
            </span>
            <span className="text-2xs text-muted-foreground uppercase">
              {isGlobal ? "Total Actions" : "Project Actions"}
            </span>
          </div>

          <div className="p-2 rounded-lg bg-muted/50 border border-border flex flex-col items-center justify-center">
            <span
              className={`text-xs font-bold ${
                isGlobal ? "text-purple-600 dark:text-purple-400" : "text-sky-600 dark:text-sky-400"
              }`}
            >
              {nodeData.fileCount}
            </span>
            <span className="text-2xs text-muted-foreground uppercase">
              {isGlobal ? "Total Files" : "Project Files"}
            </span>
          </div>
        </div>
      </div>

      {/* Downward source handle to connect to files and decisions */}
      <Handle
        type="source"
        position={Position.Bottom}
        className={`!w-3 !h-3 !border-2 !border-background !rounded-full transition hover:!scale-125 ${
          isGlobal ? "!bg-indigo-500" : "!bg-sky-500"
        }`}
      />
    </div>
  );
});

ProjectRootNode.displayName = "ProjectRootNode";
