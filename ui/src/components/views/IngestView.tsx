import React, { useState, useEffect } from "react";
import {
  FileInput,
  UploadCloud,
  CheckCircle2,
  AlertTriangle,
  FileCode,
  Layers,
  FolderOpen,
  Sparkles,
  GitBranch,
  Terminal,
  Trash2,
  RefreshCw,
  Search,
  Network,
} from "lucide-react";
import { KnoxAPI } from "@/services/api";
import { useKnoxStore } from "@/store/useKnoxStore";
import type {
  DetectedTranscript,
  IngestPreview,
  IngestReport,
} from "@/types/knox";
import { ActionBadge } from "@/components/ui/ActionBadge";
import { ActorBadge } from "@/components/ui/ActorBadge";
import { PageHeader } from "@/components/ui/PageHeader";
import { EmptyState } from "@/components/ui/EmptyState";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/Card";
import { Button } from "@/components/ui/Button";
import { Badge } from "@/components/ui/Badge";
import { cn } from "@/lib/cn";

type IngestMode = "files" | "paste" | "detect";

interface QueuedFile {
  name: string;
  path: string;
  sizeStr?: string;
  agentLabel: string;
}

function detectAgentFormat(path: string | null): {
  agent: string;
  label: string;
  variant: "default" | "secondary" | "outline" | "plan";
} {
  if (!path) return { agent: "unknown", label: "No file selected", variant: "secondary" };
  const lower = path.toLowerCase();
  if (lower.includes("antigravity")) {
    return { agent: "antigravity", label: "Antigravity JSONL", variant: "default" };
  }
  if (lower.includes("claude")) {
    return { agent: "claude", label: "Claude Code / Desktop", variant: "plan" };
  }
  if (lower.includes("cursor") || lower.includes("composer")) {
    return { agent: "cursor", label: "Cursor Composer", variant: "default" };
  }
  if (lower.includes("chatgpt") || lower.includes("openai") || lower.includes("conversations")) {
    return { agent: "chatgpt", label: "ChatGPT / OpenAI", variant: "outline" };
  }
  if (lower.endsWith(".git") || lower.includes("git")) {
    return { agent: "git", label: "Git Commit History", variant: "secondary" };
  }
  if (lower.endsWith(".md") || lower.endsWith(".markdown") || lower.endsWith(".txt")) {
    return { agent: "markdown", label: "Markdown / Plain Text Chat", variant: "secondary" };
  }
  if (lower.endsWith(".jsonl")) {
    return { agent: "jsonl", label: "Universal JSONL Log", variant: "default" };
  }
  if (lower.endsWith(".json")) {
    return { agent: "json", label: "Universal JSON Log", variant: "outline" };
  }
  return { agent: "generic", label: "Generic Transcript", variant: "secondary" };
}

export const IngestView: React.FC = () => {
  const { showToast, refreshData, setActiveTab, activeProjectScope } =
    useKnoxStore();

  const [mode, setMode] = useState<IngestMode>("files");

  // Files mode state
  const [queuedFiles, setQueuedFiles] = useState<QueuedFile[]>([]);
  const [isDragging, setIsDragging] = useState(false);

  // Paste mode state
  const [pasteContent, setPasteContent] = useState("");
  const [actorOverride, setActorOverride] = useState("auto");

  // Detect mode state
  const [isScanning, setIsScanning] = useState(false);
  const [detectedTranscripts, setDetectedTranscripts] = useState<DetectedTranscript[]>([]);
  const [gitCommitLimit, setGitCommitLimit] = useState<number>(50);

  // Preview & Processing state
  const [isPreviewing, setIsPreviewing] = useState(false);
  const [isIngesting, setIsIngesting] = useState(false);
  const [preview, setPreview] = useState<IngestPreview | null>(null);
  const [report, setReport] = useState<IngestReport | null>(null);
  const [ingestError, setIngestError] = useState<string | null>(null);
  const [expandedActionId, setExpandedActionId] = useState<string | null>(null);

  // Active target being previewed/ingested
  const [activePreviewTarget, setActivePreviewTarget] = useState<{
    type: "file" | "paste" | "git";
    path?: string;
    content?: string;
  } | null>(null);

  // Auto-scan on entering detect mode
  useEffect(() => {
    if (mode === "detect" && detectedTranscripts.length === 0) {
      handleScanLocal();
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [mode]);

  const handleBrowseFiles = async () => {
    try {
      const selected = await KnoxAPI.openFileDialog({
        name: "Transcripts & Logs",
        extensions: ["jsonl", "json", "md", "txt"],
      });
      if (selected) {
        addQueuedFile(selected);
      }
    } catch (err) {
      console.warn("File dialog fallback:", err);
      const promptPath = window.prompt(
        "Enter transcript file path:",
        "fixtures/transcript_antigravity.jsonl"
      );
      if (promptPath) {
        addQueuedFile(promptPath);
      }
    }
  };

  const addQueuedFile = (filePath: string, sizeStr?: string) => {
    const norm = filePath.trim();
    if (!norm) return;
    const name = norm.replace(/\\/g, "/").split("/").pop() || norm;
    const fmt = detectAgentFormat(norm);

    setQueuedFiles((prev) => {
      if (prev.some((f) => f.path.toLowerCase() === norm.toLowerCase())) {
        return prev;
      }
      return [
        ...prev,
        {
          name,
          path: norm,
          sizeStr,
          agentLabel: fmt.label,
        },
      ];
    });
    setReport(null);
    setIngestError(null);
  };

  const removeQueuedFile = (filePath: string) => {
    setQueuedFiles((prev) => prev.filter((f) => f.path !== filePath));
    if (activePreviewTarget?.path === filePath) {
      setPreview(null);
      setActivePreviewTarget(null);
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
      for (let i = 0; i < e.dataTransfer.files.length; i++) {
        const file = e.dataTransfer.files[i];
        const path = (file as File & { path?: string }).path || file.name;
        const sizeStr = `${(file.size / 1024).toFixed(1)} KB`;
        addQueuedFile(path, sizeStr);
      }
    }
  };

  // Run dry-run preview for a file
  const handlePreviewFile = async (filePath: string) => {
    try {
      setIsPreviewing(true);
      setIngestError(null);
      const p = await KnoxAPI.previewTranscript({ path: filePath });
      setPreview(p);
      setActivePreviewTarget({ type: "file", path: filePath });
    } catch (err) {
      const msg = String(err);
      setIngestError(msg);
      showToast(msg, true);
    } finally {
      setIsPreviewing(false);
    }
  };

  // Run dry-run preview for pasted text
  const handlePreviewPaste = async () => {
    if (!pasteContent.trim()) {
      showToast("Please enter transcript or log text to preview", "warning");
      return;
    }
    try {
      setIsPreviewing(true);
      setIngestError(null);
      const p = await KnoxAPI.previewTranscript({ content: pasteContent });
      setPreview(p);
      setActivePreviewTarget({ type: "paste", content: pasteContent });
    } catch (err) {
      const msg = String(err);
      setIngestError(msg);
      showToast(msg, true);
    } finally {
      setIsPreviewing(false);
    }
  };

  // Confirm ingestion into SQLite database
  const handleConfirmIngest = async () => {
    if (!activePreviewTarget) return;

    try {
      setIsIngesting(true);
      setIngestError(null);
      let res: IngestReport;

      if (activePreviewTarget.type === "file" && activePreviewTarget.path) {
        res = await KnoxAPI.ingestTranscript(activePreviewTarget.path);
        removeQueuedFile(activePreviewTarget.path);
      } else if (activePreviewTarget.type === "paste" && activePreviewTarget.content) {
        res = await KnoxAPI.ingestRawTranscript(activePreviewTarget.content, actorOverride);
        setPasteContent("");
      } else {
        throw new Error("No active preview target to ingest");
      }

      setReport(res);
      setPreview(null);
      setActivePreviewTarget(null);

      const dupMsg = res.duplicates_skipped
        ? ` (${res.duplicates_skipped} duplicates skipped)`
        : "";
      showToast(
        `Imported ${res.actions_imported} actions into knowledge graph${dupMsg}.`,
        "success"
      );
      await refreshData();
    } catch (err) {
      const msg = String(err);
      setIngestError(msg);
      showToast(msg, true);
    } finally {
      setIsIngesting(false);
    }
  };

  // Ingest Git repository commit history
  const handleIngestGit = async (targetPath: string) => {
    try {
      setIsIngesting(true);
      setIngestError(null);
      const res = await KnoxAPI.ingestGitHistory(targetPath, gitCommitLimit);
      setReport(res);
      const dupMsg = res.duplicates_skipped
        ? ` (${res.duplicates_skipped} duplicates skipped)`
        : "";
      showToast(
        `Imported ${res.actions_imported} Git commits across ${res.files_touched} files${dupMsg}.`,
        "success"
      );
      await refreshData();
    } catch (err) {
      const msg = String(err);
      setIngestError(msg);
      showToast(msg, true);
    } finally {
      setIsIngesting(false);
    }
  };

  // Scan local agent directories
  const handleScanLocal = async () => {
    try {
      setIsScanning(true);
      const found = await KnoxAPI.detectLocalTranscripts(activeProjectScope || undefined);
      setDetectedTranscripts(found);
      if (found.length > 0) {
        showToast(`Discovered ${found.length} local agent transcripts.`, "info");
      }
    } catch (err) {
      console.warn("Scan failed:", err);
    } finally {
      setIsScanning(false);
    }
  };

  return (
    <div className="p-5 sm:p-6 space-y-5 max-w-4xl mx-auto overflow-y-auto h-full pb-16">
      <PageHeader
        title="Universal Ingest Studio"
        description="Bootstrap the knowledge graph from AI coding agents, Git commits, or raw chat transcripts. Deterministic parsing — zero LLM tokens used."
        icon={<FileInput />}
      />

      {/* Supported Agent Suites Banner */}
      <Card>
        <CardContent className="p-3.5 flex flex-wrap items-center justify-between gap-3 text-xs">
          <div className="flex items-center gap-2">
            <Sparkles className="size-4 text-primary shrink-0" />
            <span className="font-semibold text-foreground">Supported Formats:</span>
          </div>
          <div className="flex flex-wrap items-center gap-1.5">
            <Badge variant="default">Antigravity JSONL</Badge>
            <Badge variant="plan">Claude Code & Desktop</Badge>
            <Badge variant="secondary">Cursor Composer</Badge>
            <Badge variant="outline">ChatGPT / OpenAI</Badge>
            <Badge variant="secondary">Git History</Badge>
            <Badge variant="outline">Markdown & Plain Text</Badge>
          </div>
        </CardContent>
      </Card>

      {/* Mode Switcher Tabs */}
      <div className="flex items-center gap-1 p-1 bg-muted/60 rounded-xl border border-border/80 w-fit">
        <button
          type="button"
          onClick={() => {
            setMode("files");
            setIngestError(null);
          }}
          className={cn(
            "flex items-center gap-2 px-3.5 py-1.5 rounded-lg text-xs font-medium transition-all cursor-pointer",
            mode === "files"
              ? "bg-card text-foreground shadow-xs border border-border font-semibold"
              : "text-muted-foreground hover:text-foreground"
          )}
        >
          <UploadCloud className="size-3.5" />
          <span>Files & Batch Drop</span>
          {queuedFiles.length > 0 && (
            <span className="size-4 rounded-full bg-primary/20 text-primary text-3xs font-mono flex items-center justify-center font-bold">
              {queuedFiles.length}
            </span>
          )}
        </button>

        <button
          type="button"
          onClick={() => {
            setMode("paste");
            setIngestError(null);
          }}
          className={cn(
            "flex items-center gap-2 px-3.5 py-1.5 rounded-lg text-xs font-medium transition-all cursor-pointer",
            mode === "paste"
              ? "bg-card text-foreground shadow-xs border border-border font-semibold"
              : "text-muted-foreground hover:text-foreground"
          )}
        >
          <Terminal className="size-3.5" />
          <span>Paste Log / Text</span>
        </button>

        <button
          type="button"
          onClick={() => {
            setMode("detect");
            setIngestError(null);
          }}
          className={cn(
            "flex items-center gap-2 px-3.5 py-1.5 rounded-lg text-xs font-medium transition-all cursor-pointer",
            mode === "detect"
              ? "bg-card text-foreground shadow-xs border border-border font-semibold"
              : "text-muted-foreground hover:text-foreground"
          )}
        >
          <Sparkles className="size-3.5 text-primary" />
          <span>Auto-Detect & Git</span>
        </button>
      </div>

      {/* Mode 1: Files & Batch Drop */}
      {mode === "files" && (
        <div className="space-y-4">
          <div
            role="button"
            tabIndex={0}
            aria-label="Drop transcripts or click to browse files"
            onKeyDown={(e) => {
              if (e.key === "Enter" || e.key === " ") {
                e.preventDefault();
                handleBrowseFiles();
              }
            }}
            onDragOver={handleDragOver}
            onDragEnter={handleDragOver}
            onDragLeave={handleDragLeave}
            onDrop={handleDrop}
            onClick={handleBrowseFiles}
            className={cn(
              "rounded-xl border-2 border-dashed p-8 text-center cursor-pointer space-y-3 transition-all duration-150",
              "focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary/40",
              isDragging
                ? "border-primary bg-primary/10 scale-[0.99]"
                : "border-border hover:border-primary/50 bg-card/40 hover:bg-card/70"
            )}
          >
            <div className="size-12 rounded-xl bg-primary/10 text-primary mx-auto flex items-center justify-center border border-primary/20 shadow-xs">
              <UploadCloud className="size-6" aria-hidden />
            </div>
            <div>
              <p className="text-sm font-semibold text-foreground">
                Drop transcript files or folders here, or click to browse
              </p>
              <p className="text-xs text-muted-foreground mt-1">
                Supports Antigravity (.jsonl), Claude, Cursor (.json), ChatGPT, and Markdown logs
              </p>
            </div>
            <Button
              type="button"
              variant="outline"
              size="sm"
              onClick={(e) => {
                e.stopPropagation();
                handleBrowseFiles();
              }}
            >
              <FolderOpen className="size-3.5" />
              <span>Select File(s)</span>
            </Button>
          </div>

          {/* Queued Files List */}
          {queuedFiles.length > 0 && (
            <Card>
              <CardHeader className="py-3 px-4 border-b border-border/60 flex-row items-center justify-between space-y-0">
                <CardTitle className="text-xs font-semibold flex items-center gap-2">
                  <FileCode className="size-4 text-primary" />
                  <span>Queued Transcripts ({queuedFiles.length})</span>
                </CardTitle>
                <Button
                  type="button"
                  variant="ghost"
                  size="sm"
                  onClick={() => setQueuedFiles([])}
                  className="text-2xs text-muted-foreground h-7 hover:text-danger"
                >
                  Clear Queue
                </Button>
              </CardHeader>
              <CardContent className="p-3 space-y-2">
                {queuedFiles.map((file) => {
                  const isPreviewTarget = activePreviewTarget?.path === file.path;
                  return (
                    <div
                      key={file.path}
                      className={cn(
                        "p-3 rounded-lg border text-xs flex items-center justify-between gap-3 transition-colors",
                        isPreviewTarget
                          ? "bg-primary/10 border-primary/40 ring-1 ring-primary/30"
                          : "bg-background/60 border-border/70 hover:border-border"
                      )}
                    >
                      <div className="flex items-center gap-2.5 min-w-0 flex-1">
                        <FileCode className="size-4 text-muted-foreground shrink-0" />
                        <div className="min-w-0">
                          <p className="font-mono text-foreground font-medium truncate" title={file.path}>
                            {file.name}
                          </p>
                          <p className="text-3xs text-muted-foreground font-mono truncate" title={file.path}>
                            {file.path}
                          </p>
                        </div>
                      </div>

                      <div className="flex items-center gap-2 shrink-0">
                        <Badge variant="secondary" className="text-3xs">
                          {file.agentLabel}
                        </Badge>
                        {file.sizeStr && (
                          <span className="text-3xs font-mono text-muted-foreground">
                            {file.sizeStr}
                          </span>
                        )}
                        <Button
                          type="button"
                          variant={isPreviewTarget ? "default" : "outline"}
                          size="sm"
                          disabled={isPreviewing}
                          onClick={() => handlePreviewFile(file.path)}
                          className="h-7 text-xs"
                        >
                          {isPreviewTarget ? "Active Preview" : "Preview"}
                        </Button>
                        <button
                          type="button"
                          onClick={() => removeQueuedFile(file.path)}
                          className="p-1 rounded-md text-muted-foreground hover:text-danger transition-colors cursor-pointer"
                          title="Remove from queue"
                        >
                          <Trash2 className="size-3.5" />
                        </button>
                      </div>
                    </div>
                  );
                })}
              </CardContent>
            </Card>
          )}
        </div>
      )}

      {/* Mode 2: Paste Log / Text */}
      {mode === "paste" && (
        <Card className="space-y-3">
          <CardHeader className="py-3 px-4 border-b border-border/60 flex-row items-center justify-between space-y-0">
            <div className="flex items-center gap-2">
              <Terminal className="size-4 text-primary" />
              <CardTitle className="text-xs font-semibold">
                Direct Conversation Log Paste
              </CardTitle>
            </div>
            <div className="flex items-center gap-2">
              <label className="text-2xs text-muted-foreground font-medium">
                Actor Override:
              </label>
              <select
                value={actorOverride}
                onChange={(e) => setActorOverride(e.target.value)}
                className="text-xs px-2 py-1 rounded-md bg-background border border-border text-foreground focus:outline-hidden focus:border-primary"
              >
                <option value="auto">Auto-Detect</option>
                <option value="claude">Claude</option>
                <option value="cursor">Cursor</option>
                <option value="gemini">Gemini</option>
                <option value="antigravity">Antigravity</option>
                <option value="chatgpt">ChatGPT</option>
                <option value="human">Human</option>
              </select>
            </div>
          </CardHeader>
          <CardContent className="p-4 space-y-3">
            <p className="text-xs text-muted-foreground leading-relaxed">
              Paste session transcripts, Markdown notes (e.g. <code>## User</code> / <code>## Assistant</code>), or JSON message lists. Knox deterministically extracts plans, tool writes, and decisions.
            </p>

            <textarea
              rows={10}
              value={pasteContent}
              onChange={(e) => setPasteContent(e.target.value)}
              placeholder={`Example Markdown:\n## User\nRefactor SQLite database migrations\n\n## Assistant\nUpdated schema migration logic in src/storage/migrations.rs\n\nOr paste JSON/JSONL messages directly...`}
              className="w-full p-3 rounded-lg bg-background/80 border border-border font-mono text-xs text-foreground focus:border-primary focus:outline-hidden leading-relaxed resize-y"
            />

            <div className="flex items-center justify-between pt-1">
              <span className="text-2xs text-muted-foreground font-mono">
                {pasteContent.length} characters ({pasteContent.split("\n").length} lines)
              </span>

              <div className="flex items-center gap-2">
                {pasteContent && (
                  <Button
                    type="button"
                    variant="ghost"
                    size="sm"
                    onClick={() => setPasteContent("")}
                    className="text-2xs text-muted-foreground"
                  >
                    Clear
                  </Button>
                )}
                <Button
                  type="button"
                  variant="default"
                  size="sm"
                  disabled={isPreviewing || !pasteContent.trim()}
                  onClick={handlePreviewPaste}
                >
                  <Search className="size-3.5" />
                  <span>Preview Parsed Graph</span>
                </Button>
              </div>
            </div>
          </CardContent>
        </Card>
      )}

      {/* Mode 3: Auto-Detect & Git Bootstrap */}
      {mode === "detect" && (
        <div className="space-y-4">
          {/* Git History Bootstrap Card */}
          <Card>
            <CardHeader className="py-3 px-4 border-b border-border/60 flex-row items-center justify-between space-y-0">
              <div className="flex items-center gap-2">
                <GitBranch className="size-4 text-primary" />
                <CardTitle className="text-xs font-semibold">
                  Git Repository Commit History Bootstrap
                </CardTitle>
              </div>
              <Badge variant="outline" className="text-3xs">
                Git Commit Lineage
              </Badge>
            </CardHeader>
            <CardContent className="p-4 space-y-3">
              <p className="text-xs text-muted-foreground leading-relaxed">
                Turn your active repository’s commit history into Knox knowledge graph actions. Commit subjects become plans, touched files become writes, and merges become architectural decisions.
              </p>

              <div className="flex flex-wrap items-center justify-between gap-3 p-3 rounded-lg bg-background/60 border border-border/70 text-xs">
                <div>
                  <p className="text-overline">Target Repository</p>
                  <p className="font-mono text-foreground font-medium">
                    {activeProjectScope || "Current Working Directory (Knox)"}
                  </p>
                </div>

                <div className="flex items-center gap-2">
                  <span className="text-2xs text-muted-foreground font-medium">
                    Commit Depth:
                  </span>
                  <div className="flex gap-1">
                    {[25, 50, 100, 0].map((limit) => {
                      const isSelected = gitCommitLimit === limit;
                      return (
                        <button
                          key={limit}
                          type="button"
                          onClick={() => setGitCommitLimit(limit)}
                          className={cn(
                            "px-2 py-0.5 rounded text-2xs font-mono transition-all cursor-pointer",
                            isSelected
                              ? "bg-primary text-primary-foreground font-semibold"
                              : "bg-muted text-muted-foreground hover:bg-accent"
                          )}
                        >
                          {limit === 0 ? "All" : limit}
                        </button>
                      );
                    })}
                  </div>
                </div>
              </div>

              <div className="flex justify-end pt-1">
                <Button
                  type="button"
                  variant="default"
                  size="sm"
                  disabled={isIngesting}
                  onClick={() => handleIngestGit(activeProjectScope || ".")}
                >
                  <GitBranch className="size-3.5" />
                  <span>
                    {isIngesting ? "Ingesting Commits..." : `Ingest ${gitCommitLimit === 0 ? "All" : gitCommitLimit} Git Commits`}
                  </span>
                </Button>
              </div>
            </CardContent>
          </Card>

          {/* Auto-Discovered Local Transcripts */}
          <Card>
            <CardHeader className="py-3 px-4 border-b border-border/60 flex-row items-center justify-between space-y-0">
              <div className="flex items-center gap-2">
                <Sparkles className="size-4 text-primary" />
                <CardTitle className="text-xs font-semibold">
                  Discovered Local Agent Transcripts
                </CardTitle>
              </div>
              <Button
                type="button"
                variant="ghost"
                size="sm"
                disabled={isScanning}
                onClick={handleScanLocal}
                className="h-7 text-2xs text-muted-foreground"
              >
                <RefreshCw className={cn("size-3", isScanning && "animate-spin")} />
                <span>Rescan</span>
              </Button>
            </CardHeader>
            <CardContent className="p-3 space-y-2">
              {detectedTranscripts.length === 0 ? (
                <p className="text-xs text-muted-foreground text-center py-4">
                  {isScanning ? "Scanning local drives..." : "No local agent transcripts detected. Drop files or paste logs above."}
                </p>
              ) : (
                detectedTranscripts.map((t) => (
                  <div
                    key={t.path}
                    className="p-3 rounded-lg border border-border/70 bg-background/60 flex items-center justify-between gap-3 text-xs"
                  >
                    <div className="flex items-center gap-2.5 min-w-0 flex-1">
                      <FileCode className="size-4 text-primary shrink-0" />
                      <div className="min-w-0">
                        <p className="font-mono text-foreground font-medium truncate" title={t.path}>
                          {t.path}
                        </p>
                        <p className="text-3xs text-muted-foreground">
                          {t.source} • ~{t.estimated_actions} actions
                        </p>
                      </div>
                    </div>

                    <div className="flex items-center gap-2 shrink-0">
                      <Button
                        type="button"
                        variant="outline"
                        size="sm"
                        disabled={isPreviewing}
                        onClick={() => {
                          addQueuedFile(t.path);
                          handlePreviewFile(t.path);
                        }}
                        className="h-7 text-xs"
                      >
                        Preview & Ingest
                      </Button>
                    </div>
                  </div>
                ))
              )}
            </CardContent>
          </Card>
        </div>
      )}

      {/* Error Alert */}
      {ingestError && (
        <div
          role="alert"
          className="p-3 rounded-lg border border-danger/40 bg-danger/10 text-xs text-danger flex items-start gap-2.5"
        >
          <AlertTriangle className="size-4 shrink-0 mt-0.5" />
          <div className="space-y-0.5">
            <p className="font-semibold">Ingestion Error</p>
            <p className="font-mono text-2xs break-all">{ingestError}</p>
          </div>
        </div>
      )}

      {/* Interactive Dry-Run Staging Preview Panel */}
      {preview && (
        <Card className="border-primary/40 shadow-lg motion-safe:animate-in motion-safe:fade-in">
          <CardHeader className="py-3 px-4 border-b border-border/60 flex-row items-center justify-between space-y-0 bg-primary/5">
            <div className="flex items-center gap-2">
              <Layers className="size-4 text-primary" />
              <CardTitle className="text-xs font-semibold">
                Staging Preview — {preview.tool_detected.toUpperCase()} Format
              </CardTitle>
            </div>
            <div className="flex items-center gap-2">
              <Badge variant="default" className="text-3xs uppercase font-mono">
                {preview.tool_detected}
              </Badge>
              {preview.duplicates_count > 0 && (
                <Badge variant="warning" className="text-3xs">
                  {preview.duplicates_count} Duplicates (Will Skip)
                </Badge>
              )}
            </div>
          </CardHeader>
          <CardContent className="p-4 space-y-4">
            {/* KPI Metric Breakdown */}
            <div className="grid grid-cols-2 sm:grid-cols-5 gap-2">
              <div className="p-2.5 rounded-lg bg-card/60 border border-border text-center">
                <p className="text-2xs text-muted-foreground uppercase font-semibold">Total Actions</p>
                <p className="text-lg font-bold font-mono text-foreground">{preview.total_actions}</p>
              </div>
              <div className="p-2.5 rounded-lg bg-card/60 border border-border text-center">
                <p className="text-2xs text-primary uppercase font-semibold">Plans</p>
                <p className="text-lg font-bold font-mono text-primary">{preview.plans_count}</p>
              </div>
              <div className="p-2.5 rounded-lg bg-card/60 border border-border text-center">
                <p className="text-2xs text-success uppercase font-semibold">File Writes</p>
                <p className="text-lg font-bold font-mono text-success">{preview.writes_count}</p>
              </div>
              <div className="p-2.5 rounded-lg bg-card/60 border border-border text-center">
                <p className="text-2xs text-warning uppercase font-semibold">Debug / Tests</p>
                <p className="text-lg font-bold font-mono text-warning">{preview.debugs_count}</p>
              </div>
              <div className="p-2.5 rounded-lg bg-card/60 border border-border text-center">
                <p className="text-2xs text-indigo-400 uppercase font-semibold">Decisions</p>
                <p className="text-lg font-bold font-mono text-indigo-400">{preview.decisions_count}</p>
              </div>
            </div>

            {/* Affected Files List */}
            {preview.files_touched.length > 0 && (
              <div className="space-y-1.5">
                <p className="text-2xs font-semibold text-muted-foreground uppercase tracking-wider">
                  Referenced Code Files ({preview.files_touched.length})
                </p>
                <div className="flex flex-wrap gap-1.5 max-h-24 overflow-y-auto p-2 rounded-lg bg-background/50 border border-border/60">
                  {preview.files_touched.map((f) => (
                    <span
                      key={f}
                      className="px-2 py-0.5 rounded text-3xs font-mono bg-muted text-foreground border border-border/80"
                    >
                      {f}
                    </span>
                  ))}
                </div>
              </div>
            )}

            {/* Sample Action Cards */}
            {preview.sample_actions.length > 0 && (
              <div className="space-y-2">
                <p className="text-2xs font-semibold text-muted-foreground uppercase tracking-wider">
                  Sample Actions Extracted ({preview.sample_actions.length} of {preview.total_actions})
                </p>
                <div className="space-y-1.5 max-h-56 overflow-y-auto">
                  {preview.sample_actions.map((act) => {
                    const isExpanded = expandedActionId === act.id;
                    return (
                      <div
                        key={act.id}
                        onClick={() => setExpandedActionId(isExpanded ? null : act.id)}
                        className="p-2.5 rounded-lg border border-border/70 bg-background/60 hover:bg-background cursor-pointer text-xs space-y-1.5 transition-colors"
                      >
                        <div className="flex items-center justify-between gap-2">
                          <div className="flex items-center gap-2 min-w-0 flex-1">
                            <ActionBadge type={act.action_type} />
                            <ActorBadge actor={act.actor} />
                            <p className="font-medium text-foreground truncate">{act.summary}</p>
                          </div>
                          <span className="text-3xs font-mono text-muted-foreground shrink-0">
                            {act.files.length} file(s)
                          </span>
                        </div>

                        {isExpanded && act.files.length > 0 && (
                          <div className="pt-1.5 border-t border-border/50 flex flex-wrap gap-1">
                            {act.files.map((f) => (
                              <span key={f} className="text-3xs font-mono text-muted-foreground bg-muted/60 px-1.5 py-0.5 rounded">
                                {f}
                              </span>
                            ))}
                          </div>
                        )}
                      </div>
                    );
                  })}
                </div>
              </div>
            )}

            {/* Confirmation Footer */}
            <div className="pt-3 border-t border-border/60 flex items-center justify-between">
              <Button
                type="button"
                variant="ghost"
                size="sm"
                onClick={() => {
                  setPreview(null);
                  setActivePreviewTarget(null);
                }}
              >
                Cancel Preview
              </Button>

              <Button
                type="button"
                variant="default"
                size="sm"
                disabled={isIngesting || preview.total_actions === 0}
                onClick={handleConfirmIngest}
              >
                <CheckCircle2 className="size-3.5" />
                <span>
                  {isIngesting
                    ? "Writing to Knowledge Graph..."
                    : `Confirm & Ingest ${preview.total_actions - preview.duplicates_count} Actions`}
                </span>
              </Button>
            </div>
          </CardContent>
        </Card>
      )}

      {/* Post-Ingestion Success Report */}
      {report && (
        <Card className="border-success/40 bg-success/5 shadow-md">
          <CardHeader className="py-3 px-4 border-b border-success/20 flex-row items-center justify-between space-y-0">
            <div className="flex items-center gap-2">
              <CheckCircle2 className="size-4 text-success" />
              <CardTitle className="text-xs font-semibold text-success">
                Ingestion Complete — Session Recorded
              </CardTitle>
            </div>
            {report.tool_detected && (
              <Badge variant="outline" className="text-3xs uppercase font-mono">
                {report.tool_detected}
              </Badge>
            )}
          </CardHeader>
          <CardContent className="p-4 space-y-4">
            <div className="grid grid-cols-2 sm:grid-cols-4 gap-2 text-center text-xs">
              <div className="p-2.5 rounded-lg bg-background/80 border border-border">
                <p className="text-2xs text-muted-foreground uppercase">Actions Stored</p>
                <p className="text-base font-bold font-mono text-foreground">
                  {report.actions_imported}
                </p>
              </div>
              <div className="p-2.5 rounded-lg bg-background/80 border border-border">
                <p className="text-2xs text-muted-foreground uppercase">Files Touched</p>
                <p className="text-base font-bold font-mono text-foreground">
                  {report.files_touched}
                </p>
              </div>
              <div className="p-2.5 rounded-lg bg-background/80 border border-border">
                <p className="text-2xs text-muted-foreground uppercase">Sessions Created</p>
                <p className="text-base font-bold font-mono text-foreground">
                  {report.sessions_created}
                </p>
              </div>
              <div className="p-2.5 rounded-lg bg-background/80 border border-border">
                <p className="text-2xs text-muted-foreground uppercase">Duplicates Skipped</p>
                <p className="text-base font-bold font-mono text-foreground">
                  {report.duplicates_skipped || 0}
                </p>
              </div>
            </div>

            {/* Direct Navigation Action Links */}
            <div className="pt-2 flex flex-wrap items-center justify-end gap-2">
              <Button
                type="button"
                variant="outline"
                size="sm"
                onClick={() => setActiveTab("query")}
                className="gap-1.5"
              >
                <Search className="size-3.5" />
                <span>Search Ingested Actions</span>
              </Button>
              <Button
                type="button"
                variant="default"
                size="sm"
                onClick={() => setActiveTab("graph")}
                className="gap-1.5"
              >
                <Network className="size-3.5" />
                <span>Explore in Knowledge Graph</span>
              </Button>
            </div>
          </CardContent>
        </Card>
      )}

      {/* Default Empty State when idle */}
      {!preview && !report && queuedFiles.length === 0 && mode === "files" && (
        <EmptyState
          compact
          icon={<FileInput />}
          title="No transcripts in staging queue"
          description="Drop session logs from Antigravity, Claude, Cursor, ChatGPT, or Git to parse plans, decisions, and file writes into Knox."
        />
      )}
    </div>
  );
};
