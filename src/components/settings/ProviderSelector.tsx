import { CheckCircle, XCircle } from "lucide-react";
import type { BridgeProvider, BridgeProviderType, ProviderStatus } from "@/types";

const PROVIDER_OPTIONS: { type: BridgeProviderType; label: string; description: string }[] = [
  {
    type: "windows-open-ssh",
    label: "Windows OpenSSH",
    description: "Built-in Windows SSH agent via named pipe",
  },
  {
    type: "one-password",
    label: "1Password",
    description: "1Password SSH agent for key management",
  },
  {
    type: "pageant",
    label: "Pageant",
    description: "PuTTY / KeeAgent / GPG4Win compatible",
  },
];

interface ProviderSelectorProps {
  selected: BridgeProvider;
  providerStatuses: ProviderStatus[];
  onChange: (provider: BridgeProvider) => void;
  disabled?: boolean;
}

export function ProviderSelector({ selected, providerStatuses, onChange, disabled }: ProviderSelectorProps) {
  const getStatus = (type: BridgeProviderType): ProviderStatus | undefined =>
    providerStatuses.find((p) => p.provider.type === type);

  return (
    <div className="space-y-1.5">
      <span className="text-[11px] font-medium text-muted-foreground">SSH Agent Provider</span>
      <div className="space-y-1">
        {PROVIDER_OPTIONS.map((opt) => {
          const status = getStatus(opt.type);
          const isAvailable = status?.available ?? false;
          const isSelected = selected.type === opt.type;
          const isDisabled = disabled || !isAvailable;

          return (
            <button
              key={opt.type}
              type="button"
              onClick={() => onChange({ type: opt.type })}
              disabled={isDisabled}
              className={`w-full flex items-center gap-2.5 px-3 py-2 rounded-lg text-left transition-colors ${
                isSelected
                  ? "bg-primary/10 border border-primary/30"
                  : "bg-secondary/40 border border-transparent hover:bg-secondary/70"
              } ${isDisabled ? "opacity-40 cursor-not-allowed" : "cursor-pointer"}`}
            >
              {/* Status dot */}
              {isAvailable ? (
                <CheckCircle size={13} className="text-success shrink-0" />
              ) : (
                <XCircle size={13} className="text-destructive shrink-0" />
              )}

              <div className="flex-1 min-w-0">
                <div className="flex items-center gap-1.5">
                  <span className="text-xs font-medium">{opt.label}</span>
                  {isSelected && (
                    <span className="text-[9px] px-1.5 py-0.5 rounded-full bg-primary/15 text-primary font-medium">
                      Selected
                    </span>
                  )}
                </div>
                <p className="text-[10px] text-muted-foreground/70 truncate">{opt.description}</p>
              </div>

              {/* Error hint */}
              {!isAvailable && status?.error && (
                <span className="text-[9px] text-destructive/70 max-w-[120px] truncate shrink-0" title={status.error}>
                  {status.error}
                </span>
              )}
            </button>
          );
        })}
      </div>
    </div>
  );
}
