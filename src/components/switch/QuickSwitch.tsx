import { useState, useRef, useEffect } from "react";
import { useAppStore } from "@/stores/appStore";
import { useProfileStore } from "@/stores/profileStore";
import { useLogStore } from "@/stores/logStore";
import type { ProfileSummary } from "@/types";
import { getProviderLabel } from "@/types";

interface QuickSwitchProps {
  profiles: ProfileSummary[];
}

export function QuickSwitch({ profiles }: QuickSwitchProps) {
  const [open, setOpen] = useState(false);
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

  const handleSwitch = async (profile: ProfileSummary) => {
    try {
      const result = await activateProfile(profile.id);
      await fetchProfiles();
      addLog({
        action: "switch",
        detail: `Switched to ${result.profile_name}`,
        level: "info",
      });
    } catch (err) {
      addLog({
        action: "switch",
        detail: `Failed to switch: ${err}`,
        level: "error",
      });
    }
    setOpen(false);
  };

  if (profiles.length === 0) return null;

  return (
    <div className="relative" ref={ref}>
      <button
        onClick={() => setOpen(!open)}
        className="px-3 py-1.5 text-sm rounded-md bg-primary text-primary-foreground hover:bg-primary/90 transition-colors"
      >
        Switch
      </button>
      {open && (
        <div className="absolute right-0 top-full mt-1 w-56 rounded-md border bg-popover shadow-lg z-50">
          <div className="p-1">
            {profiles.map((p) => (
              <button
                key={p.id}
                onClick={() => handleSwitch(p)}
                className={`w-full text-left px-3 py-2 text-sm rounded-sm hover:bg-accent transition-colors flex items-center justify-between ${
                  activeProfile?.id === p.id ? "bg-accent" : ""
                }`}
              >
                <div>
                  <div className="font-medium">{p.name}</div>
                  <div className="text-xs text-muted-foreground">
                    {getProviderLabel(p.provider)} - {p.email}
                  </div>
                </div>
                {activeProfile?.id === p.id && (
                  <span className="text-primary text-xs">Active</span>
                )}
              </button>
            ))}
          </div>
        </div>
      )}
    </div>
  );
}
