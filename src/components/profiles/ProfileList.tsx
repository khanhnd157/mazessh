import { useCallback, useState } from "react";
import { Plus, KeyRound } from "lucide-react";
import { useProfileStore } from "@/stores/profileStore";
import { useUiStore } from "@/stores/uiStore";
import { ProfileCard } from "./ProfileCard";
import { ProfileForm } from "./ProfileForm";

export function ProfileList() {
  const { profiles, selectedProfileId, selectProfile, loading } = useProfileStore();
  const [showForm, setShowForm] = useState(false);

  const handleSelect = useCallback((id: string) => {
    selectProfile(id);
    useUiStore.getState().setActiveTab("profiles");
  }, [selectProfile]);

  return (
    <div className="flex flex-col h-full">
      <div className="flex items-center justify-between px-3 h-10 shrink-0 border-b">
        <h2 className="text-[10.5px] font-semibold text-muted-foreground/70 uppercase tracking-wider">
          Profiles
          {profiles.length > 0 && (
            <span className="ml-1 opacity-50">{profiles.length}</span>
          )}
        </h2>
        <button
          type="button"
          onClick={() => setShowForm(true)}
          title="New profile"
          className="flex items-center gap-1 px-1.5 py-0.5 text-[10.5px] rounded-md bg-primary/12 text-primary hover:bg-primary/20 font-medium transition-colors"
        >
          <Plus size={11} />
          New
        </button>
      </div>
      <div className="flex-1 overflow-y-auto p-1.5 space-y-px">
        {loading && profiles.length === 0 && (
          <div className="flex items-center justify-center py-10 text-muted-foreground/50">
            <span className="text-xs">Loading...</span>
          </div>
        )}
        {!loading && profiles.length === 0 && (
          <div className="text-center py-10 px-4">
            <div className="w-9 h-9 rounded-xl bg-primary/8 flex items-center justify-center mx-auto mb-2.5">
              <KeyRound size={16} className="text-primary/40" />
            </div>
            <p className="text-xs text-muted-foreground/60">No profiles yet</p>
            <button
              type="button"
              onClick={() => setShowForm(true)}
              className="mt-1.5 text-[11px] text-primary hover:underline"
            >
              Create your first profile
            </button>
          </div>
        )}
        {profiles.map((profile) => (
          <ProfileCard
            key={profile.id}
            profile={profile}
            isSelected={selectedProfileId === profile.id}
            onSelect={handleSelect}
          />
        ))}
      </div>
      {showForm && <ProfileForm onClose={() => setShowForm(false)} />}
    </div>
  );
}
