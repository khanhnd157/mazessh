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
import type { DistroBridgeStatus } from "@/types";
import { ConfirmDialog } from "@/components/ui/ConfirmDialog";

export function WslBridgePanel() {
  const { overview, loading, fetchOverview, bootstrapDistro, teardownDistro, startRelay, stopRelay, restartRelay } =
    useBridgeStore();
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
    handleAction(`Setup bridge for ${distro}`, async () => {
      await bootstrapDistro(distro);
    });

  const handleTeardown = (distro: string) =>
    handleAction(`Removed bridge from ${distro}`, async () => {
      await teardownDistro(distro);
    });

  return (
    <div className="space-y-6 max-w-2xl">
      {/* Header */}
      <div>
        <h2 className="text-base font-semibold flex items-center gap-2">
          <Monitor size={16} />
          WSL Agent Bridge
        </h2>
        <p className="text-xs text-muted-foreground mt-1">
          Bridge the Windows SSH agent into WSL2 distros so your keys stay on Windows while WSL tools use them
          seamlessly.
        </p>
      </div>

      {/* Prerequisites */}
      <PrerequisitesCard
        overview={overview}
        loading={loading}
        onRefresh={() => fetchOverview()}
      />

      {/* Distro list */}
      {overview?.wsl_available && (
        <>
          {overview.distros.length === 0 ? (
            <div className="rounded-xl border bg-card p-4">
              <p className="text-xs text-muted-foreground text-center py-4">
                No WSL2 distributions detected. Install one with <code className="px-1 py-0.5 rounded bg-secondary text-[10px]">wsl --install</code>
              </p>
            </div>
          ) : (
            overview.distros.map((distro) => (
              <DistroCard
                key={distro.distro_name}
                distro={distro}
                actionLoading={actionLoading}
                onBootstrap={() => handleBootstrap(distro.distro_name)}
                onStart={() => handleAction(`Started relay in ${distro.distro_name}`, () => startRelay(distro.distro_name))}
                onStop={() => handleAction(`Stopped relay in ${distro.distro_name}`, () => stopRelay(distro.distro_name))}
                onRestart={() => handleAction(`Restarted relay in ${distro.distro_name}`, () => restartRelay(distro.distro_name))}
                onTeardown={() => setTeardownTarget(distro.distro_name)}
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
  loading,
  onRefresh,
}: {
  overview: ReturnType<typeof useBridgeStore>["overview"];
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
        <div className="space-y-1.5">
          <StatusRow label="WSL available" ok={overview.wsl_available} />
          <StatusRow label="Windows SSH Agent running" ok={overview.windows_agent_running} />
          <StatusRow
            label="npiperelay.exe installed"
            ok={overview.npiperelay_installed}
            hint={
              !overview.npiperelay_installed
                ? "Place npiperelay.exe at ~/.maze-ssh/bin/"
                : undefined
            }
          />
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
      {hint && (
        <span className="text-[10px] text-muted-foreground/60 ml-1">{hint}</span>
      )}
    </div>
  );
}

// ── Per-distro card ──

function DistroCard({
  distro,
  actionLoading,
  onBootstrap,
  onStart,
  onStop,
  onRestart,
  onTeardown,
  onRefresh,
}: {
  distro: DistroBridgeStatus;
  actionLoading: string | null;
  onBootstrap: () => void;
  onStart: () => void;
  onStop: () => void;
  onRestart: () => void;
  onTeardown: () => void;
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
              distro.distro_running
                ? "bg-success/15 text-success"
                : "bg-muted text-muted-foreground"
            }`}
          >
            {distro.distro_running ? "Running" : "Stopped"}
          </span>
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
        <p className="text-xs text-muted-foreground">
          Distro is stopped. Start it to configure the bridge.
        </p>
      )}

      {/* Running but not bootstrapped */}
      {distro.distro_running && !distro.relay_installed && (
        <div className="space-y-2">
          {/* Prerequisite checks */}
          <div className="space-y-1">
            <StatusRow label="socat installed" ok={distro.socat_installed} hint={!distro.socat_installed ? "sudo apt install socat" : undefined} />
            <StatusRow label="systemd available" ok={distro.systemd_available} hint={!distro.systemd_available ? "Add systemd=true to /etc/wsl.conf" : undefined} />
          </div>

          <button
            type="button"
            onClick={onBootstrap}
            disabled={isActionRunning || !distro.socat_installed || !distro.systemd_available}
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
