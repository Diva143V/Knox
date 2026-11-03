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

/** Accent presets offered in Settings → Appearance. */
export const ACCENT_PRESETS = [
  { hex: "#4a8ad6", label: "Signal blue" },
  { hex: "#44a282", label: "Deep teal" },
  { hex: "#d9a441", label: "Amber" },
  { hex: "#e5566f", label: "Rose" },
  { hex: "#8b7bd8", label: "Violet" },
  { hex: "#8fbf4a", label: "Lime" },
] as const;

export function isAccentHex(value: unknown): value is string {
  return typeof value === "string" && /^#[0-9a-f]{6}$/i.test(value);
}

function channelToLinear(channel: number): number {
  const c = channel / 255;
  return c <= 0.04045 ? c / 12.92 : Math.pow((c + 0.055) / 1.055, 2.4);
}

/** White or near-black, whichever reads on the chosen accent. */
export function accentForeground(hex: string): string {
  const r = channelToLinear(parseInt(hex.slice(1, 3), 16));
  const g = channelToLinear(parseInt(hex.slice(3, 5), 16));
  const b = channelToLinear(parseInt(hex.slice(5, 7), 16));
  const luminance = 0.2126 * r + 0.7152 * g + 0.0722 * b;
  return luminance > 0.45 ? "#0b0b0c" : "#ffffff";
}

/**
 * Apply (or clear) a user accent by overriding the primary ramp on <html>.
 * Derived shades use color-mix so hover/muted/focus follow the accent exactly;
 * the foreground is picked by luminance so light accents stay readable.
 */
export function applyAccent(hex: string | null | undefined): void {
  if (typeof document === "undefined") return;
  const root = document.documentElement;
  const managed = [
    "--primary",
    "--primary-foreground",
    "--primary-hover",
    "--primary-muted",
    "--focus-ring",
    "--plan",
  ];

  if (!isAccentHex(hex)) {
    for (const token of managed) root.style.removeProperty(token);
    return;
  }

  root.style.setProperty("--primary", hex);
  root.style.setProperty("--plan", hex);
  root.style.setProperty("--primary-foreground", accentForeground(hex));
  root.style.setProperty("--primary-hover", `color-mix(in oklab, ${hex} 88%, #000)`);
  root.style.setProperty("--primary-muted", `color-mix(in srgb, ${hex} 16%, transparent)`);
  root.style.setProperty("--focus-ring", `color-mix(in srgb, ${hex} 45%, transparent)`);
}
