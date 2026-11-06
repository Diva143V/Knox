import React from "react";

export const ShortcutsTab: React.FC = () => {
  return (
    <div className="space-y-6 animate-in fade-in-50 duration-150">
      <div>
        <h3 className="text-sm font-semibold text-foreground">Keyboard Shortcuts & Cheatsheet</h3>
        <p className="text-xs text-muted-foreground">
          Universal keyboard shortcuts for fast power-user navigation in Knox
        </p>
      </div>

      <div className="grid grid-cols-1 sm:grid-cols-2 gap-2.5">
        {[
          { keys: ["Ctrl", ","], label: "Toggle Settings", desc: "Open or dismiss this settings window" },
          { keys: ["Ctrl", "K"], label: "Command Palette", desc: "Search actions, files, tools and views" },
          { keys: ["Ctrl", "Alt", "N"], label: "Quick Thought", desc: "Instantly record a doubt, decision, or note" },
          { keys: ["1"], label: "Dashboard View", desc: "KPIs, trends and recent actions" },
          { keys: ["2"], label: "Knowledge Graph", desc: "Interactive causal graph canvas" },
          { keys: ["3"], label: "Search & Query", desc: "FTS5 query and multi-filter console" },
          { keys: ["4"], label: "Metrics & Stats", desc: "Actor distributions and 7-day volume" },
          { keys: ["5"], label: "Ingest Studio", desc: "Transcript import and conversation mapper" },
          { keys: ["6"], label: "Git Edit Scan", desc: "Detect and stage human code commits" },
          { keys: ["7"], label: "MCP Studio", desc: "Configure AI agent stdio JSON snippets" },
          { keys: ["8"], label: "Workspace Setup", desc: "Initialize knox.md and repository hooks" },
          { keys: ["F5"], label: "Refresh Data", desc: "Re-query SQLite database from disk" },
          { keys: ["P"], label: "Peek Window", desc: "Dock guided tour into a bottom pill" },
          { keys: ["Esc"], label: "Dismiss Dialog", desc: "Close active modal or inspector pane" },
        ].map((sc) => (
          <div
            key={sc.label}
            className="p-3 rounded-lg border border-border bg-card/60 flex items-center justify-between gap-3"
          >
            <div className="min-w-0">
              <p className="text-xs font-semibold text-foreground">{sc.label}</p>
              <p className="text-2xs text-muted-foreground truncate">{sc.desc}</p>
            </div>
            <div className="flex items-center gap-1 shrink-0 font-mono text-xs">
              {sc.keys.map((k) => (
                <kbd
                  key={k}
                  className="px-1.5 py-0.5 rounded-sm bg-muted border border-border text-foreground text-3xs font-semibold shadow-2xs"
                >
                  {k}
                </kbd>
              ))}
            </div>
          </div>
        ))}
      </div>
    </div>
  );
};

