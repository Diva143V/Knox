import { useState, useEffect, useCallback } from "react";
import { useKnoxStore } from "@/store/useKnoxStore";
import type { TourStepDef } from "@/types/tour";

export const STORAGE_KEY_TOUR_COMPLETED = "knox_has_completed_tour";

export function useTour(steps: TourStepDef[]) {
  const { isTourOpen, closeTour, setActiveTab } = useKnoxStore();
  const [currentStepIndex, setCurrentStepIndex] = useState(0);
  const [dontShowAgain, setDontShowAgain] = useState(true);

  const totalSteps = steps.length;
  const currentStep = steps[currentStepIndex] || steps[0];

  // Sync background application tab with active tour step
  useEffect(() => {
    if (!isTourOpen) return;
    const tabId = steps[currentStepIndex]?.tabId;
    if (tabId) {
      setActiveTab(tabId);
    }
  }, [isTourOpen, currentStepIndex, steps, setActiveTab]);

  // Reset to first slide whenever modal opens
  useEffect(() => {
    if (isTourOpen) {
      setCurrentStepIndex(0);
    }
  }, [isTourOpen]);

  const saveTourPreference = useCallback(() => {
    try {
      if (dontShowAgain) {
        localStorage.setItem(STORAGE_KEY_TOUR_COMPLETED, "true");
      } else {
        localStorage.removeItem(STORAGE_KEY_TOUR_COMPLETED);
      }
    } catch {
      // ignore
    }
  }, [dontShowAgain]);

  const completeTour = useCallback(() => {
    saveTourPreference();
    closeTour();
  }, [saveTourPreference, closeTour]);

  const skipTour = useCallback(() => {
    saveTourPreference();
    closeTour();
  }, [saveTourPreference, closeTour]);

  const goToNextStep = useCallback(() => {
    if (currentStepIndex < totalSteps - 1) {
      setCurrentStepIndex((prev) => prev + 1);
    } else {
      completeTour();
    }
  }, [currentStepIndex, totalSteps, completeTour]);

  const goToPrevStep = useCallback(() => {
    if (currentStepIndex > 0) {
      setCurrentStepIndex((prev) => prev - 1);
    }
  }, [currentStepIndex]);

  const goToStep = useCallback(
    (index: number) => {
      if (index >= 0 && index < totalSteps) {
        setCurrentStepIndex(index);
      }
    },
    [totalSteps]
  );

  // Keyboard navigation
  useEffect(() => {
    if (!isTourOpen) return;

    const handleKeyDown = (e: KeyboardEvent) => {
      // Don't intercept if user is typing in an input
      const target = e.target as HTMLElement;
      if (target.tagName === "INPUT" || target.tagName === "TEXTAREA") return;

      if (e.key === "Escape") {
        e.preventDefault();
        skipTour();
      } else if (e.key === "ArrowRight") {
        e.preventDefault();
        goToNextStep();
      } else if (e.key === "ArrowLeft") {
        e.preventDefault();
        goToPrevStep();
      } else if (e.key === "Enter") {
        e.preventDefault();
        goToNextStep();
      }
    };

    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [isTourOpen, goToNextStep, goToPrevStep, skipTour]);

  return {
    isTourOpen,
    currentStepIndex,
    currentStep,
    totalSteps,
    isFirstStep: currentStepIndex === 0,
    isLastStep: currentStepIndex === totalSteps - 1,
    dontShowAgain,
    setDontShowAgain,
    goToNextStep,
    goToPrevStep,
    goToStep,
    completeTour,
    skipTour,
  };
}

