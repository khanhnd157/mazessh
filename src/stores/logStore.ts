import { create } from "zustand";

export interface LogEntry {
  id: string;
  timestamp: string;
  action: string;
  detail: string;
  level: "info" | "warn" | "error";
}

interface LogStore {
  logs: LogEntry[];
  addLog: (entry: Omit<LogEntry, "id" | "timestamp">) => void;
  clearLogs: () => void;
}

let logCounter = 0;

export const useLogStore = create<LogStore>((set) => ({
  logs: [],

  addLog: (entry) => {
    const log: LogEntry = {
      ...entry,
      id: String(++logCounter),
      timestamp: new Date().toISOString(),
    };
    set((state) => ({
      logs: [log, ...state.logs].slice(0, 200),
    }));
  },

  clearLogs: () => set({ logs: [] }),
}));
