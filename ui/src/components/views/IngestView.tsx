import React, { useState } from "react";
import {
  FileInput,
  UploadCloud,
  CheckCircle2,
  AlertTriangle,
  FileCode,
  ArrowRight,
  Layers,
  ChevronDown,
  ChevronRight,
  FolderOpen,
  Info,
} from "lucide-react";
import { KnoxAPI } from "@/services/api";
import { useKnoxStore } from "@/store/useKnoxStore";
import type { IngestReport, KnoxAction } from "@/types/knox";
import { ActionBadge } from "@/components/ui/ActionBadge";
import { ActorBadge } from "@/components/ui/ActorBadge";
import { PageHeader } from "@/components/ui/PageHeader";
import { EmptyState } from "@/components/ui/EmptyState";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/Card";
import { Button } from "@/components/ui/Button";
import { Badge } from "@/components/ui/Badge";
import { cn } from "@/lib/cn";

/** Backend currently registers only AntigravityMapper (src/ingest/mod.rs). */
function detectFormat(path: string | null): {
  label: string;
  supported: boolean;
} {
  if (!path) return { label: "No file selected", supported: false };
  const lower = path.toLowerCase();
  if (lower.includes("antigravity") || lower.endsWith(".jsonl")) {
    return { label: "Antigravity JSONL (expected)", supported: true };
  }
  if (lower.endsWith(".json")) {
    return {
      label: "JSON — may fail if not Antigravity format",
      supported: false,
    };
  }
  return { label: "Unrecognized — Antigravity .jsonl required", supported: false };
}

export const IngestView: React.FC = () => {
  const { showToast, refreshData, setActiveTab, setSelectedAction } =
    useKnoxStore();
  const [selectedFile, setSelectedFile] = useState<string | null>(null);
  const [fileSize, setFileSize] = useState<string | null>(null);
  const [isIngesting, setIsIngesting] = useState(false);
  const [isDragging, setIsDragging] = useState(false);
  const [report, setReport] = useState<IngestReport | null>(null);
  const [ingestError, setIngestError] = useState<string | null>(null);
  const [previewActions, setPreviewActions] = useState<KnoxAction[]>([]);
  const [showPreview, setShowPreview] = useState(true);

  const handleBrowse = async () => {
    try {
      const selected = await KnoxAPI.openFileDialog({
        name: "Transcripts",
        extensions: ["jsonl", "json"],
      });
      if (selected) {
        setSelectedFile(selected);
        setFileSize(null);
        setReport(null);
        setIngestError(null);
      }
    } catch (err) {
      console.warn("Dialog failed or not in Tauri:", err);
      const promptPath = window.prompt(
        "Enter Antigravity transcript path:",
        "fixtures/transcript_antigravity.jsonl"
      );
      if (promptPath) {
        setSelectedFile(promptPath);
        setFileSize(null);
        setReport(null);
        setIngestError(null);
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
      const path = (file as File & { path?: string }).path || file.name;
      setSelectedFile(path);
      setFileSize(`${(file.size / 1024).toFixed(1)} KB`);
      setReport(null);
      setIngestError(null);
    }
  };

  const handleIngest = async () => {
    if (!selectedFile) return;
    try {
      setIsIngesting(true);
      setIngestError(null);
      const res = await KnoxAPI.ingestTranscript(selectedFile);
      setReport(res);
      showToast(
        `Imported ${res.actions_imported} actions into the knowledge graph.`
      );
      await refreshData();
      const updatedActions = await KnoxAPI.getGraphData(false, 5);
      setPreviewActions(updatedActions.slice(0, 5));
    } catch (err) {
      const msg = String(err);
      setIngestError(msg);
      showToast(msg, true);
    } finally {
      setIsIngesting(false);
    }
  };

  const format = detectFormat(selectedFile);

  return (
    <div className="p-5 sm:p-6 space-y-5 max-w-4xl mx-auto overflow-y-auto h-full">
      <PageHeader
        title="Ingest Transcripts"
        description="Import Antigravity session transcripts (.jsonl) to bootstrap the knowledge graph. Deterministic clustering — no LLM tokens used."
        icon={<FileInput />}
      />

      <Card>
        <CardContent className="p-4 flex gap-3 text-xs text-muted-foreground">
          <Info className="size-4 text-primary shrink-0 mt-0.5" aria-hidden />
          <div className="space-y-1 leading-relaxed">
            <p>
              <span className="text-foreground font-medium">Supported format:</span>{" "}
              Antigravity transcript JSONL (the only mapper registered in Knox today).
            </p>
            <p>
              Claude, Cursor, and other formats are not ingested by the current backend.
              Prefer files named like <code className="font-mono text-foreground">transcript.jsonl</code>.
            </p>
          </div>
        </CardContent>
      </Card>

      <div
        role="button"
        tabIndex={0}
        aria-label="Drop Antigravity transcript or browse for file"
        onKeyDown={(e) => {
          if (e.key === "Enter" || e.key === " ") {
            e.preventDefault();
            handleBrowse();
          }
        }}
        onDragOver={handleDragOver}
        onDragEnter={handleDragOver}
        onDragLeave={handleDragLeave}
        onDrop={handleDrop}
        onClick={handleBrowse}
        className={cn(
          "rounded-lg border-2 border-dashed p-8 sm:p-10 text-center cursor-pointer space-y-3 transition-colors duration-100",
          "focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-focus-ring",
          isDragging
            ? "border-primary bg-primary-muted"
            : "border-border hover:border-primary/50 bg-card/50 hover:bg-card"
        )}
      >
        <div className="size-11 rounded-md bg-primary-muted text-primary mx-auto flex items-center justify-center border border-primary/20">
          <UploadCloud className="size-5" aria-hidden />
        </div>
        <div>
          <p className="text-sm font-semibold text-foreground">
            Drop Antigravity .jsonl here, or click to browse
          </p>
          <p className="text-xs text-muted-foreground mt-1">
            Extensions: .jsonl (preferred), .json
          </p>
        </div>
        <Button
          type="button"
          variant="outline"
          size="sm"
          onClick={(e) => {
            e.stopPropagation();
            handleBrowse();
          }}
        >
          <FolderOpen className="size-3.5" />
          Select file
        </Button>
      </div>

      {!selectedFile && !report && (
        <EmptyState
          compact
          icon={<FileInput />}
          title="No transcript selected"
          description="Choose an Antigravity JSONL export to parse plans, decisions, and file writes into Knox."
        />
      )}

      {selectedFile && (
        <Card>
          <CardContent className="p-4 space-y-3">
            <div className="flex flex-wrap items-start justify-between gap-2">
              <div className="flex items-start gap-2 min-w-0">
                <FileCode className="size-4 text-primary shrink-0 mt-0.5" aria-hidden />
                <div className="min-w-0">
                  <p className="text-xs font-mono text-foreground break-all select-all">
                    {selectedFile}
                  </p>
                  {fileSize && (
                    <p className="text-xs text-muted-foreground font-mono mt-0.5">
                      {fileSize}
                    </p>
                  )}
                </div>
              </div>
              <div className="flex items-center gap-1.5 shrink-0">
                <Badge variant={format.supported ? "default" : "warning"}>
                  {format.label}
                </Badge>
                <Badge variant="secondary">Ready</Badge>
              </div>
            </div>

            {ingestError && (
              <div
                role="alert"
                className="px-3 py-2 rounded-md border border-danger/40 bg-danger/10 text-xs text-danger"
              >
                {ingestError}
              </div>
            )}

            <Button
              type="button"
              variant="primary"
              className="w-full"
              onClick={handleIngest}
              disabled={isIngesting}
              loading={isIngesting}
            >
              {isIngesting
                ? "Parsing & clustering actions…"
                : "Start ingestion"}
            </Button>
          </CardContent>
        </Card>
      )}

      {report && (
        <Card className="border-success/30">
          <CardHeader className="flex-row items-center justify-between space-y-0 pb-0">
            <CardTitle className="flex items-center gap-2 text-success">
              <CheckCircle2 className="size-4" aria-hidden />
              Ingestion complete
            </CardTitle>
            <Button
              variant="ghost"
              size="sm"
              onClick={() => setActiveTab("graph")}
              className="text-primary"
            >
              View in Graph
              <ArrowRight className="size-3" />
            </Button>
          </CardHeader>
          <CardContent className="pt-4 space-y-4">
            <div className="grid grid-cols-1 sm:grid-cols-3 gap-3 font-mono text-xs">
              {[
                { label: "Actions imported", value: report.actions_imported },
                { label: "Files touched", value: report.files_touched },
                { label: "Sessions created", value: report.sessions_created },
              ].map((m) => (
                <div
                  key={m.label}
                  className="p-3 rounded-md bg-background border border-border text-center space-y-1"
                >
                  <span className="text-overline block">{m.label}</span>
                  <span className="text-xl font-semibold text-foreground">
                    {m.value}
                  </span>
                </div>
              ))}
            </div>

            {previewActions.length > 0 && (
              <div className="pt-2 border-t border-border-subtle">
                <button
                  type="button"
                  onClick={() => setShowPreview(!showPreview)}
                  className="w-full flex items-center justify-between py-1 text-xs font-medium text-foreground hover:text-primary transition-colors cursor-pointer focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-focus-ring rounded-sm"
                  aria-expanded={showPreview}
                >
                  <span className="flex items-center gap-1.5">
                    <Layers className="size-3.5 text-primary" aria-hidden />
                    Preview ingested actions ({previewActions.length})
                  </span>
                  {showPreview ? (
                    <ChevronDown className="size-3.5" aria-hidden />
                  ) : (
                    <ChevronRight className="size-3.5" aria-hidden />
                  )}
                </button>
                {showPreview && (
                  <div className="mt-2 space-y-1.5">
                    {previewActions.map((act) => (
                      <button
                        key={act.id}
                        type="button"
                        onClick={() => setSelectedAction(act)}
                        className="w-full p-2.5 rounded-md bg-background border border-border hover:border-primary/40 text-xs transition-colors cursor-pointer flex items-center justify-between gap-3 text-left focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-focus-ring"
                      >
                        <div className="flex items-center gap-2 min-w-0 flex-1">
                          <ActorBadge actor={act.actor} />
                          <ActionBadge type={act.action_type} />
                          <span className="truncate font-medium text-foreground">
                            {act.summary}
                          </span>
                        </div>
                        <span className="text-xs font-mono text-muted-foreground shrink-0">
                          {new Date(act.timestamp).toLocaleTimeString([], {
                            hour: "2-digit",
                            minute: "2-digit",
                          })}
                        </span>
                      </button>
                    ))}
                  </div>
                )}
              </div>
            )}
          </CardContent>
        </Card>
      )}

      <Card>
        <CardContent className="p-4 flex gap-3 text-xs text-muted-foreground">
          <AlertTriangle className="size-4 text-warning shrink-0 mt-0.5" aria-hidden />
          <p className="leading-relaxed">
            Knox uses a deterministic heuristic state machine to group raw tool
            events into plans, decisions, and file writes. Processing does not
            call an LLM.
          </p>
        </CardContent>
      </Card>
    </div>
  );
};
