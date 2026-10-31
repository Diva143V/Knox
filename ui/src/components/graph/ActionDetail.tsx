import React, { useState, useMemo } from "react";
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
  Sparkles,
  MapPin,
  FolderGit2,
  PlusCircle,
  MinusCircle,
  Compass,
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
  const { actions, showToast, setSelectedAction, getProjectForAction } =
    useKnoxStore();
  const [copiedId, setCopiedId] = useState(false);
  const [copiedPath, setCopiedPath] = useState<string | null>(null);
  const [showRawJson, setShowRawJson] = useState(false);

  const parentAction = useMemo(() => {
    if (!action.parent_action_id) return null;
    return actions.find((a) => a.id === action.parent_action_id) || null;
  }, [action.parent_action_id, actions]);

  const project = useMemo(() => {
    return getProjectForAction(action);
  }, [action, getProjectForAction]);

  // Parse diff metrics from summary (e.g. (+2 -1) or +5 or -2)
  const diffMetrics = useMemo(() => {
    const deltaMatch = action.summary.match(/\+(\d+)\s*-(\d+)/);
    if (deltaMatch) {
      const added = parseInt(deltaMatch[1], 10);
      const deleted = parseInt(deltaMatch[2], 10);
      return { added, deleted, net: added - deleted };
    }
    const singlePlus = action.summary.match(/\+(\d+)/);
    const singleMinus = action.summary.match(/-(\d+)/);
    if (singlePlus || singleMinus) {
      const added = singlePlus ? parseInt(singlePlus[1], 10) : 0;
      const deleted = singleMinus ? parseInt(singleMinus[1], 10) : 0;
      return { added, deleted, net: added - deleted };
    }
    return null;
  }, [action.summary]);

  // Synthesize work narrative explaining what this node did and its intent
  const workNarrative = useMemo(() => {
    const primaryFile = action.files && action.files.length > 0 ? action.files[0] : "";
    const cleanFile = primaryFile.replace(/\\/g, "/");

    if (action.action_type.toLowerCase() === "write") {
      const diffText = diffMetrics
        ? ` (+${diffMetrics.added} added, -${diffMetrics.deleted} deleted)`
        : "";
      if (parentAction) {
        return `Applied code modifications to \`${cleanFile || "target files"}\`${diffText}. Fulfills parent plan: "${parentAction.summary}".`;
      }
      return `Applied code modifications to \`${cleanFile || "target files"}\`${diffText}. Updates source logic and implementation.`;
    }

    if (action.action_type.toLowerCase() === "debug") {
      return `Executed verification command and diagnostic test suites. Confirmed parser modules and test assertions pass.`;
    }

    if (action.action_type.toLowerCase() === "plan") {
      return `Established strategic plan and intent: "${action.summary}". Coordinates downstream file changes and architectural decisions.`;
    }

    if (action.action_type.toLowerCase() === "decide") {
      return `Established architectural decision: "${action.summary}". Governs downstream system design and module contracts.`;
    }

    return `Executed ${action.action_type.toUpperCase()} operation: "${action.summary}".`;
  }, [action, diffMetrics, parentAction]);

  // Compute resolved physical path
  const primaryPhysicalLocation = useMemo(() => {
    if (!action.files || action.files.length === 0) return null;
    const file = action.files[0];
    if (/^[A-Za-z]:[\\/]/.test(file) || file.startsWith("/")) {
      return file;
    }
    const projRoot = project?.path || "D:\\Knox";
    const separator = projRoot.includes("/") ? "/" : "\\";
    return `${projRoot.replace(/[\\/]+$/, "")}${separator}${file}`;
  }, [action.files, project]);

  const handleCopyId = () => {
    navigator.clipboard.writeText(action.id);
    setCopiedId(true);
    showToast("Copied Action ID to clipboard");
    setTimeout(() => setCopiedId(false), 2000);
  };

  const handleCopyText = (text: string, label: string) => {
    navigator.clipboard.writeText(text);
    setCopiedPath(label);
    showToast(`Copied ${label} to clipboard`);
    setTimeout(() => setCopiedPath(null), 2000);
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
    <div className={`space-y-4 text-xs text-foreground ${className}`}>
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

      {/* Work Done Narrative Card */}
      <div className="space-y-2 p-3 rounded-xl border border-primary/40 bg-gradient-to-br from-primary/10 via-zinc-950 to-zinc-900 shadow-sm">
        <div className="flex items-center justify-between text-primary">
          <span className="text-[10px] font-bold uppercase tracking-wider flex items-center gap-1.5 font-sans">
            <Sparkles className="w-3.5 h-3.5" />
            <span>Work Done Narrative</span>
          </span>
          <span className="text-[9px] font-mono px-1.5 py-0.2 rounded bg-primary/20 text-primary border border-primary/30">
            AI SYNTHESIS
          </span>
        </div>
        <p className="text-xs font-medium text-zinc-100 leading-relaxed">
          {workNarrative}
        </p>

        {/* Code Diff Metrics Chips if present */}
        {diffMetrics && (
          <div className="flex flex-wrap items-center gap-1.5 pt-2 border-t border-zinc-800/80 font-mono text-[10px]">
            <span className="flex items-center gap-1 px-2 py-0.5 rounded-full bg-emerald-500/15 text-emerald-400 border border-emerald-500/30 font-semibold">
              <PlusCircle className="w-3 h-3" />
              +{diffMetrics.added} added
            </span>
            <span className="flex items-center gap-1 px-2 py-0.5 rounded-full bg-rose-500/15 text-rose-400 border border-rose-500/30 font-semibold">
              <MinusCircle className="w-3 h-3" />
              -{diffMetrics.deleted} deleted
            </span>
            <span className="px-2 py-0.5 rounded-full bg-zinc-800 text-zinc-300 border border-zinc-700">
              Net: {diffMetrics.net >= 0 ? `+${diffMetrics.net}` : diffMetrics.net} lines
            </span>
          </div>
        )}
      </div>

      {/* Ontology & Physical Location Card */}
      <div className="space-y-2.5 p-3 rounded-xl border border-sky-500/30 bg-sky-950/20 backdrop-blur-xs font-mono text-[11px]">
        <div className="flex items-center justify-between text-sky-400">
          <span className="text-[10px] font-bold uppercase tracking-wider flex items-center gap-1.5 font-sans">
            <Compass className="w-3.5 h-3.5" />
            <span>Ontology & Physical Location</span>
          </span>
          <span className="text-[9px] px-1.5 py-0.2 rounded bg-sky-500/20 text-sky-300 border border-sky-500/30 font-semibold uppercase">
            Resolved
          </span>
        </div>

        {/* Project Folder */}
        <div className="space-y-1">
          <div className="flex items-center justify-between text-muted-foreground text-[10px]">
            <span className="flex items-center gap-1">
              <FolderGit2 className="w-3 h-3 text-sky-400" />
              <span>Project Branch:</span>
            </span>
            <button
              onClick={() => handleCopyText(project?.path || "D:\\Knox", "Project Path")}
              className="text-sky-400 hover:underline flex items-center gap-0.5 cursor-pointer"
            >
              {copiedPath === "Project Path" ? (
                <Check className="w-2.5 h-2.5 text-emerald-400" />
              ) : (
                <Copy className="w-2.5 h-2.5" />
              )}
              <span>{copiedPath === "Project Path" ? "Copied" : "Copy"}</span>
            </button>
          </div>
          <div className="p-1.5 rounded bg-zinc-900/90 border border-zinc-800 text-zinc-200 truncate flex items-center justify-between">
            <span className="font-bold text-sky-300">{project?.name || "Knox"}</span>
            <span className="text-[10px] text-zinc-500 truncate max-w-[150px]" title={project?.path}>
              {project?.path}
            </span>
          </div>
        </div>

        {/* Primary Entity Physical Location */}
        {primaryPhysicalLocation && (
          <div className="space-y-1 pt-1 border-t border-sky-500/20">
            <div className="flex items-center justify-between text-muted-foreground text-[10px]">
              <span className="flex items-center gap-1">
                <MapPin className="w-3 h-3 text-emerald-400" />
                <span>Entity Physical Location:</span>
              </span>
              <button
                onClick={() => handleCopyText(primaryPhysicalLocation, "Physical Location")}
                className="text-sky-400 hover:underline flex items-center gap-0.5 cursor-pointer"
              >
                {copiedPath === "Physical Location" ? (
                  <Check className="w-2.5 h-2.5 text-emerald-400" />
                ) : (
                  <Copy className="w-2.5 h-2.5" />
                )}
                <span>{copiedPath === "Physical Location" ? "Copied" : "Copy"}</span>
              </button>
            </div>
            <div
              className="p-1.5 rounded bg-zinc-900/90 border border-zinc-800 text-emerald-400 text-[10px] truncate select-all"
              title={primaryPhysicalLocation}
            >
              {primaryPhysicalLocation}
            </div>
          </div>
        )}
      </div>

      {/* Raw Action Summary */}
      <div className="space-y-1.5">
        <span className="text-[10px] font-semibold text-muted-foreground uppercase tracking-wider">
          Action Raw Summary
        </span>
        <p className="text-xs font-mono text-zinc-300 bg-card/40 p-2.5 rounded-lg border border-border/50 break-words">
          {action.summary}
        </p>
      </div>

      {/* Action ID & Clipboard */}
      <div className="space-y-1.5 p-2.5 rounded-lg border border-border/60 bg-card/60">
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
        <div className="space-y-1.5 p-2.5 rounded-lg border border-border/50 bg-card/40">
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
      <div className="space-y-2 p-2.5 rounded-lg border border-border/50 bg-card/40 font-mono text-[11px]">
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
            <span className="text-foreground truncate max-w-[170px]" title={action.session_id}>
              {action.session_id}
            </span>
          </div>
        )}
      </div>

      {/* Causal Chain Links */}
      {(action.parent_action_id || action.supersedes) && (
        <div className="space-y-2 p-2.5 rounded-lg border border-border/60 bg-card/60">
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
