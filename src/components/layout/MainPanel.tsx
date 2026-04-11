import { KeyRound, ArrowRight } from "lucide-react";
import { useProfileStore } from "@/stores/profileStore";
import { ProfileDetail } from "@/components/profiles/ProfileDetail";

export function MainPanel() {
  const { selectedProfile } = useProfileStore();

  if (!selectedProfile) {
    return (
      <div className="h-full flex items-center justify-center text-muted-foreground">
        <div className="text-center">
          <div className="w-14 h-14 rounded-2xl bg-primary/8 flex items-center justify-center mx-auto mb-4">
            <KeyRound size={24} className="text-primary/50" />
          </div>
          <p className="text-sm font-medium text-foreground/60">No profile selected</p>
          <p className="text-xs text-muted-foreground mt-1 flex items-center justify-center gap-1">
            Select a profile from the sidebar <ArrowRight size={12} />
          </p>
        </div>
      </div>
    );
  }

  return (
    <div className="h-full overflow-y-auto p-6">
      <ProfileDetail profile={selectedProfile} />
    </div>
  );
}
