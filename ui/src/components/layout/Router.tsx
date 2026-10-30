import React, { useEffect } from "react";
import { useKnoxStore } from "@/store/useKnoxStore";
import { isValidTab } from "@/types/knox";
import { DashboardView } from "@/components/views/DashboardView";
import { KnoxGraphCanvas } from "@/components/graph/KnoxGraphCanvas";
import { QueryView } from "@/components/views/QueryView";
import { StatsView } from "@/components/views/StatsView";
import { IngestView } from "@/components/views/IngestView";
import { ScanView } from "@/components/views/ScanView";
import { McpView } from "@/components/views/McpView";
import { WorkspaceView } from "@/components/views/WorkspaceView";

export const Router: React.FC = () => {
  const { activeTab, setActiveTab } = useKnoxStore();

  // Listen to browser / desktop hash navigation (back/forward history)
  useEffect(() => {
    const handleHashChange = () => {
      const hash = window.location.hash.replace(/^#\/?/, "").toLowerCase();
      if (hash && isValidTab(hash) && hash !== useKnoxStore.getState().activeTab) {
        setActiveTab(hash);
      }
    };

    window.addEventListener("hashchange", handleHashChange);
    return () => window.removeEventListener("hashchange", handleHashChange);
  }, [setActiveTab]);

  // Synchronize URL hash when activeTab changes internally
  useEffect(() => {
    const currentHash = window.location.hash.replace(/^#\/?/, "").toLowerCase();
    if (currentHash !== activeTab) {
      window.location.hash = `#/${activeTab}`;
    }
  }, [activeTab]);

  switch (activeTab) {
    case "dashboard":
      return <DashboardView />;
    case "graph":
      return <KnoxGraphCanvas />;
    case "query":
      return <QueryView />;
    case "stats":
      return <StatsView />;
    case "ingest":
      return <IngestView />;
    case "scanner":
      return <ScanView />;
    case "mcp":
      return <McpView />;
    case "workspace":
      return <WorkspaceView />;
    default:
      return <DashboardView />;
  }
};
