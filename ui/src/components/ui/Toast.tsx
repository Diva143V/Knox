import React from "react";
import {
  CheckCircle2,
  AlertCircle,
  AlertTriangle,
  Info,
  X,
} from "lucide-react";
import { cn } from "@/lib/cn";

export type ToastVariant = "success" | "error" | "warning" | "info";

export interface ToastProps {
  message: string;
  variant?: ToastVariant;
  /** @deprecated Prefer variant="error" */
  isError?: boolean;
  onDismiss: () => void;
}

function resolveVariant(
  variant: ToastVariant | undefined,
  isError: boolean | undefined
): ToastVariant {
  if (variant) return variant;
  return isError ? "error" : "success";
}

const VARIANT_STYLES: Record<
  ToastVariant,
  { icon: React.ReactNode; border: string; role: "status" | "alert" }
> = {
  success: {
    icon: <CheckCircle2 className="size-4 text-success shrink-0" aria-hidden />,
    border: "border-border",
    role: "status",
  },
  error: {
    icon: <AlertCircle className="size-4 text-danger shrink-0" aria-hidden />,
    border: "border-danger/40",
    role: "alert",
  },
  warning: {
    icon: (
      <AlertTriangle className="size-4 text-warning shrink-0" aria-hidden />
    ),
    border: "border-warning/40",
    role: "status",
  },
  info: {
    icon: <Info className="size-4 text-plan shrink-0" aria-hidden />,
    border: "border-border",
    role: "status",
  },
};

/** Compact Graphite Vault toast — top-right under titlebar. */
export const Toast: React.FC<ToastProps> = ({
  message,
  variant,
  isError,
  onDismiss,
}) => {
  const resolved = resolveVariant(variant, isError);
  const style = VARIANT_STYLES[resolved];

  return (
    <div
      role={style.role}
      aria-live={resolved === "error" ? "assertive" : "polite"}
      className={cn(
        "fixed top-11 right-3 z-50 flex items-center gap-2.5",
        "max-w-sm px-3.5 py-2.5 rounded-lg border bg-card text-sm font-medium shadow-md",
        "motion-safe:animate-in motion-safe:slide-in-from-top-2 motion-safe:duration-150",
        style.border
      )}
    >
      {style.icon}
      <span className="text-foreground min-w-0 flex-1 leading-snug">
        {message}
      </span>
      <button
        type="button"
        onClick={onDismiss}
        className="p-1 rounded-sm text-muted-foreground hover:text-foreground hover:bg-muted cursor-pointer focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-focus-ring"
        aria-label="Dismiss notification"
      >
        <X className="size-3.5" />
      </button>
    </div>
  );
};
