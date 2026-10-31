//! Global application state management via Zustand.

import { create } from "zustand";
import { KnoxAPI } from "@/services/api";
import type { KnoxAction, KnoxConfig, StatusReport, ViewTab } from "@/types/knox";
import { isValidTab } from "@/types/knox";
import type { ToastVariant } from "@/components/ui/Toast";
import { applyTheme, getStoredTheme, type ThemeMode } from "@/lib/theme";

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

interface KnoxStore {
  activeTab: ViewTab;
  status: StatusReport | null;
  actions: KnoxAction[];
  selectedAction: KnoxAction | null;
  searchQuery: string;
  currentOnly: boolean;
  isLoading: boolean;
  toast: { message: string; isError: boolean; variant: ToastVariant } | null;
  commandPaletteOpen: boolean;
  onboardingOpen: boolean;
  config: KnoxConfig | null;
  theme: ThemeMode;

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
  setTheme: (theme: ThemeMode) => void;
  refreshData: () => Promise<void>;
}

export const useKnoxStore = create<KnoxStore>((set, get) => ({
  activeTab: getInitialTab(),
  status: null,
  actions: [],
  selectedAction: null,
  searchQuery: "",
  currentOnly: true,
  isLoading: false,
  toast: null,
  commandPaletteOpen: false,
  onboardingOpen: isFirstRun(),
  config: null,
  theme: getStoredTheme(),

  setActiveTab: (activeTab) => {
    if (get().activeTab !== activeTab) {
      set({ activeTab });
    }
  },
  setSelectedAction: (selectedAction) => set({ selectedAction }),
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
      const [status, actions, config] = await Promise.all([
        KnoxAPI.getStatus(),
        KnoxAPI.getGraphData(get().currentOnly, 250),
        KnoxAPI.getConfig(),
      ]);
      set({ status, actions, config, isLoading: false });
    } catch (err) {
      console.error("Failed to refresh Knox state:", err);
      set({ isLoading: false });
      get().showToast(String(err), true);
    }
  },
}));

