import React from "react";
import { useKnoxStore } from "@/store/useKnoxStore";
import { cn } from "@/lib/cn";
import type { ViewTab } from "@/types/knox";

export const AppearanceTab: React.FC = () => {
  const { userSettings, updateUserSettings } = useKnoxStore();

  return (
    <div className="space-y-6 animate-in fade-in-50 duration-150">
      <div>
        <h3 className="text-sm font-semibold text-foreground">Appearance & Theme</h3>
        <p className="text-xs text-muted-foreground">Customize how Knox looks and behaves across your desktop</p>
      </div>

      <div className="space-y-4">
        {/* Theme Mode */}
        <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-3 p-3.5 rounded-lg border border-border bg-card/60">
          <div>
            <label className="text-xs font-medium text-foreground block">Color Theme</label>
            <span className="text-xs text-muted-foreground">
              Select between Graphite Vault (Dark), Graphite Light, or match OS theme
            </span>
          </div>
          <div className="flex items-center gap-1.5 shrink-0">
            {(["dark", "light", "system"] as const).map((mode) => (
              <button
                key={mode}
                type="button"
                onClick={() => updateUserSettings({ theme: mode })}
                className={cn(
                  "px-3 py-1.5 rounded-md text-xs font-medium capitalize border transition-all cursor-pointer",
                  userSettings.theme === mode
                    ? "bg-primary text-primary-foreground border-primary shadow-xs font-semibold"
                    : "bg-muted/70 text-muted-foreground border-border hover:text-foreground hover:bg-muted"
                )}
              >
                {mode}
              </button>
            ))}
          </div>
        </div>

        {/* Accent Color — personalisation */}
        <div className="space-y-3 rounded-[var(--radius-card)] border border-border bg-card/60 p-3.5">
          <div className="flex items-start justify-between gap-3">
            <div>
              <span className="text-xs font-medium text-foreground block">Accent Color</span>
              <span className="text-xs text-muted-foreground">
                Drives selection, focus rings and the graph's live paths, in both themes
              </span>
            </div>
            <button
              type="button"
              onClick={() => updateUserSettings({ accentColor: null })}
              disabled={!userSettings.accentColor}
              className={cn(
                "shrink-0 rounded-[var(--radius-control)] border border-border px-2.5 py-1 text-xs transition-colors",
                userSettings.accentColor
                  ? "text-muted-foreground hover:bg-muted hover:text-foreground cursor-pointer"
                  : "text-muted-foreground/50 cursor-not-allowed"
              )}
            >
              Reset
            </button>
          </div>

          <div
            role="radiogroup"
            aria-label="Accent color"
            className="flex flex-wrap items-center gap-2"
          >
            <button
              type="button"
              role="radio"
              aria-checked={!userSettings.accentColor}
              onClick={() => updateUserSettings({ accentColor: null })}
              title="Follow the theme accent"
              className={cn(
                "flex items-center gap-1.5 rounded-[var(--radius-control)] border px-2.5 py-1 text-xs transition-colors cursor-pointer",
                !userSettings.accentColor
                  ? "border-primary bg-primary-muted text-foreground"
                  : "border-border text-muted-foreground hover:text-foreground"
              )}
            >
              <span className="size-3 rounded-full border border-current" aria-hidden />
              Theme default
            </button>

            {([
              ["#4a8ad6", "Signal blue"],
              ["#44a282", "Deep teal"],
              ["#d9a441", "Amber"],
              ["#e5566f", "Rose"],
              ["#8b7bd8", "Violet"],
              ["#8fbf4a", "Lime"],
            ] as const).map(([hex, name]) => {
              const active =
                (userSettings.accentColor ?? "").toLowerCase() === hex;
              return (
                <button
                  key={hex}
                  type="button"
                  role="radio"
                  aria-checked={active}
                  aria-label={name}
                  title={name}
                  onClick={() => updateUserSettings({ accentColor: hex })}
                  className={cn(
                    "size-6 rounded-full border transition-transform duration-100 cursor-pointer",
                    "focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-focus-ring focus-visible:ring-offset-2 focus-visible:ring-offset-background",
                    active
                      ? "scale-110 border-foreground"
                      : "border-border hover:scale-105"
                  )}
                  style={{ backgroundColor: hex }}
                />
              );
            })}

            <label
              className="relative flex size-6 items-center justify-center rounded-full border border-dashed border-border text-muted-foreground transition-colors hover:text-foreground focus-within:ring-2 focus-within:ring-focus-ring cursor-pointer"
              title="Custom color"
            >
              <input
                type="color"
                value={userSettings.accentColor ?? "#4a8ad6"}
                onChange={(e) =>
                  updateUserSettings({ accentColor: e.target.value.toLowerCase() })
                }
                className="absolute inset-0 size-full cursor-pointer opacity-0"
                aria-label="Custom accent color"
              />
              <span aria-hidden className="text-xs leading-none">
                +
              </span>
            </label>
          </div>

          <p className="font-mono text-3xs text-muted-foreground">
            {userSettings.accentColor ?? "Following the theme accent"}
          </p>
        </div>

        {/* UI Density */}
        <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-3 p-3.5 rounded-lg border border-border bg-card/60">
          <div>
            <label className="text-xs font-medium text-foreground block">Interface Density</label>
            <span className="text-xs text-muted-foreground">
              Compact mode condenses table padding and action card sizes
            </span>
          </div>
          <div className="flex items-center gap-1.5 shrink-0">
            {(["comfortable", "compact"] as const).map((d) => (
              <button
                key={d}
                type="button"
                onClick={() => updateUserSettings({ uiDensity: d })}
                className={cn(
                  "px-3 py-1.5 rounded-md text-xs font-medium capitalize border transition-all cursor-pointer",
                  userSettings.uiDensity === d
                    ? "bg-primary text-primary-foreground border-primary shadow-xs font-semibold"
                    : "bg-muted/70 text-muted-foreground border-border hover:text-foreground hover:bg-muted"
                )}
              >
                {d}
              </button>
            ))}
          </div>
        </div>

        {/* Glow & Bloom FX */}
        <div className="flex items-center justify-between p-3.5 rounded-lg border border-border bg-card/60">
          <div>
            <label htmlFor="setting-glow-fx" className="text-xs font-medium text-foreground block cursor-pointer">
              Graph Motion & Glow FX
            </label>
            <span className="text-xs text-muted-foreground">
              Cards turn slowly like a globe and orbs carry an orbiting rim. Turn off to hold the graph still and save battery.
            </span>
          </div>
          <input
            id="setting-glow-fx"
            type="checkbox"
            checked={userSettings.enableGlowFx}
            onChange={(e) => updateUserSettings({ enableGlowFx: e.target.checked })}
            className="size-4 rounded border-border text-primary focus:ring-primary cursor-pointer"
          />
        </div>

        {/* Default Startup Tab */}
        <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-3 p-3.5 rounded-lg border border-border bg-card/60">
          <div>
            <label className="text-xs font-medium text-foreground block">Default Startup View</label>
            <span className="text-xs text-muted-foreground">
              Which view should open automatically when launching Knox
            </span>
          </div>
          <select
            value={userSettings.defaultStartupView}
            onChange={(e) => updateUserSettings({ defaultStartupView: e.target.value as ViewTab })}
            className="h-8 px-2.5 rounded-md border border-border bg-background text-xs font-mono text-foreground focus:outline-none focus:ring-1 focus:ring-primary cursor-pointer"
          >
            <option value="dashboard">Dashboard</option>
            <option value="graph">Knowledge Graph</option>
            <option value="query">Query Console</option>
            <option value="stats">Metrics & Stats</option>
            <option value="ingest">Ingest Transcripts</option>
            <option value="scanner">Git Edit Scanner</option>
            <option value="mcp">MCP Studio</option>
            <option value="workspace">Workspace</option>
          </select>
        </div>
      </div>
    </div>
  );
};

