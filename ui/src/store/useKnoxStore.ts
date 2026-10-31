//! Global application state management via Zustand.

import { create } from "zustand";
import { KnoxAPI } from "@/services/api";
import type { KnoxAction, KnoxConfig, StatusReport, ViewTab } from "@/types/knox";
import { isValidTab } from "@/types/knox";

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
  toast: { message: string; isError: boolean } | null;
  commandPaletteOpen: boolean;
  onboardingOpen: boolean;
  config: KnoxConfig | null;

  setActiveTab: (tab: ViewTab) => void;
  setSelectedAction: (action: KnoxAction | null) => void;
  setSearchQuery: (query: string) => void;
  setCurrentOnly: (currentOnly: boolean) => void;
  showToast: (message: string, isError?: boolean) => void;
  clearToast: () => void;
  toggleCommandPalette: () => void;
  setCommandPaletteOpen: (open: boolean) => void;
  setOnboardingOpen: (open: boolean) => void;
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

  showToast: (message, isError = false) => {
    set({ toast: { message, isError } });
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

