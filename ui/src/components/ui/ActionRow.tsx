import React from "react";
import type { KnoxAction } from "@/types/knox";
import { ActionBadge } from "./ActionBadge";
import { ActorBadge } from "./ActorBadge";
import { FileCode } from "lucide-react";
import { cn } from "@/lib/cn";

/**
 * Shared column template so the ledger header and its rows never drift.
 * Narrow: time / summary / type. Desktop: + actor, file, line deltas.
 */
export const ACTION_ROW_GRID =
  "grid items-center gap-x-3 grid-cols-[2.75rem_minmax(0,1fr)_4rem] lg:grid-cols-[2.75rem_6.5rem_4rem_minmax(0,1fr)_10rem_3.5rem]";

const timeFormatter = new Intl.DateTimeFormat(undefined, {
  hour: "2-digit",
  minute: "2-digit",
  hour12: false,
});

interface ActionRowProps {
  action: KnoxAction;
  isSelected?: boolean;
  onSelect?: (action: KnoxAction) => void;
  className?: string;
}

export const ActionRow: React.FC<ActionRowProps> = ({
  action,
  isSelected = false,
  onSelect,
  className = "",
}) => {
  let formattedTime = "--:--";
  if (action?.timestamp) {
    const d = new Date(action.timestamp);
    if (!isNaN(d.getTime())) {
      formattedTime = timeFormatter.format(d);
    }
  }

  const added = action.delta_summary?.lines_added ?? 0;
  const deleted = action.delta_summary?.lines_deleted ?? 0;
  const hasDelta = added > 0 || deleted > 0;

  const content = (
    <>
      <span className="order-1 truncate whitespace-nowrap font-mono text-3xs tabular-nums text-muted-foreground">
        {formattedTime}
      </span>

      <p className="order-2 min-w-0 truncate text-2xs text-foreground lg:order-4">
        {action.summary}
      </p>

      <span className="order-3 min-w-0 lg:order-3">
        <ActionBadge type={action.action_type} className="truncate" />
      </span>

      <span className="order-4 hidden min-w-0 lg:order-2 lg:flex">
        <ActorBadge actor={action.actor} />
      </span>

      <span className="order-5 hidden min-w-0 items-center gap-1 font-mono text-3xs text-muted-foreground lg:flex">
        {action.files && action.files.length > 0 ? (
          <>
            <FileCode className="size-3 shrink-0 opacity-70" aria-hidden />
            <span className="truncate" title={action.files[0]}>
              {action.files[0].replace(/\\/g, "/").split("/").pop()}
            </span>
            {action.files.length > 1 && (
              <span className="shrink-0 text-muted-foreground/70">
                +{action.files.length - 1}
              </span>
            )}
          </>
        ) : (
          <span aria-hidden>—</span>
        )}
      </span>

      <span className="order-6 hidden items-center justify-end gap-1.5 font-mono text-3xs tabular-nums lg:flex">
        {hasDelta ? (
          <>
            {added > 0 && <span className="text-success">+{added}</span>}
            {deleted > 0 && <span className="text-danger">-{deleted}</span>}
          </>
        ) : (
          <span className="text-muted-foreground/50" aria-hidden>
            —
          </span>
        )}
      </span>
    </>
  );

  const base = cn(
    ACTION_ROW_GRID,
    "relative h-[42px] w-full min-w-0 px-4 text-left transition-colors duration-100",
    className
  );

  if (!onSelect) {
    return <div className={base}>{content}</div>;
  }

  return (
    <button
      type="button"
      onClick={() => onSelect(action)}
      aria-current={isSelected ? "true" : undefined}
      className={cn(
        base,
        "cursor-pointer focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-inset focus-visible:ring-focus-ring",
        isSelected
          ? "bg-raised shadow-[inset_2px_0_0_var(--primary)] text-foreground"
          : "hover:bg-white/[0.02]"
      )}
    >
      {content}
    </button>
  );
};
