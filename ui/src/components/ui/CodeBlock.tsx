import React, { useState, useCallback } from "react";
import { Check, Copy } from "lucide-react";
import { cn } from "@/lib/cn";
import { Button } from "./Button";

export interface CodeBlockProps {
  code: string;
  language?: string;
  className?: string;
  /** Show copy button (default true) */
  copyable?: boolean;
  onCopied?: () => void;
}

/** Monospace snippet panel for MCP configs, knox.md, and shell commands. */
export const CodeBlock: React.FC<CodeBlockProps> = ({
  code,
  language,
  className,
  copyable = true,
  onCopied,
}) => {
  const [copied, setCopied] = useState(false);

  const handleCopy = useCallback(async () => {
    try {
      await navigator.clipboard.writeText(code);
      setCopied(true);
      onCopied?.();
      window.setTimeout(() => setCopied(false), 1500);
    } catch {
      /* clipboard may be unavailable in some desktop contexts */
    }
  }, [code, onCopied]);

  return (
    <div
      className={cn(
        "relative rounded-lg border border-border bg-background overflow-hidden",
        className
      )}
    >
      {(language || copyable) && (
        <div className="flex items-center justify-between px-3 py-1.5 border-b border-border-subtle bg-card">
          <span className="text-xs font-mono text-muted-foreground uppercase tracking-wider">
            {language || "text"}
          </span>
          {copyable && (
            <Button
              type="button"
              variant="ghost"
              size="sm"
              onClick={handleCopy}
              aria-label={copied ? "Copied" : "Copy to clipboard"}
              className="h-6 px-2 text-xs"
            >
              {copied ? (
                <>
                  <Check className="size-3 text-success" />
                  <span className="text-success">Copied</span>
                </>
              ) : (
                <>
                  <Copy className="size-3" />
                  <span>Copy</span>
                </>
              )}
            </Button>
          )}
        </div>
      )}
      <pre className="p-3 overflow-x-auto text-sm leading-relaxed font-mono text-foreground/90 whitespace-pre">
        <code>{code}</code>
      </pre>
    </div>
  );
};
