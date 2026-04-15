import { useEffect } from "react";
import { useSecurityStore } from "@/stores/securityStore";
import { useUiStore, type Tab } from "@/stores/uiStore";

export function useKeyboardShortcuts() {
  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      const tag = (e.target as HTMLElement)?.tagName;
      if (tag === "INPUT" || tag === "TEXTAREA" || tag === "SELECT") return;

      const ctrl = e.ctrlKey || e.metaKey;
      if (!ctrl) return;

      const tabMap: Record<string, Tab> = {
        "1": "profiles",
        "2": "vault",
        "3": "repos",
        "4": "config",
        "5": "bridge",
        "6": "settings",
      };

      if (tabMap[e.key]) {
        e.preventDefault();
        useUiStore.getState().setActiveTab(tabMap[e.key]);
      } else if (e.key === "l" || e.key === "L") {
        e.preventDefault();
        const { pinIsSet, lockApp } = useSecurityStore.getState();
        if (pinIsSet) lockApp().catch(() => {});
      }
    };

    document.addEventListener("keydown", handler);
    return () => document.removeEventListener("keydown", handler);
  }, []);
}
