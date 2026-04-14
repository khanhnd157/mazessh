import { useState } from "react";
import { CheckCircle, XCircle, Star } from "lucide-react";
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
  {
    type: "custom",
    label: "Custom",
    description: "User-defined Windows named pipe path",
  },
];

interface ProviderSelectorProps {
  selected: BridgeProvider;
  providerStatuses: ProviderStatus[];
  recommended?: BridgeProviderType | null;
  onChange: (provider: BridgeProvider) => void;
  disabled?: boolean;
}

export function ProviderSelector({ selected, providerStatuses, recommended, onChange, disabled }: ProviderSelectorProps) {
  const [customPipePath, setCustomPipePath] = useState(selected.pipe_path ?? "//./pipe/");

  const getStatus = (type: BridgeProviderType): ProviderStatus | undefined =>
    providerStatuses.find((p) => p.provider.type === type);

  const handleSelect = (type: BridgeProviderType) => {
    if (type === "custom") {
      onChange({ type: "custom", pipe_path: customPipePath });
    } else {
      onChange({ type });
    }
  };

  const handleCustomPipeChange = (value: string) => {
    setCustomPipePath(value);
    if (selected.type === "custom") {
      onChange({ type: "custom", pipe_path: value });
    }
  };

  return (
    <div className="space-y-1.5">
      <span className="text-[11px] font-medium text-muted-foreground">SSH Agent Provider</span>
      <div className="space-y-1">
        {PROVIDER_OPTIONS.map((opt) => {
          const status = getStatus(opt.type);
          // Custom provider doesn't appear in built-in statuses — always allow selection
          const isAvailable = opt.type === "custom" ? true : (status?.available ?? false);
          const isSelected = selected.type === opt.type;
          const isDisabled = disabled || (!isAvailable && opt.type !== "custom");
          const isRecommended = recommended === opt.type;

          return (
            <div key={opt.type}>
              <button
                type="button"
                onClick={() => handleSelect(opt.type)}
                disabled={isDisabled}
                className={`w-full flex items-center gap-2.5 px-3 py-2 rounded-lg text-left transition-colors ${
                  isSelected
                    ? "bg-primary/10 border border-primary/30"
                    : "bg-secondary/40 border border-transparent hover:bg-secondary/70"
                } ${isDisabled ? "opacity-40 cursor-not-allowed" : "cursor-pointer"}`}
              >
                {opt.type !== "custom" ? (
                  isAvailable ? (
                    <CheckCircle size={13} className="text-success shrink-0" />
                  ) : (
                    <XCircle size={13} className="text-destructive shrink-0" />
                  )
                ) : (
                  <div className="w-[13px] h-[13px] rounded-full border border-muted-foreground/40 shrink-0" />
                )}

                <div className="flex-1 min-w-0">
                  <div className="flex items-center gap-1.5">
                    <span className="text-xs font-medium">{opt.label}</span>
                    {isSelected && (
                      <span className="text-[9px] px-1.5 py-0.5 rounded-full bg-primary/15 text-primary font-medium">
                        Selected
                      </span>
                    )}
                    {isRecommended && (
                      <span className="text-[9px] px-1.5 py-0.5 rounded-full bg-warning/15 text-warning font-medium flex items-center gap-0.5">
                        <Star size={8} /> Recommended
                      </span>
                    )}
                  </div>
                  <p className="text-[10px] text-muted-foreground/70 truncate">{opt.description}</p>
                </div>

                {!isAvailable && opt.type !== "custom" && status?.error && (
                  <span className="text-[9px] text-destructive/70 max-w-[120px] truncate shrink-0" title={status.error}>
                    {status.error}
                  </span>
                )}
              </button>

              {/* Custom pipe path input */}
              {opt.type === "custom" && isSelected && (
                <div className="mt-1 ml-7">
                  <input
                    type="text"
                    value={customPipePath}
                    onChange={(e) => handleCustomPipeChange(e.target.value)}
                    placeholder="//./pipe/my-ssh-agent"
                    disabled={disabled}
                    className="w-full px-2.5 py-1.5 text-[11px] font-mono rounded-lg bg-secondary border border-border focus:outline-none focus:ring-1 focus:ring-ring placeholder:text-muted-foreground/30"
                  />
                  <p className="text-[9px] text-muted-foreground/50 mt-0.5">Windows named pipe path</p>
                </div>
              )}
            </div>
          );
        })}
      </div>
    </div>
  );
}
