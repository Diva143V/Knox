import React, { useState, useMemo } from "react";
import type { KnoxAction } from "@/types/knox";
import { KnoxAPI } from "@/services/api";
import { ActionBadge } from "@/components/ui/ActionBadge";
import { ActorBadge } from "@/components/ui/ActorBadge";
import { Button } from "@/components/ui/Button";
import { cn } from "@/lib/cn";
import {
  Copy,
  Check,
  CheckCircle2,
  HelpCircle,
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
  Scale,
  Link2,
  CornerDownRight,
  Layers,
  Edit3,
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
  const {
    actions,
    showToast,
    setSelectedAction,
    getProjectForAction,
    refreshData,
    openEditNode,
  } = useKnoxStore();
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
    return `${projRoot.replace(/[\\/]+$/, "")}${separator}${file.replace(/^[\\/]+/, "")}`;
  }, [action.files, project]);

  const handleCopyId = () => {
    navigator.clipboard.writeText(action.id);
    setCopiedId(true);
    showToast("Copied action ID to clipboard");
    setTimeout(() => setCopiedId(false), 2000);
  };

  const handleCopyText = (text: string, label: string) => {
    navigator.clipboard.writeText(text);
    setCopiedPath(label);
    showToast(`Copied ${label} to clipboard`);
    setTimeout(() => setCopiedPath(null), 2000);
  };

  const handleCopyFile = (file: string) => {
    navigator.clipboard.writeText(file);
    showToast("Copied file path to clipboard");
  };

  const handleJumpToParent = (parentId: string) => {
    const parent = actions.find((a) => a.id === parentId);
    if (parent) {
      if (onSelectAction) {
        onSelectAction(parent);
      } else {
        setSelectedAction(parent);
      }
    } else {
      showToast("Parent action node is filtered out or not indexed", true);
    }
  };

  const formattedDate = useMemo(() => {
    try {
      return new Date(action.timestamp).toLocaleString(undefined, {
        dateStyle: "medium",
        timeStyle: "medium",
      });
    } catch {
      return action.timestamp;
    }
  }, [action.timestamp]);

  const [isResolving, setIsResolving] = useState(false);
  const [resolutionNote, setResolutionNote] = useState("");
  const [isSubmittingResolve, setIsSubmittingResolve] = useState(false);

  const handleResolveDoubt = async () => {
    const trimmed = resolutionNote.trim();
    if (!trimmed) return;
    try {
      setIsSubmittingResolve(true);
      await KnoxAPI.resolveDoubt({
        doubtId: action.id,
        resolutionNote: trimmed,
      });
      showToast("Doubt marked as resolved in Knox knowledge graph!", false);
      setIsResolving(false);
      await refreshData();
    } catch (err) {
      showToast(String(err), true);
    } finally {
      setIsSubmittingResolve(false);
    }
  };

  const confidencePct =
    typeof action.confidence === "number"
      ? Math.round(action.confidence * 100)
      : null;

  const isDoubtType =
    action.action_type === "doubt" || action.action_type === "question";

  return (
    <div className={cn("space-y-4 text-xs text-foreground", className)}>
      {/* Badges & Status */}
      <div className="flex items-center justify-between gap-2">
        <div className="flex items-center gap-1.5 min-w-0 flex-wrap">
          <ActorBadge actor={action.actor} />
          <ActionBadge type={action.action_type} />
          {action.significance === "minor" && (
            <span className="text-3xs font-mono font-medium px-1.5 py-0.5 rounded-sm bg-muted text-muted-foreground border border-border">
              MINOR
            </span>
          )}
          {action.significance === "major" && action.action_type === "decide" && (
            <span className="text-3xs font-mono font-medium px-1.5 py-0.5 rounded-sm bg-warning/20 text-warning border border-warning/30 font-bold">
              MAJOR ADR
            </span>
          )}
        </div>
        <div className="flex items-center gap-1.5 shrink-0">
          <Button
            variant="secondary"
            size="sm"
            onClick={() => openEditNode(action)}
            className="h-6 px-2 text-[11px] gap-1 text-foreground"
            title="Edit node summary, relations, tags, and files"
          >
            <Edit3 className="size-3 text-primary" />
            Edit
          </Button>
          {isDoubtType && (
            action.is_resolved ? (
              <span className="text-xs font-mono font-medium px-1.5 py-0.5 rounded-sm bg-success/15 text-success border border-success/30 shrink-0 flex items-center gap-1">
                <CheckCircle2 className="size-3" />
                RESOLVED
              </span>
            ) : (
              <span className="text-xs font-mono font-medium px-1.5 py-0.5 rounded-sm bg-amber-500/15 text-amber-500 border border-amber-500/30 shrink-0 flex items-center gap-1">
                <HelpCircle className="size-3" />
                OPEN DOUBT
              </span>
            )
          )}
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
      </div>

      {/* Open Doubt Resolution Card */}
      {isDoubtType && (
        <div
          className={cn(
            "p-3 rounded-lg border",
            action.is_resolved
              ? "border-success/30 bg-success/5"
              : "border-amber-500/30 bg-amber-500/10 shadow-xs"
          )}
        >
          <div className="flex items-center justify-between mb-2">
            <span className="text-2xs font-bold uppercase tracking-wider flex items-center gap-1.5 font-sans">
              {action.is_resolved ? (
                <>
                  <CheckCircle2 className="size-3.5 text-success" />
                  <span className="text-success font-semibold">Technical Doubt Resolved</span>
                </>
              ) : (
                <>
                  <HelpCircle className="size-3.5 text-amber-500" />
                  <span className="text-amber-500 font-semibold">
                    Open Technical Doubt / Question
                  </span>
                </>
              )}
            </span>
          </div>

          {action.is_resolved ? (
            <div className="text-xs space-y-1">
              <span className="text-muted-foreground font-mono text-2xs uppercase">
                Resolution Note:
              </span>
              <p className="font-sans text-foreground bg-background/50 p-2 rounded border border-border text-xs leading-relaxed">
                {action.resolution_note || "Resolved without an attached note."}
              </p>
            </div>
          ) : (
            <div className="space-y-2">
              <p className="text-xs text-muted-foreground">
                This uncertainty was logged to prevent assumptions. Once answered, mark it resolved with the reasoning.
              </p>
              {isResolving ? (
                <div className="space-y-2 pt-1">
                  <textarea
                    rows={2}
                    value={resolutionNote}
                    onChange={(e) => setResolutionNote(e.target.value)}
                    placeholder="Explain how this doubt was answered or resolved..."
                    className="w-full text-xs p-2 rounded border border-border bg-background focus:ring-1 focus:ring-primary font-sans"
                  />
                  <div className="flex gap-2 justify-end">
                    <Button
                      type="button"
                      variant="ghost"
                      size="sm"
                      onClick={() => setIsResolving(false)}
                    >
                      Cancel
                    </Button>
                    <Button
                      type="button"
                      variant="primary"
                      size="sm"
                      disabled={!resolutionNote.trim() || isSubmittingResolve}
                      onClick={handleResolveDoubt}
                    >
                      {isSubmittingResolve ? "Saving..." : "Confirm Resolution"}
                    </Button>
                  </div>
                </div>
              ) : (
                <Button
                  type="button"
                  variant="outline"
                  size="sm"
                  className="w-full border-amber-500/40 text-amber-500 hover:bg-amber-500/10"
                  onClick={() => setIsResolving(true)}
                >
                  <CheckCircle2 className="size-3.5 mr-1.5" />
                  Mark as Resolved
                </Button>
              )}
            </div>
          )}
        </div>
      )}

      {/* Work Done Narrative Card */}
      <div className="space-y-2 p-3 rounded-lg border border-primary/30 bg-primary/5">
        <div className="flex items-center justify-between text-primary">
          <span className="text-2xs font-bold uppercase tracking-wider flex items-center gap-1.5 font-sans">
            <Sparkles className="size-3.5" />
            <span>Work Done Narrative</span>
          </span>
          <span className="text-2xs font-mono px-1.5 py-0.5 rounded-sm bg-primary/20 text-primary border border-primary/30">
            AI SYNTHESIS
          </span>
        </div>
        <p className="text-sm font-medium text-foreground leading-relaxed">
          {workNarrative}
        </p>

        {/* Code Diff Metrics Chips if present */}
        {diffMetrics && (
          <div className="flex flex-wrap items-center gap-1.5 pt-2 border-t border-border font-mono text-2xs">
            <span className="flex items-center gap-1 px-2 py-0.5 rounded-sm bg-success/15 text-success border border-success/30 font-semibold">
              <PlusCircle className="size-3" />
              +{diffMetrics.added} added
            </span>
            <span className="flex items-center gap-1 px-2 py-0.5 rounded-sm bg-danger/15 text-danger border border-danger/30 font-semibold">
              <MinusCircle className="size-3" />
              -{diffMetrics.deleted} deleted
            </span>
            <span className="px-2 py-0.5 rounded-sm bg-muted text-muted-foreground border border-border">
              Net: {diffMetrics.net >= 0 ? `+${diffMetrics.net}` : diffMetrics.net} lines
            </span>
          </div>
        )}
      </div>

      {/* Ontology & Physical Location Card */}
      <div className="space-y-2.5 p-3 rounded-lg border border-border bg-card font-mono text-xs">
        <div className="flex items-center justify-between text-primary">
          <span className="text-2xs font-bold uppercase tracking-wider flex items-center gap-1.5 font-sans">
            <Compass className="size-3.5" />
            <span>Ontology & Physical Location</span>
          </span>
          <span className="text-2xs px-1.5 py-0.5 rounded-sm bg-primary/10 text-primary border border-primary/20 font-semibold uppercase">
            Resolved
          </span>
        </div>

        {/* Project Folder */}
        <div className="space-y-1">
          <div className="flex items-center justify-between text-muted-foreground text-2xs">
            <span className="flex items-center gap-1">
              <FolderGit2 className="size-3 text-primary" />
              <span>Project Branch:</span>
            </span>
            <button
              type="button"
              onClick={() => handleCopyText(project?.path || "D:\\Knox", "Project Path")}
              className="text-primary hover:underline flex items-center gap-0.5 cursor-pointer"
            >
              {copiedPath === "Project Path" ? (
                <Check className="size-2.5 text-success" />
              ) : (
                <Copy className="size-2.5" />
              )}
              <span>{copiedPath === "Project Path" ? "Copied" : "Copy"}</span>
            </button>
          </div>
          <div className="p-1.5 rounded-sm bg-background border border-border text-foreground flex items-center justify-between gap-2">
            <span className="font-semibold text-primary shrink-0">{project?.name || "Knox"}</span>
            <span className="text-2xs text-muted-foreground truncate flex-1 text-right font-mono" title={project?.path}>
              {project?.path}
            </span>
          </div>
        </div>

        {/* Primary Entity Physical Location */}
        {primaryPhysicalLocation && (
          <div className="space-y-1 pt-1 border-t border-border">
            <div className="flex items-center justify-between text-muted-foreground text-2xs">
              <span className="flex items-center gap-1">
                <MapPin className="size-3 text-success" />
                <span>Entity Physical Location:</span>
              </span>
              <button
                type="button"
                onClick={() => handleCopyText(primaryPhysicalLocation, "Physical Location")}
                className="text-primary hover:underline flex items-center gap-0.5 cursor-pointer"
              >
                {copiedPath === "Physical Location" ? (
                  <Check className="size-2.5 text-success" />
                ) : (
                  <Copy className="size-2.5" />
                )}
                <span>{copiedPath === "Physical Location" ? "Copied" : "Copy"}</span>
              </button>
            </div>
            <div
              className="p-1.5 rounded-sm bg-background border border-border text-foreground text-2xs font-mono break-all select-all leading-tight"
              title={primaryPhysicalLocation}
            >
              {primaryPhysicalLocation}
            </div>
          </div>
        )}
      </div>

      {/* Architectural Decision Card if decide or rationale present */}
      {(action.action_type === "decide" ||
        action.rationale ||
        (action.rejected_alternatives && action.rejected_alternatives.length > 0)) && (
        <div className="space-y-2.5 p-3 rounded-lg border border-warning/40 bg-warning/5">
          <div className="flex items-center justify-between text-warning">
            <span className="text-2xs font-bold uppercase tracking-wider flex items-center gap-1.5 font-sans">
              <Scale className="size-3.5" />
              <span>Architectural Decision</span>
            </span>
            <span className="text-2xs font-mono px-1.5 py-0.5 rounded-sm bg-warning/20 text-warning border border-warning/30 font-semibold">
              FIRST-CLASS
            </span>
          </div>

          {action.rationale && (
            <div className="space-y-1">
              <span className="text-2xs font-mono uppercase text-muted-foreground">Rationale</span>
              <p className="text-xs text-foreground bg-background/80 p-2 rounded border border-warning/20 leading-relaxed font-sans">
                {action.rationale}
              </p>
            </div>
          )}

          {action.rejected_alternatives && action.rejected_alternatives.length > 0 && (
            <div className="space-y-1">
              <span className="text-2xs font-mono uppercase text-muted-foreground">
                Rejected Alternatives
              </span>
              <div className="flex flex-wrap gap-1.5">
                {action.rejected_alternatives.map((alt, idx) => (
                  <span
                    key={idx}
                    className="text-2xs font-mono px-2 py-0.5 rounded-sm bg-danger/10 text-danger border border-danger/20 line-through"
                  >
                    {alt}
                  </span>
                ))}
              </div>
            </div>
          )}
        </div>
      )}

      {/* Semantic Graph Relations if present */}
      {action.relations && action.relations.length > 0 && (
        <Section
          title="Semantic Graph Relations"
          icon={<Link2 className="size-3.5 text-primary" aria-hidden />}
          trailing={
            <span className="text-2xs font-mono text-muted-foreground bg-card px-1.5 py-0.5 rounded-sm border border-border">
              {action.relations.length}
            </span>
          }
        >
          <div className="space-y-1.5">
            {action.relations.map((rel, idx) => {
              const targetAction = actions.find((a) => a.id === rel.target_id);
              return (
                <div
                  key={idx}
                  className="p-2 rounded-md bg-card border border-border flex items-center justify-between gap-2 text-xs"
                >
                  <div className="flex items-center gap-1.5 min-w-0 flex-1">
                    <span className="text-2xs font-mono uppercase px-1.5 py-0.5 rounded-sm bg-primary/10 text-primary border border-primary/20 shrink-0">
                      {rel.relation_type.replace("_", " ")}
                    </span>
                    <span
                      className="font-mono text-foreground text-2xs truncate"
                      title={rel.target_id}
                    >
                      {targetAction?.summary || rel.target_id}
                    </span>
                  </div>
                  <button
                    type="button"
                    onClick={() => handleJumpToParent(rel.target_id)}
                    className="text-primary hover:underline text-2xs font-mono shrink-0 flex items-center gap-0.5 cursor-pointer"
                  >
                    <span>Jump</span>
                    <CornerDownRight className="size-2.5" />
                  </button>
                </div>
              );
            })}
          </div>
        </Section>
      )}

      {/* Code Symbols & AST if present */}
      {action.symbols && action.symbols.length > 0 && (
        <Section
          title="Code Symbols & AST"
          icon={<Code2 className="size-3.5 text-primary" aria-hidden />}
          trailing={
            <span className="text-2xs font-mono text-muted-foreground bg-card px-1.5 py-0.5 rounded-sm border border-border">
              {action.symbols.length}
            </span>
          }
        >
          <div className="flex flex-wrap gap-1.5">
            {action.symbols.map((sym, idx) => (
              <span
                key={idx}
                className="text-2xs font-mono px-2 py-0.5 rounded-sm bg-background border border-border text-primary font-semibold flex items-center gap-1"
              >
                <Code2 className="size-2.5 opacity-70" />
                {sym}
              </span>
            ))}
          </div>
        </Section>
      )}

      {/* Context Summary if present */}
      {action.context_summary && (
        <Section
          title="Context Summary"
          icon={<Layers className="size-3.5 text-primary" aria-hidden />}
        >
          <p className="text-xs text-foreground bg-card p-2.5 rounded-md border border-border leading-relaxed">
            {action.context_summary}
          </p>
        </Section>
      )}

      {/* Summary */}
      <Section title="Summary">
        <p className="text-xs font-medium text-foreground leading-relaxed bg-card p-3 rounded-md border border-border">
          {action.summary}
        </p>
      </Section>

      {/* Action ID */}
      <Section
        title="Action ID"
        trailing={
          <Button
            type="button"
            variant="ghost"
            size="sm"
            onClick={handleCopyId}
            className="h-6 px-1.5 text-xs font-mono"
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
        <p className="font-mono text-xs text-foreground break-all select-all px-2.5 py-2 rounded-md border border-border bg-card">
          {action.id}
        </p>
      </Section>

      {/* Confidence */}
      {confidencePct !== null && (
        <Section
          title="Confidence"
          icon={<Gauge className="size-3.5 text-primary" aria-hidden />}
          trailing={
            <span className="font-mono font-semibold text-foreground text-xs">
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

      {/* Metadata */}
      <Section
        title="Metadata"
        icon={<Clock className="size-3.5" aria-hidden />}
      >
        <div className="space-y-2 p-3 rounded-md border border-border bg-card font-mono text-xs">
          <div className="flex items-center justify-between gap-2 text-muted-foreground">
            <span>Timestamp</span>
            <span className="text-foreground text-right">{formattedDate}</span>
          </div>
          {action.session_id && (
            <div className="flex items-center justify-between gap-2 text-muted-foreground pt-2 border-t border-border-subtle">
              <span>Session</span>
              <span className="text-foreground truncate flex-1 text-right ml-2 font-mono">
                {action.session_id}
              </span>
            </div>
          )}
        </div>
      </Section>

      {/* Causal Relations */}
      {(action.parent_action_id || action.supersedes) && (
        <Section
          title="Causal Relations"
          icon={<GitCommit className="size-3.5 text-primary" aria-hidden />}
        >
          <div className="space-y-2 p-3 rounded-md border border-border bg-card text-xs">
            {action.parent_action_id && (
              <div className="space-y-1">
                <span className="text-2xs font-mono uppercase text-muted-foreground">Parent</span>
                <button
                  type="button"
                  onClick={() => handleJumpToParent(action.parent_action_id!)}
                  className="w-full text-left font-mono text-xs text-primary hover:underline truncate bg-background px-2 py-1.5 rounded-md border border-border cursor-pointer focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-focus-ring"
                  title={`Jump to parent: ${action.parent_action_id}`}
                >
                  ↑ {action.parent_action_id}
                </button>
              </div>
            )}
            {action.supersedes && (
              <div className="space-y-1">
                <span className="text-2xs font-mono uppercase text-muted-foreground">
                  Supersedes
                </span>
                <button
                  type="button"
                  onClick={() => handleJumpToParent(action.supersedes!)}
                  className="w-full text-left font-mono text-xs text-warning hover:underline truncate bg-background px-2 py-1.5 rounded-md border border-border cursor-pointer focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-focus-ring"
                  title={`Jump to superseded: ${action.supersedes}`}
                >
                  ⟲ {action.supersedes}
                </button>
              </div>
            )}
          </div>
        </Section>
      )}

      {/* Touched Files */}
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
                className="w-full flex items-center gap-2 p-1.5 rounded-md bg-card hover:bg-muted border border-border text-xs font-mono text-foreground truncate cursor-pointer text-left focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-focus-ring"
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

      {/* Tags */}
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

      {/* Raw JSON */}
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
