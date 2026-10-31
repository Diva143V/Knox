import React from "react";
import { X, PanelRightClose } from "lucide-react";
import { useKnoxStore } from "@/store/useKnoxStore";
import { ActionDetail } from "@/components/graph/ActionDetail";
import { ActionBadge } from "@/components/ui/ActionBadge";
import { ActorBadge } from "@/components/ui/ActorBadge";
import { Button } from "@/components/ui/Button";
import { cn } from "@/lib/cn";

export const DetailInspector: React.FC = () => {
  const { selectedAction, setSelectedAction } = useKnoxStore();

  if (!selectedAction) return null;

  return (
    <>
      {/* Narrow-viewport scrim — tap to close */}
      <button
        type="button"
        aria-label="Close inspector overlay"
        className="fixed inset-0 top-9 z-30 bg-background/60 xl:hidden cursor-pointer"
        onClick={() => setSelectedAction(null)}
      />

      <aside
        className={cn(
          "shrink-0 border-l border-border bg-sidebar flex flex-col z-40 select-none overflow-hidden",
          "w-[min(100vw-3.5rem,20rem)] xl:w-80",
          "fixed right-0 top-9 bottom-0 xl:static xl:top-auto xl:bottom-auto xl:h-full",
          "motion-safe:animate-in motion-safe:slide-in-from-right-4 motion-safe:duration-150"
        )}
        aria-label="Action detail inspector"
      >
        <div className="h-11 border-b border-border px-3 flex items-center justify-between shrink-0 gap-2">
          <div className="flex items-center gap-2 min-w-0">
            <PanelRightClose className="size-4 text-primary shrink-0" aria-hidden />
            <div className="min-w-0">
              <div className="text-overline leading-none mb-1">Inspector</div>
              <div className="flex items-center gap-1.5 min-w-0">
                <ActorBadge actor={selectedAction.actor} />
                <ActionBadge type={selectedAction.action_type} />
              </div>
            </div>
          </div>
          <Button
            type="button"
            variant="ghost"
            size="icon"
            onClick={() => setSelectedAction(null)}
            title="Close Inspector (Esc)"
            aria-label="Close inspector"
            className="size-7 shrink-0"
          >
            <X className="size-4" />
          </Button>
        </div>

        <div className="flex-1 overflow-y-auto p-4">
          <ActionDetail action={selectedAction} />
        </div>
      </aside>
    </>
  );
};
