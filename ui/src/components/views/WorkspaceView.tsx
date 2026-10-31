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
  AlertTriangle,
  Trash2,
  Database,
  Network,
} from "lucide-react";
import { KnoxAPI } from "@/services/api";
import { useKnoxStore } from "@/store/useKnoxStore";
import type { KnoxConfig } from "@/types/knox";
import { PageHeader } from "@/components/ui/PageHeader";
import { EmptyState } from "@/components/ui/EmptyState";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/Card";
import { Button } from "@/components/ui/Button";
import { Input } from "@/components/ui/Input";
import { Badge } from "@/components/ui/Badge";
import { CodeBlock } from "@/components/ui/CodeBlock";
import { cn } from "@/lib/cn";

type WizardStep = 1 | 2 | 3 | 4;

export const WorkspaceView: React.FC = () => {
  const { showToast, refreshData, setActiveTab } = useKnoxStore();
  const [workspacePath, setWorkspacePath] = useState(".");
  const [currentStep, setCurrentStep] = useState<WizardStep>(1);
  const [knoxMdContent, setKnoxMdContent] = useState<string | null>(null);
  const [isInitializing, setIsInitializing] = useState(false);
  const [isInstallingHook, setIsInstallingHook] = useState(false);
  const [hookInstalled, setHookInstalled] = useState(false);
  const [errorMessage, setErrorMessage] = useState<string | null>(null);
  const [config, setConfig] = useState<KnoxConfig | null>(null);

  const checkStatus = useCallback(async (path: string, autoAdvance = false) => {
    try {
      const [md, hook] = await Promise.all([
        KnoxAPI.getKnoxMd(path),
        KnoxAPI.getHookStatus(path),
      ]);
      setKnoxMdContent(md);
      setHookInstalled(hook);

      if (autoAdvance) {
        if (md && hook) {
          setCurrentStep(4);
        } else if (md) {
          setCurrentStep(3);
        } else {
          setCurrentStep(1);
        }
      }
    } catch {
      setKnoxMdContent(null);
      setHookInstalled(false);
    }
  }, []);

  // Check and auto-advance ONLY on initial mount
  useEffect(() => {
    checkStatus(workspacePath, true);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  // Refresh content on path change without interrupting user step navigation
  useEffect(() => {
    checkStatus(workspacePath, false);
  }, [workspacePath, checkStatus]);

  useEffect(() => {
    let cancelled = false;
    KnoxAPI.getConfig()
      .then((cfg) => {
        if (!cancelled) setConfig(cfg);
      })
      .catch(() => {
        if (!cancelled) setConfig(null);
      });
    return () => {
      cancelled = true;
    };
  }, []);

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
      await checkStatus(workspacePath, false);
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
      await checkStatus(workspacePath, false);
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
    const ok = window.confirm(
      "Uninstall the Knox pre-commit hook from this repository?"
    );
    if (!ok) return;
    try {
      setIsInstallingHook(true);
      setErrorMessage(null);
      const res = await KnoxAPI.uninstallHook(workspacePath);
      showToast(res);
      await checkStatus(workspacePath, false);
    } catch (err) {
      const msg = String(err);
      setErrorMessage(msg);
      showToast(msg, true);
    } finally {
      setIsInstallingHook(false);
    }
  };

  const steps = [
    {
      num: 1 as WizardStep,
      title: "Select path",
      desc: "Target codebase directory",
      isComplete: workspacePath.trim().length > 0,
    },
    {
      num: 2 as WizardStep,
      title: "Initialize",
      desc: "Generate knox.md & register",
      isComplete: knoxMdContent !== null,
    },
    {
      num: 3 as WizardStep,
      title: "Git hook",
      desc: "Install pre-commit hook",
      isComplete: hookInstalled,
    },
    {
      num: 4 as WizardStep,
      title: "Verify",
      desc: "Review knox.md",
      isComplete: knoxMdContent !== null && hookInstalled,
    },
  ];

  return (
    <div className="p-5 sm:p-6 space-y-5 max-w-4xl mx-auto overflow-y-auto h-full">
      <PageHeader
        title="Workspace"
        description="Initialize a repository with knox.md and optionally install the pre-commit hook that records human edits on commit."
        icon={<FolderGit2 />}
      />

      {config && (
        <Card>
          <CardContent className="p-4 flex flex-col sm:flex-row sm:items-center gap-3 text-xs">
            <div className="flex items-center gap-2 min-w-0 flex-1">
              <Database className="size-4 text-primary shrink-0" aria-hidden />
              <div className="min-w-0">
                <p className="text-overline">Knowledge graph database</p>
                <p
                  className="font-mono text-foreground truncate"
                  title={config.kg_path}
                >
                  {config.kg_path}
                </p>
              </div>
            </div>
            <div className="flex flex-wrap gap-2 shrink-0">
              <Badge variant="secondary">
                timeout {config.session_timeout_seconds}s
              </Badge>
              <Badge variant={config.global_mode ? "default" : "outline"}>
                {config.global_mode ? "global mode" : "project mode"}
              </Badge>
            </div>
          </CardContent>
        </Card>
      )}

      <div className="grid grid-cols-2 md:grid-cols-4 gap-2">
        {steps.map((s) => {
          const isActive = currentStep === s.num;
          return (
            <button
              key={s.num}
              type="button"
              onClick={() => setCurrentStep(s.num)}
              aria-current={isActive ? "step" : undefined}
              className={cn(
                "p-3 rounded-lg border text-left transition-colors",
                "focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-focus-ring",
                isActive
                  ? "bg-card border-primary"
                  : s.isComplete
                    ? "bg-card/60 border-success/30 hover:border-success/50"
                    : "bg-card/40 border-border hover:border-border"
              )}
            >
              <div className="flex items-center justify-between mb-1">
                <span
                  className={cn(
                    "size-5 rounded-full flex items-center justify-center text-xs font-mono font-bold",
                    s.isComplete
                      ? "bg-success/20 text-success border border-success/40"
                      : isActive
                        ? "bg-primary text-primary-foreground"
                        : "bg-muted text-muted-foreground"
                  )}
                >
                  {s.isComplete ? <Check className="size-3" aria-hidden /> : s.num}
                </span>
                {s.isComplete && (
                  <span className="text-2xs font-mono text-success">Ready</span>
                )}
              </div>
              <h4 className="text-xs font-semibold text-foreground">{s.title}</h4>
              <p className="text-xs text-muted-foreground truncate">{s.desc}</p>
            </button>
          );
        })}
      </div>

      {errorMessage && (
        <div
          role="alert"
          className="px-3 py-2 rounded-md border border-danger/40 bg-danger/10 text-xs text-danger flex gap-2"
        >
          <AlertTriangle className="size-3.5 shrink-0 mt-0.5" aria-hidden />
          <span>{errorMessage}</span>
        </div>
      )}

      <Card>
        <CardContent className="p-5 space-y-4">
          {currentStep === 1 && (
            <div className="space-y-4">
              <div>
                <h3 className="text-sm font-semibold text-foreground">
                  Step 1: Choose codebase directory
                </h3>
                <p className="text-xs text-muted-foreground mt-0.5">
                  Root folder of the repository where Knox should track actions.
                </p>
              </div>

              <div className="space-y-1.5">
                <label htmlFor="workspace-path" className="text-overline block">
                  Repository path
                </label>
                <div className="flex items-center gap-2">
                  <Input
                    id="workspace-path"
                    type="text"
                    value={workspacePath}
                    onChange={(e) => setWorkspacePath(e.target.value)}
                    placeholder="e.g. D:\Projects\MyRepo or ."
                    className="flex-1 min-w-0"
                  />
                  <Button
                    type="button"
                    variant="outline"
                    onClick={handleBrowseFolder}
                    className="shrink-0"
                  >
                    <FolderOpen className="size-3.5" />
                    Browse
                  </Button>
                </div>
              </div>

              <div className="flex justify-end pt-1">
                <Button
                  type="button"
                  variant="primary"
                  onClick={() => setCurrentStep(2)}
                  disabled={!workspacePath.trim()}
                >
                  Continue
                  <ChevronRight className="size-3.5" />
                </Button>
              </div>
            </div>
          )}

          {currentStep === 2 && (
            <div className="space-y-4">
              <div>
                <h3 className="text-sm font-semibold text-foreground">
                  Step 2: Initialize project & knox.md
                </h3>
                <p className="text-xs text-muted-foreground mt-0.5">
                  Generates{" "}
                  <code className="text-foreground font-mono">knox.md</code> so
                  agents know to query Knox context before acting, and registers
                  the project.
                </p>
              </div>

              <div className="p-3 rounded-md border border-border bg-background space-y-2 text-xs">
                <div className="flex items-center justify-between gap-2 text-muted-foreground">
                  <span>Target repository</span>
                  <span
                    className="font-mono text-foreground font-semibold truncate max-w-[60%]"
                    title={workspacePath}
                  >
                    {workspacePath}
                  </span>
                </div>
                <div className="flex items-center justify-between text-muted-foreground">
                  <span>Instruction file</span>
                  <span className="font-mono text-foreground">knox.md</span>
                </div>
              </div>

              <div className="flex items-center justify-between pt-1">
                <Button
                  type="button"
                  variant="ghost"
                  onClick={() => setCurrentStep(1)}
                >
                  Back
                </Button>
                <Button
                  type="button"
                  variant="primary"
                  onClick={handleInit}
                  loading={isInitializing}
                  disabled={isInitializing}
                >
                  {!isInitializing && <Sparkles className="size-3.5" />}
                  {isInitializing ? "Initializing…" : "Initialize workspace"}
                </Button>
              </div>
            </div>
          )}

          {currentStep === 3 && (
            <div className="space-y-4">
              <div>
                <h3 className="text-sm font-semibold text-foreground">
                  Step 3: Git pre-commit hook
                </h3>
                <p className="text-xs text-muted-foreground mt-0.5">
                  Optional. Installs into{" "}
                  <code className="text-foreground font-mono">
                    .git/hooks/pre-commit
                  </code>{" "}
                  so each commit can scan staged changes for human edits.
                </p>
              </div>

              <div className="p-4 rounded-md border border-border bg-background flex flex-col sm:flex-row sm:items-center justify-between gap-3">
                <div className="flex items-center gap-3 min-w-0">
                  <div className="size-9 rounded-md bg-primary/10 border border-primary/20 flex items-center justify-center text-primary shrink-0">
                    <GitCommit className="size-5" aria-hidden />
                  </div>
                  <div className="min-w-0">
                    <h4 className="text-xs font-semibold text-foreground">
                      Pre-commit hook
                    </h4>
                    <p className="text-xs text-muted-foreground mt-0.5">
                      {hookInstalled
                        ? "Installed in this repository."
                        : "Not installed yet."}
                    </p>
                  </div>
                </div>

                {hookInstalled ? (
                  <div className="flex items-center gap-2 shrink-0">
                    <Badge variant="success" className="gap-1">
                      <Check className="size-3" aria-hidden />
                      Installed
                    </Badge>
                    <Button
                      type="button"
                      variant="danger"
                      size="sm"
                      onClick={handleUninstallHook}
                      loading={isInstallingHook}
                      disabled={isInstallingHook}
                    >
                      {!isInstallingHook && <Trash2 className="size-3" />}
                      Uninstall
                    </Button>
                  </div>
                ) : (
                  <Button
                    type="button"
                    variant="primary"
                    onClick={handleInstallHook}
                    loading={isInstallingHook}
                    disabled={isInstallingHook}
                  >
                    {!isInstallingHook && <Check className="size-3.5" />}
                    {isInstallingHook ? "Installing…" : "Install hook"}
                  </Button>
                )}
              </div>

              <div className="flex items-center justify-between pt-1">
                <Button
                  type="button"
                  variant="ghost"
                  onClick={() => setCurrentStep(2)}
                >
                  Back
                </Button>
                <Button
                  type="button"
                  variant="primary"
                  onClick={() => setCurrentStep(4)}
                >
                  Continue
                  <ChevronRight className="size-3.5" />
                </Button>
              </div>
            </div>
          )}

          {currentStep === 4 && (
            <div className="space-y-4">
              <div>
                <h3 className="text-sm font-semibold text-foreground">
                  Step 4: Verification
                </h3>
                <p className="text-xs text-muted-foreground mt-0.5">
                  Review setup status and the generated instruction file.
                </p>
              </div>

              <div className="grid grid-cols-1 sm:grid-cols-2 gap-2">
                <div className="flex items-center gap-2 p-2.5 rounded-md bg-background border border-border text-xs">
                  {knoxMdContent ? (
                    <CheckCircle2
                      className="size-4 text-success shrink-0"
                      aria-hidden
                    />
                  ) : (
                    <AlertTriangle
                      className="size-4 text-warning shrink-0"
                      aria-hidden
                    />
                  )}
                  <span className="text-foreground font-medium">
                    {knoxMdContent ? "knox.md present" : "knox.md missing"}
                  </span>
                </div>
                <div className="flex items-center gap-2 p-2.5 rounded-md bg-background border border-border text-xs">
                  {hookInstalled ? (
                    <CheckCircle2
                      className="size-4 text-success shrink-0"
                      aria-hidden
                    />
                  ) : (
                    <AlertTriangle
                      className="size-4 text-warning shrink-0"
                      aria-hidden
                    />
                  )}
                  <span className="text-foreground font-medium">
                    {hookInstalled ? "Git hook installed" : "Hook optional / not installed"}
                  </span>
                </div>
              </div>

              <div className="flex items-center justify-between pt-1">
                <Button
                  type="button"
                  variant="ghost"
                  onClick={() => setCurrentStep(3)}
                >
                  Back
                </Button>
                <Button
                  type="button"
                  variant="primary"
                  onClick={() => setActiveTab("graph")}
                >
                  <Network className="size-3.5" />
                  Explore Knowledge Graph
                </Button>
              </div>
            </div>
          )}
        </CardContent>
      </Card>

      {knoxMdContent ? (
        <Card>
          <CardHeader className="flex-row items-center justify-between space-y-0 pb-0">
            <CardTitle className="flex items-center gap-2">
              <FileText className="size-4 text-primary" aria-hidden />
              Generated knox.md
            </CardTitle>
          </CardHeader>
          <CardContent className="pt-4">
            <CodeBlock
              code={knoxMdContent}
              language="markdown"
              className="max-h-[380px] overflow-y-auto"
              onCopied={() => showToast("Copied knox.md to clipboard")}
            />
          </CardContent>
        </Card>
      ) : (
        <EmptyState
          icon={<FileText />}
          title="No knox.md detected"
          description='Use “Initialize workspace” in step 2 to create the agent instruction file for this repository.'
        />
      )}
    </div>
  );
};
