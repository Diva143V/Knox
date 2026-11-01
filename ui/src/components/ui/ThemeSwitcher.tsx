import React, { useEffect, useId, useRef, useState } from "react";
import { Moon, Sun, Laptop, Check, ChevronDown } from "lucide-react";
import { cn } from "@/lib/cn";
import { useKnoxStore } from "@/store/useKnoxStore";
import type { ThemePreference } from "@/lib/theme";

const OPTIONS: {
  id: ThemePreference;
  label: string;
  Icon: React.ComponentType<{ className?: string }>;
}[] = [
  { id: "dark", label: "Dark", Icon: Moon },
  { id: "light", label: "Light", Icon: Sun },
  { id: "system", label: "System", Icon: Laptop },
];

/** Compact titlebar appearance control — Dark / Light / System dropdown. */
export const ThemeSwitcher: React.FC = () => {
  const theme = useKnoxStore((s) => s.theme);
  const resolvedTheme = useKnoxStore((s) => s.resolvedTheme);
  const setTheme = useKnoxStore((s) => s.setTheme);
  const [open, setOpen] = useState(false);
  const rootRef = useRef<HTMLDivElement>(null);
  const menuId = useId();

  useEffect(() => {
    if (!open) return;
    const onPointer = (e: MouseEvent) => {
      if (!rootRef.current?.contains(e.target as Node)) setOpen(false);
    };
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") setOpen(false);
    };
    document.addEventListener("mousedown", onPointer);
    document.addEventListener("keydown", onKey);
    return () => {
      document.removeEventListener("mousedown", onPointer);
      document.removeEventListener("keydown", onKey);
    };
  }, [open]);

  const ActiveIcon = theme === "system" ? Laptop : resolvedTheme === "light" ? Sun : Moon;

  return (
    <div ref={rootRef} className="relative no-drag">
      <button
        type="button"
        className={cn(
          "h-7 px-2 inline-flex items-center gap-1.5 rounded-md",
          "text-muted-foreground hover:text-foreground hover:bg-muted",
          "transition-colors cursor-pointer",
          "focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-focus-ring",
          open && "bg-muted text-foreground"
        )}
        aria-label="Appearance"
        aria-haspopup="menu"
        aria-expanded={open}
        aria-controls={menuId}
        title={`Appearance: ${theme} (${resolvedTheme})`}
        onClick={() => setOpen((v) => !v)}
      >
        <ActiveIcon className="size-3.5 shrink-0" aria-hidden />
        <span className="text-2xs font-medium capitalize hidden sm:inline">{theme}</span>
        <ChevronDown
          className={cn("size-3 opacity-70 transition-transform", open && "rotate-180")}
          aria-hidden
        />
      </button>

      {open && (
        <div
          id={menuId}
          role="menu"
          aria-label="Appearance"
          className={cn(
            "absolute right-0 top-full mt-1 z-[60] min-w-[9rem]",
            "rounded-md border border-border bg-popover shadow-md py-1",
            "motion-safe:animate-in motion-safe:fade-in motion-safe:zoom-in-95 motion-safe:duration-100"
          )}
        >
          {OPTIONS.map(({ id, label, Icon }) => {
            const selected = theme === id;
            return (
              <button
                key={id}
                type="button"
                role="menuitemradio"
                aria-checked={selected}
                className={cn(
                  "w-full flex items-center gap-2 px-2.5 py-1.5 text-sm text-left",
                  "hover:bg-muted cursor-pointer transition-colors",
                  "focus-visible:outline-none focus-visible:bg-muted",
                  selected ? "text-foreground font-medium" : "text-muted-foreground"
                )}
                onClick={() => {
                  setTheme(id);
                  setOpen(false);
                }}
              >
                <Icon className="size-3.5 shrink-0" aria-hidden />
                <span className="flex-1">{label}</span>
                {selected && (
                  <Check className="size-3.5 text-primary shrink-0" aria-hidden />
                )}
              </button>
            );
          })}
        </div>
      )}
    </div>
  );
};
