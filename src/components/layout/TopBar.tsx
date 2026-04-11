import { Power } from "lucide-react";
import { useAppStore } from "@/stores/appStore";
import { useProfileStore } from "@/stores/profileStore";
import { useLogStore } from "@/stores/logStore";
import { QuickSwitch } from "@/components/switch/QuickSwitch";
import { toast } from "sonner";

export function TopBar() {
  const { activeProfile, deactivateProfile } = useAppStore();
  const { profiles, fetchProfiles } = useProfileStore();
  const { addLog } = useLogStore();

  const handleDeactivate = async () => {
    await deactivateProfile();
    await fetchProfiles();
    addLog({ action: "deactivate", detail: "Profile deactivated", level: "info" });
    toast.info("Profile deactivated");
  };

  return (
    <div className="flex items-center justify-end px-4 py-2 border-b bg-card/30">
      <div className="flex items-center gap-2">
        <QuickSwitch profiles={profiles} />
        {activeProfile && (
          <button
            type="button"
            onClick={handleDeactivate}
            className="flex items-center gap-1.5 px-2.5 py-1.5 text-xs rounded-md text-muted-foreground hover:text-foreground hover:bg-secondary transition-colors"
          >
            <Power size={12} />
            Deactivate
          </button>
        )}
      </div>
    </div>
  );
}
