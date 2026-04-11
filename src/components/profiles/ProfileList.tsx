import { useProfileStore } from "@/stores/profileStore";
import { ProfileCard } from "./ProfileCard";
import { useState } from "react";
import { ProfileForm } from "./ProfileForm";

export function ProfileList() {
  const { profiles, selectedProfileId, selectProfile, loading } = useProfileStore();
  const [showForm, setShowForm] = useState(false);

  return (
    <div className="flex flex-col h-full">
      <div className="flex items-center justify-between px-4 py-3 border-b">
        <h2 className="text-sm font-semibold text-muted-foreground uppercase tracking-wider">
          Profiles
        </h2>
        <button
          onClick={() => setShowForm(true)}
          className="px-2 py-1 text-xs rounded bg-primary text-primary-foreground hover:bg-primary/90 transition-colors"
        >
          + New
        </button>
      </div>
      <div className="flex-1 overflow-y-auto p-2 space-y-1">
        {loading && profiles.length === 0 && (
          <p className="text-sm text-muted-foreground p-2">Loading...</p>
        )}
        {!loading && profiles.length === 0 && (
          <p className="text-sm text-muted-foreground p-2">
            No profiles yet. Create one to get started.
          </p>
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
