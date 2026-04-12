import { useEffect } from "react";
import { useSecurityStore } from "@/stores/securityStore";

type Tab = "profiles" | "repos" | "config" | "settings";

interface KeyboardShortcutsOptions {
  setActiveTab: (tab: Tab) => void;
}

export function useKeyboardShortcuts({ setActiveTab }: KeyboardShortcutsOptions) {
  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      // Don't trigger in input/textarea
      const tag = (e.target as HTMLElement)?.tagName;
      if (tag === "INPUT" || tag === "TEXTAREA" || tag === "SELECT") return;

      const ctrl = e.ctrlKey || e.metaKey;

      if (ctrl) {
        switch (e.key) {
          case "1":
            e.preventDefault();
            setActiveTab("profiles");
            break;
          case "2":
            e.preventDefault();
            setActiveTab("repos");
            break;
          case "3":
            e.preventDefault();
            setActiveTab("config");
            break;
          case "4":
            e.preventDefault();
            setActiveTab("settings");
            break;
          case "l":
          case "L":
            e.preventDefault();
            const { pinIsSet, lockApp } = useSecurityStore.getState();
            if (pinIsSet) lockApp().catch(() => {});
            break;
        }
      }
    };

    document.addEventListener("keydown", handler);
    return () => document.removeEventListener("keydown", handler);
  }, [setActiveTab]);
}
