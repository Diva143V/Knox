//! Type-safe IPC service layer communicating with Knox Rust backend.

import { invoke } from "@tauri-apps/api/core";
import type {
  KnoxAction,
  StatusReport,
  QueryResult,
  ScanReport,
  IngestReport,
  KnoxConfig,
  Session,
} from "@/types/knox";

const isTauri =
  typeof window !== "undefined" &&
  ("__TAURI_INTERNALS__" in window || "__TAURI__" in window);

export const KnoxAPI = {
  async getStatus(): Promise<StatusReport> {
    if (!isTauri) {
      return {
        total_actions: 42,
        total_sessions: 5,
        projects_tracked: 2,
        last_action_timestamp: new Date().toISOString(),
        actions_by_actor: { gemini: 24, cursor: 12, human: 6 },
        actions_by_type: { write: 20, decide: 10, plan: 8, debug: 4 },
      };
    }
    return invoke<StatusReport>("get_status");
  },

  async queryActions(params: {
    search?: string;
    filePath?: string;
    file_path?: string;
    actor?: string;
    actionType?: string;
    action_type?: string;
    since?: string;
    currentOnly?: boolean;
    current_only?: boolean;
    limit?: number;
  }): Promise<QueryResult> {
    if (!isTauri) {
      return {
        results: mockActions,
        total_count: mockActions.length,
        has_more: false,
      };
    }
    const filePath = params.filePath || params.file_path || null;
    const actionType = params.actionType || params.action_type || null;
    const currentOnly = params.currentOnly ?? params.current_only ?? true;

    return invoke<QueryResult>("query_actions", {
      search: params.search || null,
      filePath: filePath,
      file_path: filePath,
      actor: params.actor || null,
      actionType: actionType,
      action_type: actionType,
      since: params.since || null,
      currentOnly: currentOnly,
      current_only: currentOnly,
      limit: params.limit ?? 50,
    });
  },

  async getAction(id: string): Promise<KnoxAction | null> {
    if (!isTauri) {
      return mockActions.find((a) => a.id === id) || null;
    }
    return invoke<KnoxAction | null>("get_action", { id });
  },

  async getGraphData(currentOnly?: boolean, limit?: number): Promise<KnoxAction[]> {
    if (!isTauri) {
      return mockActions;
    }
    return invoke<KnoxAction[]>("get_graph_data", {
      currentOnly: currentOnly ?? false,
      limit: limit ?? 200,
    });
  },

  async ingestTranscript(path: string): Promise<IngestReport> {
    if (!isTauri) {
      return { actions_imported: 7, files_touched: 3, sessions_created: 1 };
    }
    return invoke<IngestReport>("ingest_transcript", { path });
  },

  async initProject(path: string): Promise<string> {
    if (!isTauri) {
      return `Initialized Knox in ${path}`;
    }
    return invoke<string>("init_project", { path });
  },

  async scanStaged(project?: string): Promise<ScanReport> {
    if (!isTauri) {
      return {
        human_edits_detected: 0,
        actions_recorded: 0,
        files_scanned: ["src/lib.rs", "src/main.rs"],
        diff_stat: " 2 files changed, 10 insertions(+), 2 deletions(-)",
      };
    }
    return invoke<ScanReport>("scan_staged", { project: project || null });
  },

  async installHook(path: string): Promise<string> {
    if (!isTauri) return "Installed hook";
    return invoke<string>("install_hook", { path });
  },

  async uninstallHook(path: string): Promise<string> {
    if (!isTauri) return "Uninstalled hook";
    return invoke<string>("uninstall_hook", { path });
  },

  async getKnoxMd(path: string): Promise<string | null> {
    if (!isTauri) return null;
    return invoke<string | null>("get_knox_md", { path });
  },

  async getHookStatus(path: string): Promise<boolean> {
    if (!isTauri) return false;
    return invoke<boolean>("get_hook_status", { path });
  },

  async startSession(actor: string, project?: string): Promise<Session> {
    if (!isTauri) {
      return {
        id: "mock-sess-001",
        actor,
        project: project || null,
        started_at: new Date().toISOString(),
        ended_at: null,
        is_open: true,
      };
    }
    return invoke<Session>("start_session", { actor, project: project || null });
  },

  async endSession(sessionId: string): Promise<Session> {
    if (!isTauri) {
      return {
        id: sessionId,
        actor: "gemini",
        project: null,
        started_at: new Date().toISOString(),
        ended_at: new Date().toISOString(),
        is_open: false,
      };
    }
    return invoke<Session>("end_session", { sessionId });
  },

  async getSessions(limit?: number): Promise<Session[]> {
    if (!isTauri) {
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
    }
    return invoke<Session[]>("get_sessions", { limit: limit ?? 20 });
  },

  async getConfig(): Promise<KnoxConfig> {
    if (!isTauri) {
      return {
        kg_path: "~/.knox/knox.db",
        session_timeout_seconds: 1800,
        global_mode: true,
      };
    }
    return invoke<KnoxConfig>("get_config");
  },

  async openFileDialog(options?: {
    name?: string;
    extensions?: string[];
  }): Promise<string | null> {
    if (!isTauri) return null;
    try {
      return await invoke<string | null>("open_file_dialog", {
        name: options?.name || null,
        extensions: options?.extensions || null,
      });
    } catch {
      const { open } = await import("@tauri-apps/plugin-dialog");
      const selected = await open({
        multiple: false,
        filters: options?.extensions
          ? [{ name: options.name || "Files", extensions: options.extensions }]
          : undefined,
      });
      return typeof selected === "string" ? selected : null;
    }
  },

  async openFolderDialog(): Promise<string | null> {
    if (!isTauri) return null;
    try {
      return await invoke<string | null>("open_folder_dialog");
    } catch {
      const { open } = await import("@tauri-apps/plugin-dialog");
      const selected = await open({
        directory: true,
        multiple: false,
      });
      return typeof selected === "string" ? selected : null;
    }
  },
};

const mockActions: KnoxAction[] = [
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

