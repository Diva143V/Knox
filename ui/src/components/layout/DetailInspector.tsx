import React from "react";
import { X, Layers } from "lucide-react";
import { useKnoxStore } from "@/store/useKnoxStore";
import { ActionDetail } from "@/components/graph/ActionDetail";

export const DetailInspector: React.FC = () => {
  const { selectedAction, setSelectedAction } = useKnoxStore();

  if (!selectedAction) return null;

  return (
    <aside className="w-80 shrink-0 border-l border-border/80 bg-sidebar/40 flex flex-col h-full z-20 select-none overflow-hidden animate-in slide-in-from-right-4 duration-150">
      {/* Inspector Header */}
      <div className="h-12 border-b border-border/60 px-4 flex items-center justify-between shrink-0">
        <div className="flex items-center gap-2">
          <Layers className="w-4 h-4 text-primary" />
          <span className="text-xs font-semibold uppercase tracking-wider text-muted-foreground">
            Action Detail
          </span>
        </div>
        <button
          onClick={() => setSelectedAction(null)}
          className="p-1 rounded-md text-muted-foreground hover:text-foreground hover:bg-card transition cursor-pointer"
          title="Close Inspector (Esc)"
        >
          <X className="w-4 h-4" />
        </button>
      </div>

      {/* Inspector Scrollable Body */}
      <div className="flex-1 overflow-y-auto p-4">
        <ActionDetail action={selectedAction} />
      </div>
    </aside>
  );
};
