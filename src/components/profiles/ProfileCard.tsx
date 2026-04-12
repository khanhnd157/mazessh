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
      className={`group w-full text-left px-2.5 py-2 rounded-lg transition-all duration-100 ${
        isSelected
          ? "bg-primary/12 ring-1 ring-primary/20"
          : "hover:bg-accent/50 active:bg-accent/70"
      }`}
    >
      <div className="flex items-center gap-2">
        <div className="relative shrink-0">
          <ProviderIcon provider={profile.provider} size={16} />
          {profile.is_active && (
            <div className="absolute -bottom-0.5 -right-0.5 w-2 h-2 rounded-full bg-success ring-[1.5px] ring-background" />
          )}
        </div>
        <div className="min-w-0 flex-1">
          <div className="flex items-center gap-1">
            <span className="font-medium text-[12.5px] truncate leading-tight">{profile.name}</span>
            {profile.is_active && <Check size={11} className="text-success shrink-0" />}
          </div>
          <div className="text-[10.5px] text-muted-foreground/70 truncate leading-tight mt-px">
            {getProviderLabel(profile.provider)} · {profile.email}
          </div>
        </div>
      </div>
    </button>
  );
}
