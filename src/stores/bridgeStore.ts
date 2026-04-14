import { create } from "zustand";
import { commands } from "@/lib/tauri-commands";
import type { BridgeOverview, DistroBridgeStatus } from "@/types";

interface BridgeStore {
  overview: BridgeOverview | null;
  loading: boolean;

  fetchOverview: () => Promise<void>;
  bootstrapDistro: (distro: string) => Promise<DistroBridgeStatus>;
  teardownDistro: (distro: string) => Promise<void>;
  startRelay: (distro: string) => Promise<void>;
  stopRelay: (distro: string) => Promise<void>;
  restartRelay: (distro: string) => Promise<void>;
  setEnabled: (distro: string, enabled: boolean) => Promise<void>;
}

export const useBridgeStore = create<BridgeStore>((set, get) => ({
  overview: null,
  loading: false,

  fetchOverview: async () => {
    set({ loading: true });
    try {
      const overview = await commands.getBridgeOverview();
      set({ overview });
    } finally {
      set({ loading: false });
    }
  },

  bootstrapDistro: async (distro: string) => {
    const status = await commands.bootstrapBridge(distro);
    // Refresh full overview after bootstrap
    await get().fetchOverview();
    return status;
  },

  teardownDistro: async (distro: string) => {
    await commands.teardownBridge(distro);
    await get().fetchOverview();
  },

  startRelay: async (distro: string) => {
    await commands.startBridgeRelay(distro);
    await get().fetchOverview();
  },

  stopRelay: async (distro: string) => {
    await commands.stopBridgeRelay(distro);
    await get().fetchOverview();
  },

  restartRelay: async (distro: string) => {
    await commands.restartBridgeRelay(distro);
    await get().fetchOverview();
  },

  setEnabled: async (distro: string, enabled: boolean) => {
    await commands.setBridgeEnabled(distro, enabled);
    await get().fetchOverview();
  },
}));
