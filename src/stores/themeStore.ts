import { create } from "zustand";

type Theme = "dark" | "light";

interface ThemeStore {
  theme: Theme;
  setTheme: (theme: Theme) => void;
  toggleTheme: () => void;
}

function getInitialTheme(): Theme {
  try {
    const stored = localStorage.getItem("maze-ssh-theme");
    if (stored === "light" || stored === "dark") return stored;
  } catch {}
  return "dark";
}

function applyTheme(theme: Theme) {
  const root = document.documentElement;
  root.classList.remove("light", "dark");
  root.classList.add(theme);
  localStorage.setItem("maze-ssh-theme", theme);
}

const initialTheme = getInitialTheme();
applyTheme(initialTheme);

export const useThemeStore = create<ThemeStore>((set) => ({
  theme: initialTheme,
  setTheme: (theme) => {
    applyTheme(theme);
    set({ theme });
  },
  toggleTheme: () => {
    set((state) => {
      const next = state.theme === "dark" ? "light" : "dark";
      applyTheme(next);
      return { theme: next };
    });
  },
}));
