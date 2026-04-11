import type { ProfileSummary } from "@/types";
import { getProviderLabel } from "@/types";

interface ActiveBadgeProps {
  profile: ProfileSummary | null;
}

export function ActiveBadge({ profile }: ActiveBadgeProps) {
  if (!profile) {
    return (
      <span className="px-2.5 py-1 text-xs rounded-full bg-secondary text-muted-foreground">
        No active profile
      </span>
    );
  }

  return (
    <span className="px-2.5 py-1 text-xs rounded-full bg-primary/20 text-primary font-medium">
      {profile.name} ({getProviderLabel(profile.provider)})
    </span>
  );
}
