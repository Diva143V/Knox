import { memo, type CSSProperties } from "react";
import { Handle, Position, type NodeProps } from "@xyflow/react";
import { Globe, FolderGit2 } from "lucide-react";
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

function globeDelayFor(key: string): string {
  let sum = 0;
  for (let i = 0; i < key.length; i++) sum += key.charCodeAt(i);
  return `-${sum % 13}s`;
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
      className={`relative select-none ${!isGlobal ? "cursor-pointer" : ""}`}
    >
      {/* Top Target Handle for non-global project branch nodes */}
      {!isGlobal && (
        <Handle
          type="target"
          position={Position.Top}
          className="!w-2.5 !h-2.5 !bg-primary !border-2 !border-background !rounded-full"
        />
      )}

      <div
        className="obsidian-globe"
        style={
          {
            "--globe-delay": globeDelayFor(nodeData.path || nodeData.name),
          } as CSSProperties
        }
      >
        {/* Main Flat Card — zero shadow at rest, 1px border */}
        <div
          className={`obsidian-face relative w-72 rounded-[var(--radius-card)] border bg-card p-3.5 space-y-2.5 transition-colors ${
            nodeData.isSelected
              ? "border-primary ring-1 ring-primary/40"
              : "border-border hover:border-border/80"
          }`}
        >
          <span className="obsidian-sheen" aria-hidden />

          {/* Top Header */}
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-2 min-w-0">
              <div className="size-7 rounded-[var(--radius-control)] border border-border bg-muted/60 flex items-center justify-center shrink-0 text-foreground">
                {isGlobal ? (
                  <Globe className="size-3.5 text-muted-foreground" />
                ) : (
                  <FolderGit2 className="size-3.5 text-primary" />
                )}
              </div>
              <div className="min-w-0">
                <div className="flex items-center gap-1.5 flex-wrap">
                  <span className="text-xs font-medium text-foreground tracking-tight truncate max-w-[150px]">
                    {nodeData.name || (isGlobal ? "Global KG" : "Project root")}
                  </span>
                  <span className="text-3xs font-mono px-1.5 py-0.5 rounded-[var(--radius-control)] border border-border bg-muted/50 text-muted-foreground shrink-0">
                    {isGlobal ? "Global" : "Project"}
                  </span>
                </div>
                <p
                  className="text-3xs font-mono text-muted-foreground truncate max-w-[180px]"
                  title={nodeData.path}
                >
                  {nodeData.path}
                </p>
              </div>
            </div>
          </div>

          {/* Counters & Metadata Bar */}
          <div className="grid grid-cols-2 gap-2 pt-2 border-t border-rule text-xs font-mono">
            <div className="p-2 rounded-[var(--radius-control)] bg-raised/40 border border-rule flex flex-col items-center justify-center">
              <span className="text-xs font-normal tabular-nums text-foreground">
                {nodeData.actionCount}
              </span>
              <span className="text-3xs text-muted-foreground">
                {isGlobal ? "Total actions" : "Actions"}
              </span>
            </div>

            <div className="p-2 rounded-[var(--radius-control)] bg-raised/40 border border-rule flex flex-col items-center justify-center">
              <span className="text-xs font-normal tabular-nums text-foreground">
                {nodeData.fileCount}
              </span>
              <span className="text-3xs text-muted-foreground">
                {isGlobal ? "Total files" : "Files"}
              </span>
            </div>
          </div>
        </div>
      </div>

      {/* Downward source handle to connect to files and decisions */}
      <Handle
        type="source"
        position={Position.Bottom}
        className="!w-2.5 !h-2.5 !border-2 !border-background !rounded-full !bg-primary"
      />
    </div>
  );
});

ProjectRootNode.displayName = "ProjectRootNode";
