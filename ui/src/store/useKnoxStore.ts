//! Global application state management via Zustand.

import { create } from "zustand";
import { KnoxAPI } from "@/services/api";
import type {
  KnoxAction,
  KnoxConfig,
  Project,
  StatusReport,
  ViewTab,
  UserSettings,
  SettingsTabId,
} from "@/types/knox";
import { isValidTab, DEFAULT_USER_SETTINGS } from "@/types/knox";
import type { ToastVariant } from "@/components/ui/Toast";
import { applyTheme, getStoredTheme, type ThemeMode } from "@/lib/theme";

const SETTINGS_STORAGE_KEY = "knox_user_settings_v1";

const getSavedUserSettings = (): UserSettings => {
  try {
    const raw = localStorage.getItem(SETTINGS_STORAGE_KEY);
    if (raw) {
      const parsed = JSON.parse(raw);
      return { ...DEFAULT_USER_SETTINGS, ...parsed };
    }
  } catch {
    // fallback
  }
  return DEFAULT_USER_SETTINGS;
};

const getInitialTab = (): ViewTab => {
  if (typeof window !== "undefined" && window.location.hash) {
    const hash = window.location.hash.replace(/^#\/?/, "").toLowerCase();
    if (isValidTab(hash)) {
      return hash;
    }
  }
  return "dashboard";
};

const isFirstRun = (): boolean => {
  try {
    return typeof window !== "undefined" && !localStorage.getItem("knox_onboarding_completed");
  } catch {
    return false;
  }
};

const isTourFirstRun = (): boolean => {
  try {
    return (
      typeof window !== "undefined" &&
      localStorage.getItem("knox_has_completed_tour") !== "true"
    );
  } catch {
    return false;
  }
};

const STORAGE_KEY_RECENT_PROJECTS = "knox_recent_projects";
const STORAGE_KEY_ACTIVE_PROJECT = "knox_active_project";

export interface RecentProject {
  name: string;
  path: string;
}

const getSavedRecentProjects = (): RecentProject[] => {
  try {
    const raw = localStorage.getItem(STORAGE_KEY_RECENT_PROJECTS);
    if (raw) return JSON.parse(raw);
  } catch {
    // ignore
  }
  return [];
};

const getSavedActiveProject = (): string | null => {
  try {
    return localStorage.getItem(STORAGE_KEY_ACTIVE_PROJECT);
  } catch {
    return null;
  }
};

interface KnoxStore {
  activeTab: ViewTab;
  status: StatusReport | null;
  actions: KnoxAction[];
  selectedAction: KnoxAction | null;
  projects: Project[];
  selectedProject: Project | null;
  searchQuery: string;
  currentOnly: boolean;
  isLoading: boolean;
  toast: { message: string; isError: boolean; variant: ToastVariant } | null;
  commandPaletteOpen: boolean;
  onboardingOpen: boolean;
  isTourOpen: boolean;
  isSettingsOpen: boolean;
  activeSettingsTab: SettingsTabId;
  isEditNodeOpen: boolean;
  editingAction: KnoxAction | null;
  config: KnoxConfig | null;
  theme: ThemeMode;
  userSettings: UserSettings;

  // Global KG vs Project Folder Scope
  activeProjectScope: string | null;
  recentProjects: RecentProject[];
  setProjectScope: (scope: string | null) => void;
  addRecentProject: (path: string) => void;
  removeRecentProject: (path: string) => void;
  setSelectedProject: (project: Project | null) => void;
  getProjectForAction: (action: KnoxAction) => Project | null;

  setActiveTab: (tab: ViewTab) => void;
  setSelectedAction: (action: KnoxAction | null) => void;
  setSearchQuery: (query: string) => void;
  setCurrentOnly: (currentOnly: boolean) => void;
  /** Pass `true` for error, or a ToastVariant for success/error/warning/info. */
  showToast: (message: string, isErrorOrVariant?: boolean | ToastVariant) => void;
  clearToast: () => void;
  toggleCommandPalette: () => void;
  setCommandPaletteOpen: (open: boolean) => void;
  setOnboardingOpen: (open: boolean) => void;
  openTour: () => void;
  closeTour: () => void;
  openSettings: (tab?: SettingsTabId) => void;
  closeSettings: () => void;
  openEditNode: (action: KnoxAction) => void;
  closeEditNode: () => void;
  updateActionInStore: (action: KnoxAction) => void;
  refreshDataQuietly: () => Promise<void>;
  setTheme: (theme: ThemeMode) => void;
  updateUserSettings: (patch: Partial<UserSettings>) => void;
  resetUserSettings: () => void;
  updateConfig: (params: {
    sessionTimeoutSeconds?: number;
    globalMode?: boolean;
    kgPath?: string;
  }) => Promise<KnoxConfig>;
  refreshData: () => Promise<void>;
}

export const useKnoxStore = create<KnoxStore>((set, get) => ({
  activeTab: getInitialTab(),
  status: null,
  actions: [],
  selectedAction: null,
  projects: [],
  selectedProject: null,
  searchQuery: "",
  currentOnly: true,
  isLoading: false,
  toast: null,
  commandPaletteOpen: false,
  onboardingOpen: !isTourFirstRun() && isFirstRun(),
  isTourOpen: isTourFirstRun(),
  isSettingsOpen: false,
  activeSettingsTab: "general",
  isEditNodeOpen: false,
  editingAction: null,
  openEditNode: (action) => set({ isEditNodeOpen: true, editingAction: action }),
  closeEditNode: () => set({ isEditNodeOpen: false, editingAction: null }),
  updateActionInStore: (action) =>
    set((state) => ({
      actions: state.actions.map((a) => (a.id === action.id ? action : a)),
      selectedAction:
        state.selectedAction?.id === action.id ? action : state.selectedAction,
    })),
  openTour: () => set({ isTourOpen: true }),
  closeTour: () => set({ isTourOpen: false }),
  openSettings: (tab) =>
    set({ isSettingsOpen: true, activeSettingsTab: tab ?? "general" }),
  closeSettings: () => set({ isSettingsOpen: false }),
  config: null,
  theme: getStoredTheme(),
  userSettings: getSavedUserSettings(),

  updateUserSettings: (patch) => {
    const current = get().userSettings;
    const updated = { ...current, ...patch };
    try {
      localStorage.setItem(SETTINGS_STORAGE_KEY, JSON.stringify(updated));
    } catch {
      // ignore
    }
    if (patch.theme && patch.theme !== current.theme) {
      if (patch.theme === "system") {
        const isDark =
          typeof window !== "undefined" &&
          window.matchMedia("(prefers-color-scheme: dark)").matches;
        applyTheme(isDark ? "dark" : "light");
      } else {
        applyTheme(patch.theme);
      }
      set({ theme: patch.theme === "light" ? "light" : "dark" });
    }
    set({ userSettings: updated });
  },

  resetUserSettings: () => {
    try {
      localStorage.removeItem(SETTINGS_STORAGE_KEY);
    } catch {
      // ignore
    }
    applyTheme("dark");
    set({ userSettings: DEFAULT_USER_SETTINGS, theme: "dark" });
  },

  activeProjectScope: getSavedActiveProject(),
  recentProjects: getSavedRecentProjects(),

  setProjectScope: (scope: string | null) => {
    try {
      if (scope) {
        localStorage.setItem(STORAGE_KEY_ACTIVE_PROJECT, scope);
      } else {
        localStorage.removeItem(STORAGE_KEY_ACTIVE_PROJECT);
      }
    } catch {
      // ignore
    }
    set({ activeProjectScope: scope });
    get().refreshData();
  },

  addRecentProject: (path: string) => {
    const normalized = path.trim();
    if (!normalized) return;
    const parts = normalized.replace(/\\/g, "/").split("/");
    const name = parts.filter(Boolean).pop() || normalized;
    const existing = get().recentProjects.filter(
      (p) => p.path.toLowerCase() !== normalized.toLowerCase()
    );
    const updated = [{ name, path: normalized }, ...existing].slice(0, 8);
    try {
      localStorage.setItem(STORAGE_KEY_RECENT_PROJECTS, JSON.stringify(updated));
    } catch {
      // ignore
    }
    set({ recentProjects: updated });
    get().setProjectScope(normalized);
  },

  removeRecentProject: (path: string) => {
    const updated = get().recentProjects.filter(
      (p) => p.path.toLowerCase() !== path.toLowerCase()
    );
    try {
      localStorage.setItem(STORAGE_KEY_RECENT_PROJECTS, JSON.stringify(updated));
    } catch {
      // ignore
    }
    set({ recentProjects: updated });
    if (get().activeProjectScope?.toLowerCase() === path.toLowerCase()) {
      get().setProjectScope(null);
    }
  },

  setSelectedProject: (selectedProject) =>
    set({ selectedProject, selectedAction: null }),

  getProjectForAction: (action) => {
    const projects = get().projects;
    if (!projects || projects.length === 0) {
      const scope = get().activeProjectScope;
      if (scope) {
        const name = scope.replace(/\\/g, "/").split("/").filter(Boolean).pop() || scope;
        return { id: "scope", name, path: scope, created_at: "" };
      }
      return { id: "default", name: "Knox", path: "D:\\Knox", created_at: "" };
    }

    // 1. Match touched files against project paths
    for (const proj of projects) {
      const normProjPath = proj.path.toLowerCase().replace(/\\/g, "/");
      const projName = proj.name.toLowerCase();
      if (
        action.files &&
        action.files.some((f) => {
          const normF = f.toLowerCase().replace(/\\/g, "/");
          return normF.startsWith(normProjPath) || normF.includes(projName);
        })
      ) {
        return proj;
      }
    }

    // 2. Match tags or summary
    for (const proj of projects) {
      const projName = proj.name.toLowerCase();
      if (
        (action.tags && action.tags.some((t) => t.toLowerCase() === projName)) ||
        action.summary.toLowerCase().includes(projName)
      ) {
        return proj;
      }
    }

    // 3. Fallback to first project
    return projects[0];
  },

  setActiveTab: (activeTab) => {
    if (get().activeTab !== activeTab) {
      set({ activeTab });
    }
  },
  setSelectedAction: (selectedAction) =>
    set({ selectedAction, selectedProject: null }),
  setSearchQuery: (searchQuery) => set({ searchQuery }),
  setCurrentOnly: (currentOnly) => {
    set({ currentOnly });
    get().refreshData();
  },

  showToast: (message, isErrorOrVariant = false) => {
    const variant: ToastVariant =
      typeof isErrorOrVariant === "string"
        ? isErrorOrVariant
        : isErrorOrVariant
          ? "error"
          : "success";
    const isError = variant === "error";
    set({ toast: { message, isError, variant } });
    setTimeout(() => {
      if (get().toast?.message === message) {
        set({ toast: null });
      }
    }, 4000);
  },

  clearToast: () => set({ toast: null }),
  toggleCommandPalette: () =>
    set((state) => ({ commandPaletteOpen: !state.commandPaletteOpen })),
  setCommandPaletteOpen: (commandPaletteOpen) => set({ commandPaletteOpen }),
  setOnboardingOpen: (onboardingOpen) => set({ onboardingOpen }),
  setTheme: (theme) => {
    applyTheme(theme);
    set({ theme });
  },

  refreshData: async () => {
    try {
      set({ isLoading: true });
      const scope = get().activeProjectScope;
      const [status, allActions, config, rawProjects] = await Promise.all([
        KnoxAPI.getStatus(),
        KnoxAPI.getGraphData(get().currentOnly, 350),
        KnoxAPI.getConfig(),
        KnoxAPI.getProjects().catch(() => []),
      ]);

      // Filter out temporary / scratch test directories
      const isTemporary = (p: Project) => {
        const norm = p.path.toLowerCase().replace(/\\/g, "/");
        return (
          norm.includes("/temp/") ||
          norm.includes("/appdata/local/temp") ||
          p.name.toLowerCase().startsWith("tmp") ||
          p.name.toLowerCase().includes("verification")
        );
      };

      const validProjects = (rawProjects || []).filter((p) => !isTemporary(p));

      // Normalize tracked projects and ensure at least Knox is tracked if empty
      const trackedProjects =
        validProjects.length > 0
          ? validProjects
          : [
              {
                id: "default-knox",
                name: "Knox",
                path: "D:\\Knox",
                created_at: new Date().toISOString(),
              },
            ];

      // Sync tracked projects into recentProjects
      const currentRecent = get().recentProjects;
      const mergedRecent = [...currentRecent];
      trackedProjects.forEach((tp) => {
        const normPath = tp.path.toLowerCase().replace(/\\/g, "/");
        if (
          !mergedRecent.some(
            (r) => r.path.toLowerCase().replace(/\\/g, "/") === normPath
          )
        ) {
          mergedRecent.push({ name: tp.name, path: tp.path });
        }
      });

      let actions = allActions;
      if (scope) {
        const normScope = scope.toLowerCase().replace(/\\/g, "/");
        const folderName = normScope.split("/").filter(Boolean).pop() || normScope;
        actions = allActions.filter((a) => {
          const hasMatchingFile =
            a.files &&
            a.files.some((f) => {
              const normF = f.toLowerCase().replace(/\\/g, "/");
              return normF.startsWith(normScope) || normF.includes(folderName);
            });
          const matchesTag =
            a.tags && a.tags.some((t) => t.toLowerCase().includes(folderName));
          const matchesSummary = a.summary.toLowerCase().includes(folderName);
          return hasMatchingFile || matchesTag || matchesSummary;
        });
      }

      set({
        status,
        actions,
        config,
        projects: trackedProjects,
        recentProjects: mergedRecent.slice(0, 8),
        isLoading: false,
      });
    } catch (err) {
      console.error("Failed to refresh Knox state:", err);
      set({ isLoading: false });
      get().showToast(String(err), true);
    }
  },

  refreshDataQuietly: async () => {
    try {
      const scope = get().activeProjectScope;
      const [status, queryRes, rawProjects] = await Promise.all([
        KnoxAPI.getStatus(),
        KnoxAPI.queryActions({ limit: 1000 }),
        KnoxAPI.getProjects().catch(() => []),
      ]);

      const allActions = queryRes.results || [];
      const isTemporary = (p: Project) => {
        const norm = p.path.toLowerCase().replace(/\\/g, "/");
        return (
          norm.includes("/temp/") ||
          norm.includes("/tmp/") ||
          norm.includes("/tests/") ||
          p.name.toLowerCase().includes("temp") ||
          p.name.toLowerCase().includes("verification")
        );
      };
      const validProjects = (rawProjects || []).filter((p) => !isTemporary(p));
      const trackedProjects =
        validProjects.length > 0
          ? validProjects
          : [
              {
                id: "default-knox",
                name: "Knox",
                path: "D:\\Knox",
                created_at: new Date().toISOString(),
              },
            ];

      let actions = allActions;
      if (scope) {
        const normScope = scope.toLowerCase().replace(/\\/g, "/");
        const folderName = normScope.split("/").filter(Boolean).pop() || normScope;
        actions = allActions.filter((a) => {
          const hasMatchingFile =
            a.files &&
            a.files.some((f) => {
              const normF = f.toLowerCase().replace(/\\/g, "/");
              return normF.startsWith(normScope) || normF.includes(folderName);
            });
          const matchesTag =
            a.tags && a.tags.some((t) => t.toLowerCase().includes(folderName));
          const matchesSummary = a.summary.toLowerCase().includes(folderName);
          return hasMatchingFile || matchesTag || matchesSummary;
        });
      }

      const currentSelected = get().selectedAction;
      const updatedSelected = currentSelected
        ? actions.find((a) => a.id === currentSelected.id) || currentSelected
        : null;

      set({
        status,
        actions,
        projects: trackedProjects,
        selectedAction: updatedSelected,
      });
    } catch (err) {
      console.debug("Quiet refresh skipped or failed:", err);
    }
  },

  updateConfig: async (params) => {
    try {
      const updated = await KnoxAPI.updateConfig(params);
      set({ config: updated });
      return updated;
    } catch (err) {
      console.error("Failed to update config:", err);
      get().showToast(String(err), true);
      throw err;
    }
  },
}));

