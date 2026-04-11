import { Check } from "lucide-react";
import type { ProfileSummary } from "@/types";
import { getProviderLabel } from "@/types";
import { ProviderIcon } from "./ProviderIcon";

interface ProfileCardProps {
  profile: ProfileSummary;
  isSelected: boolean;
  onClick: () => void;
}

export function ProfileCard({ profile, isSelected, onClick }: ProfileCardProps) {
  return (
    <button
      type="button"
      onClick={onClick}
      className={`group w-full text-left px-3 py-2.5 rounded-lg transition-all ${
        isSelected
          ? "bg-primary/10 ring-1 ring-primary/25"
          : "hover:bg-accent/60"
      }`}
    >
      <div className="flex items-center gap-2.5">
        <div className="relative shrink-0">
          <ProviderIcon provider={profile.provider} size={18} />
          {profile.is_active && (
            <div className="absolute -bottom-0.5 -right-0.5 w-2.5 h-2.5 rounded-full bg-success ring-2 ring-background" />
          )}
        </div>
        <div className="min-w-0 flex-1">
          <div className="flex items-center gap-1.5">
            <span className="font-medium text-[13px] truncate">{profile.name}</span>
            {profile.is_active && (
              <Check size={12} className="text-success shrink-0" />
            )}
          </div>
          <div className="text-[11px] text-muted-foreground truncate mt-0.5">
            {getProviderLabel(profile.provider)} · {profile.email}
          </div>
        </div>
      </div>
    </button>
  );
}
