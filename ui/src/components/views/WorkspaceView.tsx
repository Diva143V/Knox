import React, { useState, useEffect, useCallback, useRef } from "react";
import {
  FolderGit2,
  Sparkles,
  CheckCircle2,
  FileText,
  FolderOpen,
  GitCommit,
  Check,
  ChevronRight,
  ChevronDown,
  AlertTriangle,
  Trash2,
  Database,
  Network,
  Clock,
  Globe,
  Folder,
  X,
  Copy,
  ExternalLink,
  Activity,
} from "lucide-react";
import { KnoxAPI } from "@/services/api";
import { useKnoxStore } from "@/store/useKnoxStore";
import type { KnoxConfig } from "@/types/knox";
import { PageHeader } from "@/components/ui/PageHeader";
import { EmptyState } from "@/components/ui/EmptyState";
import { Skeleton } from "@/components/ui/Skeleton";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/Card";
import { Button } from "@/components/ui/Button";
import { Input } from "@/components/ui/Input";
import { Badge } from "@/components/ui/Badge";
import { CodeBlock } from "@/components/ui/CodeBlock";
import { cn } from "@/lib/cn";

type WizardStep = 1 | 2 | 3 | 4;

export const WorkspaceView: React.FC = () => {
  const {
    showToast,
    refreshData,
    setActiveTab,
    updateConfig,
    activeProjectScope,
    setProjectScope,
    projects,
    recentProjects,
    addRecentProject,
    actions,
  } = useKnoxStore();

  // Merge tracked projects and recent projects for the switcher menu
  const availableProjects = React.useMemo(() => {
    const map = new Map<string, { name: string; path: string; id?: string }>();
    projects.forEach((p) => {
      map.set(p.path.toLowerCase().replace(/\\/g, "/"), { name: p.name, path: p.path, id: p.id });
    });
    recentProjects.forEach((r) => {
      const key = r.path.toLowerCase().replace(/\\/g, "/");
      if (!map.has(key)) {
        map.set(key, { name: r.name, path: r.path });
      }
    });
    if (map.size === 0) {
      map.set("d:/knox", { name: "Knox", path: "D:\\Knox", id: "default-knox" });
    }
    return Array.from(map.values());
  }, [projects, recentProjects]);

  const currentProject = React.useMemo(() => {
    if (activeProjectScope) {
      const match = availableProjects.find(
        (p) => p.path.toLowerCase().replace(/\\/g, "/") === activeProjectScope.toLowerCase().replace(/\\/g, "/")
      );
      if (match) return match;
      const name = activeProjectScope.replace(/\\/g, "/").split("/").filter(Boolean).pop() || activeProjectScope;
      return { name, path: activeProjectScope, id: "active-scope" };
    }
    if (availableProjects.length > 0) {
      return availableProjects[0];
    }
    return {
      id: "default-knox",
      name: "Knox",
      path: "D:\\Knox",
    };
  }, [activeProjectScope, availableProjects]);

  const [workspacePath, setWorkspacePath] = useState(currentProject.path || ".");
  const [projectDropdownOpen, setProjectDropdownOpen] = useState(false);
  const dropdownRef = useRef<HTMLDivElement>(null);
  const [copiedPath, setCopiedPath] = useState(false);

  useEffect(() => {
    if (currentProject.path && (workspacePath === "." || !workspacePath)) {
      setWorkspacePath(currentProject.path);
    }
  }, [currentProject.path]);

  useEffect(() => {
    const handleClickOutside = (event: MouseEvent) => {
      if (dropdownRef.current && !dropdownRef.current.contains(event.target as Node)) {
        setProjectDropdownOpen(false);
      }
    };
    if (projectDropdownOpen) {
      document.addEventListener("mousedown", handleClickOutside);
    }
    return () => {
      document.removeEventListener("mousedown", handleClickOutside);
    };
  }, [projectDropdownOpen]);

  const handleSelectProject = (projectPath: string) => {
    setProjectScope(projectPath);
    setWorkspacePath(projectPath);
    setProjectDropdownOpen(false);
  };

  const handleCopyPath = async () => {
    try {
      await navigator.clipboard.writeText(currentProject.path);
      setCopiedPath(true);
      showToast("Workspace path copied to clipboard");
      setTimeout(() => setCopiedPath(false), 2000);
    } catch (err) {
      showToast("Failed to copy path: " + String(err), true);
    }
  };

  const handleOpenExplorer = async () => {
    try {
      await KnoxAPI.revealInExplorer(currentProject.path);
      showToast(`Opened ${currentProject.name} in file explorer`);
    } catch (err) {
      showToast("Could not open file explorer: " + String(err), true);
    }
  };

  const projectActionsCount = React.useMemo(() => {
    if (!currentProject || !actions) return 0;
    const normProjPath = currentProject.path.toLowerCase().replace(/\\/g, "/");
    const projName = currentProject.name.toLowerCase();
    return actions.filter((a) => {
      const matchFile =
        a.files &&
        a.files.some((f) => {
          const normF = f.toLowerCase().replace(/\\/g, "/");
          return normF.startsWith(normProjPath) || normF.includes(projName);
        });
      const matchTag = a.tags && a.tags.some((t) => t.toLowerCase() === projName);
      const matchSummary = a.summary.toLowerCase().includes(projName);
      return matchFile || matchTag || matchSummary;
    }).length;
  }, [currentProject, actions]);

  const [currentStep, setCurrentStep] = useState<WizardStep>(1);
  const [knoxMdContent, setKnoxMdContent] = useState<string | null>(null);
  const [isInitializing, setIsInitializing] = useState(false);
  const [isInstallingHook, setIsInstallingHook] = useState(false);
  const [hookInstalled, setHookInstalled] = useState(false);
  const [errorMessage, setErrorMessage] = useState<string | null>(null);
  const [config, setConfig] = useState<KnoxConfig | null>(null);

  // Config interaction state (Timeout modal)
  const [isTimeoutModalOpen, setIsTimeoutModalOpen] = useState(false);
  const [selectedTimeout, setSelectedTimeout] = useState<number>(1800);
  const [customTimeoutInput, setCustomTimeoutInput] = useState<string>("1800");
  const [isCustomTimeout, setIsCustomTimeout] = useState(false);
  const [isSavingConfig, setIsSavingConfig] = useState(false);

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

  const isFirstMountRef = useRef(true);

  // Check workspace status with clean cancellation teardown; auto-advance only on mount
  useEffect(() => {
    let cancelled = false;
    const run = async () => {
      try {
        const [md, hook] = await Promise.all([
          KnoxAPI.getKnoxMd(workspacePath),
          KnoxAPI.getHookStatus(workspacePath),
        ]);
        if (cancelled) return;
        setKnoxMdContent(md);
        setHookInstalled(hook);

        if (isFirstMountRef.current) {
          isFirstMountRef.current = false;
          if (md && hook) {
            setCurrentStep(4);
          } else if (md) {
            setCurrentStep(3);
          } else {
            setCurrentStep(1);
          }
        }
      } catch {
        if (!cancelled) {
          setKnoxMdContent(null);
          setHookInstalled(false);
        }
      }
    };
    run();
    return () => {
      cancelled = true;
    };
  }, [workspacePath]);

  useEffect(() => {
    let cancelled = false;
    KnoxAPI.getConfig()
      .then((cfg) => {
        if (!cancelled) {
          setConfig(cfg);
          setSelectedTimeout(cfg.session_timeout_seconds);
          setCustomTimeoutInput(String(cfg.session_timeout_seconds));
        }
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
        addRecentProject(folder);
        setProjectScope(folder);
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
      addRecentProject(workspacePath);
      setProjectScope(workspacePath);
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

      {/* Connected Workspace Hero Card */}
      <Card className="border-primary/30 bg-gradient-to-br from-card via-card to-primary/[0.04] shadow-sm relative overflow-visible">
        <CardContent className="p-4 sm:p-5 space-y-3.5">
          {/* Top Row: Workspace Identity & Switcher */}
          <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-3">
            <div className="flex items-center gap-3 min-w-0">
              <div className="size-10 rounded-lg bg-primary/10 border border-primary/25 flex items-center justify-center text-primary shrink-0 shadow-2xs">
                <FolderGit2 className="size-5" aria-hidden />
              </div>
              <div className="min-w-0">
                <div className="flex items-center gap-2">
                  <span className="text-3xs font-mono uppercase tracking-wider text-primary font-semibold">
                    Connected Workspace
                  </span>
                  <Badge variant="outline" className="text-3xs font-mono py-0 px-1.5 h-4 bg-primary/5 text-primary border-primary/30">
                    Active
                  </Badge>
                </div>
                <h2 className="text-base sm:text-lg font-bold text-foreground truncate mt-0.5 tracking-tight" title={currentProject.name}>
                  {currentProject.name}
                </h2>
              </div>
            </div>

            {/* Switch Project Dropdown */}
            <div className="relative shrink-0" ref={dropdownRef}>
              <Button
                type="button"
                variant="outline"
                size="sm"
                onClick={() => setProjectDropdownOpen(!projectDropdownOpen)}
                className="gap-1.5 text-xs h-8 border-border/80 hover:border-primary/40"
              >
                <FolderOpen className="size-3.5 text-muted-foreground" />
                <span>Switch Project</span>
                <ChevronDown
                  className={cn(
                    "size-3.5 text-muted-foreground transition-transform duration-200",
                    projectDropdownOpen && "rotate-180"
                  )}
                />
              </Button>

              {projectDropdownOpen && (
                <div className="absolute right-0 top-full mt-1.5 w-72 sm:w-80 rounded-lg border border-border bg-card/95 backdrop-blur-xl shadow-xl p-2 z-50 space-y-1 text-xs">
                  <div className="px-2 py-1 text-2xs font-mono uppercase tracking-wider text-muted-foreground flex items-center justify-between">
                    <span>Available Workspaces</span>
                    <span className="text-3xs font-mono">{availableProjects.length}</span>
                  </div>

                  <div className="max-h-56 overflow-y-auto space-y-0.5 pr-0.5">
                    {availableProjects.map((proj) => {
                      const isSelected =
                        currentProject.path.toLowerCase().replace(/\\/g, "/") ===
                        proj.path.toLowerCase().replace(/\\/g, "/");
                      return (
                        <button
                          key={proj.path}
                          type="button"
                          onClick={() => handleSelectProject(proj.path)}
                          className={cn(
                            "w-full flex items-center justify-between p-2 rounded-md transition-colors text-left cursor-pointer",
                            isSelected
                              ? "bg-primary/15 border border-primary/30 text-foreground"
                              : "hover:bg-muted text-muted-foreground hover:text-foreground"
                          )}
                        >
                          <div className="min-w-0 pr-2 flex-1">
                            <p className="font-semibold text-foreground truncate">{proj.name}</p>
                            <p className="font-mono text-3xs text-muted-foreground truncate" title={proj.path}>
                              {proj.path}
                            </p>
                          </div>
                          {isSelected && <Check className="size-3.5 text-primary shrink-0" />}
                        </button>
                      );
                    })}
                  </div>

                  <div className="h-px bg-border my-1" />

                  <button
                    type="button"
                    onClick={() => {
                      setProjectDropdownOpen(false);
                      handleBrowseFolder();
                    }}
                    className="w-full flex items-center gap-2 p-2 rounded-md hover:bg-muted text-primary transition-colors cursor-pointer font-medium text-xs"
                  >
                    <FolderOpen className="size-3.5 shrink-0" />
                    <span>Browse for new folder…</span>
                  </button>
                </div>
              )}
            </div>
          </div>

          {/* Path Box with Copy & Open Explorer actions */}
          <div className="p-2.5 rounded-md bg-muted/50 border border-border/80 flex items-center justify-between gap-3 text-xs">
            <div className="flex items-center gap-2 min-w-0 flex-1">
              <Folder className="size-3.5 text-muted-foreground shrink-0" aria-hidden />
              <span className="font-mono text-foreground text-xs truncate" title={currentProject.path}>
                {currentProject.path}
              </span>
            </div>
            <div className="flex items-center gap-1 shrink-0">
              <button
                type="button"
                onClick={handleCopyPath}
                className="p-1.5 rounded-md hover:bg-card border border-transparent hover:border-border text-muted-foreground hover:text-foreground transition-all cursor-pointer"
                title="Copy directory path"
              >
                {copiedPath ? <Check className="size-3.5 text-success" /> : <Copy className="size-3.5" />}
              </button>
              <button
                type="button"
                onClick={handleOpenExplorer}
                className="p-1.5 rounded-md hover:bg-card border border-transparent hover:border-border text-muted-foreground hover:text-foreground transition-all cursor-pointer"
                title="Open folder in File Explorer"
              >
                <ExternalLink className="size-3.5" />
              </button>
            </div>
          </div>

          {/* Real-time Status Badges (knox.md, Git Hook, Actions Count) */}
          <div className="flex flex-wrap items-center gap-2 pt-0.5">
            {/* knox.md status */}
            <div
              className={cn(
                "inline-flex items-center gap-1.5 px-2.5 py-1 rounded-full text-xs font-medium border transition-colors",
                knoxMdContent
                  ? "bg-success/10 text-success border-success/30"
                  : "bg-warning/10 text-warning border-warning/30"
              )}
            >
              {knoxMdContent ? (
                <CheckCircle2 className="size-3.5 shrink-0" aria-hidden />
              ) : (
                <AlertTriangle className="size-3.5 shrink-0" aria-hidden />
              )}
              <span>{knoxMdContent ? "knox.md active" : "knox.md missing"}</span>
            </div>

            {/* Hook status */}
            <div
              className={cn(
                "inline-flex items-center gap-1.5 px-2.5 py-1 rounded-full text-xs font-medium border transition-colors",
                hookInstalled
                  ? "bg-success/10 text-success border-success/30"
                  : "bg-muted/70 text-muted-foreground border-border"
              )}
            >
              {hookInstalled ? (
                <Check className="size-3.5 shrink-0 text-success" aria-hidden />
              ) : (
                <X className="size-3.5 shrink-0" aria-hidden />
              )}
              <span>{hookInstalled ? "Pre-commit hook installed" : "Hook not installed"}</span>
            </div>

            {/* Actions count */}
            <div className="inline-flex items-center gap-1.5 px-2.5 py-1 rounded-full text-xs font-medium bg-muted/60 text-foreground border border-border">
              <Activity className="size-3.5 text-primary shrink-0" aria-hidden />
              <span>{projectActionsCount} actions recorded</span>
            </div>
          </div>
        </CardContent>
      </Card>

      {config ? (
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
            <div className="flex flex-wrap items-center gap-2 shrink-0">
              <button
                type="button"
                onClick={() => {
                  setSelectedTimeout(config.session_timeout_seconds);
                  setCustomTimeoutInput(String(config.session_timeout_seconds));
                  setIsCustomTimeout(![300, 900, 1800, 3600, 7200, 14400].includes(config.session_timeout_seconds));
                  setIsTimeoutModalOpen(true);
                }}
                className={cn(
                  "group inline-flex items-center gap-1.5 px-2.5 py-1 rounded-full text-xs font-mono font-medium",
                  "bg-muted/80 text-foreground border border-border/80 shadow-xs",
                  "hover:bg-accent hover:border-primary/50 hover:text-primary transition-all cursor-pointer",
                  "focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary/40"
                )}
                title="Click to configure session timeout"
              >
                <Clock className="size-3 text-muted-foreground group-hover:text-primary transition-colors" />
                <span>timeout {config.session_timeout_seconds}s</span>
                <span className="text-3xs text-muted-foreground group-hover:text-primary/80 uppercase font-sans tracking-wide ml-0.5 opacity-70">
                  edit
                </span>
              </button>

              <div
                className="inline-flex items-center gap-1.5 px-2.5 py-1 rounded-full text-xs font-medium bg-primary/15 text-primary border border-primary/30 shadow-xs"
                title="Knox operates on a unified global knowledge graph"
              >
                <Globe className="size-3 text-primary shrink-0" />
                <span>global storage</span>
              </div>
            </div>
          </CardContent>
        </Card>
      ) : (
        <Card className="p-4 animate-pulse">
          <div className="flex flex-col sm:flex-row sm:items-center gap-3">
            <div className="flex items-center gap-2 min-w-0 flex-1">
              <Skeleton className="size-4 rounded-sm" />
              <div className="space-y-1.5 min-w-0 flex-1">
                <Skeleton className="h-3 w-28 rounded-sm" />
                <Skeleton className="h-3 w-48 rounded-sm" />
              </div>
            </div>
            <div className="flex gap-2">
              <Skeleton className="h-6 w-24 rounded-full" />
              <Skeleton className="h-6 w-24 rounded-full" />
            </div>
          </div>
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

              {isInitializing ? (
                <div className="p-4 rounded-md border border-border bg-background space-y-3 animate-pulse">
                  <div className="flex items-center justify-between">
                    <Skeleton className="h-4 w-32 rounded-sm" />
                    <Skeleton className="h-4 w-24 rounded-sm" />
                  </div>
                  <Skeleton className="h-16 w-full rounded-md" />
                </div>
              ) : (
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
              )}

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

      {/* Session Timeout Modal */}
      {isTimeoutModalOpen && (
        <div className="fixed inset-0 z-50 flex items-center justify-center p-4 bg-background/80 backdrop-blur-sm motion-safe:animate-in motion-safe:fade-in motion-safe:duration-150">
          <div className="w-full max-w-md bg-card border border-border rounded-xl shadow-2xl p-6 space-y-4">
            <div className="flex items-center justify-between border-b border-border/60 pb-3">
              <div className="flex items-center gap-2">
                <Clock className="size-4 text-primary" />
                <h3 className="text-sm font-semibold text-foreground">
                  Configure Session Timeout
                </h3>
              </div>
              <Button
                type="button"
                variant="ghost"
                size="icon"
                className="size-7"
                onClick={() => setIsTimeoutModalOpen(false)}
                aria-label="Close modal"
              >
                <X className="size-4" />
              </Button>
            </div>

            <p className="text-xs text-muted-foreground leading-relaxed">
              Knox seals idle agent sessions automatically after this inactivity window, creating clean architectural checkpoints.
            </p>

            <div className="space-y-3">
              <label className="block text-2xs font-semibold uppercase tracking-wider text-muted-foreground">
                Presets
              </label>
              <div className="grid grid-cols-3 gap-2">
                {[
                  { label: "5 mins", secs: 300, desc: "Quick testing" },
                  { label: "15 mins", secs: 900, desc: "Short tasks" },
                  { label: "30 mins", secs: 1800, desc: "Default" },
                  { label: "1 hour", secs: 3600, desc: "Extended sprint" },
                  { label: "2 hours", secs: 7200, desc: "Deep focus" },
                  { label: "4 hours", secs: 14400, desc: "All-day work" },
                ].map((preset) => {
                  const isSelected = !isCustomTimeout && selectedTimeout === preset.secs;
                  return (
                    <button
                      key={preset.secs}
                      type="button"
                      onClick={() => {
                        setSelectedTimeout(preset.secs);
                        setCustomTimeoutInput(String(preset.secs));
                        setIsCustomTimeout(false);
                      }}
                      className={cn(
                        "p-2.5 rounded-lg border text-left transition-all text-xs flex flex-col justify-between cursor-pointer",
                        isSelected
                          ? "bg-primary/10 border-primary text-primary font-medium shadow-xs"
                          : "bg-card/50 border-border hover:border-border-hover text-foreground"
                      )}
                    >
                      <span className="font-semibold">{preset.label}</span>
                      <span className="text-3xs text-muted-foreground">{preset.secs}s</span>
                    </button>
                  );
                })}
              </div>

              <div className="pt-2 border-t border-border/50">
                <label className="block text-2xs font-semibold uppercase tracking-wider text-muted-foreground mb-1.5">
                  Custom Timeout (seconds)
                </label>
                <div className="flex gap-2 items-center">
                  <Input
                    type="number"
                    min="30"
                    max="86400"
                    value={customTimeoutInput}
                    onChange={(e) => {
                      setCustomTimeoutInput(e.target.value);
                      setIsCustomTimeout(true);
                      const num = parseInt(e.target.value, 10);
                      if (!isNaN(num) && num > 0) {
                        setSelectedTimeout(num);
                      }
                    }}
                    placeholder="e.g. 1800"
                    className="font-mono text-xs flex-1"
                  />
                  <span className="shrink-0 text-xs text-muted-foreground font-mono">
                    {selectedTimeout >= 60 ? `${Math.round(selectedTimeout / 60)} min` : `${selectedTimeout}s`}
                  </span>
                </div>
              </div>
            </div>

            <div className="flex items-center justify-end gap-2 pt-3 border-t border-border/60">
              <Button
                type="button"
                variant="ghost"
                size="sm"
                onClick={() => setIsTimeoutModalOpen(false)}
              >
                Cancel
              </Button>
              <Button
                type="button"
                variant="default"
                size="sm"
                disabled={isSavingConfig || selectedTimeout <= 0}
                onClick={async () => {
                  try {
                    setIsSavingConfig(true);
                    const updated = await updateConfig({ sessionTimeoutSeconds: selectedTimeout });
                    setConfig(updated);
                    showToast(`Session timeout updated to ${selectedTimeout}s`, "success");
                    setIsTimeoutModalOpen(false);
                  } catch (err) {
                    showToast(String(err), true);
                  } finally {
                    setIsSavingConfig(false);
                  }
                }}
              >
                {isSavingConfig ? "Saving..." : "Save Timeout"}
              </Button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
};
