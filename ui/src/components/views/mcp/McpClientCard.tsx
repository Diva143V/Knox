import React from "react";
import { Check, Trash2 } from "lucide-react";
import { Button } from "@/components/ui/Button";
import { Badge } from "@/components/ui/Badge";
import type { DetectedMcpApp } from "@/types/knox";

interface McpClientCardProps {
  app: DetectedMcpApp;
  actionLoadingId: string | null;
  onInject: (app: DetectedMcpApp) => void;
  onRemove: (app: DetectedMcpApp) => void;
  onDeleteCustomApp: (id: string, e: React.MouseEvent) => void;
}

export const McpClientCard: React.FC<McpClientCardProps> = ({
  app,
  actionLoadingId,
  onInject,
  onRemove,
  onDeleteCustomApp,
}) => {
  const isLoading = actionLoadingId === app.id;

  return (
    <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-3 p-3.5 rounded-lg border border-border bg-card/60 hover:bg-card transition-colors">
      <div className="min-w-0 space-y-1">
        <div className="flex items-center gap-2 flex-wrap">
          <span className="font-semibold text-xs text-foreground">
            {app.name}
          </span>
          <Badge variant="secondary" className="uppercase font-mono text-2xs">
            {app.category}
          </Badge>
          {app.is_custom && (
            <Badge variant="outline" className="text-2xs">
              Custom App
            </Badge>
          )}

          {app.is_configured ? (
            <span className="inline-flex items-center gap-1 text-2xs font-mono px-2 py-0.5 rounded-sm bg-success/15 text-success border border-success/30 font-medium">
              <Check className="size-3" />
              Configured
            </span>
          ) : app.exists ? (
            <span className="inline-flex items-center gap-1 text-2xs font-mono px-2 py-0.5 rounded-sm bg-warning/15 text-warning border border-warning/30">
              Detected
            </span>
          ) : (
            <span className="text-2xs font-mono px-2 py-0.5 rounded-sm bg-muted text-muted-foreground border border-border">
              Not Installed
            </span>
          )}
        </div>

        <p
          className="font-mono text-2xs text-muted-foreground truncate"
          title={app.file_path}
        >
          {app.file_path}
        </p>
      </div>

      <div className="flex items-center gap-2 shrink-0">
        {app.is_configured ? (
          <Button
            type="button"
            variant="outline"
            size="sm"
            onClick={() => onRemove(app)}
            disabled={isLoading}
            className="text-danger hover:bg-danger/10 border-danger/30"
          >
            {isLoading ? "Removing..." : "Remove Knox"}
          </Button>
        ) : (
          <Button
            type="button"
            variant="primary"
            size="sm"
            onClick={() => onInject(app)}
            disabled={isLoading}
          >
            {isLoading ? "Adding..." : "Add Knox"}
          </Button>
        )}

        {app.is_custom && (
          <Button
            type="button"
            variant="ghost"
            size="icon"
            onClick={(e) => onDeleteCustomApp(app.id, e)}
            title="Remove custom application from list"
            className="size-7 text-muted-foreground hover:text-danger"
          >
            <Trash2 className="size-3.5" />
          </Button>
        )}
      </div>
    </div>
  );
};
