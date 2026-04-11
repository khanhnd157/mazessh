import { useState, useRef, useEffect, useCallback } from "react";
import { ArrowLeftRight, Check } from "lucide-react";
import { toast } from "sonner";
import { useAppStore } from "@/stores/appStore";
import { useProfileStore } from "@/stores/profileStore";
import { useLogStore } from "@/stores/logStore";
import type { ProfileSummary } from "@/types";
import { ProviderIcon } from "@/components/profiles/ProviderIcon";

interface QuickSwitchProps {
  profiles: ProfileSummary[];
}

export function QuickSwitch({ profiles }: QuickSwitchProps) {
  const [open, setOpen] = useState(false);
  const [focusIdx, setFocusIdx] = useState(-1);
  const ref = useRef<HTMLDivElement>(null);
  const { activateProfile, activeProfile } = useAppStore();
  const { fetchProfiles } = useProfileStore();
  const { addLog } = useLogStore();

  useEffect(() => {
    function handleClickOutside(event: MouseEvent) {
      if (ref.current && !ref.current.contains(event.target as Node)) {
        setOpen(false);
      }
    }
    document.addEventListener("mousedown", handleClickOutside);
    return () => document.removeEventListener("mousedown", handleClickOutside);
  }, []);

  const handleSwitch = useCallback(
    async (profile: ProfileSummary) => {
      setOpen(false);
      try {
        const result = await activateProfile(profile.id);
        await fetchProfiles();
        addLog({
          action: "switch",
          detail: `Switched to ${result.profile_name}`,
          level: "info",
        });
        toast.success(`Switched to ${result.profile_name}`);
      } catch (err) {
        addLog({
          action: "switch",
          detail: `Failed to switch: ${err}`,
          level: "error",
        });
        toast.error("Switch failed", { description: String(err) });
      }
    },
    [activateProfile, fetchProfiles, addLog],
  );

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (!open) return;
    if (e.key === "ArrowDown") {
      e.preventDefault();
      setFocusIdx((i) => Math.min(i + 1, profiles.length - 1));
    } else if (e.key === "ArrowUp") {
      e.preventDefault();
      setFocusIdx((i) => Math.max(i - 1, 0));
    } else if (e.key === "Enter" && focusIdx >= 0) {
      e.preventDefault();
      handleSwitch(profiles[focusIdx]);
    } else if (e.key === "Escape") {
      setOpen(false);
    }
  };

  if (profiles.length === 0) return null;

  return (
    <div className="relative" ref={ref} onKeyDown={handleKeyDown}>
      <button
        type="button"
        onClick={() => {
          setOpen(!open);
          setFocusIdx(-1);
        }}
        className="flex items-center gap-1.5 px-3 py-1.5 text-xs font-medium rounded-md bg-primary text-primary-foreground hover:bg-primary/90 transition-colors"
      >
        <ArrowLeftRight size={12} />
        Switch
      </button>
      {open && (
        <div className="absolute right-0 top-full mt-1.5 w-64 rounded-lg border bg-popover shadow-xl shadow-black/20 z-50 overflow-hidden">
          <div className="px-3 py-2 border-b">
            <p className="text-[10px] font-medium text-muted-foreground uppercase tracking-wider">
              Switch Identity
            </p>
          </div>
          <div className="p-1 max-h-64 overflow-y-auto">
            {profiles.map((p, i) => (
              <button
                type="button"
                key={p.id}
                onClick={() => handleSwitch(p)}
                className={`w-full text-left px-2.5 py-2 text-sm rounded-md transition-colors flex items-center gap-2.5 ${
                  focusIdx === i
                    ? "bg-accent"
                    : activeProfile?.id === p.id
                      ? "bg-primary/8"
                      : "hover:bg-accent"
                }`}
              >
                <ProviderIcon provider={p.provider} size={16} />
                <div className="min-w-0 flex-1">
                  <div className="font-medium text-sm truncate">{p.name}</div>
                  <div className="text-[11px] text-muted-foreground truncate">
                    {p.email}
                  </div>
                </div>
                {activeProfile?.id === p.id && (
                  <Check size={14} className="text-success shrink-0" />
                )}
              </button>
            ))}
          </div>
        </div>
      )}
    </div>
  );
}
