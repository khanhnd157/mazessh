import { useEffect, useRef } from "react";
import { commands } from "@/lib/tauri-commands";

const DEBOUNCE_MS = 30_000;

export function useInactivityTracker() {
  const lastReportedRef = useRef(0);
  const timerRef = useRef<number | null>(null);

  useEffect(() => {
    const report = () => {
      const now = Date.now();
      if (now - lastReportedRef.current > DEBOUNCE_MS) {
        lastReportedRef.current = now;
        commands.touchActivity().catch(() => {});
      }
    };

    // Throttled handler — only schedule one report per debounce window
    const handler = () => {
      if (timerRef.current) return;
      report();
      timerRef.current = window.setTimeout(() => {
        timerRef.current = null;
      }, DEBOUNCE_MS);
    };

    document.addEventListener("mousemove", handler, { passive: true });
    document.addEventListener("keydown", handler, { passive: true });

    return () => {
      document.removeEventListener("mousemove", handler);
      document.removeEventListener("keydown", handler);
      if (timerRef.current) clearTimeout(timerRef.current);
    };
  }, []);
}
