import { create } from "zustand";
import { commands } from "@/lib/tauri-commands";
import type { CreateRepoMappingInput, RepoMapping, RepoMappingSummary } from "@/types";

interface RepoMappingStore {
  mappings: RepoMappingSummary[];
  loading: boolean;

  fetchMappings: () => Promise<void>;
  fetchMappingsForProfile: (profileId: string) => Promise<RepoMappingSummary[]>;
  createMapping: (input: CreateRepoMappingInput) => Promise<RepoMapping>;
  deleteMapping: (id: string) => Promise<void>;
}

export const useRepoMappingStore = create<RepoMappingStore>((set, get) => ({
  mappings: [],
  loading: false,

  fetchMappings: async () => {
    set({ loading: true });
    try {
      const mappings = await commands.getRepoMappings();
      set({ mappings });
    } finally {
      set({ loading: false });
    }
  },

  fetchMappingsForProfile: async (profileId: string) => {
    return await commands.getRepoMappingsForProfile(profileId);
  },

  createMapping: async (input: CreateRepoMappingInput) => {
    const mapping = await commands.createRepoMapping(input);
    await get().fetchMappings();
    return mapping;
  },

  deleteMapping: async (id: string) => {
    await commands.deleteRepoMapping(id);
    await get().fetchMappings();
  },
}));
