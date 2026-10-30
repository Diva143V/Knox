import React, { useState } from "react";
import {
  FileInput,
  UploadCloud,
  CheckCircle2,
  AlertTriangle,
  FileCode,
  Sparkles,
  ArrowRight,
  Layers,
  ChevronDown,
  ChevronRight,
  RefreshCw,
  FolderOpen,
} from "lucide-react";
import { KnoxAPI } from "@/services/api";
import { useKnoxStore } from "@/store/useKnoxStore";
import type { IngestReport, KnoxAction } from "@/types/knox";
import { ActionBadge } from "@/components/ui/ActionBadge";
import { ActorBadge } from "@/components/ui/ActorBadge";

export const IngestView: React.FC = () => {
  const { showToast, refreshData, setActiveTab, setSelectedAction } = useKnoxStore();
  const [selectedFile, setSelectedFile] = useState<string | null>(null);
  const [fileSize, setFileSize] = useState<string | null>(null);
  const [isIngesting, setIsIngesting] = useState(false);
  const [isDragging, setIsDragging] = useState(false);
  const [report, setReport] = useState<IngestReport | null>(null);
  const [previewActions, setPreviewActions] = useState<KnoxAction[]>([]);
  const [showPreview, setShowPreview] = useState(true);

  // Auto-detect transcript format mapper name from path
  const detectFormat = (path: string | null): string => {
    if (!path) return "Unknown";
    const lower = path.toLowerCase();
    if (lower.includes("antigravity") || lower.includes("transcript.jsonl")) {
      return "Antigravity JSONL";
    }
    if (lower.includes("claude")) {
      return "Claude Chat History";
    }
    if (lower.includes("cursor")) {
      return "Cursor Transcript";
    }
    return "Generic JSONL Transcript";
  };

  const handleBrowse = async () => {
    try {
      const selected = await KnoxAPI.openFileDialog({
        name: "Transcripts",
        extensions: ["jsonl", "json"],
      });
      if (selected) {
        setSelectedFile(selected);
        setReport(null);
      }
    } catch (err) {
      console.warn("Dialog failed or not in Tauri:", err);
      const promptPath = window.prompt(
        "Enter transcript file path:",
        "fixtures/transcript_antigravity.jsonl"
      );
      if (promptPath) {
        setSelectedFile(promptPath);
        setReport(null);
      }
    }
  };

  const handleDragOver = (e: React.DragEvent) => {
    e.preventDefault();
    e.stopPropagation();
    setIsDragging(true);
  };

  const handleDragLeave = (e: React.DragEvent) => {
    e.preventDefault();
    e.stopPropagation();
    setIsDragging(false);
  };

  const handleDrop = (e: React.DragEvent) => {
    e.preventDefault();
    e.stopPropagation();
    setIsDragging(false);

    if (e.dataTransfer.files && e.dataTransfer.files.length > 0) {
      const file = e.dataTransfer.files[0];
      // In Tauri or web, get file path or name
      const path = (file as any).path || file.name;
      setSelectedFile(path);
      setFileSize(`${(file.size / 1024).toFixed(1)} KB`);
      setReport(null);
    }
  };

  const handleIngest = async () => {
    if (!selectedFile) return;
    try {
      setIsIngesting(true);
      const res = await KnoxAPI.ingestTranscript(selectedFile);
      setReport(res);
      showToast(`Successfully imported ${res.actions_imported} actions into knowledge graph!`);

      // Refresh global store and grab preview of latest actions
      await refreshData();
      const updatedActions = await KnoxAPI.getGraphData(false, 5);
      setPreviewActions(updatedActions.slice(0, 5));
    } catch (err) {
      showToast(String(err), true);
    } finally {
      setIsIngesting(false);
    }
  };

  const detectedFormat = detectFormat(selectedFile);

  return (
    <div className="p-6 space-y-6 max-w-4xl mx-auto overflow-y-auto h-full">
      {/* Header */}
      <div>
        <h1 className="text-xl font-bold tracking-tight text-foreground flex items-center gap-2">
          <FileInput className="w-5 h-5 text-primary" />
          <span>Ingest Conversation Transcripts</span>
        </h1>
        <p className="text-xs text-muted-foreground mt-1">
          Import Antigravity, Claude, or Cursor session history to bootstrap your knowledge graph.
        </p>
      </div>

      {/* Drag & Drop Area */}
      <div
        onDragOver={handleDragOver}
        onDragEnter={handleDragOver}
        onDragLeave={handleDragLeave}
        onDrop={handleDrop}
        onClick={handleBrowse}
        className={`rounded-xl border-2 border-dashed p-10 text-center transition-all cursor-pointer space-y-3 ${
          isDragging
            ? "border-primary bg-primary/10 shadow-lg shadow-primary/10"
            : "border-border/80 hover:border-primary/60 bg-card/40 hover:bg-card/70"
        }`}
      >
        <div className="w-12 h-12 rounded-full bg-primary/10 text-primary mx-auto flex items-center justify-center">
          <UploadCloud className="w-6 h-6" />
        </div>
        <div>
          <p className="text-sm font-semibold text-foreground">
            Drop .jsonl transcript here, or click to browse
          </p>
          <p className="text-xs text-muted-foreground mt-1">
            Supports Antigravity transcript.jsonl with automatic intent clustering
          </p>
        </div>

        <div className="pt-2">
          <button
            type="button"
            onClick={(e) => {
              e.stopPropagation();
              handleBrowse();
            }}
            className="inline-flex items-center gap-1.5 px-3 py-1.5 rounded-lg border border-border bg-card hover:bg-background text-xs font-medium text-foreground transition cursor-pointer shadow-sm"
          >
            <FolderOpen className="w-3.5 h-3.5 text-primary" />
            <span>Select Local File</span>
          </button>
        </div>
      </div>

      {/* Selected File Chip & Ingest Action */}
      {selectedFile && (
        <div className="p-4 rounded-xl border border-border bg-card space-y-3 animate-in fade-in-50 duration-150">
          <div className="flex flex-wrap items-center justify-between gap-2">
            <div className="flex items-center gap-2 min-w-0">
              <FileCode className="w-4 h-4 text-primary shrink-0" />
              <span className="text-xs font-mono text-foreground font-medium truncate max-w-md select-all">
                {selectedFile}
              </span>
              {fileSize && (
                <span className="text-[10px] text-muted-foreground font-mono">
                  ({fileSize})
                </span>
              )}
            </div>

            <div className="flex items-center gap-2 shrink-0">
              {/* Format Auto-Detection Badge */}
              <span className="text-[10px] px-2 py-0.5 rounded bg-indigo-500/10 text-indigo-400 border border-indigo-500/20 font-mono font-medium">
                {detectedFormat}
              </span>

              <span className="text-[10px] px-2 py-0.5 rounded bg-emerald-500/10 text-emerald-400 border border-emerald-500/20 font-mono">
                Ready
              </span>
            </div>
          </div>

          <div className="pt-1">
            <button
              onClick={handleIngest}
              disabled={isIngesting}
              className="w-full py-2.5 rounded-lg bg-primary text-primary-foreground text-xs font-semibold hover:bg-primary/90 transition flex items-center justify-center gap-2 cursor-pointer shadow-sm shadow-primary/25 disabled:opacity-50"
            >
              {isIngesting ? (
                <>
                  <RefreshCw className="w-3.5 h-3.5 animate-spin" />
                  <span>Parsing & Clustering Actions…</span>
                </>
              ) : (
                <>
                  <Sparkles className="w-3.5 h-3.5" />
                  <span>Start Ingestion into Knowledge Graph</span>
                </>
              )}
            </button>
          </div>
        </div>
      )}

      {/* Ingest Report Post-Ingestion */}
      {report && (
        <div className="p-5 rounded-xl border border-emerald-500/30 bg-emerald-500/5 space-y-4 animate-in fade-in-50 duration-200">
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-2 text-emerald-400 font-semibold text-sm">
              <CheckCircle2 className="w-4 h-4" />
              <span>Ingestion Completed Successfully</span>
            </div>

            <button
              onClick={() => setActiveTab("graph")}
              className="flex items-center gap-1 text-xs text-primary hover:underline font-medium cursor-pointer"
            >
              <span>View in Graph</span>
              <ArrowRight className="w-3 h-3" />
            </button>
          </div>

          <div className="grid grid-cols-1 sm:grid-cols-3 gap-4 pt-1 font-mono text-xs">
            <div className="p-3.5 rounded-lg bg-card border border-border text-center space-y-1">
              <span className="text-muted-foreground block text-[10px] uppercase font-semibold">
                Actions Imported
              </span>
              <span className="text-2xl font-bold text-foreground">
                {report.actions_imported}
              </span>
            </div>
            <div className="p-3.5 rounded-lg bg-card border border-border text-center space-y-1">
              <span className="text-muted-foreground block text-[10px] uppercase font-semibold">
                Files Touched
              </span>
              <span className="text-2xl font-bold text-foreground">
                {report.files_touched}
              </span>
            </div>
            <div className="p-3.5 rounded-lg bg-card border border-border text-center space-y-1">
              <span className="text-muted-foreground block text-[10px] uppercase font-semibold">
                Sessions Created
              </span>
              <span className="text-2xl font-bold text-foreground">
                {report.sessions_created}
              </span>
            </div>
          </div>

          {/* First 5 Parsed Actions Preview Accordion */}
          {previewActions.length > 0 && (
            <div className="pt-2 border-t border-emerald-500/20">
              <button
                type="button"
                onClick={() => setShowPreview(!showPreview)}
                className="w-full flex items-center justify-between py-1 text-xs font-medium text-foreground hover:text-primary transition cursor-pointer"
              >
                <span className="flex items-center gap-1.5">
                  <Layers className="w-3.5 h-3.5 text-primary" />
                  <span>Preview Ingested Actions ({previewActions.length})</span>
                </span>
                {showPreview ? (
                  <ChevronDown className="w-3.5 h-3.5" />
                ) : (
                  <ChevronRight className="w-3.5 h-3.5" />
                )}
              </button>

              {showPreview && (
                <div className="mt-2 space-y-2">
                  {previewActions.map((act) => (
                    <div
                      key={act.id}
                      onClick={() => setSelectedAction(act)}
                      className="p-2.5 rounded-lg bg-card/80 border border-border/60 hover:border-primary/40 text-xs transition cursor-pointer flex items-center justify-between gap-3"
                    >
                      <div className="flex items-center gap-2 min-w-0 flex-1">
                        <ActorBadge actor={act.actor} />
                        <ActionBadge type={act.action_type} />
                        <span className="truncate font-medium text-foreground">
                          {act.summary}
                        </span>
                      </div>
                      <span className="text-[10px] font-mono text-muted-foreground shrink-0">
                        {new Date(act.timestamp).toLocaleTimeString([], {
                          hour: "2-digit",
                          minute: "2-digit",
                        })}
                      </span>
                    </div>
                  ))}
                </div>
              )}
            </div>
          )}
        </div>
      )}

      {/* Explanatory Info Card */}
      <div className="p-4 rounded-xl border border-border/60 bg-card/30 flex gap-3 text-xs text-muted-foreground">
        <AlertTriangle className="w-4 h-4 text-amber-400 shrink-0 mt-0.5" />
        <p className="leading-relaxed">
          Knox uses a deterministic heuristic state machine to group noisy raw tool events into
          meaningful high-level plans, decisions, and file writes. Zero LLM tokens are wasted
          during transcript processing.
        </p>
      </div>
    </div>
  );
};
