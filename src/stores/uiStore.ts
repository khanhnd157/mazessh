import { create } from "zustand";

export type Tab = "profiles" | "vault" | "repos" | "config" | "bridge" | "settings";

interface UiStore {
  activeTab: Tab;
  setActiveTab: (tab: Tab) => void;
}

export const useUiStore = create<UiStore>((set) => ({
  activeTab: "profiles",
  setActiveTab: (tab) => set({ activeTab: tab }),
}));
