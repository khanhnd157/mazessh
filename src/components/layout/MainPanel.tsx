import { useProfileStore } from "@/stores/profileStore";
import { ProfileDetail } from "@/components/profiles/ProfileDetail";

export function MainPanel() {
  const { selectedProfile } = useProfileStore();

  if (!selectedProfile) {
    return (
      <div className="flex-1 flex items-center justify-center text-muted-foreground">
        <div className="text-center">
          <div className="text-4xl mb-3">🔑</div>
          <p className="text-lg">Select a profile to view details</p>
          <p className="text-sm mt-1">Or create a new one to get started</p>
        </div>
      </div>
    );
  }

  return (
    <div className="flex-1 overflow-y-auto p-6">
      <ProfileDetail profile={selectedProfile} />
    </div>
  );
}
