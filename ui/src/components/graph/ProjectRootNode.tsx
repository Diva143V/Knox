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
          className="!w-3 !h-3 !bg-sky-400 !border-2 !border-zinc-950 !rounded-full transition hover:!scale-125"
        />
      )}

      {/* Outer Glow */}
      <div
        className={`absolute -inset-1 rounded-2xl blur-sm opacity-70 group-hover:opacity-100 transition duration-300 ${
          isGlobal
            ? "bg-gradient-to-r from-indigo-500/30 via-purple-500/20 to-indigo-500/30"
            : "bg-gradient-to-r from-sky-500/30 via-cyan-500/20 to-teal-500/30"
        }`}
      />

      {/* Main Card */}
      <div
        className={`relative w-80 rounded-2xl border-2 bg-zinc-950/95 backdrop-blur-md shadow-2xl p-4 space-y-3 transition ${
          isGlobal
            ? "border-indigo-500/60 hover:border-indigo-400"
            : nodeData.isSelected
            ? "border-sky-400 ring-2 ring-sky-400/50 shadow-sky-500/20 shadow-xl"
            : "border-sky-500/60 hover:border-sky-400"
        }`}
      >
        {/* Top Header */}
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-2">
            <div
              className={`w-8 h-8 rounded-xl border flex items-center justify-center shadow-xs ${
                isGlobal
                  ? "bg-indigo-500/20 border-indigo-500/40 text-indigo-400"
                  : "bg-sky-500/20 border-sky-500/40 text-sky-400"
              }`}
            >
              {isGlobal ? (
                <Globe className="w-4 h-4 text-indigo-400 animate-pulse" />
              ) : (
                <FolderGit2 className="w-4 h-4 text-sky-400" />
              )}
            </div>
            <div>
              <div className="flex items-center gap-1.5">
                <span className="text-xs font-bold text-zinc-100 font-sans tracking-tight">
                  {nodeData.name || (isGlobal ? "Global KG" : "Project Root")}
                </span>
                <span
                  className={`text-[9px] font-mono px-1.5 py-0.2 rounded-full border uppercase font-semibold ${
                    isGlobal
                      ? "bg-indigo-500/20 text-indigo-300 border-indigo-500/30"
                      : "bg-sky-500/20 text-sky-300 border-sky-500/30"
                  }`}
                >
                  {isGlobal ? "Global KG" : "Project Branch"}
                </span>
              </div>
              <p
                className="text-[10px] font-mono text-zinc-400 truncate max-w-[190px]"
                title={nodeData.path}
              >
                {nodeData.path}
              </p>
            </div>
          </div>

          <Sparkles
            className={`w-4 h-4 opacity-80 ${
              isGlobal ? "text-indigo-400" : "text-sky-400"
            }`}
          />
        </div>

        {/* Counters & Metadata Bar */}
        <div className="grid grid-cols-2 gap-2 pt-2 border-t border-zinc-800/80 text-[11px] font-mono">
          <div className="p-2 rounded-lg bg-zinc-900/80 border border-zinc-800 flex flex-col items-center justify-center">
            <span
              className={`text-xs font-bold ${
                isGlobal ? "text-indigo-400" : "text-emerald-400"
              }`}
            >
              {nodeData.actionCount}
            </span>
            <span className="text-[9px] text-zinc-500 uppercase">
              {isGlobal ? "Total Actions" : "Project Actions"}
            </span>
          </div>

          <div className="p-2 rounded-lg bg-zinc-900/80 border border-zinc-800 flex flex-col items-center justify-center">
            <span
              className={`text-xs font-bold ${
                isGlobal ? "text-purple-400" : "text-cyan-400"
              }`}
            >
              {nodeData.fileCount}
            </span>
            <span className="text-[9px] text-zinc-500 uppercase">
              {isGlobal ? "Total Files" : "Project Files"}
            </span>
          </div>
        </div>
      </div>

      {/* Downward source handle to connect to files and decisions */}
      <Handle
        type="source"
        position={Position.Bottom}
        className={`!w-3 !h-3 !border-2 !border-zinc-950 !rounded-full transition hover:!scale-125 ${
          isGlobal ? "!bg-indigo-400" : "!bg-sky-400"
        }`}
      />
    </div>
  );
});

ProjectRootNode.displayName = "ProjectRootNode";
