import { KeyRound } from "lucide-react";
import { useProfileStore } from "@/stores/profileStore";
import { ProfileDetail } from "@/components/profiles/ProfileDetail";

export function MainPanel() {
  const selectedProfile = useProfileStore((s) => s.selectedProfile);

  if (!selectedProfile) {
    return (
      <div className="h-full flex items-center justify-center">
        <div className="text-center">
          <div className="w-12 h-12 rounded-2xl bg-primary/6 flex items-center justify-center mx-auto mb-3">
            <KeyRound size={20} className="text-primary/30" />
          </div>
          <p className="text-sm text-muted-foreground/50">Select a profile</p>
        </div>
      </div>
    );
  }

  return (
    <div className="h-full min-h-0 overflow-y-auto p-6">
      <ProfileDetail profile={selectedProfile} />
    </div>
  );
}
