import React from "react";
import {
  LayoutDashboard,
  Network,
  Search,
  BarChart2,
  FileInput,
  ShieldCheck,
  PlugZap,
  FolderGit2,
  Database,
  RefreshCw,
} from "lucide-react";
import { useKnoxStore } from "@/store/useKnoxStore";
import type { ViewTab } from "@/types/knox";

interface NavItem {
  id: ViewTab;
  label: string;
  icon: React.ReactNode;
  shortcut: string;
}

const navItems: NavItem[] = [
  {
    id: "dashboard",
    label: "Dashboard",
    icon: <LayoutDashboard className="w-4 h-4" />,
    shortcut: "1",
  },
  {
    id: "graph",
    label: "Knowledge Graph",
    icon: <Network className="w-4 h-4" />,
    shortcut: "2",
  },
  {
    id: "query",
    label: "Query Console",
    icon: <Search className="w-4 h-4" />,
    shortcut: "3",
  },
  {
    id: "stats",
    label: "Metrics & Stats",
    icon: <BarChart2 className="w-4 h-4" />,
    shortcut: "4",
  },
  {
    id: "ingest",
    label: "Ingest Transcripts",
    icon: <FileInput className="w-4 h-4" />,
    shortcut: "5",
  },
  {
    id: "scanner",
    label: "Human Edit Scan",
    icon: <ShieldCheck className="w-4 h-4" />,
    shortcut: "6",
  },
  {
    id: "mcp",
    label: "MCP Server Config",
    icon: <PlugZap className="w-4 h-4" />,
    shortcut: "7",
  },
  {
    id: "workspace",
    label: "Workspace & Hook",
    icon: <FolderGit2 className="w-4 h-4" />,
    shortcut: "8",
  },
];

export const Sidebar: React.FC = () => {
  const { activeTab, setActiveTab, status, isLoading, refreshData } = useKnoxStore();

  return (
    <aside className="w-56 shrink-0 border-r border-border/70 bg-sidebar/50 flex flex-col justify-between select-none">
      {/* Top Nav Rail */}
      <div className="p-2 space-y-1">
        <div className="px-3 py-2 text-[10px] font-semibold tracking-wider text-muted-foreground uppercase">
          Views & Tools
        </div>

        {navItems.map((item) => {
          const isActive = activeTab === item.id;
          return (
            <button
              key={item.id}
              onClick={() => setActiveTab(item.id)}
              className={`w-full flex items-center justify-between px-3 py-2 rounded-lg text-xs font-medium transition-all cursor-pointer ${
                isActive
                  ? "bg-primary text-primary-foreground shadow-sm shadow-primary/25"
                  : "text-muted-foreground hover:text-foreground hover:bg-card"
              }`}
            >
              <div className="flex items-center gap-2.5">
                <span className={isActive ? "text-primary-foreground" : "text-muted-foreground"}>
                  {item.icon}
                </span>
                <span>{item.label}</span>
              </div>
              <span
                className={`text-[10px] font-mono px-1 py-0.2 rounded ${
                  isActive
                    ? "bg-primary-foreground/20 text-primary-foreground"
                    : "text-muted-foreground/50"
                }`}
              >
                {item.shortcut}
              </span>
            </button>
          );
        })}
      </div>

      {/* Bottom Status Box */}
      <div className="p-3 border-t border-border/60 bg-sidebar/80 space-y-2">
        <div className="flex items-center justify-between text-[11px] text-muted-foreground">
          <div className="flex items-center gap-1.5">
            <Database className="w-3.5 h-3.5 text-emerald-400" />
            <span>SQLite Active</span>
          </div>
          <button
            onClick={() => refreshData()}
            disabled={isLoading}
            className="p-1 hover:text-foreground rounded transition cursor-pointer"
            title="Refresh Knowledge Graph"
          >
            <RefreshCw className={`w-3 h-3 ${isLoading ? "animate-spin text-primary" : ""}`} />
          </button>
        </div>

        {status && (
          <div className="text-[10px] text-muted-foreground font-mono space-y-0.5">
            <div className="flex justify-between">
              <span>Actions:</span>
              <span className="text-foreground font-semibold">{status.total_actions}</span>
            </div>
            <div className="flex justify-between">
              <span>Sessions:</span>
              <span className="text-foreground">{status.total_sessions}</span>
            </div>
            <div className="flex justify-between">
              <span>Projects:</span>
              <span className="text-foreground">{status.projects_tracked}</span>
            </div>
          </div>
        )}
      </div>
    </aside>
  );
};

