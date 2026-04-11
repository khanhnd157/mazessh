import { Circle } from "lucide-react";
import type { ProfileSummary } from "@/types";
import { getProviderLabel } from "@/types";

interface ActiveBadgeProps {
  profile: ProfileSummary | null;
}

export function ActiveBadge({ profile }: ActiveBadgeProps) {
  if (!profile) {
    return (
      <span className="flex items-center gap-1.5 px-2.5 py-1 text-xs rounded-md bg-secondary text-muted-foreground">
        <Circle size={6} className="fill-muted-foreground" />
        No active profile
      </span>
    );
  }

  return (
    <span className="flex items-center gap-1.5 px-2.5 py-1 text-xs rounded-md bg-primary/15 text-primary font-medium">
      <Circle size={6} className="fill-success text-success animate-pulse" />
      {profile.name}
      <span className="text-primary/60">({getProviderLabel(profile.provider)})</span>
    </span>
  );
}
