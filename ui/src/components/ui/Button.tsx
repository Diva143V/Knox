import * as React from "react";
import { clsx, type ClassValue } from "clsx";
import { twMerge } from "tailwind-merge";

export function cn(...inputs: ClassValue[]) {
  return twMerge(clsx(inputs));
}

export interface ButtonProps
  extends React.ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: "default" | "primary" | "secondary" | "outline" | "ghost" | "danger";
  size?: "sm" | "md" | "lg" | "icon";
}

export const Button = React.forwardRef<HTMLButtonElement, ButtonProps>(
  ({ className, variant = "default", size = "md", ...props }, ref) => {
    const variantStyles = {
      default: "bg-card text-foreground border border-border hover:bg-card/80",
      primary: "bg-primary text-primary-foreground hover:bg-primary/90 shadow-sm shadow-primary/25",
      secondary: "bg-muted text-foreground hover:bg-muted/80",
      outline: "border border-border text-foreground hover:bg-card",
      ghost: "text-muted-foreground hover:text-foreground hover:bg-card/60",
      danger: "bg-danger text-foreground hover:bg-danger/90",
    };

    const sizeStyles = {
      sm: "h-7 px-2.5 text-xs rounded-md",
      md: "h-8 px-3.5 text-xs rounded-lg",
      lg: "h-9 px-4 text-sm rounded-lg",
      icon: "h-8 w-8 p-0 rounded-lg flex items-center justify-center",
    };

    return (
      <button
        ref={ref}
        className={cn(
          "inline-flex items-center justify-center font-medium transition-colors cursor-pointer disabled:opacity-50 disabled:pointer-events-none select-none focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-primary",
          variantStyles[variant],
          sizeStyles[size],
          className
        )}
        {...props}
      />
    );
  }
);
Button.displayName = "Button";

