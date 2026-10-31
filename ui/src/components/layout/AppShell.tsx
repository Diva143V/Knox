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
import { OnboardingTourModal } from "@/components/OnboardingTourModal";
import { ErrorBoundary } from "@/components/ErrorBoundary";
import { Toast } from "@/components/ui/Toast";

export const AppShell: React.FC = () => {
  const { refreshData, toast, clearToast } = useKnoxStore();

  useKeyboard();

  useEffect(() => {
    refreshData();
  }, [refreshData]);

  return (
    <div className="h-screen w-screen flex flex-col bg-background text-foreground overflow-hidden">
      <CustomTitlebar />

      <div className="flex-1 flex min-h-0 overflow-hidden">
        <Sidebar />

        <main className="flex-1 min-w-0 bg-background overflow-hidden relative flex flex-col">
          <Header />
          <div className="flex-1 min-h-0 overflow-hidden relative">
            <ErrorBoundary fallbackTitle="View Rendering Error">
              <Router />
            </ErrorBoundary>
          </div>
        </main>

        <DetailInspector />
      </div>

      <CommandPalette />
      <OnboardingWizard />
      <OnboardingTourModal />

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
