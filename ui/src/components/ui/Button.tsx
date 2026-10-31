import * as React from "react";
import { cn } from "@/lib/cn";

export { cn };

export interface ButtonProps
  extends React.ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: "default" | "primary" | "secondary" | "outline" | "ghost" | "danger";
  size?: "sm" | "md" | "lg" | "icon";
  loading?: boolean;
}

export const Button = React.forwardRef<HTMLButtonElement, ButtonProps>(
  (
    {
      className,
      variant = "default",
      size = "md",
      loading = false,
      disabled,
      children,
      ...props
    },
    ref
  ) => {
    const variantStyles = {
      default:
        "bg-card text-foreground border border-border hover:bg-muted active:bg-muted/80",
      primary:
        "bg-primary text-primary-foreground hover:bg-primary-hover active:bg-primary-hover/90",
      secondary:
        "bg-muted text-foreground hover:bg-muted/80 active:bg-muted/70",
      outline:
        "border border-border bg-transparent text-foreground hover:bg-card active:bg-muted",
      ghost:
        "text-muted-foreground hover:text-foreground hover:bg-muted/60 active:bg-muted",
      danger:
        "bg-danger/15 text-danger border border-danger/30 hover:bg-danger/25 active:bg-danger/30",
    };

    const sizeStyles = {
      sm: "h-7 px-2.5 text-sm rounded-md gap-1.5",
      md: "h-8 px-3 text-sm rounded-md gap-1.5",
      lg: "h-9 px-4 text-base rounded-md gap-2",
      icon: "h-8 w-8 p-0 rounded-md inline-flex items-center justify-center",
    };

    const isDisabled = disabled || loading;

    return (
      <button
        ref={ref}
        disabled={isDisabled}
        aria-busy={loading || undefined}
        className={cn(
          "inline-flex items-center justify-center font-medium transition-colors duration-100 cursor-pointer select-none",
          "disabled:opacity-50 disabled:cursor-not-allowed disabled:pointer-events-none",
          "focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-focus-ring focus-visible:ring-offset-1 focus-visible:ring-offset-background",
          variantStyles[variant],
          sizeStyles[size],
          className
        )}
        {...props}
      >
        {loading ? (
          <>
            <span
              className="inline-block size-3.5 shrink-0 rounded-full border-2 border-current border-r-transparent animate-spin"
              aria-hidden
            />
            <span>{children}</span>
          </>
        ) : (
          children
        )}
      </button>
    );
  }
);
Button.displayName = "Button";
