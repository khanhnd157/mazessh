import { create } from "zustand";
import { listen } from "@tauri-apps/api/event";
import { commands } from "@/lib/tauri-commands";
import type {
  BinaryVersion,
  BridgeOverview,
  BridgeProvider,
  DiagnosticsResult,
  DistroBridgeStatus,
  DownloadProgress,
  ProviderStatus,
  RelayMode,
} from "@/types";

interface BridgeStore {
  overview: BridgeOverview | null;
  providers: ProviderStatus[];
  recommendedProvider: BridgeProvider | null;
  binaryVersions: BinaryVersion | null;
  downloadProgress: Record<string, number>;
  diagnostics: Record<string, DiagnosticsResult>;
  loading: boolean;

  fetchOverview: () => Promise<void>;
  fetchProviders: () => Promise<void>;
  fetchRecommended: () => Promise<void>;
  fetchBinaryVersions: () => Promise<void>;
  downloadBinary: (binary: string) => Promise<void>;
  runDiagnostics: (distro: string) => Promise<void>;
  bootstrapDistro: (distro: string, relayMode?: RelayMode) => Promise<DistroBridgeStatus>;
  teardownDistro: (distro: string) => Promise<void>;
  startRelay: (distro: string) => Promise<void>;
  stopRelay: (distro: string) => Promise<void>;
  restartRelay: (distro: string) => Promise<void>;
  setEnabled: (distro: string, enabled: boolean) => Promise<void>;
  setDistroProvider: (distro: string, provider: BridgeProvider) => Promise<void>;
  setAgentForwarding: (distro: string, enabled: boolean) => Promise<void>;
}

export const useBridgeStore = create<BridgeStore>((set, get) => ({
  overview: null,
  providers: [],
  recommendedProvider: null,
  binaryVersions: null,
  downloadProgress: {},
  diagnostics: {},
  loading: false,

  fetchOverview: async () => {
    set({ loading: true });
    try {
      const overview = await commands.getBridgeOverview();
      set({ overview, providers: overview.provider_statuses });
    } finally {
      set({ loading: false });
    }
  },

  fetchProviders: async () => {
    const providers = await commands.listBridgeProviders();
    set({ providers });
  },

  fetchRecommended: async () => {
    try {
      const rec = await commands.getRecommendedProvider();
      set({ recommendedProvider: rec });
    } catch {
      set({ recommendedProvider: null });
    }
  },

  fetchBinaryVersions: async () => {
    try {
      const versions = await commands.getRelayBinaryVersions();
      set({ binaryVersions: versions });
    } catch {
      set({ binaryVersions: null });
    }
  },

  downloadBinary: async (binary: string) => {
    // Listen for progress events before starting download
    const unlisten = await listen<DownloadProgress>("binary-download-progress", (event) => {
      const { binary: b, percent, status } = event.payload;
      set((state) => ({
        downloadProgress: {
          ...state.downloadProgress,
          [b]: status === "done" ? 100 : percent,
        },
      }));
    });

    try {
      set((state) => ({ downloadProgress: { ...state.downloadProgress, [binary]: 0 } }));
      await commands.downloadRelayBinary(binary);
      // Refresh versions after download
      await get().fetchBinaryVersions();
      await get().fetchOverview();
    } finally {
      unlisten();
    }
  },

  runDiagnostics: async (distro: string) => {
    const result = await commands.runBridgeDiagnostics(distro);
    set((state) => ({
      diagnostics: { ...state.diagnostics, [distro]: result },
    }));
  },

  bootstrapDistro: async (distro: string, relayMode?: RelayMode) => {
    const status = await commands.bootstrapBridge(distro, relayMode);
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

  setDistroProvider: async (distro: string, provider: BridgeProvider) => {
    await commands.setDistroProvider(distro, provider);
    await get().fetchOverview();
  },

  setAgentForwarding: async (distro: string, enabled: boolean) => {
    await commands.setAgentForwarding(distro, enabled);
    await get().fetchOverview();
  },
}));
