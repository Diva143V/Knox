import React, { useEffect } from "react";
import { useKnoxStore } from "@/store/useKnoxStore";
import { useKeyboard } from "@/hooks/useKeyboard";
import { CustomTitlebar } from "@/components/layout/CustomTitlebar";
import { Sidebar } from "@/components/layout/Sidebar";
import { Header } from "@/components/layout/Header";
import { Router } from "@/components/layout/Router";
import { DetailInspector } from "@/components/layout/DetailInspector";
import { CommandPalette } from "@/components/CommandPalette";
import { OnboardingWizard } from "@/components/OnboardingWizard";
import { ErrorBoundary } from "@/components/ErrorBoundary";
import { CheckCircle2, AlertCircle, X } from "lucide-react";

export const AppShell: React.FC = () => {
  const { refreshData, toast, clearToast, onboardingOpen } = useKnoxStore();

  // Register global desktop keyboard shortcuts (F5, Esc, Ctrl+K, G-prefix, 1-8)
  useKeyboard();

  useEffect(() => {
    refreshData();
  }, [refreshData]);

  return (
    <div className="h-screen w-screen flex flex-col bg-background text-foreground overflow-hidden">
      {/* Custom Frameless Window Titlebar */}
      <CustomTitlebar />

      {/* Main Workspace: Three-Zone Shell */}
      <div className="flex-1 flex min-h-0 overflow-hidden">
        {/* Zone 1: Left Navigation Rail (w-56) */}
        <Sidebar />

        {/* Zone 2: Main Content Viewport (flex-1) */}
        <main className="flex-1 min-w-0 bg-background overflow-hidden relative flex flex-col">
          <Header />
          <div className="flex-1 min-h-0 overflow-hidden relative">
            <ErrorBoundary fallbackTitle="View Rendering Error">
              <Router />
            </ErrorBoundary>
          </div>
        </main>

        {/* Zone 3: Action Detail Inspector (w-80, conditional) */}
        <DetailInspector />
      </div>

      {/* Global Command Palette (Ctrl+K) */}
      <CommandPalette />

      {/* In-App Onboarding & Setup Wizard */}
      {onboardingOpen && <OnboardingWizard />}

      {/* Desktop Toast Notification Banner (Top-Right under titlebar, avoiding Graph MiniMap) */}
      {toast && (
        <div className="fixed top-12 right-4 z-50 flex items-center gap-2.5 px-4 py-2.5 rounded-xl border border-border bg-card/95 backdrop-blur shadow-2xl text-xs font-medium animate-in slide-in-from-top-2 duration-150">
          {toast.isError ? (
            <AlertCircle className="w-4 h-4 text-danger shrink-0" />
          ) : (
            <CheckCircle2 className="w-4 h-4 text-success shrink-0" />
          )}
          <span className="text-foreground max-w-sm truncate">{toast.message}</span>
          <button
            onClick={clearToast}
            className="p-1 hover:text-foreground text-muted-foreground ml-2 cursor-pointer"
          >
            <X className="w-3.5 h-3.5" />
          </button>
        </div>
      )}
    </div>
  );
};
