import React from "react";
import { FileCode, X, FolderOpen, Check } from "lucide-react";
import { Button } from "@/components/ui/Button";

interface McpManualConfigModalProps {
  showCustomModal: boolean;
  setShowCustomModal: (show: boolean) => void;
  customName: string;
  setCustomName: (name: string) => void;
  customPath: string;
  setCustomPath: (path: string) => void;
  customLoading: boolean;
  onBrowseCustomFile: () => void;
  onSaveCustomApp: () => void;
}

export const McpManualConfigModal: React.FC<McpManualConfigModalProps> = ({
  showCustomModal,
  setShowCustomModal,
  customName,
  setCustomName,
  customPath,
  setCustomPath,
  customLoading,
  onBrowseCustomFile,
  onSaveCustomApp,
}) => {
  if (!showCustomModal) return null;

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center p-4 bg-background/80 backdrop-blur-sm motion-safe:animate-in motion-safe:fade-in motion-safe:duration-150">
      <div className="w-full max-w-lg bg-card border border-border rounded-xl shadow-2xl p-6 space-y-4">
        <div className="flex items-center justify-between border-b border-border-subtle pb-3">
          <h3 className="text-sm font-semibold text-foreground flex items-center gap-2">
            <FileCode className="size-4 text-primary" />
            <span>Configure Custom or Private Application</span>
          </h3>
          <Button
            type="button"
            variant="ghost"
            size="icon"
            className="size-7"
            onClick={() => setShowCustomModal(false)}
            aria-label="Close modal"
          >
            <X className="size-4" />
          </Button>
        </div>

        <p className="text-xs text-muted-foreground leading-relaxed">
          Knox will auto-detect the MCP schema (`mcpServers`, `mcp.servers`, or `context_servers`), create a safe `.bak` backup, and inject the Knox server entry directly.
        </p>

        <div className="space-y-3 text-xs">
          <div>
            <label className="block text-2xs font-semibold text-foreground mb-1">
              Application Name
            </label>
            <input
              type="text"
              placeholder="e.g. My Studio Tool, Roo Code, Custom IDE"
              value={customName}
              onChange={(e) => setCustomName(e.target.value)}
              className="w-full px-3 py-2 rounded-md bg-background border border-border focus:border-primary focus:outline-hidden text-foreground text-xs"
            />
          </div>

          <div>
            <label className="block text-2xs font-semibold text-foreground mb-1">
              Configuration File Path (.json)
            </label>
            <div className="flex gap-2">
              <input
                type="text"
                placeholder="e.g. C:\Users\...\settings.json"
                value={customPath}
                onChange={(e) => setCustomPath(e.target.value)}
                className="flex-1 px-3 py-2 rounded-md bg-background border border-border focus:border-primary focus:outline-hidden text-foreground text-xs font-mono"
              />
              <Button
                type="button"
                variant="outline"
                onClick={onBrowseCustomFile}
                className="shrink-0"
              >
                <FolderOpen className="size-3.5 text-primary" />
                <span>Browse...</span>
              </Button>
            </div>
          </div>
        </div>

        <div className="pt-2 flex items-center justify-end gap-2 border-t border-border-subtle">
          <Button
            type="button"
            variant="ghost"
            onClick={() => setShowCustomModal(false)}
          >
            Cancel
          </Button>
          <Button
            type="button"
            variant="primary"
            onClick={onSaveCustomApp}
            disabled={customLoading || !customPath.trim()}
          >
            <Check className="size-3.5" />
            <span>
              {customLoading ? "Saving..." : "Save & Configure Knox"}
            </span>
          </Button>
        </div>
      </div>
    </div>
  );
};
