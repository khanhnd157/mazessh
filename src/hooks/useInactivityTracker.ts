import { useEffect, useRef } from "react";
import { commands } from "@/lib/tauri-commands";

const DEBOUNCE_MS = 30_000; // Report activity at most once per 30 seconds

export function useInactivityTracker() {
  const lastReportedRef = useRef(0);

  useEffect(() => {
    const report = () => {
      const now = Date.now();
      if (now - lastReportedRef.current > DEBOUNCE_MS) {
        lastReportedRef.current = now;
        commands.touchActivity().catch(() => {});
      }
    };

    const events = ["mousemove", "keydown", "mousedown", "scroll"];
    events.forEach((e) => document.addEventListener(e, report, { passive: true }));

    return () => {
      events.forEach((e) => document.removeEventListener(e, report));
    };
  }, []);
}
