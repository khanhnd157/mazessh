import { create } from "zustand";
import { commands } from "@/lib/tauri-commands";
import type {
  GenerateKeyRequest,
  ImportKeyRequest,
  MigrationPreview,
  MigrationReport,
  SshKeyItem,
  SshKeyItemSummary,
  UpdateKeyRequest,
  VaultStateResponse,
} from "@/types";

interface VaultStore {
  vaultState: VaultStateResponse | null;
  keys: SshKeyItemSummary[];
  selectedKeyId: string | null;
  selectedKey: SshKeyItem | null;
  loading: boolean;
  keysLoading: boolean;

  fetchVaultState: () => Promise<void>;
  initVault: (passphrase: string) => Promise<void>;
  unlockVault: (passphrase: string) => Promise<void>;
  lockVault: () => Promise<void>;
  fetchKeys: () => Promise<void>;
  selectKey: (id: string | null) => Promise<void>;
  generateKey: (input: GenerateKeyRequest) => Promise<SshKeyItem>;
  importKey: (request: ImportKeyRequest) => Promise<SshKeyItem>;
  updateKey: (id: string, input: UpdateKeyRequest) => Promise<void>;
  archiveKey: (id: string) => Promise<void>;
  deleteKey: (id: string) => Promise<void>;
  getMigrationPreview: () => Promise<MigrationPreview>;
  migrateProfiles: (ids: string[]) => Promise<MigrationReport>;
  setVaultState: (state: VaultStateResponse) => void;
}

export const useVaultStore = create<VaultStore>((set, get) => ({
  vaultState: null,
  keys: [],
  selectedKeyId: null,
  selectedKey: null,
  loading: false,
  keysLoading: false,

  fetchVaultState: async () => {
    const vaultState = await commands.vaultGetState();
    set({ vaultState });
  },

  initVault: async (passphrase) => {
    set({ loading: true });
    try {
      await commands.vaultInit(passphrase);
      await get().fetchVaultState();
    } finally {
      set({ loading: false });
    }
  },

  unlockVault: async (passphrase) => {
    set({ loading: true });
    try {
      await commands.vaultUnlock(passphrase);
      await get().fetchVaultState();
      await get().fetchKeys();
    } finally {
      set({ loading: false });
    }
  },

  lockVault: async () => {
    set({ keys: [], selectedKey: null, selectedKeyId: null });
    await commands.vaultLock();
    await get().fetchVaultState();
  },

  fetchKeys: async () => {
    set({ keysLoading: true });
    try {
      const keys = await commands.vaultListKeys();
      set({ keys });
    } catch {
      // May fail if vault locked
    } finally {
      set({ keysLoading: false });
    }
  },

  selectKey: async (id) => {
    if (!id) {
      set({ selectedKeyId: null, selectedKey: null });
      return;
    }
    set({ selectedKeyId: id });
    const key = await commands.vaultGetKey(id);
    set({ selectedKey: key });
  },

  generateKey: async (input) => {
    const key = await commands.vaultGenerateKey(input);
    // Optimistic append — avoids a full list re-fetch
    set((s) => ({
      keys: [
        ...s.keys,
        {
          id: key.id,
          name: key.name,
          algorithm: key.algorithm,
          fingerprint: key.fingerprint,
          state: key.state,
          created_at: key.created_at,
        },
      ],
    }));
    return key;
  },

  importKey: async (request) => {
    const key = await commands.vaultImportKey(request);
    set((s) => ({
      keys: [
        ...s.keys,
        {
          id: key.id,
          name: key.name,
          algorithm: key.algorithm,
          fingerprint: key.fingerprint,
          state: key.state,
          created_at: key.created_at,
        },
      ],
    }));
    return key;
  },

  updateKey: async (id, input) => {
    await commands.vaultUpdateKey(id, input);
    // Optimistic: patch name in summary list without a full re-fetch
    if (input.name != null) {
      set((s) => ({
        keys: s.keys.map((k) => (k.id === id ? { ...k, name: input.name! } : k)),
      }));
    }
    // Always refresh the full detail view so comment/export policy stay in sync
    if (get().selectedKeyId === id) {
      await get().selectKey(id);
    }
  },

  archiveKey: async (id) => {
    // Optimistic: flip state in list immediately
    set((s) => ({
      keys: s.keys.map((k) => (k.id === id ? { ...k, state: "archived" as const } : k)),
      selectedKey: s.selectedKey?.id === id
        ? { ...s.selectedKey, state: "archived" as const }
        : s.selectedKey,
    }));
    try {
      await commands.vaultArchiveKey(id);
    } catch (err) {
      await get().fetchKeys();
      throw err;
    }
  },

  deleteKey: async (id) => {
    // Optimistic: remove from list immediately
    set((s) => ({
      keys: s.keys.filter((k) => k.id !== id),
      selectedKeyId: s.selectedKeyId === id ? null : s.selectedKeyId,
      selectedKey: s.selectedKeyId === id ? null : s.selectedKey,
    }));
    try {
      await commands.vaultDeleteKey(id);
    } catch (err) {
      await get().fetchKeys();
      throw err;
    }
  },

  getMigrationPreview: async () => {
    return await commands.getMigrationPreview();
  },

  migrateProfiles: async (ids) => {
    const report = await commands.migrateProfilesToVault(ids);
    await get().fetchKeys();
    return report;
  },

  setVaultState: (vaultState) => set({ vaultState }),
}));
