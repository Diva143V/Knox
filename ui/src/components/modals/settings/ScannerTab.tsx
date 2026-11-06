import React, { useState } from "react";
import { Plus, X } from "lucide-react";
import { useKnoxStore } from "@/store/useKnoxStore";
import { Button } from "@/components/ui/Button";
import { Input } from "@/components/ui/Input";

export const ScannerTab: React.FC = () => {
  const { userSettings, updateUserSettings, showToast } = useKnoxStore();
  const [newIgnoreInput, setNewIgnoreInput] = useState("");

  const handleAddIgnorePattern = () => {
    const val = newIgnoreInput.trim();
    if (!val) return;
    if (userSettings.scannerIgnorePatterns.includes(val)) {
      showToast("Pattern already exists", true);
      return;
    }
    updateUserSettings({
      scannerIgnorePatterns: [...userSettings.scannerIgnorePatterns, val],
    });
    setNewIgnoreInput("");
    showToast(`Added pattern: ${val}`);
  };

  const handleRemoveIgnorePattern = (pattern: string) => {
    updateUserSettings({
      scannerIgnorePatterns: userSettings.scannerIgnorePatterns.filter(
        (p) => p !== pattern
      ),
    });
  };

  return (
    <div className="space-y-6 animate-in fade-in-50 duration-150">
      <div>
        <h3 className="text-sm font-semibold text-foreground">Git & Human Edit Scanner</h3>
        <p className="text-xs text-muted-foreground">
          Preferences for detecting manual code changes made outside AI sessions
        </p>
      </div>

      <div className="space-y-4">
        {/* Default Author Attribution */}
        <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-3 p-3.5 rounded-lg border border-border bg-card/60">
          <div>
            <label className="text-xs font-medium text-foreground block">Default Human Actor Handle</label>
            <span className="text-xs text-muted-foreground">
              Actor tag assigned to manual human edits detected via Git staged changes
            </span>
          </div>
          <Input
            type="text"
            value={userSettings.defaultAuthorName}
            onChange={(e) => updateUserSettings({ defaultAuthorName: e.target.value })}
            className="w-48 font-mono text-xs"
          />
        </div>

        {/* Pre-commit hook policy */}
        <div className="flex items-center justify-between p-3.5 rounded-lg border border-border bg-card/60">
          <div>
            <label className="text-xs font-medium text-foreground block">Strict Pre-Commit Hook</label>
            <span className="text-xs text-muted-foreground">
              Abort git commit if Knox database is locked, rather than proceeding silently
            </span>
          </div>
          <input
            type="checkbox"
            checked={userSettings.strictGitHook}
            onChange={(e) => updateUserSettings({ strictGitHook: e.target.checked })}
            className="size-4 rounded border-border text-primary focus:ring-primary cursor-pointer"
          />
        </div>

        {/* Diff Scan Depth */}
        <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-3 p-3.5 rounded-lg border border-border bg-card/60">
          <div>
            <label className="text-xs font-medium text-foreground block">Diff Scan Depth Limit</label>
            <span className="text-xs text-muted-foreground">
              Maximum staged diff lines analyzed to summarize human commits
            </span>
          </div>
          <div className="flex items-center gap-2">
            <Input
              type="number"
              value={userSettings.diffScanDepth}
              onChange={(e) =>
                updateUserSettings({
                  diffScanDepth: parseInt(e.target.value, 10) || 1000,
                })
              }
              className="w-28 font-mono text-xs"
            />
            <span className="text-xs text-muted-foreground">lines</span>
          </div>
        </div>

        {/* File Ignore Patterns */}
        <div className="p-3.5 rounded-lg border border-border bg-card/60 space-y-3">
          <div>
            <label className="text-xs font-medium text-foreground block">Scan Excluded Patterns</label>
            <span className="text-xs text-muted-foreground">
              File paths matching these glob patterns will be ignored during staged diff inspection
            </span>
          </div>

          <div className="flex flex-wrap gap-1.5 max-h-36 overflow-y-auto p-1.5 rounded-md bg-muted/40 border border-border">
            {userSettings.scannerIgnorePatterns.map((pat) => (
              <span
                key={pat}
                className="inline-flex items-center gap-1.5 px-2 py-0.5 rounded bg-card border border-border text-2xs font-mono text-foreground"
              >
                <span>{pat}</span>
                <button
                  type="button"
                  onClick={() => handleRemoveIgnorePattern(pat)}
                  className="hover:text-danger cursor-pointer"
                  title={`Remove ${pat}`}
                >
                  <X className="size-3" />
                </button>
              </span>
            ))}
          </div>

          <div className="flex items-center gap-2">
            <Input
              type="text"
              placeholder="e.g. *.lock, build/**"
              value={newIgnoreInput}
              onChange={(e) => setNewIgnoreInput(e.target.value)}
              onKeyDown={(e) => {
                if (e.key === "Enter") {
                  e.preventDefault();
                  handleAddIgnorePattern();
                }
              }}
              className="font-mono text-xs"
            />
            <Button
              type="button"
              variant="outline"
              size="sm"
              onClick={handleAddIgnorePattern}
              className="shrink-0"
            >
              <Plus className="size-3.5" />
              <span>Add Pattern</span>
            </Button>
          </div>
        </div>
      </div>
    </div>
  );
};

