import { Shield, ExternalLink } from "lucide-react";
import type { SecuritySettings, AgentMode, VaultUnlockMode } from "@/types";

interface Props {
  settings: SecuritySettings;
  onUpdate: (settings: SecuritySettings) => void;
}

export function AgentModeSettings({ settings, onUpdate }: Props) {
  const setAgentMode = (mode: AgentMode) => {
    onUpdate({ ...settings, agent_mode: mode });
  };

  const setVaultUnlockMode = (mode: VaultUnlockMode) => {
    onUpdate({ ...settings, vault_unlock_mode: mode });
  };

  return (
    <div className="space-y-4">
      <div>
        <h3 className="text-xs font-semibold uppercase tracking-wider text-muted-foreground/70 mb-3">
          SSH Agent Mode
        </h3>
        <div className="grid grid-cols-2 gap-3">
          <AgentModeCard
            icon={<Shield size={16} />}
            title="MazeSSH Agent"
            description="Vault-managed keys with per-key consent and host restrictions"
            selected={settings.agent_mode === "vault"}
            onClick={() => setAgentMode("vault")}
          />
          <AgentModeCard
            icon={<ExternalLink size={16} />}
            title="External Agent"
            description="Forward to Windows OpenSSH, Pageant, or 1Password agent"
            selected={settings.agent_mode === "file_system"}
            onClick={() => setAgentMode("file_system")}
          />
        </div>
      </div>

      {settings.agent_mode === "vault" && (
        <div className="pl-1">
          <h4 className="text-[11px] font-medium text-muted-foreground mb-2">Vault Unlock Mode</h4>
          <div className="space-y-2">
            <label className="flex items-start gap-2.5 cursor-pointer group">
              <input
                type="radio"
                name="vault-unlock-mode"
                checked={settings.vault_unlock_mode === "same_as_pin"}
                onChange={() => setVaultUnlockMode("same_as_pin")}
                className="mt-0.5 accent-primary"
              />
              <div>
                <div className="text-xs font-medium group-hover:text-foreground transition-colors">Shared PIN</div>
                <div className="text-[10px] text-muted-foreground/60">Vault unlocks automatically when you unlock the app</div>
              </div>
            </label>
            <label className="flex items-start gap-2.5 cursor-pointer group">
              <input
                type="radio"
                name="vault-unlock-mode"
                checked={settings.vault_unlock_mode === "separate_passphrase"}
                onChange={() => setVaultUnlockMode("separate_passphrase")}
                className="mt-0.5 accent-primary"
              />
              <div>
                <div className="text-xs font-medium group-hover:text-foreground transition-colors">Separate Passphrase</div>
                <div className="text-[10px] text-muted-foreground/60">Vault requires its own passphrase, independent of the app PIN</div>
              </div>
            </label>
          </div>
        </div>
      )}
    </div>
  );
}

function AgentModeCard({
  icon,
  title,
  description,
  selected,
  onClick,
}: {
  icon: React.ReactNode;
  title: string;
  description: string;
  selected: boolean;
  onClick: () => void;
}) {
  return (
    <button
      type="button"
      onClick={onClick}
      className={`text-left p-3 rounded-lg border transition-all ${
        selected
          ? "bg-primary/8 border-primary/25 ring-1 ring-primary/20"
          : "bg-secondary/50 border-border hover:bg-accent/50"
      }`}
    >
      <div className={`mb-1.5 ${selected ? "text-primary" : "text-muted-foreground"}`}>
        {icon}
      </div>
      <div className="text-xs font-medium">{title}</div>
      <div className="text-[10px] text-muted-foreground/60 mt-0.5 leading-relaxed">{description}</div>
    </button>
  );
}
