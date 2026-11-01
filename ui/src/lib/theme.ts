/** Appearance theme — CSS variables via `data-theme` on `<html>`. */

export type ThemePreference = "dark" | "light" | "system";
export type ResolvedTheme = "dark" | "light";

// Backwards-compatible alias for existing code referencing ThemeMode
export type ThemeMode = ResolvedTheme;

export const THEME_STORAGE_KEY = "knox_theme";
export const SETTINGS_STORAGE_KEY = "knox_user_settings_v1";
export const DEFAULT_THEME_PREFERENCE: ThemePreference = "dark";
export const DEFAULT_THEME: ResolvedTheme = "dark";

export function isThemePreference(value: unknown): value is ThemePreference {
  return value === "light" || value === "dark" || value === "system";
}

export function isResolvedTheme(value: unknown): value is ResolvedTheme {
  return value === "light" || value === "dark";
}

/** Check OS dark mode preference. */
export function getSystemTheme(): ResolvedTheme {
  if (typeof window === "undefined" || !window.matchMedia) return "dark";
  return window.matchMedia("(prefers-color-scheme: dark)").matches ? "dark" : "light";
}

/** Resolve a user preference ('dark' | 'light' | 'system') to concrete 'dark' | 'light'. */
export function resolveTheme(preference: ThemePreference): ResolvedTheme {
  if (preference === "system") {
    return getSystemTheme();
  }
  return preference;
}

/**
 * Read persisted preference with safe migration.
 * Checks `knox_theme`, falls back to `knox_user_settings_v1.theme`, and normalizes.
 */
export function getStoredThemePreference(): ThemePreference {
  try {
    if (typeof window === "undefined") return DEFAULT_THEME_PREFERENCE;
    const raw = localStorage.getItem(THEME_STORAGE_KEY);
    if (isThemePreference(raw)) return raw;

    // Safe migration from knox_user_settings_v1
    const rawSettings = localStorage.getItem(SETTINGS_STORAGE_KEY);
    if (rawSettings) {
      try {
        const parsed = JSON.parse(rawSettings);
        if (parsed && isThemePreference(parsed.theme)) {
          localStorage.setItem(THEME_STORAGE_KEY, parsed.theme);
          return parsed.theme;
        }
      } catch {}
    }
    return DEFAULT_THEME_PREFERENCE;
  } catch {
    return DEFAULT_THEME_PREFERENCE;
  }
}

/** Backwards-compatible getter returning resolved theme mode ('dark' | 'light'). */
export function getStoredTheme(): ResolvedTheme {
  return resolveTheme(getStoredThemePreference());
}

/**
 * Apply resolved theme to the document and persist preference to `localStorage`.
 * Clears FOUC inline colors from `index.html` so CSS variables take over.
 */
export function applyTheme(resolved: ResolvedTheme): void {
  if (typeof document === "undefined") return;

  const root = document.documentElement;
  root.setAttribute("data-theme", resolved);
  root.style.colorScheme = resolved;
  root.classList.toggle("dark", resolved === "dark");
  root.classList.toggle("light", resolved === "light");

  root.style.removeProperty("background-color");
  root.style.removeProperty("color");
  if (document.body) {
    document.body.style.removeProperty("background-color");
    document.body.style.removeProperty("color");
  }
}

/**
 * Listen for live OS system theme changes.
 * Calls handler with new resolved theme when system theme shifts.
 */
export function initThemeListener(onSystemChange: (resolved: ResolvedTheme) => void): () => void {
  if (typeof window === "undefined" || !window.matchMedia) return () => {};
  const mediaQuery = window.matchMedia("(prefers-color-scheme: dark)");
  const handler = (e: MediaQueryListEvent) => {
    onSystemChange(e.matches ? "dark" : "light");
  };
  mediaQuery.addEventListener("change", handler);
  return () => mediaQuery.removeEventListener("change", handler);
}

export function toggleTheme(current: ResolvedTheme): ResolvedTheme {
  return current === "dark" ? "light" : "dark";
}
