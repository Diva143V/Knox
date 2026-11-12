//! Standalone demo fixtures and mock API implementation.
//! Only bundled when built with VITE_DEMO=1.

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
} from "@/types/knox";

export const mockActions: KnoxAction[] = [
  {
    id: "a-001",
    actor: "gemini",
    action_type: "write",
    summary: "Implemented Tauri v2 IPC command architecture",
    files: ["src-tauri/src/lib.rs", "src-tauri/src/commands/mod.rs"],
    tags: ["tauri", "ipc", "architecture"],
    confidence: 0.95,
    timestamp: "2026-10-30T09:00:00Z",
    session_id: "s-001",
    parent_action_id: null,
    supersedes: null,
    is_current: true,
  },
  {
    id: "a-002",
    actor: "antigravity",
    action_type: "decide",
    summary: "Adopted Linear/Raycast dark OKLCH aesthetic and React Flow",
    files: ["ui/src/globals.css", "ui/src/components/graph/KnoxGraphCanvas.tsx"],
    tags: ["ui", "design-system", "oklch"],
    confidence: 0.9,
    timestamp: "2026-10-30T09:10:00Z",
    session_id: "s-001",
    parent_action_id: "a-001",
    supersedes: null,
    is_current: true,
  },
  {
    id: "a-003",
    actor: "human",
    action_type: "plan",
    summary: "Create consumer-grade desktop application for Knox v1.2.0",
    files: ["v1.2.0.md"],
    tags: ["planning", "roadmap"],
    confidence: 1.0,
    timestamp: "2026-10-30T08:30:00Z",
    session_id: "s-001",
    parent_action_id: null,
    supersedes: null,
    is_current: true,
  },
  {
    id: "a-004",
    actor: "cursor",
    action_type: "debug",
    summary: "cargo check on knox-desktop passes with 0 warnings",
    files: ["src-tauri/Cargo.toml"],
    tags: ["build", "cargo"],
    confidence: 0.85,
    timestamp: "2026-10-30T09:20:00Z",
    session_id: "s-001",
    parent_action_id: "a-002",
    supersedes: null,
    is_current: true,
  },
];

export const DemoAPI = {
  async getStatus(): Promise<StatusReport> {
    return {
      total_actions: 42,
      total_sessions: 5,
      projects_tracked: 2,
      last_action_timestamp: new Date().toISOString(),
      actions_by_actor: { gemini: 24, cursor: 12, human: 6 },
      actions_by_type: { write: 20, decide: 10, plan: 8, debug: 4 },
    };
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
    let filtered = [...mockActions];
    if (params.significance) {
      filtered = filtered.filter((a) => (a.significance || "major") === params.significance);
    }
    if (typeof params.is_resolved === "boolean") {
      filtered = filtered.filter((a) => (a.is_resolved ?? true) === params.is_resolved);
    }
    return {
      results: filtered,
      total_count: filtered.length,
      has_more: false,
    };
  },

  async getAction(id: string): Promise<KnoxAction | null> {
    return mockActions.find((a) => a.id === id) || null;
  },

  async getActionDeltas(_actionId: string): Promise<ActionDelta[]> {
    return [];
  },

  async getGraphData(_currentOnly?: boolean, _limit?: number): Promise<KnoxAction[]> {
    return mockActions;
  },

  async updateAction(params: UpdateActionPayload): Promise<KnoxAction> {
    const idx = mockActions.findIndex((a) => a.id === params.id);
    if (idx !== -1) {
      mockActions[idx] = {
        ...mockActions[idx],
        ...params,
        relations: params.relations ?? mockActions[idx].relations,
      };
      return mockActions[idx];
    }
    throw new Error("Action not found in mock");
  },

  async listenDbChanged(_callback: () => void): Promise<() => void> {
    return () => {};
  },

  async ingestTranscript(_path: string): Promise<IngestReport> {
    return {
      actions_imported: 7,
      files_touched: 3,
      sessions_created: 1,
      duplicates_skipped: 0,
      tool_detected: "antigravity",
    };
  },

  async ingestRawTranscript(
    _content: string,
    actorOverride?: string
  ): Promise<IngestReport> {
    return {
      actions_imported: 4,
      files_touched: 2,
      sessions_created: 1,
      duplicates_skipped: 0,
      tool_detected: actorOverride || "generic",
    };
  },

  async previewTranscript(params: {
    path?: string;
    content?: string;
  }): Promise<IngestPreview> {
    return {
      tool_detected: params.path?.includes("claude") ? "claude" : "antigravity",
      total_actions: 5,
      plans_count: 2,
      writes_count: 2,
      debugs_count: 1,
      decisions_count: 0,
      files_touched: ["src/lib.rs", "src/main.rs"],
      duplicates_count: 0,
      sample_actions: [],
    };
  },

  async ingestGitHistory(
    _projectPath: string,
    _maxCommits?: number
  ): Promise<IngestReport> {
    return {
      actions_imported: 15,
      files_touched: 8,
      sessions_created: 1,
      duplicates_skipped: 0,
      tool_detected: "git",
    };
  },

  async detectLocalTranscripts(
    _projectPath?: string
  ): Promise<DetectedTranscript[]> {
    return [
      {
        path: "fixtures/transcript_antigravity.jsonl",
        source: "Antigravity",
        size_bytes: 4200,
        last_modified: new Date().toISOString(),
        estimated_actions: 6,
      },
    ];
  },

  async initProject(path: string): Promise<string> {
    return `Initialized Knox in ${path}`;
  },

  async getProjects(): Promise<Project[]> {
    return [
      {
        id: "p-001",
        name: "Knox",
        path: "D:\\Knox",
        created_at: new Date().toISOString(),
      },
    ];
  },

  async scanStaged(_project?: string): Promise<ScanReport> {
    return {
      human_edits_detected: 0,
      actions_recorded: 0,
      files_scanned: ["src/lib.rs", "src/main.rs"],
      diff_stat: " 2 files changed, 10 insertions(+), 2 deletions(-)",
    };
  },

  async installHook(_path: string): Promise<string> {
    return "Installed hook";
  },

  async uninstallHook(_path: string): Promise<string> {
    return "Uninstalled hook";
  },

  async getKnoxMd(_path: string): Promise<string | null> {
    return null;
  },

  async getHookStatus(_path: string): Promise<boolean> {
    return false;
  },

  async startSession(actor: string, project?: string): Promise<Session> {
    return {
      id: "mock-sess-001",
      actor,
      project: project || null,
      started_at: new Date().toISOString(),
      ended_at: null,
      is_open: true,
    };
  },

  async endSession(sessionId: string): Promise<Session> {
    return {
      id: sessionId,
      actor: "gemini",
      project: null,
      started_at: new Date().toISOString(),
      ended_at: new Date().toISOString(),
      is_open: false,
    };
  },

  async getSessions(_limit?: number): Promise<Session[]> {
    return [
      {
        id: "s-001",
        actor: "gemini",
        project: "knox",
        started_at: new Date().toISOString(),
        ended_at: null,
        is_open: true,
      },
    ];
  },

  async getConfig(): Promise<KnoxConfig> {
    return {
      kg_path: "~/.knox/knox.db",
      session_timeout_seconds: 1800,
      global_mode: true,
    };
  },

  async updateConfig(params: {
    sessionTimeoutSeconds?: number;
    globalMode?: boolean;
    kgPath?: string;
  }): Promise<KnoxConfig> {
    return {
      kg_path: params.kgPath ?? "~/.knox/knox.db",
      session_timeout_seconds: params.sessionTimeoutSeconds ?? 1800,
      global_mode: params.globalMode ?? true,
    };
  },

  async saveConfig(config: KnoxConfig): Promise<KnoxConfig> {
    return config;
  },

  async getDbStats(): Promise<DbStats> {
    return {
      db_size_bytes: 4200000,
      wal_size_bytes: 131072,
      page_count: 1024,
      page_size: 4096,
      total_actions: 42,
      total_sessions: 5,
      total_relations: 18,
      schema_version: 8,
    };
  },

  async runDbIntegrity(): Promise<string> {
    return "ok";
  },

  async runDbVacuum(): Promise<string> {
    return "Database vacuum completed successfully.";
  },

  async revealDbFolder(): Promise<void> {},

  async openFileDialog(_options?: {
    name?: string;
    extensions?: string[];
  }): Promise<string | null> {
    return null;
  },

  async openFolderDialog(): Promise<string | null> {
    return null;
  },

  async revealInExplorer(path: string): Promise<void> {
    console.info("[DemoAPI] revealInExplorer:", path);
  },

  async detectMcpConfigs(): Promise<DetectedMcpApp[]> {
    return [
      {
        id: "cursor",
        name: "Cursor",
        category: "ide",
        file_path: "~/.cursor/mcp.json",
        exists: true,
        is_configured: false,
        format: "standard",
        last_modified: "2026-10-30 14:20:00 UTC",
        is_custom: false,
      },
      {
        id: "claude-desktop",
        name: "Claude Desktop",
        category: "ide",
        file_path: "%APPDATA%\\Claude\\claude_desktop_config.json",
        exists: true,
        is_configured: false,
        format: "standard",
        last_modified: "2026-10-29 11:15:00 UTC",
        is_custom: false,
      },
      {
        id: "antigravity",
        name: "Antigravity / Gemini CLI",
        category: "ide",
        file_path: "~/.gemini/config/mcp_config.json",
        exists: true,
        is_configured: true,
        format: "standard",
        last_modified: "2026-10-31 09:12:00 UTC",
        is_custom: false,
      },
    ];
  },

  async deepScanMcpConfigs(): Promise<DetectedMcpApp[]> {
    return this.detectMcpConfigs();
  },

  async inspectCustomMcpConfig(filePath: string): Promise<DetectedMcpApp> {
    return {
      id: "custom-mock",
      name: "Custom App",
      category: "custom",
      file_path: filePath,
      exists: true,
      is_configured: false,
      format: "standard",
      last_modified: new Date().toISOString(),
      is_custom: true,
    };
  },

  async injectMcpConfig(_filePath: string, _useSystemCommand?: boolean): Promise<boolean> {
    return true;
  },

  async removeMcpConfig(_filePath: string): Promise<boolean> {
    return true;
  },

  async listenMcpScanLogs(_callback: (log: McpScanLog) => void): Promise<() => void> {
    return () => {};
  },

  async recordQuickThought(params: {
    summary: string;
    category: "doubt" | "minor_decision" | "major_decision" | "thought";
    actor?: string;
    contextSummary?: string;
    files?: string[];
    project?: string;
  }): Promise<string> {
    const mockId = `mock-action-${Date.now()}`;
    mockActions.unshift({
      id: mockId,
      actor: params.actor || "human",
      action_type: params.category === "doubt" ? "doubt" : params.category.includes("decision") ? "decide" : "dialogue",
      summary: params.summary,
      files: params.files || [],
      tags: [params.category],
      confidence: 1.0,
      timestamp: new Date().toISOString(),
      session_id: null,
      parent_action_id: null,
      supersedes: null,
      is_current: true,
      significance: params.category === "major_decision" ? "major" : "minor",
      is_resolved: params.category !== "doubt",
      context_summary: params.contextSummary || null,
      project_id: params.project || null,
    });
    return mockId;
  },

  async resolveDoubt(params: {
    doubtId: string;
    resolutionNote: string;
    resolverActionId?: string;
  }): Promise<boolean> {
    const target = mockActions.find((a) => a.id === params.doubtId);
    if (target) {
      target.is_resolved = true;
      target.resolution_note = params.resolutionNote;
    }
    return true;
  },
};

