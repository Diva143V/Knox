import React, { useState, useEffect, useRef } from "react";
import {
  HelpCircle,
  Sparkles,
  GitCommit,
  MessageSquare,
  X,
  Plus,
  FileCode,
} from "lucide-react";
import { KnoxAPI } from "@/services/api";
import { useKnoxStore } from "@/store/useKnoxStore";
import { Button } from "@/components/ui/Button";
import { cn } from "@/lib/cn";

interface QuickThoughtModalProps {
  isOpen: boolean;
  onClose: () => void;
}

type ThoughtCategory = "doubt" | "minor_decision" | "major_decision" | "thought";

export const QuickThoughtModal: React.FC<QuickThoughtModalProps> = ({
  isOpen,
  onClose,
}) => {
  const { showToast, refreshData } = useKnoxStore();
  const [summary, setSummary] = useState("");
  const [category, setCategory] = useState<ThoughtCategory>("doubt");
  const [contextSummary, setContextSummary] = useState("");
  const [fileInput, setFileInput] = useState("");
  const [isSubmitting, setIsSubmitting] = useState(false);
  const textareaRef = useRef<HTMLTextAreaElement>(null);

  useEffect(() => {
    if (isOpen) {
      setTimeout(() => textareaRef.current?.focus(), 50);
    } else {
      setSummary("");
      setContextSummary("");
      setFileInput("");
      setCategory("doubt");
    }
  }, [isOpen]);

  if (!isOpen) return null;

  const handleSubmit = async (e?: React.FormEvent) => {
    if (e) e.preventDefault();
    const trimmed = summary.trim();
    if (!trimmed) return;

    try {
      setIsSubmitting(true);
      const files = fileInput
        .split(/[,;\s]+/)
        .map((f) => f.trim())
        .filter(Boolean);

      await KnoxAPI.recordQuickThought({
        summary: trimmed,
        category,
        actor: "human",
        contextSummary: contextSummary.trim() || undefined,
        files: files.length > 0 ? files : undefined,
      });

      showToast(
        category === "doubt"
          ? "Open technical doubt recorded! Track and resolve it anytime."
          : "Thought recorded into Knox knowledge graph.",
        false
      );
      await refreshData();
      onClose();
    } catch (err) {
      showToast(String(err), true);
    } finally {
      setIsSubmitting(false);
    }
  };

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === "Escape") {
      onClose();
    } else if (e.key === "Enter" && (e.ctrlKey || e.metaKey)) {
      e.preventDefault();
      handleSubmit();
    }
  };

  const categories: {
    id: ThoughtCategory;
    label: string;
    desc: string;
    icon: React.ReactNode;
    color: string;
    border: string;
  }[] = [
    {
      id: "doubt",
      label: "Open Doubt / Question",
      desc: "Hypothesis, uncertainty, or architectural question to verify",
      icon: <HelpCircle className="size-4" />,
      color: "text-amber-500 bg-amber-500/10",
      border: "border-amber-500/30",
    },
    {
      id: "minor_decision",
      label: "Minor Implementation Choice",
      desc: "CSS tweak, naming choice, or localized code assumption",
      icon: <GitCommit className="size-4" />,
      color: "text-muted-foreground bg-muted",
      border: "border-border",
    },
    {
      id: "major_decision",
      label: "Major ADR Decision",
      desc: "System contract, dependency choice, or architectural boundary",
      icon: <Sparkles className="size-4" />,
      color: "text-warning bg-warning/10",
      border: "border-warning/30",
    },
    {
      id: "thought",
      label: "Dialogue / Project Note",
      desc: "User rationale, prompt context, or exploration log",
      icon: <MessageSquare className="size-4" />,
      color: "text-cyan-500 bg-cyan-500/10",
      border: "border-cyan-500/30",
    },
  ];

  return (
    <div
      role="dialog"
      aria-modal="true"
      aria-label="Record Cognitive Thought or Doubt"
      onKeyDown={handleKeyDown}
      className="fixed inset-0 z-50 bg-black/60 backdrop-blur-xs flex items-center justify-center p-4 animate-in fade-in duration-150"
    >
      <div className="bg-card border border-border shadow-2xl rounded-2xl w-full max-w-lg overflow-hidden flex flex-col animate-in zoom-in-95 duration-150">
        {/* Header */}
        <div className="px-5 py-4 border-b border-border flex items-center justify-between shrink-0">
          <div className="flex items-center gap-2.5">
            <div className="p-1.5 rounded-lg bg-primary/10 text-primary border border-primary/20">
              <Plus className="size-4" />
            </div>
            <div>
              <h2 className="text-sm font-semibold text-foreground">
                Capture Thought / Doubt
              </h2>
              <p className="text-xs text-muted-foreground">
                Continuous cognitive memory logging for Knox
              </p>
            </div>
          </div>
          <Button
            type="button"
            variant="ghost"
            size="icon"
            onClick={onClose}
            className="size-7"
            aria-label="Close dialog"
          >
            <X className="size-4" />
          </Button>
        </div>

        {/* Form Body */}
        <form onSubmit={handleSubmit} className="p-5 space-y-4 overflow-y-auto">
          {/* Category Selector */}
          <div className="space-y-1.5">
            <label className="text-xs font-medium text-foreground block">
              Cognitive Type
            </label>
            <div className="grid grid-cols-2 gap-2">
              {categories.map((cat) => (
                <button
                  key={cat.id}
                  type="button"
                  onClick={() => setCategory(cat.id)}
                  className={cn(
                    "flex items-start gap-2.5 p-2.5 rounded-lg border text-left transition-all cursor-pointer",
                    category === cat.id
                      ? cn(cat.border, "ring-1 ring-primary/40 bg-accent/40 shadow-xs")
                      : "border-border/80 hover:bg-accent/30"
                  )}
                >
                  <div className={cn("p-1.5 rounded-md shrink-0", cat.color)}>
                    {cat.icon}
                  </div>
                  <div className="min-w-0">
                    <div className="text-xs font-semibold text-foreground truncate">
                      {cat.label}
                    </div>
                    <div className="text-3xs text-muted-foreground line-clamp-1">
                      {cat.desc}
                    </div>
                  </div>
                </button>
              ))}
            </div>
          </div>

          {/* Thought Textarea */}
          <div className="space-y-1.5">
            <label className="text-xs font-medium text-foreground flex items-center justify-between">
              <span>{category === "doubt" ? "Question or Doubt" : "Decision / Thought Summary"}</span>
              <span className="text-3xs text-muted-foreground font-mono">
                Ctrl+Enter to save
              </span>
            </label>
            <textarea
              ref={textareaRef}
              rows={3}
              value={summary}
              onChange={(e) => setSummary(e.target.value)}
              placeholder={
                category === "doubt"
                  ? "e.g., Will SQLite concurrency block when two agents call tools simultaneously?"
                  : category === "minor_decision"
                  ? "e.g., Default button radius set to 8px for consistency with dialog cards"
                  : "e.g., Decided to use unified SQLite v3 schema for cognitive memory"
              }
              className="w-full text-xs p-3 rounded-lg border border-border bg-background text-foreground placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary/40 resize-none font-sans"
              required
            />
          </div>

          {/* Context / Background */}
          <div className="space-y-1.5">
            <label className="text-xs font-medium text-muted-foreground">
              Context / Rationale (Optional)
            </label>
            <input
              type="text"
              value={contextSummary}
              onChange={(e) => setContextSummary(e.target.value)}
              placeholder="e.g., Discussed during UI refactoring session"
              className="w-full text-xs h-8 px-3 rounded-md border border-border bg-background text-foreground placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary/40"
            />
          </div>

          {/* Files */}
          <div className="space-y-1.5">
            <label className="text-xs font-medium text-muted-foreground flex items-center gap-1">
              <FileCode className="size-3" />
              <span>Related Files (Optional, comma-separated)</span>
            </label>
            <input
              type="text"
              value={fileInput}
              onChange={(e) => setFileInput(e.target.value)}
              placeholder="e.g., src/storage/nodes.rs, ui/src/types/knox.ts"
              className="w-full text-xs h-8 px-3 rounded-md border border-border bg-background text-foreground placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary/40 font-mono"
            />
          </div>

          {/* Footer Actions */}
          <div className="pt-2 flex items-center justify-end gap-2 border-t border-border">
            <Button type="button" variant="ghost" size="sm" onClick={onClose}>
              Cancel
            </Button>
            <Button
              type="submit"
              variant="primary"
              size="sm"
              disabled={!summary.trim() || isSubmitting}
            >
              {isSubmitting ? "Recording..." : "Record to Knowledge Graph"}
            </Button>
          </div>
        </form>
      </div>
    </div>
  );
};
