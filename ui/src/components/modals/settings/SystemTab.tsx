import React from "react";
import { useKnoxStore } from "@/store/useKnoxStore";

export const SystemTab: React.FC = () => {
  const { userSettings, updateUserSettings } = useKnoxStore();

  return (
    <div className="space-y-6 animate-in fade-in-50 duration-150">
      <div>
        <h3 className="text-sm font-semibold text-foreground">Desktop & System Shell</h3>
        <p className="text-xs text-muted-foreground">
          Native Windows desktop integration, system tray, and background daemon execution
        </p>
      </div>

      <div className="space-y-4">
        {/* Minimize to Tray */}
        <div className="flex items-center justify-between p-3.5 rounded-lg border border-border bg-card/60">
          <div>
            <label className="text-xs font-medium text-foreground block">Minimize to System Tray on Close</label>
            <span className="text-xs text-muted-foreground">
              Clicking the titlebar &times; hides Knox to the Windows notification tray so AI agents can still query
            </span>
          </div>
          <input
            type="checkbox"
            checked={userSettings.closeToTray}
            onChange={(e) => updateUserSettings({ closeToTray: e.target.checked })}
            className="size-4 rounded border-border text-primary focus:ring-primary cursor-pointer"
          />
        </div>

        {/* Notifications */}
        <div className="flex items-center justify-between p-3.5 rounded-lg border border-border bg-card/60">
          <div>
            <label className="text-xs font-medium text-foreground block">Desktop Toast Notifications</label>
            <span className="text-xs text-muted-foreground">
              Display in-app toast alerts when actions are recorded, doubts resolved, or settings updated
            </span>
          </div>
          <input
            type="checkbox"
            checked={userSettings.showDesktopNotifications}
            onChange={(e) => updateUserSettings({ showDesktopNotifications: e.target.checked })}
            className="size-4 rounded border-border text-primary focus:ring-primary cursor-pointer"
          />
        </div>
      </div>
    </div>
  );
};

