import React from "react";
import { Terminal, Copy, Trash2, X } from "lucide-react";
import { Button } from "@/components/ui/Button";
import { cn } from "@/lib/cn";
import type { McpScanLog } from "@/types/knox";

interface McpLogsTerminalProps {
  showTerminal: boolean;
  setShowTerminal: (show: boolean) => void;
  logs: McpScanLog[];
  autoScroll: boolean;
  setAutoScroll: (scroll: boolean) => void;
  logContainerRef: React.RefObject<HTMLDivElement | null>;
  onClearLogs: () => void;
  onCopyLogs: () => void;
  isScanning: boolean;
  isDeepScanning: boolean;
}

export const McpLogsTerminal: React.FC<McpLogsTerminalProps> = ({
  showTerminal,
  setShowTerminal,
  logs,
  autoScroll,
  setAutoScroll,
  logContainerRef,
  onClearLogs,
  onCopyLogs,
  isScanning,
  isDeepScanning,
}) => {
  if (!showTerminal) return null;

  const renderLogLevelBadge = (level: McpScanLog["level"]) => {
    switch (level) {
      case "info":
        return (
          <span className="text-2xs font-bold px-1 py-0.5 rounded-sm bg-warning/20 text-warning border border-warning/30 shrink-0">
            INFO
          </span>
        );
      case "scan":
        return (
          <span className="text-2xs font-bold px-1 py-0.5 rounded-sm bg-primary/20 text-primary border border-primary/30 shrink-0">
            SCAN
          </span>
        );
      case "found":
        return (
          <span className="text-2xs font-bold px-1 py-0.5 rounded-sm bg-success/20 text-success border border-success/30 shrink-0">
            FOUND
          </span>
        );
      case "config":
        return (
          <span className="text-2xs font-bold px-1 py-0.5 rounded-sm bg-primary/20 text-primary border border-primary/30 shrink-0">
            CONFIG
          </span>
        );
      case "skip":
        return (
          <span className="text-2xs font-bold px-1 py-0.5 rounded-sm bg-muted text-muted-foreground border border-border shrink-0">
            SKIP
          </span>
        );
      case "error":
        return (
          <span className="text-2xs font-bold px-1 py-0.5 rounded-sm bg-danger/20 text-danger border border-danger/30 shrink-0">
            ERROR
          </span>
        );
      default:
        return null;
    }
  };

  return (
    <div
      className={cn(
        "w-full lg:w-96 rounded-xl border border-border bg-card p-4 shadow-xl shrink-0 flex flex-col",
        "h-[500px] lg:h-[calc(100vh-210px)] sticky top-6 motion-safe:animate-in motion-safe:slide-in-from-right-4 motion-safe:duration-200"
      )}
      role="region"
      aria-label="Real-time scan logs"
    >
      {/* Header */}
      <div className="flex items-center justify-between pb-3 border-b border-border shrink-0">
        <div className="flex items-center gap-2">
          <Terminal className="size-4 text-primary" aria-hidden />
          <h3 className="text-xs font-semibold text-foreground">Discovery Live Console</h3>
          {(isScanning || isDeepScanning) && (
            <span className="size-2 rounded-full bg-primary animate-ping" />
          )}
        </div>
        <div className="flex items-center gap-1.5">
          <button
            type="button"
            onClick={() => setAutoScroll(!autoScroll)}
            className={cn(
              "px-2 py-0.5 rounded-sm text-2xs font-mono border transition-colors cursor-pointer",
              autoScroll
                ? "bg-primary/20 text-primary border-primary/40 font-semibold"
                : "bg-muted text-muted-foreground border-border"
            )}
            title="Toggle auto-scrolling to bottom"
          >
            Scroll: {autoScroll ? "ON" : "OFF"}
          </button>

          <Button
            type="button"
            variant="ghost"
            size="icon"
            className="size-7"
            onClick={onCopyLogs}
            title="Copy logs to clipboard"
            aria-label="Copy logs"
          >
            <Copy className="size-3.5" />
          </Button>

          <Button
            type="button"
            variant="ghost"
            size="icon"
            className="size-7"
            onClick={onClearLogs}
            title="Clear console"
            aria-label="Clear console"
          >
            <Trash2 className="size-3.5" />
          </Button>

          <Button
            type="button"
            variant="ghost"
            size="icon"
            className="size-7 text-muted-foreground hover:text-danger"
            onClick={() => setShowTerminal(false)}
            title="Close terminal panel"
            aria-label="Close terminal panel"
          >
            <X className="size-3.5" />
          </Button>
        </div>
      </div>

      {/* Console Output */}
      <div
        ref={logContainerRef}
        className="flex-1 overflow-y-auto font-mono text-2xs p-2 space-y-1.5 scrollbar-thin select-text my-2 bg-background rounded-md border border-border"
      >
        {logs.length === 0 ? (
          <div className="h-full flex flex-col items-center justify-center text-center p-6 text-muted-foreground space-y-2">
            <Terminal className="size-8 opacity-40 mx-auto" />
            <p className="text-xs font-medium text-foreground">Terminal Ready</p>
            <p className="text-2xs text-muted-foreground max-w-[240px]">
              Click Quick Scan or Deep Search PC to stream real-time filesystem discovery events here.
            </p>
          </div>
        ) : (
          logs.map((log, i) => (
            <div key={i} className="leading-tight flex items-start gap-1.5 text-foreground break-all">
              <span className="text-muted-foreground shrink-0 text-2xs">{log.timestamp}</span>
              {renderLogLevelBadge(log.level)}
              <span className="flex-1 text-foreground">{log.message}</span>
            </div>
          ))
        )}
      </div>

      {/* Status Footer */}
      <div className="pt-2 border-t border-border flex items-center justify-between text-2xs text-muted-foreground font-mono shrink-0">
        <span>{logs.length} events logged</span>
        <span className="truncate max-w-[220px]">
          {isScanning
            ? "Quick scan in progress..."
            : isDeepScanning
            ? "Deep scanning system volumes..."
            : "Idle / Complete"}
        </span>
      </div>
    </div>
  );
};
