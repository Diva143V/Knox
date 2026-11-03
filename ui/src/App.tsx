import React, { useEffect } from "react";
import { AppShell } from "@/components/layout/AppShell";
import { useKnoxStore } from "@/store/useKnoxStore";
import {
  initThemeListener,
  applyTheme,
  applyAccent,
  resolveTheme,
  getStoredThemePreference,
} from "@/lib/theme";

export const App: React.FC = () => {
  const accentColor = useKnoxStore((s) => s.userSettings.accentColor ?? null);
  const enableGlowFx = useKnoxStore((s) => s.userSettings.enableGlowFx);

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

  // Personalisation: override the accent ramp; the theme keeps everything else.
  useEffect(() => {
    applyAccent(accentColor);
  }, [accentColor]);

  // Appearance → Glow FX gates the graph's ambient motion (battery / focus).
  useEffect(() => {
    document.documentElement.classList.toggle("knox-fx", enableGlowFx);
  }, [enableGlowFx]);

  return <AppShell />;
};

export default App;
