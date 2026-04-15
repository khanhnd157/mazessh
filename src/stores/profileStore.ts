import { create } from "zustand";
import { commands } from "@/lib/tauri-commands";
import type { CreateProfileInput, DetectedKey, ProfileSummary, SshProfile, UpdateProfileInput } from "@/types";

interface ProfileStore {
  profiles: ProfileSummary[];
  selectedProfileId: string | null;
  selectedProfile: SshProfile | null;
  detectedKeys: DetectedKey[];
  loading: boolean;

  fetchProfiles: () => Promise<void>;
  selectProfile: (id: string | null) => Promise<void>;
  createProfile: (input: CreateProfileInput) => Promise<SshProfile>;
  updateProfile: (id: string, input: UpdateProfileInput) => Promise<void>;
  deleteProfile: (id: string) => Promise<void>;
  scanKeys: () => Promise<void>;
}

export const useProfileStore = create<ProfileStore>((set, get) => ({
  profiles: [],
  selectedProfileId: null,
  selectedProfile: null,
  detectedKeys: [],
  loading: false,

  fetchProfiles: async () => {
    set({ loading: true });
    try {
      const profiles = await commands.getProfiles();
      set({ profiles });
    } finally {
      set({ loading: false });
    }
  },

  selectProfile: async (id: string | null) => {
    if (!id) {
      set({ selectedProfileId: null, selectedProfile: null });
      return;
    }
    set({ selectedProfileId: id });
    const profile = await commands.getProfile(id);
    set({ selectedProfile: profile });
  },

  createProfile: async (input: CreateProfileInput) => {
    const profile = await commands.createProfile(input);
    await get().fetchProfiles();
    return profile;
  },

  updateProfile: async (id: string, input: UpdateProfileInput) => {
    await commands.updateProfile(id, input);
    await get().fetchProfiles();
    if (get().selectedProfileId === id) {
      await get().selectProfile(id);
    }
  },

  deleteProfile: async (id: string) => {
    // Optimistic: remove from list immediately so UI feels instant
    set((s) => ({
      profiles: s.profiles.filter((p) => p.id !== id),
      selectedProfileId: s.selectedProfileId === id ? null : s.selectedProfileId,
      selectedProfile: s.selectedProfileId === id ? null : s.selectedProfile,
    }));
    try {
      await commands.deleteProfile(id);
    } catch (err) {
      // Roll back on failure
      await get().fetchProfiles();
      throw err;
    }
  },

  scanKeys: async () => {
    const keys = await commands.scanSshKeys();
    set({ detectedKeys: keys });
  },
}));
