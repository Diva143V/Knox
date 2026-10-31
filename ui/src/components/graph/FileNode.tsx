import { memo } from "react";
import { Handle, Position } from "@xyflow/react";
import { FileCode } from "lucide-react";
import { cn } from "@/lib/cn";

export interface FileNodeData {
  filePath: string;
  count: number;
  [key: string]: unknown;
}

export const FileNode = memo(({ data }: { data: FileNodeData }) => {
  const shortName =
    data.filePath.split(/[/\\]/).pop() || data.filePath;

  return (
    <div
      title={data.filePath}
      className={cn(
        "flex items-center gap-2 px-2.5 py-1.5 rounded-md border",
        "border-plan/35 bg-card text-sm font-mono text-foreground",
        "hover:border-plan/70 transition-colors duration-100 cursor-default select-none"
      )}
    >
      <Handle
        type="target"
        position={Position.Top}
        className="!size-2 !bg-plan !border !border-background"
      />
      <FileCode className="size-3.5 text-plan shrink-0" aria-hidden />
      <span className="truncate max-w-[160px]">{shortName}</span>
      <span className="text-xs bg-plan/15 text-plan px-1 py-0.5 rounded-sm border border-plan/25 shrink-0 font-mono">
        {data.count}
      </span>
      <Handle
        type="source"
        position={Position.Bottom}
        className="!size-2 !bg-plan !border !border-background"
      />
    </div>
  );
});

FileNode.displayName = "FileNode";
