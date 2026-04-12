import { create } from "zustand";
import { commands } from "@/lib/tauri-commands";
import type { SecuritySettings } from "@/types";

interface SecurityStore {
  isLocked: boolean;
  pinIsSet: boolean;
  settings: SecuritySettings | null;
  initialized: boolean;

  fetchLockState: () => Promise<void>;
  fetchSettings: () => Promise<void>;
  setupPin: (pin: string) => Promise<void>;
  unlock: (pin: string) => Promise<boolean>;
  lockApp: () => Promise<void>;
  changePin: (oldPin: string, newPin: string) => Promise<void>;
  removePin: (pin: string) => Promise<void>;
  updateSettings: (settings: SecuritySettings) => Promise<void>;
  setLocked: (locked: boolean) => void;
}

export const useSecurityStore = create<SecurityStore>((set) => ({
  isLocked: false,
  pinIsSet: false,
  settings: null,
  initialized: false,

  fetchLockState: async () => {
    const state = await commands.getLockState();
    set({ isLocked: state.is_locked, pinIsSet: state.pin_is_set, initialized: true });
  },

  fetchSettings: async () => {
    try {
      const settings = await commands.getSecuritySettings();
      set({ settings });
    } catch {
      // May fail if locked
    }
  },

  setupPin: async (pin: string) => {
    await commands.setupPin(pin);
    set({ pinIsSet: true });
  },

  unlock: async (pin: string) => {
    const valid = await commands.verifyPin(pin);
    if (valid) {
      set({ isLocked: false });
    }
    return valid;
  },

  lockApp: async () => {
    await commands.lockApp();
    set({ isLocked: true });
  },

  changePin: async (oldPin: string, newPin: string) => {
    await commands.changePin(oldPin, newPin);
  },

  removePin: async (pin: string) => {
    await commands.removePin(pin);
    set({ pinIsSet: false, isLocked: false });
  },

  updateSettings: async (settings: SecuritySettings) => {
    await commands.updateSecuritySettings(settings);
    set({ settings });
  },

  setLocked: (locked: boolean) => set({ isLocked: locked }),
}));
