//! Custom React hook providing real-time data synchronization with Knox backend.

import { useEffect, useRef } from "react";
import { KnoxAPI } from "@/services/api";
import { useKnoxStore } from "@/store/useKnoxStore";

/**
 * Listens for SQLite database modifications emitted by the Tauri backend,
 * window focus transitions, and tab visibility changes to keep the Knowledge Graph,
 * status counters, and active action details synchronized in real time.
 */
export function useRealtimeSync() {
  const refreshDataQuietly = useKnoxStore((state) => state.refreshDataQuietly);
  const lastSyncRef = useRef<number>(Date.now());

  useEffect(() => {
    let unlisten: (() => void) | undefined;
    let isMounted = true;

    // 1. Subscribe to Tauri backend's SQLite change emitter
    KnoxAPI.listenDbChanged(() => {
      if (!isMounted) return;
      const now = Date.now();
      // Debounce rapid bursts within 400ms
      if (now - lastSyncRef.current < 400) return;
      lastSyncRef.current = now;
      refreshDataQuietly();
    }).then((cleanup) => {
      if (isMounted) {
        unlisten = cleanup;
      } else {
        cleanup();
      }
    });

    // 2. Synchronize immediately when developer focuses or switches back to Knox window
    const handleFocus = () => {
      const now = Date.now();
      if (now - lastSyncRef.current > 1000) {
        lastSyncRef.current = now;
        refreshDataQuietly();
      }
    };

    const handleVisibility = () => {
      if (document.visibilityState === "visible") {
        handleFocus();
      }
    };

    window.addEventListener("focus", handleFocus);
    document.addEventListener("visibilitychange", handleVisibility);

    // 3. Gentle 3-second heartbeat polling check as fallback safety net
    const interval = setInterval(() => {
      if (!isMounted) return;
      const now = Date.now();
      if (now - lastSyncRef.current >= 3000) {
        lastSyncRef.current = now;
        refreshDataQuietly();
      }
    }, 3000);

    return () => {
      isMounted = false;
      if (unlisten) unlisten();
      window.removeEventListener("focus", handleFocus);
      document.removeEventListener("visibilitychange", handleVisibility);
      clearInterval(interval);
    };
  }, [refreshDataQuietly]);
}
