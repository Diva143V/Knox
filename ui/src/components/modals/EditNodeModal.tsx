import React, { useState, useEffect } from "react";
import {
  Edit3,
  X,
  Plus,
  Trash2,
  FileCode,
  Tag,
  Zap,
  HelpCircle,
  Link as LinkIcon,
  Save,
} from "lucide-react";
import { KnoxAPI } from "@/services/api";
import { useKnoxStore } from "@/store/useKnoxStore";
import { Button } from "@/components/ui/Button";
import { ActionBadge } from "@/components/ui/ActionBadge";
import { cn } from "@/lib/cn";
import type { ActionRelation, UpdateActionPayload } from "@/types/knox";

const RELATION_TYPES = [
  { value: "depends_on", label: "Depends On", desc: "Requires target to function" },
  { value: "implements", label: "Implements", desc: "Fulfills architectural decision" },
  { value: "verifies", label: "Verifies", desc: "Confirms correctness or passes tests" },
  { value: "relates_to", label: "Relates To", desc: "General contextual link" },
  { value: "reverts", label: "Reverts", desc: "Undoes or counteracts target" },
  { value: "resolves", label: "Resolves", desc: "Answers or settles an open doubt" },
];

const ACTION_TYPES = [
  "write",
  "decide",
  "plan",
  "debug",
  "doubt",
  "change",
  "review",
  "read",
];

export const EditNodeModal: React.FC = () => {
  const {
    isEditNodeOpen,
    editingAction,
    closeEditNode,
    updateActionInStore,
    showToast,
    actions,
  } = useKnoxStore();

  const [summary, setSummary] = useState("");
  const [actionType, setActionType] = useState("write");
  const [significance, setSignificance] = useState("minor");
  const [rationale, setRationale] = useState("");
  const [contextSummary, setContextSummary] = useState("");
  const [files, setFiles] = useState<string[]>([]);
  const [newFile, setNewFile] = useState("");
  const [tags, setTags] = useState<string[]>([]);
  const [newTag, setNewTag] = useState("");
  const [symbols, setSymbols] = useState<string[]>([]);
  const [newSymbol, setNewSymbol] = useState("");

  // Doubt resolution fields
  const [isResolved, setIsResolved] = useState(true);
  const [resolutionNote, setResolutionNote] = useState("");

  // Relations manager
  const [relations, setRelations] = useState<ActionRelation[]>([]);
  const [newRelTargetId, setNewRelTargetId] = useState("");
  const [newRelType, setNewRelType] = useState("depends_on");

  const [isSubmitting, setIsSubmitting] = useState(false);

  useEffect(() => {
    if (editingAction) {
      setSummary(editingAction.summary || "");
      setActionType(editingAction.action_type || "write");
      setSignificance(editingAction.significance || "minor");
      setRationale(editingAction.rationale || "");
      setContextSummary(editingAction.context_summary || "");
      setFiles(editingAction.files ? [...editingAction.files] : []);
      setTags(editingAction.tags ? [...editingAction.tags] : []);
      setSymbols(editingAction.symbols ? [...editingAction.symbols] : []);
      setIsResolved(editingAction.is_resolved ?? true);
      setResolutionNote(editingAction.resolution_note || "");
      setRelations(editingAction.relations ? [...editingAction.relations] : []);
      setNewFile("");
      setNewTag("");
      setNewSymbol("");
      setNewRelTargetId("");
      setNewRelType("depends_on");
    }
  }, [editingAction]);

  if (!isEditNodeOpen || !editingAction) return null;

  const handleAddFile = () => {
    const trimmed = newFile.trim();
    if (trimmed && !files.includes(trimmed)) {
      setFiles([...files, trimmed]);
      setNewFile("");
    }
  };

  const handleRemoveFile = (f: string) => {
    setFiles(files.filter((item) => item !== f));
  };

  const handleAddTag = () => {
    const trimmed = newTag.trim();
    if (trimmed && !tags.includes(trimmed)) {
      setTags([...tags, trimmed]);
      setNewTag("");
    }
  };

  const handleRemoveTag = (t: string) => {
    setTags(tags.filter((item) => item !== t));
  };

  const handleAddSymbol = () => {
    const trimmed = newSymbol.trim();
    if (trimmed && !symbols.includes(trimmed)) {
      setSymbols([...symbols, trimmed]);
      setNewSymbol("");
    }
  };

  const handleRemoveSymbol = (s: string) => {
    setSymbols(symbols.filter((item) => item !== s));
  };

  const handleAddRelation = () => {
    if (!newRelTargetId) return;
    if (relations.some((r) => r.target_id === newRelTargetId && r.relation_type === newRelType)) {
      return;
    }
    setRelations([...relations, { target_id: newRelTargetId, relation_type: newRelType }]);
    setNewRelTargetId("");
  };

  const handleRemoveRelation = (targetId: string, relType: string) => {
    setRelations(relations.filter((r) => !(r.target_id === targetId && r.relation_type === relType)));
  };

  const handleSubmit = async (e?: React.FormEvent) => {
    if (e) e.preventDefault();
    const trimmedSummary = summary.trim();
    if (!trimmedSummary) {
      showToast("Summary cannot be empty", true);
      return;
    }

    try {
      setIsSubmitting(true);
      const payload: UpdateActionPayload = {
        id: editingAction.id,
        summary: trimmedSummary,
        action_type: actionType,
        significance,
        rationale: rationale.trim() || null,
        context_summary: contextSummary.trim() || null,
        files,
        tags,
        symbols,
        is_resolved: isResolved,
        resolution_note: resolutionNote.trim() || null,
        relations,
      };

      const updated = await KnoxAPI.updateAction(payload);
      updateActionInStore(updated);
      showToast("Node and relations updated successfully", false);
      closeEditNode();
    } catch (err) {
      console.error("Failed to update node:", err);
      showToast(String(err), true);
    } finally {
      setIsSubmitting(false);
    }
  };

  const otherActions = actions.filter((a) => a.id !== editingAction.id);

  return (
    <div
      role="dialog"
      aria-modal="true"
      aria-label="Edit Knowledge Node"
      onKeyDown={(e) => {
        if (e.key === "Escape") closeEditNode();
        if (e.key === "Enter" && (e.ctrlKey || e.metaKey)) {
          e.preventDefault();
          handleSubmit();
        }
      }}
      className="fixed inset-0 z-50 bg-black/60 backdrop-blur-xs flex items-center justify-center p-3 sm:p-5 animate-in fade-in duration-150"
    >
      <div className="bg-card border border-border rounded-xl shadow-2xl max-w-2xl w-full max-h-[90vh] flex flex-col overflow-hidden text-foreground">
        {/* Header */}
        <div className="flex items-center justify-between px-5 py-4 border-b border-border bg-muted/20 shrink-0">
          <div className="flex items-center gap-2.5">
            <div className="p-2 rounded-lg bg-primary/10 text-primary border border-primary/20">
              <Edit3 className="size-4" />
            </div>
            <div>
              <h2 className="text-base font-semibold text-foreground flex items-center gap-2">
                Edit Knowledge Node
                <ActionBadge type={actionType} className="text-[10px] px-1.5 py-0" />
              </h2>
              <p className="text-xs text-muted-foreground font-mono truncate max-w-md">
                ID: {editingAction.id}
              </p>
            </div>
          </div>
          <button
            onClick={closeEditNode}
            className="text-muted-foreground hover:text-foreground p-1.5 rounded-md hover:bg-muted transition-colors cursor-pointer"
            aria-label="Close modal"
          >
            <X className="size-4" />
          </button>
        </div>

        {/* Scrollable Form Content */}
        <form onSubmit={handleSubmit} className="flex-1 overflow-y-auto p-5 space-y-4">
          {/* Summary Input */}
          <div className="space-y-1.5">
            <label className="text-xs font-semibold uppercase tracking-wider text-muted-foreground">
              Summary <span className="text-danger">*</span>
            </label>
            <textarea
              value={summary}
              onChange={(e) => setSummary(e.target.value)}
              rows={2}
              required
              className="w-full bg-background border border-border rounded-lg p-2.5 text-sm text-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-focus-ring resize-none"
              placeholder="What this action accomplishes..."
            />
          </div>

          {/* Action Type & Significance Row */}
          <div className="grid grid-cols-1 sm:grid-cols-2 gap-3">
            <div className="space-y-1.5">
              <label className="text-xs font-semibold uppercase tracking-wider text-muted-foreground">
                Action Type
              </label>
              <select
                value={actionType}
                onChange={(e) => setActionType(e.target.value)}
                className="w-full h-9 bg-background border border-border rounded-lg px-2.5 text-sm text-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-focus-ring cursor-pointer font-mono"
              >
                {ACTION_TYPES.map((t) => (
                  <option key={t} value={t}>
                    {t}
                  </option>
                ))}
              </select>
            </div>

            <div className="space-y-1.5">
              <label className="text-xs font-semibold uppercase tracking-wider text-muted-foreground">
                Significance
              </label>
              <div className="flex gap-2 h-9 items-center">
                <button
                  type="button"
                  onClick={() => setSignificance("minor")}
                  className={cn(
                    "flex-1 h-full rounded-lg text-xs font-medium border transition-colors cursor-pointer flex items-center justify-center gap-1.5",
                    significance === "minor"
                      ? "bg-muted text-foreground border-border font-semibold shadow-xs"
                      : "bg-background text-muted-foreground border-border hover:bg-muted/40"
                  )}
                >
                  Minor (Code/Detail)
                </button>
                <button
                  type="button"
                  onClick={() => setSignificance("major")}
                  className={cn(
                    "flex-1 h-full rounded-lg text-xs font-medium border transition-colors cursor-pointer flex items-center justify-center gap-1.5",
                    significance === "major"
                      ? "bg-warning/15 text-warning border-warning/40 font-semibold shadow-xs"
                      : "bg-background text-muted-foreground border-border hover:bg-muted/40"
                  )}
                >
                  Major (ADR Decision)
                </button>
              </div>
            </div>
          </div>

          {/* Rationale Textarea */}
          <div className="space-y-1.5">
            <label className="text-xs font-semibold uppercase tracking-wider text-muted-foreground">
              Architectural Rationale / Justification
            </label>
            <textarea
              value={rationale}
              onChange={(e) => setRationale(e.target.value)}
              rows={2}
              className="w-full bg-background border border-border rounded-lg p-2.5 text-xs text-foreground font-mono focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-focus-ring"
              placeholder="Why this choice was made, constraints, alternatives evaluated..."
            />
          </div>

          {/* Context Summary */}
          <div className="space-y-1.5">
            <label className="text-xs font-semibold uppercase tracking-wider text-muted-foreground">
              Context Summary
            </label>
            <input
              type="text"
              value={contextSummary}
              onChange={(e) => setContextSummary(e.target.value)}
              className="w-full h-8 bg-background border border-border rounded-lg px-2.5 text-xs text-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-focus-ring"
              placeholder="Active task context or execution background..."
            />
          </div>

          {/* Doubt Resolution Section */}
          {(actionType === "doubt" || editingAction.action_type === "doubt" || editingAction.is_resolved !== undefined) && (
            <div className="p-3 rounded-lg border border-border/80 bg-muted/30 space-y-2.5">
              <div className="flex items-center justify-between">
                <span className="text-xs font-semibold uppercase tracking-wider text-muted-foreground flex items-center gap-1.5">
                  <HelpCircle className="size-3.5 text-warning" /> Doubt Resolution Status
                </span>
                <label className="flex items-center gap-2 cursor-pointer select-none">
                  <input
                    type="checkbox"
                    checked={isResolved}
                    onChange={(e) => setIsResolved(e.target.checked)}
                    className="rounded border-border accent-primary cursor-pointer"
                  />
                  <span className="text-xs font-medium text-foreground">
                    {isResolved ? "Resolved" : "Open Doubt"}
                  </span>
                </label>
              </div>
              <input
                type="text"
                value={resolutionNote}
                onChange={(e) => setResolutionNote(e.target.value)}
                placeholder="Resolution note: how this question was answered or settled..."
                className="w-full h-8 bg-background border border-border rounded-lg px-2.5 text-xs text-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-focus-ring"
              />
            </div>
          )}

          {/* Outgoing Semantic Relations Section */}
          <div className="p-3.5 rounded-lg border border-border/80 bg-muted/20 space-y-3">
            <div className="flex items-center justify-between">
              <span className="text-xs font-semibold uppercase tracking-wider text-muted-foreground flex items-center gap-1.5">
                <LinkIcon className="size-3.5 text-primary" /> Outgoing Semantic Relations ({relations.length})
              </span>
            </div>

            {/* List of current relations */}
            {relations.length > 0 ? (
              <div className="space-y-1.5 max-h-36 overflow-y-auto pr-1">
                {relations.map((rel, idx) => {
                  const targetAction = actions.find((a) => a.id === rel.target_id);
                  return (
                    <div
                      key={`${rel.target_id}-${rel.relation_type}-${idx}`}
                      className="flex items-center justify-between gap-2 p-2 rounded-md bg-background border border-border text-xs"
                    >
                      <div className="flex items-center gap-2 min-w-0">
                        <span className="font-mono text-[10px] uppercase font-semibold text-primary px-1.5 py-0.5 rounded-xs bg-primary/10 border border-primary/20 shrink-0">
                          {rel.relation_type}
                        </span>
                        <span className="text-muted-foreground shrink-0 font-mono text-[11px]">
                          →
                        </span>
                        <span className="truncate text-foreground font-medium" title={targetAction?.summary || rel.target_id}>
                          {targetAction ? targetAction.summary : rel.target_id}
                        </span>
                      </div>
                      <button
                        type="button"
                        onClick={() => handleRemoveRelation(rel.target_id, rel.relation_type)}
                        className="text-muted-foreground hover:text-danger p-1 rounded hover:bg-muted transition-colors cursor-pointer shrink-0"
                        title="Remove relation"
                      >
                        <Trash2 className="size-3" />
                      </button>
                    </div>
                  );
                })}
              </div>
            ) : (
              <p className="text-xs text-muted-foreground italic">No outgoing semantic relations attached.</p>
            )}

            {/* Add new relation row */}
            <div className="flex flex-wrap sm:flex-nowrap items-center gap-2 pt-1 border-t border-border/50">
              <select
                value={newRelType}
                onChange={(e) => setNewRelType(e.target.value)}
                className="h-8 bg-background border border-border rounded-md px-2 text-xs text-foreground focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-focus-ring cursor-pointer font-mono shrink-0"
              >
                {RELATION_TYPES.map((rt) => (
                  <option key={rt.value} value={rt.value}>
                    {rt.label}
                  </option>
                ))}
              </select>

              <select
                value={newRelTargetId}
                onChange={(e) => setNewRelTargetId(e.target.value)}
                className="flex-1 h-8 bg-background border border-border rounded-md px-2 text-xs text-foreground focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-focus-ring cursor-pointer truncate"
              >
                <option value="">Select target action node…</option>
                {otherActions.map((a) => (
                  <option key={a.id} value={a.id}>
                    [{a.action_type.toUpperCase()}] {a.summary.slice(0, 50)}
                  </option>
                ))}
              </select>

              <Button
                type="button"
                variant="secondary"
                size="sm"
                onClick={handleAddRelation}
                disabled={!newRelTargetId}
                className="h-8 shrink-0 text-xs gap-1"
              >
                <Plus className="size-3" /> Add Link
              </Button>
            </div>
          </div>

          {/* Files Touched Section */}
          <div className="space-y-2">
            <label className="text-xs font-semibold uppercase tracking-wider text-muted-foreground flex items-center gap-1.5">
              <FileCode className="size-3 text-plan" /> Touched Files ({files.length})
            </label>
            <div className="flex flex-wrap gap-1.5">
              {files.map((file) => (
                <span
                  key={file}
                  className="inline-flex items-center gap-1 font-mono text-xs bg-muted/60 text-foreground border border-border px-2 py-0.5 rounded-md"
                >
                  <span className="truncate max-w-[200px]">{file}</span>
                  <button
                    type="button"
                    onClick={() => handleRemoveFile(file)}
                    className="text-muted-foreground hover:text-danger cursor-pointer"
                  >
                    <X className="size-3" />
                  </button>
                </span>
              ))}
            </div>
            <div className="flex items-center gap-2">
              <input
                type="text"
                value={newFile}
                onChange={(e) => setNewFile(e.target.value)}
                onKeyDown={(e) => {
                  if (e.key === "Enter") {
                    e.preventDefault();
                    handleAddFile();
                  }
                }}
                placeholder="src/core/types.rs (press Enter to add)"
                className="flex-1 h-8 bg-background border border-border rounded-lg px-2.5 text-xs font-mono text-foreground focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-focus-ring"
              />
              <Button type="button" variant="ghost" size="sm" onClick={handleAddFile} className="h-8 text-xs">
                <Plus className="size-3" /> Add
              </Button>
            </div>
          </div>

          {/* Code Symbols Section */}
          <div className="space-y-2">
            <label className="text-xs font-semibold uppercase tracking-wider text-muted-foreground flex items-center gap-1.5">
              <Zap className="size-3 text-warning" /> Code Symbols ({symbols.length})
            </label>
            <div className="flex flex-wrap gap-1.5">
              {symbols.map((sym) => (
                <span
                  key={sym}
                  className="inline-flex items-center gap-1 font-mono text-xs bg-warning/10 text-warning border border-warning/30 px-2 py-0.5 rounded-md"
                >
                  <span>{sym}</span>
                  <button
                    type="button"
                    onClick={() => handleRemoveSymbol(sym)}
                    className="text-muted-foreground hover:text-danger cursor-pointer"
                  >
                    <X className="size-3" />
                  </button>
                </span>
              ))}
            </div>
            <div className="flex items-center gap-2">
              <input
                type="text"
                value={newSymbol}
                onChange={(e) => setNewSymbol(e.target.value)}
                onKeyDown={(e) => {
                  if (e.key === "Enter") {
                    e.preventDefault();
                    handleAddSymbol();
                  }
                }}
                placeholder="fn_name, StructName (press Enter to add)"
                className="flex-1 h-8 bg-background border border-border rounded-lg px-2.5 text-xs font-mono text-foreground focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-focus-ring"
              />
              <Button type="button" variant="ghost" size="sm" onClick={handleAddSymbol} className="h-8 text-xs">
                <Plus className="size-3" /> Add
              </Button>
            </div>
          </div>

          {/* Tags Section */}
          <div className="space-y-2">
            <label className="text-xs font-semibold uppercase tracking-wider text-muted-foreground flex items-center gap-1.5">
              <Tag className="size-3 text-primary" /> Tags ({tags.length})
            </label>
            <div className="flex flex-wrap gap-1.5">
              {tags.map((tag) => (
                <span
                  key={tag}
                  className="inline-flex items-center gap-1 font-mono text-xs bg-primary/10 text-primary border border-primary/30 px-2 py-0.5 rounded-md"
                >
                  <span>{tag}</span>
                  <button
                    type="button"
                    onClick={() => handleRemoveTag(tag)}
                    className="text-muted-foreground hover:text-danger cursor-pointer"
                  >
                    <X className="size-3" />
                  </button>
                </span>
              ))}
            </div>
            <div className="flex items-center gap-2">
              <input
                type="text"
                value={newTag}
                onChange={(e) => setNewTag(e.target.value)}
                onKeyDown={(e) => {
                  if (e.key === "Enter") {
                    e.preventDefault();
                    handleAddTag();
                  }
                }}
                placeholder="architecture, mcp, refactor (press Enter to add)"
                className="flex-1 h-8 bg-background border border-border rounded-lg px-2.5 text-xs font-mono text-foreground focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-focus-ring"
              />
              <Button type="button" variant="ghost" size="sm" onClick={handleAddTag} className="h-8 text-xs">
                <Plus className="size-3" /> Add
              </Button>
            </div>
          </div>
        </form>

        {/* Footer */}
        <div className="flex items-center justify-between px-5 py-3.5 border-t border-border bg-muted/20 shrink-0">
          <span className="text-[11px] text-muted-foreground font-mono">
            Press <kbd className="px-1 py-0.5 bg-muted rounded border border-border">Ctrl+Enter</kbd> to save
          </span>
          <div className="flex items-center gap-2">
            <Button
              type="button"
              variant="ghost"
              onClick={closeEditNode}
              disabled={isSubmitting}
            >
              Cancel
            </Button>
            <Button
              type="button"
              variant="primary"
              onClick={() => handleSubmit()}
              disabled={isSubmitting}
              className="gap-1.5"
            >
              <Save className="size-3.5" />
              {isSubmitting ? "Saving…" : "Save Changes"}
            </Button>
          </div>
        </div>
      </div>
    </div>
  );
};
