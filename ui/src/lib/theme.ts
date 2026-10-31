/** Appearance theme — CSS variables via `data-theme` on `<html>`. */

export type ThemeMode = "light" | "dark";

export const THEME_STORAGE_KEY = "knox_theme";
export const DEFAULT_THEME: ThemeMode = "dark";

export function isThemeMode(value: unknown): value is ThemeMode {
  return value === "light" || value === "dark";
}

/** Read persisted preference; defaults to dark on missing/invalid/unavailable storage. */
export function getStoredTheme(): ThemeMode {
  try {
    if (typeof window === "undefined") return DEFAULT_THEME;
    const raw = localStorage.getItem(THEME_STORAGE_KEY);
    return isThemeMode(raw) ? raw : DEFAULT_THEME;
  } catch {
    return DEFAULT_THEME;
  }
}

/**
 * Apply theme to the document and persist.
 * Clears FOUC inline colors from `index.html` so CSS variables take over.
 */
export function applyTheme(theme: ThemeMode): void {
  if (typeof document === "undefined") return;

  const root = document.documentElement;
  root.setAttribute("data-theme", theme);
  root.style.colorScheme = theme;
  root.classList.toggle("dark", theme === "dark");
  root.classList.toggle("light", theme === "light");

  root.style.removeProperty("background-color");
  root.style.removeProperty("color");
  if (document.body) {
    document.body.style.removeProperty("background-color");
    document.body.style.removeProperty("color");
  }

  try {
    localStorage.setItem(THEME_STORAGE_KEY, theme);
  } catch {
    /* storage unavailable — theme still applies for this session */
  }
}

export function toggleTheme(current: ThemeMode): ThemeMode {
  return current === "dark" ? "light" : "dark";
}
