import React, { useState } from "react";
import type { KnoxAction } from "@/types/knox";
import { ActionBadge } from "@/components/ui/ActionBadge";
import { ActorBadge } from "@/components/ui/ActorBadge";
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
      showToast(`Parent action ${parentId} not found in active graph cache`, true);
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
    <div className={`space-y-5 text-xs text-foreground ${className}`}>
      {/* Badges & Current Status */}
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-1.5">
          <ActorBadge actor={action.actor} />
          <ActionBadge type={action.action_type} />
        </div>

        {action.is_current ? (
          <span className="text-[10px] font-mono font-medium px-2 py-0.5 rounded bg-emerald-500/15 text-emerald-400 border border-emerald-500/30">
            CURRENT
          </span>
        ) : (
          <span className="text-[10px] font-mono font-medium px-2 py-0.5 rounded bg-amber-500/15 text-amber-400 border border-amber-500/30">
            SUPERSEDED
          </span>
        )}
      </div>

      {/* Summary */}
      <div className="space-y-1.5">
        <span className="text-[10px] font-semibold text-muted-foreground uppercase tracking-wider">
          Summary
        </span>
        <p className="text-sm font-medium text-foreground leading-relaxed bg-card/40 p-3 rounded-lg border border-border/50">
          {action.summary}
        </p>
      </div>

      {/* ID & Clipboard */}
      <div className="space-y-1.5 p-3 rounded-lg border border-border/60 bg-card/60">
        <div className="flex items-center justify-between text-[11px] text-muted-foreground">
          <span className="font-semibold uppercase tracking-wider text-[10px]">
            Action ID
          </span>
          <button
            onClick={handleCopyId}
            className="flex items-center gap-1 text-[11px] text-primary hover:underline cursor-pointer"
          >
            {copiedId ? (
              <>
                <Check className="w-3 h-3 text-emerald-400" />
                <span className="text-emerald-400">Copied</span>
              </>
            ) : (
              <>
                <Copy className="w-3 h-3" />
                <span>Copy</span>
              </>
            )}
          </button>
        </div>
        <p className="font-mono text-[11px] text-foreground truncate select-all">
          {action.id}
        </p>
      </div>

      {/* Confidence Bar */}
      {confidencePct !== null && (
        <div className="space-y-1.5 p-3 rounded-lg border border-border/50 bg-card/40">
          <div className="flex items-center justify-between text-[11px]">
            <span className="flex items-center gap-1.5 text-muted-foreground">
              <Gauge className="w-3.5 h-3.5 text-indigo-400" />
              <span>Confidence:</span>
            </span>
            <span className="font-mono font-semibold text-foreground">
              {confidencePct}%
            </span>
          </div>
          <div className="h-1.5 w-full bg-background rounded-full overflow-hidden border border-border/40">
            <div
              className={`h-full rounded-full ${
                confidencePct >= 80
                  ? "bg-emerald-500"
                  : confidencePct >= 50
                  ? "bg-amber-500"
                  : "bg-rose-500"
              }`}
              style={{ width: `${confidencePct}%` }}
            />
          </div>
        </div>
      )}

      {/* Timestamp & Session Meta */}
      <div className="space-y-2 p-3 rounded-lg border border-border/50 bg-card/40 font-mono text-[11px]">
        <div className="flex items-center justify-between text-muted-foreground">
          <span className="flex items-center gap-1.5">
            <Clock className="w-3.5 h-3.5 text-muted-foreground/80" />
            <span>Timestamp:</span>
          </span>
          <span className="text-foreground">{formattedDate}</span>
        </div>

        {action.session_id && (
          <div className="flex items-center justify-between text-muted-foreground pt-1 border-t border-border/40">
            <span>Session:</span>
            <span className="text-foreground truncate max-w-[170px]">
              {action.session_id}
            </span>
          </div>
        )}
      </div>

      {/* Causal Chain Links */}
      {(action.parent_action_id || action.supersedes) && (
        <div className="space-y-2 p-3 rounded-lg border border-border/60 bg-card/60">
          <span className="text-[10px] font-semibold text-muted-foreground uppercase tracking-wider flex items-center gap-1.5">
            <GitCommit className="w-3.5 h-3.5 text-primary" />
            <span>Causal Graph Relation</span>
          </span>

          {action.parent_action_id && (
            <div className="space-y-1 pt-1">
              <span className="text-[11px] text-muted-foreground">Parent Action:</span>
              <button
                onClick={() => handleJumpToParent(action.parent_action_id!)}
                className="w-full text-left font-mono text-[11px] text-primary hover:underline truncate bg-background px-2 py-1 rounded border border-border/60 cursor-pointer block"
                title={`Jump to parent: ${action.parent_action_id}`}
              >
                ↑ {action.parent_action_id}
              </button>
            </div>
          )}

          {action.supersedes && (
            <div className="space-y-1 pt-1">
              <span className="text-[11px] text-muted-foreground">Supersedes:</span>
              <button
                onClick={() => handleJumpToParent(action.supersedes!)}
                className="w-full text-left font-mono text-[11px] text-amber-400 hover:underline truncate bg-background px-2 py-1 rounded border border-border/60 cursor-pointer block"
                title={`Jump to superseded: ${action.supersedes}`}
              >
                ⟲ {action.supersedes}
              </button>
            </div>
          )}
        </div>
      )}

      {/* Touched Files List */}
      <div className="space-y-2">
        <div className="flex items-center justify-between">
          <span className="text-[10px] font-semibold text-muted-foreground uppercase tracking-wider flex items-center gap-1.5">
            <FileCode className="w-3.5 h-3.5 text-indigo-400" />
            <span>Touched Files</span>
          </span>
          <span className="text-[10px] font-mono text-muted-foreground bg-card px-1.5 py-0.2 rounded border border-border/50">
            {action.files?.length || 0}
          </span>
        </div>

        {action.files && action.files.length > 0 ? (
          <div className="space-y-1 max-h-40 overflow-y-auto pr-1">
            {action.files.map((file, idx) => (
              <div
                key={idx}
                className="flex items-center gap-2 p-1.5 rounded bg-card/50 hover:bg-card border border-border/50 text-[11px] font-mono text-foreground truncate select-all"
              >
                <FileCode className="w-3 h-3 text-muted-foreground shrink-0" />
                <span className="truncate">{file}</span>
              </div>
            ))}
          </div>
        ) : (
          <p className="text-[11px] text-muted-foreground italic">
            No specific files recorded for this decision.
          </p>
        )}
      </div>

      {/* Tags Chips */}
      {action.tags && action.tags.length > 0 && (
        <div className="space-y-2">
          <span className="text-[10px] font-semibold text-muted-foreground uppercase tracking-wider flex items-center gap-1.5">
            <Tag className="w-3.5 h-3.5 text-primary" />
            <span>Tags & Categories</span>
          </span>

          <div className="flex flex-wrap gap-1.5">
            {action.tags.map((tag, idx) => (
              <span
                key={idx}
                className="text-[10px] font-mono px-2 py-0.5 rounded-full bg-card border border-border text-foreground/90"
              >
                #{tag}
              </span>
            ))}
          </div>
        </div>
      )}

      {/* Raw JSON Collapsible */}
      <div className="pt-2 border-t border-border/50">
        <button
          onClick={() => setShowRawJson(!showRawJson)}
          className="w-full flex items-center justify-between py-1.5 text-muted-foreground hover:text-foreground text-[11px] font-medium transition cursor-pointer"
        >
          <span className="flex items-center gap-1.5">
            <Code2 className="w-3.5 h-3.5 text-muted-foreground" />
            <span>Raw Action Payload (JSON)</span>
          </span>
          {showRawJson ? (
            <ChevronDown className="w-3.5 h-3.5" />
          ) : (
            <ChevronRight className="w-3.5 h-3.5" />
          )}
        </button>

        {showRawJson && (
          <pre className="mt-2 p-3 rounded-lg bg-background border border-border text-[10px] font-mono text-muted-foreground overflow-x-auto max-h-60 leading-relaxed select-all">
            {JSON.stringify(action, null, 2)}
          </pre>
        )}
      </div>
    </div>
  );
};
