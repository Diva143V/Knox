import React from "react";
import { useKnoxStore } from "@/store/useKnoxStore";
import { cn } from "@/lib/cn";

export const IngestionTab: React.FC = () => {
  const { userSettings, updateUserSettings } = useKnoxStore();

  return (
    <div className="space-y-6 animate-in fade-in-50 duration-150">
      <div>
        <h3 className="text-sm font-semibold text-foreground">Ingestion & Transcripts</h3>
        <p className="text-xs text-muted-foreground">
          Rules for importing IDE conversations and conversation transcripts into the knowledge graph
        </p>
      </div>

      <div className="space-y-4">
        {/* Deduplication Strategy */}
        <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-3 p-3.5 rounded-lg border border-border bg-card/60">
          <div>
            <label className="text-xs font-medium text-foreground block">Deduplication Strategy</label>
            <span className="text-xs text-muted-foreground">
              How to handle action items with matching SHA-256 fingerprints
            </span>
          </div>
          <div className="flex items-center gap-1.5 shrink-0">
            <button
              type="button"
              onClick={() => updateUserSettings({ deduplicationMode: "skip" })}
              className={cn(
                "px-3 py-1.5 rounded-md text-xs font-medium border transition-all cursor-pointer",
                userSettings.deduplicationMode === "skip"
                  ? "bg-primary text-primary-foreground border-primary shadow-xs font-semibold"
                  : "bg-muted/70 text-muted-foreground border-border hover:text-foreground hover:bg-muted"
              )}
            >
              Skip Existing
            </button>
            <button
              type="button"
              onClick={() => updateUserSettings({ deduplicationMode: "supersede" })}
              className={cn(
                "px-3 py-1.5 rounded-md text-xs font-medium border transition-all cursor-pointer",
                userSettings.deduplicationMode === "supersede"
                  ? "bg-primary text-primary-foreground border-primary shadow-xs font-semibold"
                  : "bg-muted/70 text-muted-foreground border-border hover:text-foreground hover:bg-muted"
              )}
            >
              Record as Revision
            </button>
          </div>
        </div>

        {/* Require Ingestion Preview */}
        <div className="flex items-center justify-between p-3.5 rounded-lg border border-border bg-card/60">
          <div>
            <label className="text-xs font-medium text-foreground block">Require Interactive Preview</label>
            <span className="text-xs text-muted-foreground">
              Always display preview modal with categorized action count breakdown before committing writes
            </span>
          </div>
          <input
            type="checkbox"
            checked={userSettings.requireIngestPreview}
            onChange={(e) => updateUserSettings({ requireIngestPreview: e.target.checked })}
            className="size-4 rounded border-border text-primary focus:ring-primary cursor-pointer"
          />
        </div>
      </div>
    </div>
  );
};

