import React from "react";
import { Compass, BookOpen } from "lucide-react";
import { useKnoxStore } from "@/store/useKnoxStore";
import { Button } from "@/components/ui/Button";
import { APP_VERSION } from "@/lib/navigation";

export const AboutTab: React.FC = () => {
  const { closeSettings, openTour, setOnboardingOpen } = useKnoxStore();

  return (
    <div className="space-y-6 animate-in fade-in-50 duration-150">
      <div>
        <h3 className="text-sm font-semibold text-foreground">About Knox</h3>
        <p className="text-xs text-muted-foreground">
          Architectural knowledge graph for AI-assisted software development
        </p>
      </div>

      <div className="p-4 rounded-xl border border-border bg-card/60 space-y-4">
        <div className="flex items-center gap-3">
          <div className="size-12 rounded-xl bg-primary/15 border border-primary/30 flex items-center justify-center text-primary text-xl font-bold">
            ️
          </div>
          <div>
            <h4 className="text-base font-bold text-foreground tracking-tight">Knox Desktop</h4>
            <p className="text-xs text-muted-foreground font-mono">
              v{APP_VERSION} &bull; Rust 1.85+ (Edition 2024) &bull; Tauri v2.2
            </p>
          </div>
        </div>

        <p className="text-xs text-muted-foreground leading-relaxed">
          Knox acts as persistent, external long-term memory for AI coding agents (Antigravity, Cursor, Claude Code, Gemini CLI).
          All architectural decisions, implementation actions, and human git commits are indexed into a local-first SQLite WAL knowledge graph with zero telemetry tracking.
        </p>

        <div className="grid grid-cols-2 sm:grid-cols-4 gap-2 pt-2 border-t border-border font-mono text-2xs">
          <div>
            <span className="text-muted-foreground block uppercase">Subsystem</span>
            <span className="text-foreground font-semibold">Windows GUI</span>
          </div>
          <div>
            <span className="text-muted-foreground block uppercase">Database</span>
            <span className="text-foreground font-semibold">SQLite 3.48 (WAL)</span>
          </div>
          <div>
            <span className="text-muted-foreground block uppercase">Frontend</span>
            <span className="text-foreground font-semibold">React 19 + Tailwind v4</span>
          </div>
          <div>
            <span className="text-muted-foreground block uppercase">License</span>
            <span className="text-foreground font-semibold">MIT License</span>
          </div>
        </div>

        {/* Quick Action Guides */}
        <div className="pt-2 border-t border-border flex flex-wrap items-center gap-2">
          <Button
            type="button"
            variant="outline"
            size="sm"
            onClick={() => {
              closeSettings();
              openTour();
            }}
            className="gap-1.5"
          >
            <Compass className="size-3.5 text-primary" />
            <span>Getting Started Tour</span>
          </Button>

          <Button
            type="button"
            variant="outline"
            size="sm"
            onClick={() => {
              closeSettings();
              setOnboardingOpen(true);
            }}
            className="gap-1.5"
          >
            <BookOpen className="size-3.5 text-primary" />
            <span>Setup & MCP Wizard</span>
          </Button>
        </div>
      </div>
    </div>
  );
};
