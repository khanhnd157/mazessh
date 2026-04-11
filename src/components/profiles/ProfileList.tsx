import { Plus, ScanSearch } from "lucide-react";
import { useProfileStore } from "@/stores/profileStore";
import { ProfileCard } from "./ProfileCard";
import { useState } from "react";
import { ProfileForm } from "./ProfileForm";

export function ProfileList() {
  const { profiles, selectedProfileId, selectProfile, loading } = useProfileStore();
  const [showForm, setShowForm] = useState(false);

  return (
    <div className="flex flex-col h-full">
      <div className="flex items-center justify-between px-4 h-10 shrink-0 border-b">
        <h2 className="text-[11px] font-semibold text-muted-foreground uppercase tracking-wider">
          Profiles
          {profiles.length > 0 && (
            <span className="ml-1.5 text-muted-foreground/50">{profiles.length}</span>
          )}
        </h2>
        <button
          type="button"
          onClick={() => setShowForm(true)}
          className="flex items-center gap-1 px-2 py-1 text-[11px] rounded-md bg-primary/15 text-primary hover:bg-primary/25 font-medium transition-colors"
        >
          <Plus size={12} />
          New
        </button>
      </div>
      <div className="flex-1 overflow-y-auto p-2 space-y-0.5">
        {loading && profiles.length === 0 && (
          <div className="flex items-center justify-center py-8 text-muted-foreground">
            <ScanSearch size={16} className="animate-spin mr-2" />
            <span className="text-sm">Loading...</span>
          </div>
        )}
        {!loading && profiles.length === 0 && (
          <div className="text-center py-8">
            <div className="w-10 h-10 rounded-full bg-primary/10 flex items-center justify-center mx-auto mb-3">
              <Plus size={18} className="text-primary" />
            </div>
            <p className="text-sm text-muted-foreground">No profiles yet</p>
            <button
              type="button"
              onClick={() => setShowForm(true)}
              className="mt-2 text-xs text-primary hover:underline"
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
            onClick={() => selectProfile(profile.id)}
          />
        ))}
      </div>
      {showForm && <ProfileForm onClose={() => setShowForm(false)} />}
    </div>
  );
}
