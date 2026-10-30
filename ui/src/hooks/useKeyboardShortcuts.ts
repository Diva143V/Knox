import { useEffect, useRef } from "react";
import { useKnoxStore } from "@/store/useKnoxStore";
import type { ViewTab } from "@/types/knox";

export const useKeyboardShortcuts = () => {
  const {
    setActiveTab,
    selectedAction,
    setSelectedAction,
    commandPaletteOpen,
    setCommandPaletteOpen,
    refreshData,
  } = useKnoxStore();

  const lastKeyRef = useRef<{ key: string; time: number }>({ key: "", time: 0 });

  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      // Don't intercept if user is typing in an input or textarea
      const target = e.target as HTMLElement;
      if (
        target.tagName === "INPUT" ||
        target.tagName === "TEXTAREA" ||
        target.isContentEditable
      ) {
        if (e.key === "Escape") {
          target.blur();
        }
        return;
      }

      // F5: Refresh
      if (e.key === "F5") {
        e.preventDefault();
        refreshData();
        return;
      }

      // Escape: Dismiss selection or palette
      if (e.key === "Escape") {
        if (commandPaletteOpen) {
          setCommandPaletteOpen(false);
          return;
        }
        if (selectedAction) {
          setSelectedAction(null);
          return;
        }
      }

      // Check for 'G' prefix navigation sequence (e.g. G D, G G, G Q)
      const now = Date.now();
      const prev = lastKeyRef.current;
      const key = e.key.toLowerCase();

      if (prev.key === "g" && now - prev.time < 1200) {
        let targetTab: ViewTab | null = null;
        switch (key) {
          case "d":
            targetTab = "dashboard";
            break;
          case "g":
            targetTab = "graph";
            break;
          case "q":
            targetTab = "query";
            break;
          case "s":
            targetTab = "stats";
            break;
          case "i":
            targetTab = "ingest";
            break;
          case "c":
            targetTab = "scanner";
            break;
          case "m":
            targetTab = "mcp";
            break;
          case "w":
            targetTab = "workspace";
            break;
        }

        if (targetTab) {
          e.preventDefault();
          setActiveTab(targetTab);
          lastKeyRef.current = { key: "", time: 0 };
          return;
        }
      }

      // Number keys 1-8 for instant tab switching
      if (!e.ctrlKey && !e.metaKey && !e.altKey) {
        const numberMap: Record<string, ViewTab> = {
          "1": "dashboard",
          "2": "graph",
          "3": "query",
          "4": "stats",
          "5": "ingest",
          "6": "scanner",
          "7": "mcp",
          "8": "workspace",
        };
        if (numberMap[e.key]) {
          e.preventDefault();
          setActiveTab(numberMap[e.key]);
          return;
        }
      }

      // Record 'g' press
      if (key === "g" && !e.ctrlKey && !e.metaKey) {
        lastKeyRef.current = { key: "g", time: now };
      }
    };

    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [
    setActiveTab,
    selectedAction,
    setSelectedAction,
    commandPaletteOpen,
    setCommandPaletteOpen,
    refreshData,
  ]);
};

