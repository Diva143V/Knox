import React, { useEffect } from "react";
import { AppShell } from "@/components/layout/AppShell";
import { useKnoxStore } from "@/store/useKnoxStore";
import {
  initThemeListener,
  applyTheme,
  resolveTheme,
  getStoredThemePreference,
} from "@/lib/theme";

export const App: React.FC = () => {
  useEffect(() => {
    // Ensure initial theme is applied immediately
    const initialPref = getStoredThemePreference();
    applyTheme(resolveTheme(initialPref));

    // Listen for live OS system theme changes
    const cleanup = initThemeListener((systemResolved) => {
      const currentPref = useKnoxStore.getState().theme;
      if (currentPref === "system") {
        applyTheme(systemResolved);
        useKnoxStore.setState({ resolvedTheme: systemResolved });
      }
    });

    return cleanup;
  }, []);

  return <AppShell />;
};

export default App;
