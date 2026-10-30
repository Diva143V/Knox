import React, { useState, useEffect, useCallback } from "react";
import {
  FolderGit2,
  Sparkles,
  CheckCircle2,
  FileText,
  FolderOpen,
  GitCommit,
  Check,
  ChevronRight,
  RefreshCw,
  Copy,
  AlertTriangle,
  Trash2,
} from "lucide-react";
import { KnoxAPI } from "@/services/api";
import { useKnoxStore } from "@/store/useKnoxStore";

type WizardStep = 1 | 2 | 3 | 4;

export const WorkspaceView: React.FC = () => {
  const { showToast, refreshData } = useKnoxStore();
  const [workspacePath, setWorkspacePath] = useState(".");
  const [currentStep, setCurrentStep] = useState<WizardStep>(1);
  const [knoxMdContent, setKnoxMdContent] = useState<string | null>(null);
  const [isInitializing, setIsInitializing] = useState(false);
  const [isInstallingHook, setIsInstallingHook] = useState(false);
  const [hookInstalled, setHookInstalled] = useState<boolean>(false);
  const [copiedKnoxMd, setCopiedKnoxMd] = useState(false);
  const [errorMessage, setErrorMessage] = useState<string | null>(null);

  // Load knox.md and check hook status
  const checkStatus = useCallback(async (path: string) => {
    try {
      const [md, hook] = await Promise.all([
        KnoxAPI.getKnoxMd(path),
        KnoxAPI.getHookStatus(path),
      ]);
      setKnoxMdContent(md);
      setHookInstalled(hook);

      // Determine step based on actual backend status
      if (md && hook) {
        setCurrentStep(4);
      } else if (md) {
        setCurrentStep(3);
      } else {
        setCurrentStep(1);
      }
    } catch {
      setKnoxMdContent(null);
      setHookInstalled(false);
    }
  }, []);

  useEffect(() => {
    checkStatus(workspacePath);
  }, [workspacePath, checkStatus]);

  const handleBrowseFolder = async () => {
    try {
      const folder = await KnoxAPI.openFolderDialog();
      if (folder) {
        setWorkspacePath(folder);
        setErrorMessage(null);
      }
    } catch (err) {
      showToast(String(err), true);
    }
  };

  const handleInit = async () => {
    try {
      setIsInitializing(true);
      setErrorMessage(null);
      const res = await KnoxAPI.initProject(workspacePath);
      showToast(res);
      await checkStatus(workspacePath);
      await refreshData();
      setCurrentStep(3);
    } catch (err) {
      const msg = String(err);
      setErrorMessage(msg);
      showToast(msg, true);
    } finally {
      setIsInitializing(false);
    }
  };

  const handleInstallHook = async () => {
    try {
      setIsInstallingHook(true);
      setErrorMessage(null);
      const res = await KnoxAPI.installHook(workspacePath);
      showToast(res);
      await checkStatus(workspacePath);
      setCurrentStep(4);
    } catch (err) {
      const msg = String(err);
      setErrorMessage(msg);
      showToast(msg, true);
    } finally {
      setIsInstallingHook(false);
    }
  };

  const handleUninstallHook = async () => {
    try {
      setIsInstallingHook(true);
      setErrorMessage(null);
      const res = await KnoxAPI.uninstallHook(workspacePath);
      showToast(res);
      await checkStatus(workspacePath);
    } catch (err) {
      const msg = String(err);
      setErrorMessage(msg);
      showToast(msg, true);
    } finally {
      setIsInstallingHook(false);
    }
  };

  const handleCopyKnoxMd = () => {
    if (knoxMdContent) {
      navigator.clipboard.writeText(knoxMdContent);
      setCopiedKnoxMd(true);
      showToast("Copied knox.md content to clipboard");
      setTimeout(() => setCopiedKnoxMd(false), 2000);
    }
  };

  const steps = [
    {
      num: 1,
      title: "Select Path",
      desc: "Target codebase directory",
      isComplete: workspacePath.trim().length > 0,
    },
    {
      num: 2,
      title: "Initialize",
      desc: "Generate knox.md & register",
      isComplete: knoxMdContent !== null,
    },
    {
      num: 3,
      title: "Git Hook",
      desc: "Install pre-commit hook",
      isComplete: hookInstalled,
    },
    {
      num: 4,
      title: "Verify",
      desc: "Review & verify knox.md",
      isComplete: knoxMdContent !== null && hookInstalled,
    },
  ];

  return (
    <div className="p-6 space-y-6 max-w-4xl mx-auto overflow-y-auto h-full select-none">
      {/* Header */}
      <div>
        <h1 className="text-xl font-bold tracking-tight text-foreground flex items-center gap-2">
          <FolderGit2 className="w-5 h-5 text-primary" />
          <span>Workspace Setup & knox.md Generator</span>
        </h1>
        <p className="text-xs text-muted-foreground mt-1">
          Configure a codebase for Knox. Generates a repository instruction file (<code className="text-foreground font-mono">knox.md</code>)
          and git hook that enforce causal tracking for AI agent sessions.
        </p>
      </div>

      {/* 4-Step Wizard Navigation */}
      <div className="grid grid-cols-2 md:grid-cols-4 gap-2.5">
        {steps.map((s) => {
          const isActive = currentStep === s.num;
          return (
            <div
              key={s.num}
              onClick={() => setCurrentStep(s.num as WizardStep)}
              className={`p-3 rounded-xl border transition cursor-pointer ${
                isActive
                  ? "bg-card border-primary ring-1 ring-primary/40 shadow-sm"
                  : s.isComplete
                  ? "bg-card/60 border-emerald-500/30 hover:border-emerald-500/50"
                  : "bg-card/40 border-border hover:border-border/80"
              }`}
            >
              <div className="flex items-center justify-between mb-1">
                <span
                  className={`w-5 h-5 rounded-full flex items-center justify-center text-[10px] font-mono font-bold ${
                    s.isComplete
                      ? "bg-emerald-500/20 text-emerald-400 border border-emerald-500/40"
                      : isActive
                      ? "bg-primary text-primary-foreground"
                      : "bg-muted text-muted-foreground"
                  }`}
                >
                  {s.isComplete ? <Check className="w-3 h-3" /> : s.num}
                </span>
                {s.isComplete && (
                  <span className="text-[9px] font-mono text-emerald-400">Ready</span>
                )}
              </div>
              <h4 className="text-xs font-semibold text-foreground">{s.title}</h4>
              <p className="text-[10px] text-muted-foreground truncate">{s.desc}</p>
            </div>
          );
        })}
      </div>

      {/* Error state if backend command fails */}
      {errorMessage && (
        <div className="p-3.5 rounded-xl border border-rose-500/30 bg-rose-500/10 text-rose-300 text-xs flex items-center gap-2">
          <AlertTriangle className="w-4 h-4 text-rose-400 shrink-0" />
          <span className="leading-snug">{errorMessage}</span>
        </div>
      )}

      {/* Wizard Step Content Card */}
      <div className="p-5 rounded-xl border border-border bg-card space-y-4">
        {/* Step 1: Select Path */}
        {currentStep === 1 && (
          <div className="space-y-4">
            <div>
              <h3 className="text-sm font-semibold text-foreground">Step 1: Choose Codebase Directory</h3>
              <p className="text-xs text-muted-foreground mt-0.5">
                Specify the root folder of your project repository where Knox should track actions.
              </p>
            </div>

            <div className="flex gap-2">
              <input
                type="text"
                value={workspacePath}
                onChange={(e) => setWorkspacePath(e.target.value)}
                placeholder="e.g. D:\Projects\MyRepo or ."
                className="flex-1 bg-background border border-border rounded-lg px-3 py-2 text-xs font-mono text-foreground focus:outline-none focus:border-primary"
              />
              <button
                onClick={handleBrowseFolder}
                className="flex items-center gap-1.5 px-3 py-2 rounded-lg border border-border bg-card hover:bg-background text-xs font-medium text-foreground transition cursor-pointer"
              >
                <FolderOpen className="w-3.5 h-3.5 text-muted-foreground" />
                <span>Browse…</span>
              </button>
            </div>

            <div className="flex justify-end pt-2">
              <button
                onClick={() => setCurrentStep(2)}
                className="flex items-center gap-1.5 px-4 py-2 rounded-lg bg-primary text-primary-foreground text-xs font-semibold hover:bg-primary/90 transition cursor-pointer shadow-sm shadow-primary/25"
              >
                <span>Continue to Step 2</span>
                <ChevronRight className="w-3.5 h-3.5" />
              </button>
            </div>
          </div>
        )}

        {/* Step 2: Initialize Workspace */}
        {currentStep === 2 && (
          <div className="space-y-4">
            <div>
              <h3 className="text-sm font-semibold text-foreground">Step 2: Initialize Project & knox.md</h3>
              <p className="text-xs text-muted-foreground mt-0.5">
                Knox will generate a standardized <code className="text-foreground font-mono">knox.md</code> file instructing AI tools (Cursor, Claude, Antigravity) to query context before acting.
              </p>
            </div>

            <div className="p-3.5 rounded-lg border border-border/60 bg-background/50 space-y-2 text-xs">
              <div className="flex items-center justify-between text-muted-foreground">
                <span>Target Repository:</span>
                <span className="font-mono text-foreground font-semibold">{workspacePath}</span>
              </div>
              <div className="flex items-center justify-between text-muted-foreground">
                <span>Instruction File:</span>
                <span className="font-mono text-foreground">knox.md (Root directory)</span>
              </div>
              <div className="flex items-center justify-between text-muted-foreground">
                <span>Database Status:</span>
                <span className="font-mono text-emerald-400">Registers project in Knox SQLite</span>
              </div>
            </div>

            <div className="flex items-center justify-between pt-2">
              <button
                onClick={() => setCurrentStep(1)}
                className="px-3 py-1.5 rounded-lg text-xs text-muted-foreground hover:text-foreground transition cursor-pointer"
              >
                Back to Path
              </button>
              <button
                onClick={handleInit}
                disabled={isInitializing}
                className="flex items-center gap-1.5 px-4 py-2 rounded-lg bg-primary text-primary-foreground text-xs font-semibold hover:bg-primary/90 transition cursor-pointer shadow-sm shadow-primary/25 disabled:opacity-50"
              >
                {isInitializing ? (
                  <>
                    <RefreshCw className="w-3.5 h-3.5 animate-spin" />
                    <span>Initializing knox.md…</span>
                  </>
                ) : (
                  <>
                    <Sparkles className="w-3.5 h-3.5" />
                    <span>Initialize Workspace</span>
                  </>
                )}
              </button>
            </div>
          </div>
        )}

        {/* Step 3: Git Hook Setup */}
        {currentStep === 3 && (
          <div className="space-y-4">
            <div>
              <h3 className="text-sm font-semibold text-foreground">Step 3: Install Git Pre-Commit Hook</h3>
              <p className="text-xs text-muted-foreground mt-0.5">
                Install the automated git hook into <code className="text-foreground font-mono">.git/hooks/pre-commit</code>. Every commit will scan staged changes to record human developer contributions.
              </p>
            </div>

            <div className="p-4 rounded-xl border border-border/80 bg-background/50 flex items-center justify-between">
              <div className="flex items-center gap-3">
                <div className="w-9 h-9 rounded-lg bg-primary/10 border border-primary/20 flex items-center justify-center text-primary">
                  <GitCommit className="w-5 h-5" />
                </div>
                <div>
                  <h4 className="text-xs font-semibold text-foreground">Pre-Commit Git Hook</h4>
                  <p className="text-[11px] text-muted-foreground mt-0.5">
                    {hookInstalled
                      ? "Hook is currently installed and active in this repository."
                      : "Hook is not yet installed in .git/hooks/pre-commit."}
                  </p>
                </div>
              </div>

              {hookInstalled ? (
                <div className="flex items-center gap-2">
                  <span className="flex items-center gap-1 text-xs font-mono text-emerald-400 bg-emerald-500/10 px-2.5 py-1 rounded-lg border border-emerald-500/30">
                    <Check className="w-3.5 h-3.5" />
                    <span>Installed</span>
                  </span>
                  <button
                    onClick={handleUninstallHook}
                    disabled={isInstallingHook}
                    className="flex items-center gap-1 px-2.5 py-1 rounded-lg border border-rose-500/30 bg-rose-500/10 hover:bg-rose-500/20 text-xs font-medium text-rose-400 transition cursor-pointer disabled:opacity-50"
                  >
                    <Trash2 className="w-3.5 h-3.5" />
                    <span>Uninstall</span>
                  </button>
                </div>
              ) : (
                <button
                  onClick={handleInstallHook}
                  disabled={isInstallingHook}
                  className="flex items-center gap-1.5 px-4 py-2 rounded-lg bg-primary text-primary-foreground text-xs font-semibold hover:bg-primary/90 transition cursor-pointer shadow-sm shadow-primary/25 disabled:opacity-50"
                >
                  {isInstallingHook ? (
                    <>
                      <RefreshCw className="w-3.5 h-3.5 animate-spin" />
                      <span>Installing…</span>
                    </>
                  ) : (
                    <>
                      <Check className="w-3.5 h-3.5" />
                      <span>Install Hook</span>
                    </>
                  )}
                </button>
              )}
            </div>

            <div className="flex items-center justify-between pt-2">
              <button
                onClick={() => setCurrentStep(2)}
                className="px-3 py-1.5 rounded-lg text-xs text-muted-foreground hover:text-foreground transition cursor-pointer"
              >
                Back to Init
              </button>
              <button
                onClick={() => setCurrentStep(4)}
                className="flex items-center gap-1.5 px-4 py-2 rounded-lg bg-primary text-primary-foreground text-xs font-semibold hover:bg-primary/90 transition cursor-pointer shadow-sm shadow-primary/25"
              >
                <span>Continue to Verification</span>
                <ChevronRight className="w-3.5 h-3.5" />
              </button>
            </div>
          </div>
        )}

        {/* Step 4: Verification & Preview */}
        {currentStep === 4 && (
          <div className="space-y-4">
            <div>
              <h3 className="text-sm font-semibold text-foreground">Step 4: Verification & knox.md Specification</h3>
              <p className="text-xs text-muted-foreground mt-0.5">
                Knox workspace setup is fully verified. Review the generated instruction file below.
              </p>
            </div>

            {/* Verification checklist badges */}
            <div className="grid grid-cols-1 sm:grid-cols-3 gap-2">
              <div className="flex items-center gap-2 p-2.5 rounded-lg bg-background border border-border/70 text-xs">
                <CheckCircle2 className="w-4 h-4 text-emerald-400 shrink-0" />
                <span className="text-foreground font-medium">knox.md Generated</span>
              </div>
              <div className="flex items-center gap-2 p-2.5 rounded-lg bg-background border border-border/70 text-xs">
                {hookInstalled ? (
                  <CheckCircle2 className="w-4 h-4 text-emerald-400 shrink-0" />
                ) : (
                  <AlertTriangle className="w-4 h-4 text-amber-400 shrink-0" />
                )}
                <span className="text-foreground font-medium">
                  {hookInstalled ? "Git Hook Active" : "Hook Optional"}
                </span>
              </div>
              <div className="flex items-center gap-2 p-2.5 rounded-lg bg-background border border-border/70 text-xs">
                <CheckCircle2 className="w-4 h-4 text-emerald-400 shrink-0" />
                <span className="text-foreground font-medium">SQLite WAL Tracking</span>
              </div>
            </div>
          </div>
        )}
      </div>

      {/* Live knox.md Preview Card */}
      {knoxMdContent ? (
        <div className="p-5 rounded-xl border border-border bg-card space-y-3">
          <div className="flex items-center justify-between pb-2 border-b border-border/60">
            <span className="text-xs font-semibold text-foreground flex items-center gap-2">
              <FileText className="w-4 h-4 text-primary" />
              <span>Generated knox.md Specification</span>
            </span>
            <button
              onClick={handleCopyKnoxMd}
              className="flex items-center gap-1.5 px-2.5 py-1 rounded bg-background hover:bg-card border border-border text-xs text-foreground cursor-pointer transition"
            >
              {copiedKnoxMd ? (
                <>
                  <Check className="w-3 h-3 text-emerald-400" />
                  <span className="text-emerald-400">Copied</span>
                </>
              ) : (
                <>
                  <Copy className="w-3 h-3" />
                  <span>Copy knox.md</span>
                </>
              )}
            </button>
          </div>

          <pre className="p-4 rounded-lg bg-background font-mono text-xs text-foreground border border-border/60 overflow-x-auto max-h-[380px] leading-relaxed select-text">
            {knoxMdContent}
          </pre>
        </div>
      ) : (
        /* Empty State */
        <div className="p-8 rounded-xl border border-dashed border-border bg-card/40 text-center space-y-2">
          <FileText className="w-8 h-8 text-muted-foreground/60 mx-auto" />
          <h4 className="text-xs font-semibold text-foreground">No knox.md Detected</h4>
          <p className="text-[11px] text-muted-foreground max-w-sm mx-auto">
            Click &ldquo;Initialize Workspace&rdquo; in Step 2 above to create a standardized instructions file for your agents.
          </p>
        </div>
      )}
    </div>
  );
};
