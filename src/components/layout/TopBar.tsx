import { useAppStore } from "@/stores/appStore";
import { useProfileStore } from "@/stores/profileStore";
import { ActiveBadge } from "@/components/switch/ActiveBadge";
import { QuickSwitch } from "@/components/switch/QuickSwitch";

export function TopBar() {
  const { activeProfile, deactivateProfile } = useAppStore();
  const { profiles } = useProfileStore();

  return (
    <div className="flex items-center justify-between px-4 py-3 border-b bg-card">
      <div className="flex items-center gap-3">
        <h1 className="text-lg font-bold tracking-tight">Maze SSH</h1>
        <ActiveBadge profile={activeProfile} />
      </div>
      <div className="flex items-center gap-2">
        <QuickSwitch profiles={profiles} />
        {activeProfile && (
          <button
            onClick={deactivateProfile}
            className="px-3 py-1.5 text-sm rounded-md bg-secondary hover:bg-secondary/80 transition-colors"
          >
            Deactivate
          </button>
        )}
      </div>
    </div>
  );
}
