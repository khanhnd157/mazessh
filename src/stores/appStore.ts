import { create } from "zustand";
import { invoke } from "@tauri-apps/api/core";
import { commands } from "@/lib/tauri-commands";
import type { ActivationResult, ConnectionTestResult, GitIdentityInfo, ProfileSummary } from "@/types";
import { getProviderLabel } from "@/types";

function updateTrayTooltip(profile: ProfileSummary | null) {
  const tooltip = profile
    ? `Maze SSH - ${profile.name} (${getProviderLabel(profile.provider)})`
    : "Maze SSH - No active profile";
  invoke("update_tray_tooltip", { tooltip }).catch(() => {});
}

interface AppStore {
  activeProfile: ProfileSummary | null;
  lastActivation: ActivationResult | null;
  currentGitIdentity: GitIdentityInfo | null;
  loading: boolean;

  fetchActiveProfile: () => Promise<void>;
  activateProfile: (id: string) => Promise<ActivationResult>;
  deactivateProfile: () => Promise<void>;
  testConnection: (id: string) => Promise<ConnectionTestResult>;
  fetchGitIdentity: () => Promise<void>;
}

export const useAppStore = create<AppStore>((set) => ({
  activeProfile: null,
  lastActivation: null,
  currentGitIdentity: null,
  loading: false,

  fetchActiveProfile: async () => {
    const profile = await commands.getActiveProfile();
    set({ activeProfile: profile });
    updateTrayTooltip(profile);
  },

  activateProfile: async (id: string) => {
    set({ loading: true });
    try {
      const result = await commands.activateProfile(id);
      const profile = await commands.getActiveProfile();
      set({ activeProfile: profile, lastActivation: result });
      updateTrayTooltip(profile);
      return result;
    } finally {
      set({ loading: false });
    }
  },

  deactivateProfile: async () => {
    await commands.deactivateProfile();
    set({ activeProfile: null, lastActivation: null });
    updateTrayTooltip(null);
  },

  testConnection: async (id: string) => {
    return await commands.testSshConnection(id);
  },

  fetchGitIdentity: async () => {
    try {
      const identity = await commands.getCurrentGitIdentity();
      set({ currentGitIdentity: identity });
    } catch {
      set({ currentGitIdentity: null });
    }
  },
}));
