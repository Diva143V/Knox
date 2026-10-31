import React, { useState } from "react";
import type { KnoxAction } from "@/types/knox";
import { ActionBadge } from "@/components/ui/ActionBadge";
import { ActorBadge } from "@/components/ui/ActorBadge";
import { Button } from "@/components/ui/Button";
import { cn } from "@/lib/cn";
import {
  Copy,
  Check,
  FileCode,
  Tag,
  GitCommit,
  Clock,
  ChevronDown,
  ChevronRight,
  Gauge,
  Code2,
} from "lucide-react";
import { useKnoxStore } from "@/store/useKnoxStore";

interface ActionDetailProps {
  action: KnoxAction;
  onSelectAction?: (action: KnoxAction) => void;
  className?: string;
}

const Section: React.FC<{
  title: string;
  icon?: React.ReactNode;
  trailing?: React.ReactNode;
  children: React.ReactNode;
  className?: string;
}> = ({ title, icon, trailing, children, className }) => (
  <section className={cn("space-y-2", className)}>
    <div className="flex items-center justify-between gap-2">
      <h3 className="text-overline flex items-center gap-1.5">
        {icon}
        <span>{title}</span>
      </h3>
      {trailing}
    </div>
    {children}
  </section>
);

export const ActionDetail: React.FC<ActionDetailProps> = ({
  action,
  onSelectAction,
  className = "",
}) => {
  const { actions, showToast, setSelectedAction } = useKnoxStore();
  const [copiedId, setCopiedId] = useState(false);
  const [showRawJson, setShowRawJson] = useState(false);

  const handleCopyId = () => {
    navigator.clipboard.writeText(action.id);
    setCopiedId(true);
    showToast("Copied Action ID to clipboard");
    setTimeout(() => setCopiedId(false), 2000);
  };

  const handleJumpToParent = (parentId: string) => {
    const parent = actions.find((a) => a.id === parentId);
    if (parent) {
      if (onSelectAction) onSelectAction(parent);
      else setSelectedAction(parent);
    } else {
      showToast(
        `Parent action ${parentId} not found in active graph cache`,
        true
      );
    }
  };

  const handleCopyFile = async (file: string) => {
    try {
      await navigator.clipboard.writeText(file);
      showToast("Copied file path");
    } catch {
      showToast("Could not copy file path", true);
    }
  };

  const formattedDate = new Date(action.timestamp).toLocaleString(undefined, {
    year: "numeric",
    month: "short",
    day: "numeric",
    hour: "2-digit",
    minute: "2-digit",
    second: "2-digit",
  });

  const confidencePct =
    action.confidence !== null && action.confidence !== undefined
      ? Math.round(action.confidence * 100)
      : null;

  return (
    <div className={cn("space-y-5 text-xs text-foreground", className)}>
      <div className="flex items-center justify-between gap-2">
        <div className="flex items-center gap-1.5 min-w-0">
          <ActorBadge actor={action.actor} />
          <ActionBadge type={action.action_type} />
        </div>
        {action.is_current ? (
          <span className="text-xs font-mono font-medium px-1.5 py-0.5 rounded-sm bg-success/15 text-success border border-success/30 shrink-0">
            CURRENT
          </span>
        ) : (
          <span className="text-xs font-mono font-medium px-1.5 py-0.5 rounded-sm bg-warning/15 text-warning border border-warning/30 shrink-0">
            SUPERSEDED
          </span>
        )}
      </div>

      <Section title="Summary">
        <p className="text-sm font-medium text-foreground leading-relaxed bg-card p-3 rounded-md border border-border">
          {action.summary}
        </p>
      </Section>

      <Section
        title="Action ID"
        trailing={
          <Button
            type="button"
            variant="ghost"
            size="sm"
            onClick={handleCopyId}
            className="h-6 px-1.5 text-sm"
            aria-label="Copy action ID"
          >
            {copiedId ? (
              <>
                <Check className="size-3 text-success" />
                <span className="text-success">Copied</span>
              </>
            ) : (
              <>
                <Copy className="size-3" />
                <span>Copy</span>
              </>
            )}
          </Button>
        }
      >
        <p className="font-mono text-sm text-foreground truncate select-all px-2.5 py-2 rounded-md border border-border bg-card">
          {action.id}
        </p>
      </Section>

      {confidencePct !== null && (
        <Section
          title="Confidence"
          icon={<Gauge className="size-3.5 text-primary" aria-hidden />}
          trailing={
            <span className="font-mono font-semibold text-foreground text-sm">
              {confidencePct}%
            </span>
          }
        >
          <div className="h-1.5 w-full bg-background rounded-full overflow-hidden border border-border">
            <div
              className={cn(
                "h-full rounded-full",
                confidencePct >= 80
                  ? "bg-success"
                  : confidencePct >= 50
                    ? "bg-warning"
                    : "bg-danger"
              )}
              style={{ width: `${confidencePct}%` }}
            />
          </div>
        </Section>
      )}

      <Section
        title="Metadata"
        icon={<Clock className="size-3.5" aria-hidden />}
      >
        <div className="space-y-2 p-3 rounded-md border border-border bg-card font-mono text-sm">
          <div className="flex items-center justify-between gap-2 text-muted-foreground">
            <span>Timestamp</span>
            <span className="text-foreground text-right">{formattedDate}</span>
          </div>
          {action.session_id && (
            <div className="flex items-center justify-between gap-2 text-muted-foreground pt-2 border-t border-border-subtle">
              <span>Session</span>
              <span className="text-foreground truncate max-w-[170px]">
                {action.session_id}
              </span>
            </div>
          )}
        </div>
      </Section>

      {(action.parent_action_id || action.supersedes) && (
        <Section
          title="Causal Relations"
          icon={<GitCommit className="size-3.5 text-primary" aria-hidden />}
        >
          <div className="space-y-2 p-3 rounded-md border border-border bg-card">
            {action.parent_action_id && (
              <div className="space-y-1">
                <span className="text-sm text-muted-foreground">Parent</span>
                <button
                  type="button"
                  onClick={() => handleJumpToParent(action.parent_action_id!)}
                  className="w-full text-left font-mono text-sm text-primary hover:underline truncate bg-background px-2 py-1.5 rounded-md border border-border cursor-pointer focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-focus-ring"
                  title={`Jump to parent: ${action.parent_action_id}`}
                >
                  ↑ {action.parent_action_id}
                </button>
              </div>
            )}
            {action.supersedes && (
              <div className="space-y-1">
                <span className="text-sm text-muted-foreground">
                  Supersedes
                </span>
                <button
                  type="button"
                  onClick={() => handleJumpToParent(action.supersedes!)}
                  className="w-full text-left font-mono text-sm text-warning hover:underline truncate bg-background px-2 py-1.5 rounded-md border border-border cursor-pointer focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-focus-ring"
                  title={`Jump to superseded: ${action.supersedes}`}
                >
                  ⟲ {action.supersedes}
                </button>
              </div>
            )}
          </div>
        </Section>
      )}

      <Section
        title="Touched Files"
        icon={<FileCode className="size-3.5 text-primary" aria-hidden />}
        trailing={
          <span className="text-xs font-mono text-muted-foreground bg-card px-1.5 py-0.5 rounded-sm border border-border">
            {action.files?.length || 0}
          </span>
        }
      >
        {action.files && action.files.length > 0 ? (
          <div className="space-y-1 max-h-40 overflow-y-auto pr-0.5">
            {action.files.map((file, idx) => (
              <button
                key={idx}
                type="button"
                onClick={() => handleCopyFile(file)}
                title="Copy file path"
                className="w-full flex items-center gap-2 p-1.5 rounded-md bg-card hover:bg-muted border border-border text-sm font-mono text-foreground truncate cursor-pointer text-left focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-focus-ring"
              >
                <FileCode className="size-3 text-muted-foreground shrink-0" />
                <span className="truncate">{file}</span>
              </button>
            ))}
          </div>
        ) : (
          <p className="text-sm text-muted-foreground italic">
            No specific files recorded for this decision.
          </p>
        )}
      </Section>

      {action.tags && action.tags.length > 0 && (
        <Section
          title="Tags"
          icon={<Tag className="size-3.5 text-primary" aria-hidden />}
        >
          <div className="flex flex-wrap gap-1.5">
            {action.tags.map((tag, idx) => (
              <span
                key={idx}
                className="text-xs font-mono px-2 py-0.5 rounded-sm bg-card border border-border text-foreground/90"
              >
                #{tag}
              </span>
            ))}
          </div>
        </Section>
      )}

      <div className="pt-1 border-t border-border-subtle">
        <button
          type="button"
          onClick={() => setShowRawJson(!showRawJson)}
          className="w-full flex items-center justify-between py-1.5 text-muted-foreground hover:text-foreground text-sm font-medium transition-colors cursor-pointer focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-focus-ring rounded-sm"
          aria-expanded={showRawJson}
        >
          <span className="flex items-center gap-1.5">
            <Code2 className="size-3.5" aria-hidden />
            <span>Raw JSON</span>
          </span>
          {showRawJson ? (
            <ChevronDown className="size-3.5" aria-hidden />
          ) : (
            <ChevronRight className="size-3.5" aria-hidden />
          )}
        </button>

        {showRawJson && (
          <pre className="mt-2 p-3 rounded-md bg-background border border-border text-xs font-mono text-muted-foreground overflow-x-auto max-h-60 leading-relaxed select-all">
            {JSON.stringify(action, null, 2)}
          </pre>
        )}
      </div>
    </div>
  );
};
