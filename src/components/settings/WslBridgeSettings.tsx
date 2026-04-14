import { useState, useEffect } from "react";
import {
  Monitor,
  Link,
  Play,
  Square,
  RefreshCw,
  Trash2,
  CheckCircle,
  XCircle,
  AlertCircle,
  Loader2,
  ExternalLink,
} from "lucide-react";
import { toast } from "sonner";
import { useBridgeStore } from "@/stores/bridgeStore";
import type { BridgeProvider, DistroBridgeStatus, ProviderStatus } from "@/types";
import { ConfirmDialog } from "@/components/ui/ConfirmDialog";
import { ProviderSelector } from "./ProviderSelector";

const PROVIDER_LABELS: Record<string, string> = {
  "windows-open-ssh": "OpenSSH",
  "one-password": "1Password",
  pageant: "Pageant",
};

function providerLabel(provider: BridgeProvider): string {
  return PROVIDER_LABELS[provider.type] ?? provider.type;
}

export function WslBridgePanel() {
  const {
    overview,
    providers,
    loading,
    fetchOverview,
    bootstrapDistro,
    teardownDistro,
    startRelay,
    stopRelay,
    restartRelay,
    setDistroProvider,
  } = useBridgeStore();
  const [actionLoading, setActionLoading] = useState<string | null>(null);
  const [teardownTarget, setTeardownTarget] = useState<string | null>(null);

  useEffect(() => {
    fetchOverview();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const handleAction = async (label: string, action: () => Promise<void>) => {
    setActionLoading(label);
    try {
      await action();
      toast.success(label);
    } catch (err) {
      toast.error(label + " failed", { description: String(err) });
    } finally {
      setActionLoading(null);
    }
  };

  const handleBootstrap = (distro: string) =>
    handleAction(`Setup bridge for ${distro}`, () => bootstrapDistro(distro).then(() => {}));

  const handleTeardown = (distro: string) =>
    handleAction(`Removed bridge from ${distro}`, () => teardownDistro(distro));

  return (
    <div className="space-y-6 max-w-2xl">
      {/* Header */}
      <div>
        <h2 className="text-base font-semibold flex items-center gap-2">
          <Monitor size={16} />
          WSL Agent Bridge
        </h2>
        <p className="text-xs text-muted-foreground mt-1">
          Bridge an SSH agent from Windows into WSL2 distros. Keys stay on Windows while WSL tools use them seamlessly.
        </p>
      </div>

      {/* Prerequisites */}
      <PrerequisitesCard
        overview={overview}
        providers={providers}
        loading={loading}
        onRefresh={() => fetchOverview()}
      />

      {/* Distro list */}
      {overview?.wsl_available && (
        <>
          {overview.distros.length === 0 ? (
            <div className="rounded-xl border bg-card p-4">
              <p className="text-xs text-muted-foreground text-center py-4">
                No WSL2 distributions detected. Install one with{" "}
                <code className="px-1 py-0.5 rounded bg-secondary text-[10px]">wsl --install</code>
              </p>
            </div>
          ) : (
            overview.distros.map((distro) => (
              <DistroCard
                key={distro.distro_name}
                distro={distro}
                providers={providers}
                actionLoading={actionLoading}
                onBootstrap={() => handleBootstrap(distro.distro_name)}
                onStart={() =>
                  handleAction(`Started relay in ${distro.distro_name}`, () => startRelay(distro.distro_name))
                }
                onStop={() =>
                  handleAction(`Stopped relay in ${distro.distro_name}`, () => stopRelay(distro.distro_name))
                }
                onRestart={() =>
                  handleAction(`Restarted relay in ${distro.distro_name}`, () => restartRelay(distro.distro_name))
                }
                onTeardown={() => setTeardownTarget(distro.distro_name)}
                onProviderChange={(provider) =>
                  handleAction(`Changed provider for ${distro.distro_name}`, () =>
                    setDistroProvider(distro.distro_name, provider),
                  )
                }
                onRefresh={() => fetchOverview()}
              />
            ))
          )}
        </>
      )}

      {/* Teardown confirm */}
      <ConfirmDialog
        open={teardownTarget !== null}
        title={`Remove bridge from ${teardownTarget}?`}
        description="This will stop the relay service, remove installed files, and clear SSH_AUTH_SOCK from shell profiles."
        variant="danger"
        confirmLabel="Remove"
        onConfirm={() => {
          if (teardownTarget) handleTeardown(teardownTarget);
          setTeardownTarget(null);
        }}
        onCancel={() => setTeardownTarget(null)}
      />
    </div>
  );
}

// ── Prerequisites card ──

function PrerequisitesCard({
  overview,
  providers,
  loading,
  onRefresh,
}: {
  overview: ReturnType<typeof useBridgeStore>["overview"];
  providers: ProviderStatus[];
  loading: boolean;
  onRefresh: () => void;
}) {
  return (
    <div className="rounded-xl border bg-card p-4 space-y-3">
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-2">
          <Link size={14} className="text-muted-foreground" />
          <span className="text-sm font-medium">Prerequisites</span>
        </div>
        <button
          type="button"
          onClick={onRefresh}
          disabled={loading}
          className="px-3 py-1.5 text-xs font-medium rounded-lg bg-secondary hover:bg-accent disabled:opacity-50 flex items-center gap-1.5"
        >
          {loading ? <Loader2 size={12} className="animate-spin" /> : <RefreshCw size={12} />}
          Refresh
        </button>
      </div>

      {!overview ? (
        <div className="flex items-center justify-center py-4">
          <Loader2 size={16} className="animate-spin text-muted-foreground" />
        </div>
      ) : (
        <div className="space-y-3">
          {/* WSL availability */}
          <StatusRow label="WSL available" ok={overview.wsl_available} />

          {/* Provider statuses */}
          <div className="space-y-1">
            <span className="text-[10px] font-medium text-muted-foreground uppercase tracking-wider">
              Agent Providers
            </span>
            {providers.map((p) => (
              <StatusRow
                key={p.display_name}
                label={p.display_name}
                ok={p.available}
                hint={!p.available ? p.error ?? undefined : undefined}
              />
            ))}
          </div>

          {/* Relay binaries */}
          {overview.relay_binaries.length > 0 && (
            <div className="space-y-1">
              <span className="text-[10px] font-medium text-muted-foreground uppercase tracking-wider">
                Relay Binaries
              </span>
              {overview.relay_binaries.map((b) => (
                <StatusRow
                  key={b.binary}
                  label={b.binary === "Npiperelay" ? "npiperelay.exe" : "wsl-ssh-pageant.exe"}
                  ok={b.installed}
                  hint={!b.installed ? `Place at ${b.path}` : undefined}
                />
              ))}
            </div>
          )}
        </div>
      )}
    </div>
  );
}

function StatusRow({ label, ok, hint }: { label: string; ok: boolean; hint?: string }) {
  return (
    <div className="flex items-center gap-2 text-xs">
      {ok ? (
        <CheckCircle size={13} className="text-success shrink-0" />
      ) : (
        <XCircle size={13} className="text-destructive shrink-0" />
      )}
      <span className={ok ? "text-foreground" : "text-muted-foreground"}>{label}</span>
      {hint && <span className="text-[10px] text-muted-foreground/60 ml-1 truncate max-w-[280px]">{hint}</span>}
    </div>
  );
}

// ── Per-distro card ──

function DistroCard({
  distro,
  providers,
  actionLoading,
  onBootstrap,
  onStart,
  onStop,
  onRestart,
  onTeardown,
  onProviderChange,
  onRefresh,
}: {
  distro: DistroBridgeStatus;
  providers: ProviderStatus[];
  actionLoading: string | null;
  onBootstrap: () => void;
  onStart: () => void;
  onStop: () => void;
  onRestart: () => void;
  onTeardown: () => void;
  onProviderChange: (provider: BridgeProvider) => void;
  onRefresh: () => void;
}) {
  const isActionRunning = actionLoading !== null;

  return (
    <div className="rounded-xl border bg-card p-4 space-y-3">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-2">
          <Monitor size={14} className="text-muted-foreground" />
          <span className="text-sm font-medium">{distro.distro_name}</span>
          <span className="text-[10px] px-1.5 py-0.5 rounded-full font-medium bg-primary/10 text-primary">
            WSL{distro.wsl_version}
          </span>
          <span
            className={`text-[10px] px-1.5 py-0.5 rounded-full font-medium ${
              distro.distro_running ? "bg-success/15 text-success" : "bg-muted text-muted-foreground"
            }`}
          >
            {distro.distro_running ? "Running" : "Stopped"}
          </span>
          {/* Provider badge (when bridge is installed) */}
          {distro.relay_installed && (
            <span className="text-[10px] px-1.5 py-0.5 rounded-full font-medium bg-secondary text-muted-foreground">
              {providerLabel(distro.provider)}
            </span>
          )}
        </div>
        <button
          type="button"
          onClick={onRefresh}
          className="text-muted-foreground/60 hover:text-muted-foreground transition-colors"
          title="Refresh status"
        >
          <RefreshCw size={12} />
        </button>
      </div>

      {/* Not running */}
      {!distro.distro_running && (
        <p className="text-xs text-muted-foreground">Distro is stopped. Start it to configure the bridge.</p>
      )}

      {/* Running but not bootstrapped */}
      {distro.distro_running && !distro.relay_installed && (
        <div className="space-y-3">
          {/* Provider selector */}
          <ProviderSelector
            selected={distro.provider}
            providerStatuses={providers}
            onChange={onProviderChange}
            disabled={isActionRunning}
          />

          {/* Prerequisite checks (contextual) */}
          <div className="space-y-1">
            {distro.provider.type !== "pageant" && (
              <StatusRow
                label="socat installed"
                ok={distro.socat_installed}
                hint={!distro.socat_installed ? "sudo apt install socat" : undefined}
              />
            )}
            <StatusRow
              label="systemd available"
              ok={distro.systemd_available}
              hint={!distro.systemd_available ? "Add systemd=true to /etc/wsl.conf" : undefined}
            />
          </div>

          <button
            type="button"
            onClick={onBootstrap}
            disabled={
              isActionRunning ||
              (distro.provider.type !== "pageant" && !distro.socat_installed) ||
              !distro.systemd_available
            }
            className="px-3 py-1.5 text-xs font-medium rounded-lg bg-primary text-primary-foreground hover:bg-primary/90 disabled:opacity-50 flex items-center gap-1.5"
          >
            {isActionRunning ? <Loader2 size={12} className="animate-spin" /> : <ExternalLink size={12} />}
            Setup Bridge
          </button>
        </div>
      )}

      {/* Bootstrapped — show full status */}
      {distro.distro_running && distro.relay_installed && (
        <div className="space-y-3">
          {/* Status indicators */}
          <div className="space-y-1">
            <StatusRow label="Relay service" ok={distro.service_active} />
            <StatusRow label="Socket exists" ok={distro.socket_exists} />
            <StatusRow label="Agent reachable" ok={distro.agent_reachable} />
          </div>

          {/* Error display */}
          {distro.error && (
            <div className="flex items-start gap-1.5 text-xs text-warning">
              <AlertCircle size={12} className="mt-0.5 shrink-0" />
              {distro.error}
            </div>
          )}

          {/* Actions */}
          <div className="flex items-center gap-2">
            {distro.service_active ? (
              <>
                <button
                  type="button"
                  onClick={onStop}
                  disabled={isActionRunning}
                  className="px-2.5 py-1 text-[11px] font-medium rounded-lg bg-secondary hover:bg-accent disabled:opacity-50 flex items-center gap-1"
                >
                  <Square size={10} /> Stop
                </button>
                <button
                  type="button"
                  onClick={onRestart}
                  disabled={isActionRunning}
                  className="px-2.5 py-1 text-[11px] font-medium rounded-lg bg-secondary hover:bg-accent disabled:opacity-50 flex items-center gap-1"
                >
                  <RefreshCw size={10} /> Restart
                </button>
              </>
            ) : (
              <button
                type="button"
                onClick={onStart}
                disabled={isActionRunning}
                className="px-2.5 py-1 text-[11px] font-medium rounded-lg bg-primary text-primary-foreground hover:bg-primary/90 disabled:opacity-50 flex items-center gap-1"
              >
                <Play size={10} /> Start
              </button>
            )}

            <div className="flex-1" />

            <button
              type="button"
              onClick={onTeardown}
              disabled={isActionRunning}
              className="px-2.5 py-1 text-[11px] font-medium rounded-lg text-destructive hover:bg-destructive/10 disabled:opacity-50 flex items-center gap-1"
            >
              <Trash2 size={10} /> Remove
            </button>
          </div>
        </div>
      )}
    </div>
  );
}
