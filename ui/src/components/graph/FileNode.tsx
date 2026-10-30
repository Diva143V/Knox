import { memo } from "react";
import { Handle, Position } from "@xyflow/react";
import { FileCode } from "lucide-react";

export interface FileNodeData {
  filePath: string;
  count: number;
}

export const FileNode = memo(({ data }: { data: FileNodeData }) => {
  return (
    <div className="flex items-center gap-2 px-3 py-1.5 rounded-lg border border-cyan-500/30 bg-cyan-950/20 backdrop-blur text-xs font-mono text-cyan-300 shadow-sm hover:border-cyan-400 transition cursor-pointer select-none">
      <Handle
        type="target"
        position={Position.Top}
        className="w-2 h-2 bg-cyan-400 border border-background"
      />
      <FileCode className="w-3.5 h-3.5 text-cyan-400" />
      <span className="truncate max-w-[180px]">{data.filePath}</span>
      <span className="text-[10px] bg-cyan-900/40 px-1 py-0.2 rounded border border-cyan-500/20 text-cyan-200">
        {data.count}
      </span>
      <Handle
        type="source"
        position={Position.Bottom}
        className="w-2 h-2 bg-cyan-400 border border-background"
      />
    </div>
  );
});

FileNode.displayName = "FileNode";
