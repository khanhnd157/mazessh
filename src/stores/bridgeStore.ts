import { create } from "zustand";
import { listen } from "@tauri-apps/api/event";
import { toast } from "sonner";
import { commands } from "@/lib/tauri-commands";
import type {
  BinaryUpdateStatus,
  BinaryVersion,
  BootstrapAllResult,
  BridgeHistoryEvent,
  BridgeOverview,
  BridgeProvider,
  DiagnosticsResult,
  DistroBridgeStatus,
  DownloadProgress,
  ProviderStatus,
  RelayMode,
  RelayRestartFailedEvent,
  ShellInjection,
} from "@/types";

interface BridgeStore {
  overview: BridgeOverview | null;
  providers: ProviderStatus[];
  recommendedProvider: BridgeProvider | null;
  binaryVersions: BinaryVersion | null;
  downloadProgress: Record<string, number>;
  diagnostics: Record<string, DiagnosticsResult>;
  updateStatuses: BinaryUpdateStatus[];
  shellInjections: Record<string, ShellInjection[]>;
  bridgeHistory: Record<string, BridgeHistoryEvent[]>;
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
  setAutoRestart: (distro: string, enabled: boolean) => Promise<void>;
  checkUpdates: () => Promise<void>;
  setSocketPath: (distro: string, path: string) => Promise<void>;
  // Phase 6
  resetRestartCount: (distro: string) => Promise<void>;
  runFix: (distro: string, cmd: string) => Promise<string>;
  exportConfig: () => Promise<string>;
  importConfig: (json: string) => Promise<number>;
  bootstrapAll: () => Promise<BootstrapAllResult[]>;
  // Phase 7
  refreshRelayScript: (distro: string) => Promise<void>;
  fetchShellInjections: (distro: string) => Promise<void>;
  removeShellInjection: (distro: string, rcFile: string) => Promise<void>;
  // Phase 8
  fetchBridgeHistory: (distro: string, limit?: number) => Promise<void>;
  setMaxRestarts: (distro: string, maxRestarts: number) => Promise<void>;
  previewWindowsSshHost: (distro: string) => Promise<string>;
  upsertWindowsSshHost: (distro: string) => Promise<void>;
  removeWindowsSshHost: (distro: string) => Promise<void>;
}

export const useBridgeStore = create<BridgeStore>((set, get) => ({
  overview: null,
  providers: [],
  recommendedProvider: null,
  binaryVersions: null,
  downloadProgress: {},
  diagnostics: {},
  updateStatuses: [],
  shellInjections: {},
  bridgeHistory: {},
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

  setAutoRestart: async (distro: string, enabled: boolean) => {
    await commands.setAutoRestart(distro, enabled);
    await get().fetchOverview();
  },

  checkUpdates: async () => {
    const statuses = await commands.checkRelayBinaryUpdates();
    set({ updateStatuses: statuses });
  },

  setSocketPath: async (distro: string, path: string) => {
    await commands.setDistroSocketPath(distro, path);
    await get().fetchOverview();
  },

  // Phase 6
  resetRestartCount: async (distro: string) => {
    await commands.resetWatchdogRestartCount(distro);
    await get().fetchOverview();
  },

  runFix: async (distro: string, cmd: string): Promise<string> => {
    const output = await commands.runDiagnosticFix(distro, cmd);
    await get().runDiagnostics(distro);
    return output;
  },

  exportConfig: async (): Promise<string> => {
    return commands.exportBridgeConfig();
  },

  importConfig: async (json: string): Promise<number> => {
    const count = await commands.importBridgeConfig(json);
    await get().fetchOverview();
    return count;
  },

  bootstrapAll: async (): Promise<BootstrapAllResult[]> => {
    const results = await commands.bootstrapAllDistros();
    await get().fetchOverview();
    return results;
  },

  // Phase 7
  refreshRelayScript: async (distro: string) => {
    await commands.refreshRelayScript(distro);
    await get().fetchOverview();
  },

  fetchShellInjections: async (distro: string) => {
    const injections = await commands.getShellInjections(distro);
    set((state) => ({ shellInjections: { ...state.shellInjections, [distro]: injections } }));
  },

  removeShellInjection: async (distro: string, rcFile: string) => {
    await commands.removeShellInjection(distro, rcFile);
    await get().fetchShellInjections(distro);
  },

  // Phase 8
  fetchBridgeHistory: async (distro: string, limit = 50) => {
    const events = await commands.getBridgeHistory(distro, limit);
    set((state) => ({ bridgeHistory: { ...state.bridgeHistory, [distro]: events } }));
  },

  setMaxRestarts: async (distro: string, maxRestarts: number) => {
    await commands.setDistroMaxRestarts(distro, maxRestarts);
    await get().fetchOverview();
  },

  previewWindowsSshHost: (distro: string) => commands.previewWindowsSshHost(distro),

  upsertWindowsSshHost: async (distro: string) => {
    await commands.upsertWindowsSshHost(distro);
  },

  removeWindowsSshHost: async (distro: string) => {
    await commands.removeWindowsSshHost(distro);
  },
}));

// Listen for relay-restarted events from the watchdog and refresh overview + history
listen<string>("relay-restarted", (event) => {
  const store = useBridgeStore.getState();
  store.fetchOverview();
  store.fetchBridgeHistory(event.payload);
}).catch(() => {});

// Listen for relay-restart-failed events (watchdog gave up after max restarts)
listen<RelayRestartFailedEvent>("relay-restart-failed", (event) => {
  const { distro, count } = event.payload;
  toast.warning(`Auto-restart paused for ${distro} after ${count} failed attempts`);
  const store = useBridgeStore.getState();
  store.fetchOverview();
  store.fetchBridgeHistory(distro);
}).catch(() => {});
// Note: no unlisten — these listeners persist for the app lifetime
