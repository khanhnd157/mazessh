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
    await get().fetchKeys();
    return key;
  },

  importKey: async (request) => {
    const key = await commands.vaultImportKey(request);
    await get().fetchKeys();
    return key;
  },

  updateKey: async (id, input) => {
    await commands.vaultUpdateKey(id, input);
    await get().fetchKeys();
    if (get().selectedKeyId === id) {
      await get().selectKey(id);
    }
  },

  archiveKey: async (id) => {
    await commands.vaultArchiveKey(id);
    await get().fetchKeys();
    if (get().selectedKeyId === id) {
      await get().selectKey(id);
    }
  },

  deleteKey: async (id) => {
    await commands.vaultDeleteKey(id);
    if (get().selectedKeyId === id) {
      set({ selectedKeyId: null, selectedKey: null });
    }
    await get().fetchKeys();
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
