import React from "react";
import { useKnoxStore } from "@/store/useKnoxStore";
import { useKeyboard } from "@/hooks/useKeyboard";
import { CustomTitlebar } from "@/components/layout/CustomTitlebar";
import { Sidebar } from "@/components/layout/Sidebar";
import { Router } from "@/components/layout/Router";
import { DetailInspector } from "@/components/layout/DetailInspector";
import { CommandPalette } from "@/components/CommandPalette";
import { OnboardingWizard } from "@/components/OnboardingWizard";
import { OnboardingTourModal } from "@/components/OnboardingTourModal";
import { SettingsModal } from "@/components/modals/SettingsModal";
import { EditNodeModal } from "@/components/modals/EditNodeModal";
import { useRealtimeSync } from "@/hooks/useRealtimeSync";
import { ErrorBoundary } from "@/components/ErrorBoundary";
import { Toast } from "@/components/ui/Toast";
import { useEffect } from "react";

export const AppShell: React.FC = () => {
  const { refreshData, toast, clearToast } = useKnoxStore();

  useKeyboard();
  useRealtimeSync();

  useEffect(() => {
    refreshData();
  }, [refreshData]);

  return (
    <div className="flex h-screen w-screen flex-col overflow-hidden bg-background text-foreground">
      <a
        href="#knox-main"
        className="sr-only focus:not-sr-only focus:absolute focus:left-2 focus:top-2 focus:z-[100] focus:rounded-[var(--radius-card)] focus:border focus:border-border focus:bg-card focus:px-3 focus:py-2 focus:text-sm focus:text-foreground"
      >
        Skip to main content
      </a>

      <CustomTitlebar />

      {/* Column on narrow windows (rail becomes a bottom bar), row on desktop. */}
      <div className="flex min-h-0 flex-1 flex-col-reverse overflow-hidden md:flex-row">
        <Sidebar />

        <main
          id="knox-main"
          tabIndex={-1}
          className="relative flex min-w-0 flex-1 flex-col overflow-hidden bg-background focus:outline-none"
        >
          <ErrorBoundary fallbackTitle="View Rendering Error">
            <Router />
          </ErrorBoundary>
        </main>

        <DetailInspector />
      </div>

      <CommandPalette />
      <OnboardingWizard />
      <OnboardingTourModal />
      <SettingsModal />
      <EditNodeModal />

      {toast && (
        <Toast
          message={toast.message}
          variant={toast.variant}
          isError={toast.isError}
          onDismiss={clearToast}
        />
      )}
    </div>
  );
};
