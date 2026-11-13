//! Type-safe IPC service layer communicating with Knox Rust backend.

import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type {
  KnoxAction,
  StatusReport,
  QueryResult,
  ScanReport,
  IngestReport,
  IngestPreview,
  DetectedTranscript,
  KnoxConfig,
  DbStats,
  Session,
  Project,
  DetectedMcpApp,
  McpScanLog,
  UpdateActionPayload,
  ActionDelta,
  StrayDatabaseDto,
  ConsolidationReport,
  ModelSettingsDto,
  ModelTestResultDto,
} from "@/types/knox";

const isTauri =
  typeof window !== "undefined" &&
  ("__TAURI_INTERNALS__" in window || "__TAURI__" in window);

const isDemo = import.meta.env.VITE_DEMO === "1" || !isTauri;

function ensureRuntime(operation: string) {
  if (!isTauri && !isDemo) {
    throw new Error(
      `Knox Desktop runtime error: '${operation}' requires Tauri IPC. For web development preview, run with VITE_DEMO=1.`
    );
  }
}

let demoApiPromise: Promise<typeof import("./demo")["DemoAPI"]> | null = null;
function getDemoApi() {
  if (!demoApiPromise) {
    demoApiPromise = import("./demo").then((m) => m.DemoAPI);
  }
  return demoApiPromise;
}

export const KnoxAPI = {
  async getStatus(): Promise<StatusReport> {
    if (isDemo) {
      return (await getDemoApi()).getStatus();
    }
    ensureRuntime("getStatus");
    return invoke<StatusReport>("get_status");
  },

  async queryActions(params: {
    search?: string;
    project?: string;
    filePath?: string;
    file_path?: string;
    actor?: string;
    actionType?: string;
    action_type?: string;
    since?: string;
    currentOnly?: boolean;
    current_only?: boolean;
    limit?: number;
    significance?: string;
    is_resolved?: boolean;
  }): Promise<QueryResult> {
    if (isDemo) {
      return (await getDemoApi()).queryActions(params);
    }
    ensureRuntime("queryActions");
    const filePath = params.filePath || params.file_path || null;
    const actionType = params.actionType || params.action_type || null;
    const currentOnly = params.currentOnly ?? params.current_only ?? true;

    return invoke<QueryResult>("query_actions", {
      search: params.search || null,
      project: params.project || null,
      filePath: filePath,
      file_path: filePath,
      actor: params.actor || null,
      actionType: actionType,
      action_type: actionType,
      since: params.since || null,
      currentOnly: currentOnly,
      current_only: currentOnly,
      limit: params.limit ?? 50,
      significance: params.significance || null,
      isResolved: typeof params.is_resolved === "boolean" ? params.is_resolved : null,
      is_resolved: typeof params.is_resolved === "boolean" ? params.is_resolved : null,
    });
  },

  async getAction(id: string): Promise<KnoxAction | null> {
    if (isDemo) {
      return (await getDemoApi()).getAction(id);
    }
    ensureRuntime("getAction");
    return invoke<KnoxAction | null>("get_action", { id });
  },

  async getActionDeltas(actionId: string): Promise<ActionDelta[]> {
    if (isDemo) {
      return (await getDemoApi()).getActionDeltas(actionId);
    }
    ensureRuntime("getActionDeltas");
    return invoke<ActionDelta[]>("get_action_deltas", { actionId, action_id: actionId });
  },

  async getGraphData(currentOnly?: boolean, limit?: number): Promise<KnoxAction[]> {
    if (isDemo) {
      return (await getDemoApi()).getGraphData(currentOnly, limit);
    }
    ensureRuntime("getGraphData");
    const current = currentOnly ?? false;
    return invoke<KnoxAction[]>("get_graph_data", {
      currentOnly: current,
      current_only: current,
      limit: limit ?? 200,
    });
  },

  async updateAction(params: UpdateActionPayload): Promise<KnoxAction> {
    if (isDemo) {
      return (await getDemoApi()).updateAction(params);
    }
    ensureRuntime("updateAction");
    return invoke<KnoxAction>("update_action", { params });
  },

  async listenDbChanged(callback: () => void): Promise<() => void> {
    if (isDemo) {
      return (await getDemoApi()).listenDbChanged(callback);
    }
    if (!isTauri) {
      return () => {};
    }
    return listen("knox://db-changed", () => {
      callback();
    });
  },

  async ingestTranscript(path: string): Promise<IngestReport> {
    if (isDemo) {
      return (await getDemoApi()).ingestTranscript(path);
    }
    ensureRuntime("ingestTranscript");
    return invoke<IngestReport>("ingest_transcript", { path });
  },

  async ingestRawTranscript(
    content: string,
    actorOverride?: string
  ): Promise<IngestReport> {
    if (isDemo) {
      return (await getDemoApi()).ingestRawTranscript(content, actorOverride);
    }
    ensureRuntime("ingestRawTranscript");
    return invoke<IngestReport>("ingest_raw_transcript", {
      content,
      actorOverride: actorOverride || null,
      actor_override: actorOverride || null,
    });
  },

  async previewTranscript(params: {
    path?: string;
    content?: string;
  }): Promise<IngestPreview> {
    if (isDemo) {
      return (await getDemoApi()).previewTranscript(params);
    }
    ensureRuntime("previewTranscript");
    return invoke<IngestPreview>("preview_transcript", {
      path: params.path || null,
      content: params.content || null,
    });
  },

  async ingestGitHistory(
    projectPath: string,
    maxCommits?: number
  ): Promise<IngestReport> {
    if (isDemo) {
      return (await getDemoApi()).ingestGitHistory(projectPath, maxCommits);
    }
    ensureRuntime("ingestGitHistory");
    return invoke<IngestReport>("ingest_git_history", {
      projectPath,
      project_path: projectPath,
      maxCommits: maxCommits ?? 50,
      max_commits: maxCommits ?? 50,
    });
  },

  async detectLocalTranscripts(
    projectPath?: string
  ): Promise<DetectedTranscript[]> {
    if (isDemo) {
      return (await getDemoApi()).detectLocalTranscripts(projectPath);
    }
    ensureRuntime("detectLocalTranscripts");
    return invoke<DetectedTranscript[]>("detect_local_transcripts", {
      projectPath: projectPath || null,
      project_path: projectPath || null,
    });
  },

  async initProject(path: string): Promise<string> {
    if (isDemo) {
      return (await getDemoApi()).initProject(path);
    }
    ensureRuntime("initProject");
    return invoke<string>("init_project", { path });
  },

  async getProjects(): Promise<Project[]> {
    if (isDemo) {
      return (await getDemoApi()).getProjects();
    }
    ensureRuntime("getProjects");
    return invoke<Project[]>("get_projects");
  },

  async setProjectEgress(projectId: string, allowed: boolean): Promise<void> {
    if (isDemo) {
      return (await getDemoApi()).setProjectEgress(projectId, allowed);
    }
    ensureRuntime("setProjectEgress");
    return invoke<void>("set_project_egress", { projectId, allowed });
  },

  async scanStaged(project?: string): Promise<ScanReport> {
    if (isDemo) {
      return (await getDemoApi()).scanStaged(project);
    }
    ensureRuntime("scanStaged");
    return invoke<ScanReport>("scan_staged", { project: project || null });
  },

  async installHook(path: string): Promise<string> {
    if (isDemo) {
      return (await getDemoApi()).installHook(path);
    }
    ensureRuntime("installHook");
    return invoke<string>("install_hook", { path });
  },

  async uninstallHook(path: string): Promise<string> {
    if (isDemo) {
      return (await getDemoApi()).uninstallHook(path);
    }
    ensureRuntime("uninstallHook");
    return invoke<string>("uninstall_hook", { path });
  },

  async getKnoxMd(path: string): Promise<string | null> {
    if (isDemo) {
      return (await getDemoApi()).getKnoxMd(path);
    }
    ensureRuntime("getKnoxMd");
    return invoke<string | null>("get_knox_md", { path });
  },

  async getHookStatus(path: string): Promise<boolean> {
    if (isDemo) {
      return (await getDemoApi()).getHookStatus(path);
    }
    ensureRuntime("getHookStatus");
    return invoke<boolean>("get_hook_status", { path });
  },

  async startSession(actor: string, project?: string): Promise<Session> {
    if (isDemo) {
      return (await getDemoApi()).startSession(actor, project);
    }
    ensureRuntime("startSession");
    return invoke<Session>("start_session", { actor, project: project || null });
  },

  async endSession(sessionId: string): Promise<Session> {
    if (isDemo) {
      return (await getDemoApi()).endSession(sessionId);
    }
    ensureRuntime("endSession");
    return invoke<Session>("end_session", {
      sessionId,
      session_id: sessionId,
    });
  },

  async getSessions(limit?: number): Promise<Session[]> {
    if (isDemo) {
      return (await getDemoApi()).getSessions(limit);
    }
    ensureRuntime("getSessions");
    return invoke<Session[]>("get_sessions", { limit: limit ?? 20 });
  },

  async getConfig(): Promise<KnoxConfig> {
    if (isDemo) {
      return (await getDemoApi()).getConfig();
    }
    ensureRuntime("getConfig");
    return invoke<KnoxConfig>("get_config");
  },

  async updateConfig(params: {
    sessionTimeoutSeconds?: number;
    globalMode?: boolean;
    kgPath?: string;
    modelProfile?: string;
    modelEndpoint?: string | null;
    modelApiKey?: string | null;
    embeddingModel?: string | null;
    chatModel?: string | null;
    allowCloudEgress?: boolean;
    strayDbPrompted?: boolean;
  }): Promise<KnoxConfig> {
    if (isDemo) {
      return (await getDemoApi()).updateConfig(params);
    }
    ensureRuntime("updateConfig");
    return invoke<KnoxConfig>("update_config", {
      sessionTimeoutSeconds: params.sessionTimeoutSeconds,
      session_timeout_seconds: params.sessionTimeoutSeconds,
      globalMode: params.globalMode,
      global_mode: params.globalMode,
      kgPath: params.kgPath,
      kg_path: params.kgPath,
      modelProfile: params.modelProfile,
      model_profile: params.modelProfile,
      modelEndpoint: params.modelEndpoint,
      model_endpoint: params.modelEndpoint,
      modelApiKey: params.modelApiKey,
      model_api_key: params.modelApiKey,
      embeddingModel: params.embeddingModel,
      embedding_model: params.embeddingModel,
      chatModel: params.chatModel,
      chat_model: params.chatModel,
      allowCloudEgress: params.allowCloudEgress,
      allow_cloud_egress: params.allowCloudEgress,
      strayDbPrompted: params.strayDbPrompted,
      stray_db_prompted: params.strayDbPrompted,
    });
  },

  async scanStrayDatabases(): Promise<StrayDatabaseDto[]> {
    if (isDemo) {
      return (await getDemoApi()).scanStrayDatabases();
    }
    ensureRuntime("scanStrayDatabases");
    return invoke<StrayDatabaseDto[]>("scan_stray_databases");
  },

  async consolidateStrayDatabase(path: string): Promise<ConsolidationReport> {
    if (isDemo) {
      return (await getDemoApi()).consolidateStrayDatabase(path);
    }
    ensureRuntime("consolidateStrayDatabase");
    return invoke<ConsolidationReport>("consolidate_stray_database", { path });
  },

  async relocateDatabase(newDir: string): Promise<KnoxConfig> {
    if (isDemo) {
      return (await getDemoApi()).relocateDatabase(newDir);
    }
    ensureRuntime("relocateDatabase");
    return invoke<KnoxConfig>("relocate_database", { newDir, new_dir: newDir });
  },

  async testModelConnection(draft: ModelSettingsDto): Promise<ModelTestResultDto> {
    if (isDemo) {
      return (await getDemoApi()).testModelConnection(draft);
    }
    ensureRuntime("testModelConnection");
    return invoke<ModelTestResultDto>("test_model_connection", { draft });
  },

  async saveConfig(config: KnoxConfig): Promise<KnoxConfig> {
    if (isDemo) {
      return (await getDemoApi()).saveConfig(config);
    }
    ensureRuntime("saveConfig");
    return invoke<KnoxConfig>("save_config", { config });
  },

  async getDbStats(): Promise<DbStats> {
    if (isDemo) {
      return (await getDemoApi()).getDbStats();
    }
    ensureRuntime("getDbStats");
    return invoke<DbStats>("get_db_stats");
  },

  async runDbIntegrity(): Promise<string> {
    if (isDemo) {
      return (await getDemoApi()).runDbIntegrity();
    }
    ensureRuntime("runDbIntegrity");
    return invoke<string>("run_db_integrity");
  },

  async runDbVacuum(): Promise<string> {
    if (isDemo) {
      return (await getDemoApi()).runDbVacuum();
    }
    ensureRuntime("runDbVacuum");
    return invoke<string>("run_db_vacuum");
  },

  async revealDbFolder(): Promise<void> {
    if (isDemo) {
      return (await getDemoApi()).revealDbFolder();
    }
    ensureRuntime("revealDbFolder");
    return invoke<void>("reveal_db_folder");
  },

  async openFileDialog(options?: {
    name?: string;
    extensions?: string[];
  }): Promise<string | null> {
    if (isDemo) {
      return (await getDemoApi()).openFileDialog(options);
    }
    if (!isTauri) return null;
    try {
      return await invoke<string | null>("open_file_dialog", {
        name: options?.name || null,
        extensions: options?.extensions || null,
      });
    } catch (err) {
      console.warn("Native file dialog failed:", err);
      return null;
    }
  },

  async openFolderDialog(): Promise<string | null> {
    if (isDemo) {
      return (await getDemoApi()).openFolderDialog();
    }
    if (!isTauri) return null;
    try {
      return await invoke<string | null>("open_folder_dialog");
    } catch (err) {
      console.warn("Native folder dialog failed:", err);
      return null;
    }
  },

  async revealInExplorer(path: string): Promise<void> {
    if (isDemo) {
      return (await getDemoApi()).revealInExplorer(path);
    }
    if (!isTauri) return;
    try {
      await invoke("reveal_in_explorer", { path });
    } catch (err) {
      console.warn("Failed to reveal folder in native explorer:", err);
      throw err;
    }
  },

  async detectMcpConfigs(): Promise<DetectedMcpApp[]> {
    if (isDemo) {
      return (await getDemoApi()).detectMcpConfigs();
    }
    ensureRuntime("detectMcpConfigs");
    return invoke<DetectedMcpApp[]>("detect_mcp_configs");
  },

  async deepScanMcpConfigs(): Promise<DetectedMcpApp[]> {
    if (isDemo) {
      return (await getDemoApi()).deepScanMcpConfigs();
    }
    ensureRuntime("deepScanMcpConfigs");
    return invoke<DetectedMcpApp[]>("deep_scan_mcp_configs");
  },

  async inspectCustomMcpConfig(filePath: string): Promise<DetectedMcpApp> {
    if (isDemo) {
      return (await getDemoApi()).inspectCustomMcpConfig(filePath);
    }
    ensureRuntime("inspectCustomMcpConfig");
    return invoke<DetectedMcpApp>("inspect_custom_mcp_config", {
      filePath,
      file_path: filePath,
    });
  },

  async injectMcpConfig(filePath: string, useSystemCommand?: boolean): Promise<boolean> {
    if (isDemo) {
      return (await getDemoApi()).injectMcpConfig(filePath, useSystemCommand);
    }
    ensureRuntime("injectMcpConfig");
    return invoke<boolean>("inject_mcp_config", {
      filePath,
      file_path: filePath,
      useSystemCommand: useSystemCommand ?? false,
      use_system_command: useSystemCommand ?? false,
    });
  },

  async removeMcpConfig(filePath: string): Promise<boolean> {
    if (isDemo) {
      return (await getDemoApi()).removeMcpConfig(filePath);
    }
    ensureRuntime("removeMcpConfig");
    return invoke<boolean>("remove_mcp_config", {
      filePath,
      file_path: filePath,
    });
  },

  async listenMcpScanLogs(callback: (log: McpScanLog) => void): Promise<() => void> {
    if (isDemo) {
      return (await getDemoApi()).listenMcpScanLogs(callback);
    }
    if (!isTauri) {
      return () => {};
    }
    return listen<McpScanLog>("mcp-scan-log", (event) => {
      callback(event.payload);
    });
  },

  async recordQuickThought(params: {
    summary: string;
    category: "doubt" | "minor_decision" | "major_decision" | "thought";
    actor?: string;
    contextSummary?: string;
    files?: string[];
    project?: string;
  }): Promise<string> {
    if (isDemo) {
      return (await getDemoApi()).recordQuickThought(params);
    }
    ensureRuntime("recordQuickThought");
    return invoke<string>("record_quick_thought", {
      summary: params.summary,
      category: params.category,
      actor: params.actor || null,
      contextSummary: params.contextSummary || null,
      context_summary: params.contextSummary || null,
      files: params.files || null,
      project: params.project || null,
    });
  },

  async resolveDoubt(params: {
    doubtId: string;
    resolutionNote: string;
    resolverActionId?: string;
  }): Promise<boolean> {
    if (isDemo) {
      return (await getDemoApi()).resolveDoubt(params);
    }
    ensureRuntime("resolveDoubt");
    return invoke<boolean>("resolve_doubt", {
      doubtId: params.doubtId,
      doubt_id: params.doubtId,
      resolutionNote: params.resolutionNote,
      resolution_note: params.resolutionNote,
      resolverActionId: params.resolverActionId || null,
      resolver_action_id: params.resolverActionId || null,
    });
  },
};
