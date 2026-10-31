import * as React from "react";
import { cn } from "@/lib/cn";

export interface BadgeProps extends React.HTMLAttributes<HTMLDivElement> {
  variant?:
    | "default"
    | "secondary"
    | "outline"
    | "success"
    | "warning"
    | "danger"
    | "plan";
}

export const Badge: React.FC<BadgeProps> = ({
  className,
  variant = "default",
  ...props
}) => {
  const variantStyles = {
    default: "bg-primary/15 text-primary border-primary/30",
    secondary: "bg-muted text-foreground border-border",
    outline: "bg-transparent text-foreground border-border",
    success: "bg-success/15 text-success border-success/30",
    warning: "bg-warning/15 text-warning border-warning/30",
    danger: "bg-danger/15 text-danger border-danger/30",
    plan: "bg-plan/15 text-plan border-plan/30",
  };

  return (
    <div
      className={cn(
        "inline-flex items-center rounded-sm px-1.5 py-0.5 text-xs font-medium border font-mono tracking-tight",
        variantStyles[variant],
        className
      )}
      {...props}
    />
  );
};
