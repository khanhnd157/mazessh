import { useState, useEffect, useRef, useCallback } from "react";
import { createPortal } from "react-dom";
import {
  Minus, Square, X, Copy, Moon, Sun, Circle,
  ArrowLeftRight, Power, Check, Lock, Loader2,
} from "lucide-react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { toast } from "sonner";
import { useThemeStore } from "@/stores/themeStore";
import { useAppStore } from "@/stores/appStore";
import { useProfileStore } from "@/stores/profileStore";
import { useUiStore } from "@/stores/uiStore";
import { useSecurityStore } from "@/stores/securityStore";
import { useLogStore } from "@/stores/logStore";
import { getProviderLabel } from "@/types";
import { ProviderIcon } from "@/components/profiles/ProviderIcon";
import type { ProfileSummary } from "@/types";

export function TitleBar() {
  const [maximized, setMaximized] = useState(false);
  const { theme, toggleTheme } = useThemeStore();
  const { activeProfile, deactivateProfile } = useAppStore();
  const { profiles, fetchProfiles } = useProfileStore();
  const { pinIsSet, lockApp } = useSecurityStore();
  const { addLog } = useLogStore();
  const appWindow = getCurrentWindow();

  useEffect(() => {
    const unlisten = appWindow.onResized(async () => {
      setMaximized(await appWindow.isMaximized());
    });
    appWindow.isMaximized().then(setMaximized);
    return () => {
      unlisten.then((fn) => fn());
    };
  }, [appWindow]);

  const handleDeactivate = async () => {
    await deactivateProfile();
    await fetchProfiles();
    addLog({ action: "deactivate", detail: "Profile deactivated", level: "info" });
    toast.info("Profile deactivated");
  };

  return (
    <div
      className="titlebar-bg flex items-center justify-between h-9 select-none shrink-0"
      data-tauri-drag-region
    >
      {/* Left: Branding + active status */}
      <div className="flex items-center pl-3.5 pointer-events-none">
        <div className="flex items-center gap-2">
          <img src="/logo.png" alt="Maze SSH" className="w-4.5 h-4.5 rounded-[4px]" />
          <span className="text-[11px] text-muted-foreground/80 font-medium tracking-wide">
            Maze SSH
          </span>
        </div>

        <span className="text-muted-foreground/30 text-[11px] mx-2">|</span>

        {activeProfile ? (
          <div className="flex items-center gap-1.5">
            <Circle size={5} className="fill-success text-success animate-pulse" />
            <span className="text-[11px] text-foreground/80 font-medium">
              {activeProfile.name}
            </span>
            <span className="text-[10px] text-muted-foreground/50">
              {getProviderLabel(activeProfile.provider)}
            </span>
          </div>
        ) : (
          <div className="flex items-center gap-1.5">
            <Circle size={5} className="fill-muted-foreground/40 text-muted-foreground/40" />
            <span className="text-[11px] text-muted-foreground/50">No active profile</span>
          </div>
        )}
      </div>

      {/* Center: Switch + Deactivate */}
      <div className="absolute left-1/2 -translate-x-1/2 flex items-center gap-1">
        {profiles.length > 0 && <SwitchDropdown />}
        {activeProfile && (
          <button
            type="button"
            onClick={handleDeactivate}
            title="Deactivate profile"
            className="flex items-center gap-1 px-2 py-1 text-[11px] rounded-md border border-border/60 text-muted-foreground/60 hover:text-foreground hover:bg-foreground/5 hover:border-border transition-colors"
          >
            <Power size={11} />
            <span className="hidden sm:inline">Deactivate</span>
          </button>
        )}
      </div>

      {/* Right: Theme + Window controls */}
      <div className="flex items-center h-full">
        {pinIsSet && (
          <button
            type="button"
            onClick={() => lockApp().catch(() => {})}
            title="Lock app"
            aria-label="Lock app"
            className="h-full w-10 flex items-center justify-center text-muted-foreground/60 hover:text-muted-foreground hover:bg-foreground/5 transition-colors"
          >
            <Lock size={13} />
          </button>
        )}
        <button
          type="button"
          onClick={toggleTheme}
          title={theme === "dark" ? "Switch to light" : "Switch to dark"}
          aria-label={theme === "dark" ? "Switch to light theme" : "Switch to dark theme"}
          className="h-full w-10 flex items-center justify-center text-muted-foreground/60 hover:text-muted-foreground hover:bg-foreground/5 transition-colors"
        >
          {theme === "dark" ? <Sun size={13} /> : <Moon size={13} />}
        </button>
        <div className="w-px h-3.5 bg-border/50 mx-0.5" aria-hidden="true" />
        <button
          type="button"
          onClick={() => appWindow.minimize()}
          title="Minimize"
          aria-label="Minimize window"
          className="h-full w-11.5 flex items-center justify-center text-muted-foreground/60 hover:text-foreground hover:bg-foreground/5 transition-colors"
        >
          <Minus size={15} strokeWidth={1} />
        </button>
        <button
          type="button"
          onClick={() => appWindow.toggleMaximize()}
          title={maximized ? "Restore" : "Maximize"}
          aria-label={maximized ? "Restore window" : "Maximize window"}
          className="h-full w-11.5 flex items-center justify-center text-muted-foreground/60 hover:text-foreground hover:bg-foreground/5 transition-colors"
        >
          {maximized ? (
            <Copy size={11} strokeWidth={1.5} className="rotate-180" />
          ) : (
            <Square size={11} strokeWidth={1.5} />
          )}
        </button>
        <button
          type="button"
          onClick={() => appWindow.hide()}
          title="Close"
          aria-label="Close window"
          className="h-full w-11.5 flex items-center justify-center text-muted-foreground/60 hover:bg-[#c42b1c] hover:text-white transition-colors"
        >
          <X size={15} strokeWidth={1.5} />
        </button>
      </div>
    </div>
  );
}

/* ── Inline Switch Dropdown (compact for titlebar) ── */
function SwitchDropdown() {
  const [open, setOpen] = useState(false);
  const [switching, setSwitching] = useState(false);
  const ref = useRef<HTMLDivElement>(null);
  const dropdownRef = useRef<HTMLDivElement>(null);
  const { activateProfile, activeProfile } = useAppStore();
  const { profiles, fetchProfiles } = useProfileStore();
  const { addLog } = useLogStore();

  useEffect(() => {
    const handler = (e: MouseEvent) => {
      const target = e.target as Node;
      // Check both trigger button and portal dropdown
      if (
        ref.current && !ref.current.contains(target) &&
        dropdownRef.current && !dropdownRef.current.contains(target)
      ) {
        setOpen(false);
      }
    };
    document.addEventListener("mousedown", handler);
    return () => document.removeEventListener("mousedown", handler);
  }, []);

  const handleSwitch = useCallback(
    async (p: ProfileSummary) => {
      setOpen(false);
      setSwitching(true);

      // Select profile + switch to Profiles tab immediately
      useProfileStore.getState().selectProfile(p.id);
      useUiStore.getState().setActiveTab("profiles");

      try {
        const result = await activateProfile(p.id);
        await fetchProfiles();
        addLog({ action: "switch", detail: `Switched to ${result.profile_name}`, level: "info" });
        toast.success(`Switched to ${result.profile_name}`);
      } catch (err) {
        toast.error("Switch failed", { description: String(err) });
      } finally {
        setSwitching(false);
      }
    },
    [activateProfile, fetchProfiles, addLog],
  );

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === "Escape") setOpen(false);
  };

  const btnRef = useRef<HTMLButtonElement>(null);
  const [dropdownPos, setDropdownPos] = useState({ top: 0, left: 0 });

  useEffect(() => {
    if (open && btnRef.current) {
      const rect = btnRef.current.getBoundingClientRect();
      setDropdownPos({
        top: rect.bottom + 4,
        left: rect.left + rect.width / 2 - 120, // 120 = half of w-60 (240px)
      });
    }
  }, [open]);

  return (
    <div ref={ref} onKeyDown={handleKeyDown}>
      <button
        ref={btnRef}
        type="button"
        onClick={() => setOpen(!open)}
        className="flex items-center gap-1 px-2.5 py-1 text-[11px] font-medium rounded-md bg-primary/15 text-primary hover:bg-primary/25 transition-colors"
      >
        {switching ? <Loader2 size={11} className="animate-spin" /> : <ArrowLeftRight size={11} />}
        {switching ? "Switching..." : "Switch"}
      </button>
      {open && createPortal(
        <div
          ref={dropdownRef}
          style={{ top: dropdownPos.top, left: dropdownPos.left }}
          className="fixed w-60 rounded-lg border bg-popover shadow-xl shadow-black/20 z-50 overflow-hidden animate-fade-in"
        >
          <div className="px-3 py-1.5 border-b">
            <p className="text-[10px] font-medium text-muted-foreground uppercase tracking-wider">
              Switch Identity
            </p>
          </div>
          <div className="p-1 max-h-56 overflow-y-auto">
            {profiles.map((p) => (
              <button
                type="button"
                key={p.id}
                onClick={() => handleSwitch(p)}
                className={`w-full text-left px-2.5 py-1.5 text-[12px] rounded-md transition-colors flex items-center gap-2 ${
                  activeProfile?.id === p.id ? "bg-primary/8" : "hover:bg-accent"
                }`}
              >
                <ProviderIcon provider={p.provider} size={14} />
                <div className="min-w-0 flex-1">
                  <div className="font-medium truncate">{p.name}</div>
                  <div className="text-[10px] text-muted-foreground truncate">{p.email}</div>
                </div>
                {activeProfile?.id === p.id && (
                  <Check size={12} className="text-success shrink-0" />
                )}
              </button>
            ))}
          </div>
        </div>,
        document.body,
      )}
    </div>
  );
}
