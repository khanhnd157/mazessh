import type { ProfileSummary } from "@/types";
import { getProviderLabel } from "@/types";

interface ProfileCardProps {
  profile: ProfileSummary;
  isSelected: boolean;
  onClick: () => void;
}

const providerColors: Record<string, string> = {
  github: "bg-gray-700",
  gitlab: "bg-orange-700",
  gitea: "bg-green-700",
  bitbucket: "bg-blue-700",
};

export function ProfileCard({ profile, isSelected, onClick }: ProfileCardProps) {
  const providerKey = typeof profile.provider === "string" ? profile.provider : "custom";
  const colorClass = providerColors[providerKey] || "bg-purple-700";

  return (
    <button
      onClick={onClick}
      className={`w-full text-left px-3 py-2.5 rounded-md transition-colors ${
        isSelected
          ? "bg-accent border border-primary/30"
          : "hover:bg-accent/50"
      }`}
    >
      <div className="flex items-center gap-2.5">
        <div className={`w-2 h-2 rounded-full ${profile.is_active ? "bg-green-500" : colorClass}`} />
        <div className="min-w-0 flex-1">
          <div className="font-medium text-sm truncate">{profile.name}</div>
          <div className="text-xs text-muted-foreground truncate">
            {getProviderLabel(profile.provider)} · {profile.email}
          </div>
        </div>
        {profile.is_active && (
          <span className="text-[10px] px-1.5 py-0.5 rounded bg-green-500/20 text-green-400 font-medium shrink-0">
            ACTIVE
          </span>
        )}
      </div>
    </button>
  );
}
