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
  AlertTriangle,
  Loader2,
  ExternalLink,
  Download,
  Stethoscope,
  FileText,
  ChevronDown,
  ChevronUp,
} from "lucide-react";
import { toast } from "sonner";
import { commands } from "@/lib/tauri-commands";
import { useBridgeStore } from "@/stores/bridgeStore";
import type { BridgeProvider, DistroBridgeStatus, ProviderStatus, RelayMode } from "@/types";
import { ConfirmDialog } from "@/components/ui/ConfirmDialog";
import { ProviderSelector } from "./ProviderSelector";
import { DiagnosticsPanel } from "./DiagnosticsPanel";

const PROVIDER_LABELS: Record<string, string> = {
  "windows-open-ssh": "OpenSSH",
  "one-password": "1Password",
  pageant: "Pageant",
  custom: "Custom",
};

function providerLabel(provider: BridgeProvider): string {
  return PROVIDER_LABELS[provider.type] ?? provider.type;
}

export function WslBridgePanel() {
  const {
    overview,
    providers,
    recommendedProvider,
    binaryVersions,
    downloadProgress,
    diagnostics,
    loading,
    fetchOverview,
    fetchRecommended,
    fetchBinaryVersions,
    downloadBinary,
    runDiagnostics,
    bootstrapDistro,
    teardownDistro,
    startRelay,
    stopRelay,
    restartRelay,
    setDistroProvider,
    setAgentForwarding,
  } = useBridgeStore();
  const [actionLoading, setActionLoading] = useState<string | null>(null);
  const [teardownTarget, setTeardownTarget] = useState<string | null>(null);

  useEffect(() => {
    fetchOverview();
    fetchRecommended();
    fetchBinaryVersions();
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

  const handleBootstrap = (distro: string, relayMode?: RelayMode) =>
    handleAction(`Setup bridge for ${distro}`, () => bootstrapDistro(distro, relayMode).then(() => {}));

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
        binaryVersions={binaryVersions}
        downloadProgress={downloadProgress}
        onRefresh={() => fetchOverview()}
        onDownload={(binary) =>
          handleAction(`Downloading ${binary}`, () => downloadBinary(binary))
        }
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
                recommendedProvider={recommendedProvider}
                actionLoading={actionLoading}
                diagnosticsResult={diagnostics[distro.distro_name] ?? null}
                onBootstrap={(relayMode) => handleBootstrap(distro.distro_name, relayMode)}
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
                onForwardingChange={(enabled) =>
                  handleAction(
                    enabled ? "Agent forwarding enabled" : "Agent forwarding disabled",
                    () => setAgentForwarding(distro.distro_name, enabled),
                  )
                }
                onRunDiagnostics={() =>
                  handleAction(`Diagnostics for ${distro.distro_name}`, () => runDiagnostics(distro.distro_name))
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
  binaryVersions,
  downloadProgress,
  onRefresh,
  onDownload,
}: {
  overview: ReturnType<typeof useBridgeStore>["overview"];
  providers: ProviderStatus[];
  loading: boolean;
  binaryVersions: ReturnType<typeof useBridgeStore>["binaryVersions"];
  downloadProgress: Record<string, number>;
  onRefresh: () => void;
  onDownload: (binary: string) => void;
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
          <StatusRow label="WSL available" ok={overview.wsl_available} />

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

          {overview.relay_binaries.length > 0 && (
            <div className="space-y-1">
              <span className="text-[10px] font-medium text-muted-foreground uppercase tracking-wider">
                Relay Binaries
              </span>
              {overview.relay_binaries.map((b) => {
                const binaryKey = b.binary === "Npiperelay" ? "npiperelay" : "wsl-ssh-pageant";
                const progress = downloadProgress[binaryKey];
                const isDownloading = progress !== undefined && progress < 100;

                return (
                  <div key={b.binary}>
                    <div className="flex items-center gap-2 text-xs">
                      {b.installed ? (
                        <CheckCircle size={13} className="text-success shrink-0" />
                      ) : (
                        <XCircle size={13} className="text-destructive shrink-0" />
                      )}
                      <span className={b.installed ? "text-foreground" : "text-muted-foreground"}>
                        {b.binary === "Npiperelay" ? "npiperelay.exe" : "wsl-ssh-pageant.exe"}
                      </span>
                      {b.installed && binaryVersions && (
                        <span className="text-[10px] text-muted-foreground/50">
                          {b.binary === "Npiperelay" ? binaryVersions.npiperelay : binaryVersions.wsl_ssh_pageant}
                        </span>
                      )}
                      {!b.installed && !isDownloading && (
                        <button
                          type="button"
                          onClick={() => onDownload(binaryKey)}
                          className="ml-auto px-2 py-0.5 text-[10px] font-medium rounded-md bg-primary text-primary-foreground hover:bg-primary/90 flex items-center gap-1"
                        >
                          <Download size={9} />
                          Download
                        </button>
                      )}
                    </div>
                    {isDownloading && (
                      <div className="ml-5 mt-1">
                        <div className="flex items-center gap-2">
                          <div className="flex-1 h-1.5 rounded-full bg-secondary overflow-hidden">
                            <div
                              className="h-full bg-primary rounded-full transition-all"
                              style={{ width: `${progress}%` }}
                            />
                          </div>
                          <span className="text-[10px] text-muted-foreground w-8 text-right">{progress}%</span>
                        </div>
                      </div>
                    )}
                  </div>
                );
              })}
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
  recommendedProvider,
  actionLoading,
  diagnosticsResult,
  onBootstrap,
  onStart,
  onStop,
  onRestart,
  onTeardown,
  onProviderChange,
  onForwardingChange,
  onRunDiagnostics,
  onRefresh,
}: {
  distro: DistroBridgeStatus;
  providers: ProviderStatus[];
  recommendedProvider: BridgeProvider | null;
  actionLoading: string | null;
  diagnosticsResult: ReturnType<typeof useBridgeStore>["diagnostics"][string] | null;
  onBootstrap: (relayMode?: RelayMode) => void;
  onStart: () => void;
  onStop: () => void;
  onRestart: () => void;
  onTeardown: () => void;
  onProviderChange: (provider: BridgeProvider) => void;
  onForwardingChange: (enabled: boolean) => void;
  onRunDiagnostics: () => void;
  onRefresh: () => void;
}) {
  const isActionRunning = actionLoading !== null;
  const [relayMode, setRelayMode] = useState<RelayMode>("systemd");
  const [showDiagnostics, setShowDiagnostics] = useState(false);
  const [showLogs, setShowLogs] = useState(false);
  const [logs, setLogs] = useState<string>("");
  const [logLines, setLogLines] = useState<number>(50);
  const [logsLoading, setLogsLoading] = useState(false);

  const handleLoadLogs = async () => {
    setLogsLoading(true);
    try {
      const output = await commands.getRelayLogs(distro.distro_name, logLines);
      setLogs(output);
    } catch (err) {
      setLogs(`Error loading logs: ${String(err)}`);
    } finally {
      setLogsLoading(false);
    }
  };

  const toggleLogs = () => {
    const next = !showLogs;
    setShowLogs(next);
    if (next && !logs) {
      handleLoadLogs();
    }
  };

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
          {distro.relay_installed && (
            <span className="text-[10px] px-1.5 py-0.5 rounded-full font-medium bg-secondary text-muted-foreground">
              {providerLabel(distro.provider)}
            </span>
          )}
          {distro.relay_mode === "daemon" && (
            <span className="text-[10px] px-1.5 py-0.5 rounded-full font-medium bg-secondary text-muted-foreground">
              daemon
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
          <ProviderSelector
            selected={distro.provider}
            providerStatuses={providers}
            recommended={recommendedProvider?.type ?? null}
            onChange={onProviderChange}
            disabled={isActionRunning}
          />

          <div className="space-y-1">
            {distro.provider.type !== "pageant" && (
              <StatusRow
                label="socat installed"
                ok={distro.socat_installed}
                hint={!distro.socat_installed ? "sudo apt install socat" : undefined}
              />
            )}
            {/* Systemd/Daemon relay mode selector */}
            <div className="flex items-center gap-2 pt-1">
              <span className="text-[11px] text-muted-foreground">Relay mode:</span>
              <div className="flex rounded-lg overflow-hidden border border-border">
                <button
                  type="button"
                  onClick={() => setRelayMode("systemd")}
                  disabled={isActionRunning}
                  className={`px-2.5 py-1 text-[11px] font-medium transition-colors ${
                    relayMode === "systemd"
                      ? "bg-primary text-primary-foreground"
                      : "bg-secondary text-muted-foreground hover:bg-accent"
                  }`}
                >
                  Systemd
                </button>
                <button
                  type="button"
                  onClick={() => setRelayMode("daemon")}
                  disabled={isActionRunning}
                  className={`px-2.5 py-1 text-[11px] font-medium transition-colors ${
                    relayMode === "daemon"
                      ? "bg-primary text-primary-foreground"
                      : "bg-secondary text-muted-foreground hover:bg-accent"
                  }`}
                >
                  Daemon
                </button>
              </div>
              {relayMode === "daemon" && (
                <span className="text-[9px] text-muted-foreground/60">No systemd required</span>
              )}
            </div>
            {relayMode === "systemd" && (
              <StatusRow
                label="systemd available"
                ok={distro.systemd_available}
                hint={!distro.systemd_available ? "Add systemd=true to /etc/wsl.conf" : undefined}
              />
            )}
          </div>

          <button
            type="button"
            onClick={() => onBootstrap(relayMode)}
            disabled={
              isActionRunning ||
              (distro.provider.type !== "pageant" && !distro.socat_installed) ||
              (relayMode === "systemd" && !distro.systemd_available)
            }
            className="px-3 py-1.5 text-xs font-medium rounded-lg bg-primary text-primary-foreground hover:bg-primary/90 disabled:opacity-50 flex items-center gap-1.5"
          >
            {isActionRunning ? <Loader2 size={12} className="animate-spin" /> : <ExternalLink size={12} />}
            Setup Bridge{relayMode === "daemon" ? " (Daemon Mode)" : ""}
          </button>
        </div>
      )}

      {/* Bootstrapped — show full status */}
      {distro.distro_running && distro.relay_installed && (
        <div className="space-y-3">
          <div className="space-y-1">
            <StatusRow label="Relay service" ok={distro.service_active} />
            <StatusRow label="Socket exists" ok={distro.socket_exists} />
            <StatusRow label="Agent reachable" ok={distro.agent_reachable} />
          </div>

          {distro.error && (
            <div className="flex items-start gap-1.5 text-xs text-warning">
              <AlertCircle size={12} className="mt-0.5 shrink-0" />
              {distro.error}
            </div>
          )}

          {/* Agent forwarding toggle */}
          <div className="flex items-center justify-between py-1">
            <div className="flex-1">
              <span className="text-xs text-muted-foreground">Forward SSH agent to remote hosts</span>
              {distro.allow_agent_forwarding && (
                <div className="flex items-center gap-1 mt-0.5 text-[10px] text-warning">
                  <AlertTriangle size={10} />
                  Only enable if you trust all remote hosts
                </div>
              )}
            </div>
            <button
              type="button"
              onClick={() => onForwardingChange(!distro.allow_agent_forwarding)}
              disabled={isActionRunning}
              aria-label="Toggle agent forwarding"
              className={`w-9 h-5 rounded-full transition-colors relative shrink-0 ${
                distro.allow_agent_forwarding ? "bg-warning" : "bg-secondary"
              }`}
            >
              <div
                className={`absolute top-0.5 w-4 h-4 rounded-full bg-white shadow transition-transform ${
                  distro.allow_agent_forwarding ? "translate-x-4" : "translate-x-0.5"
                }`}
              />
            </button>
          </div>

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

            <button
              type="button"
              onClick={() => {
                onRunDiagnostics();
                setShowDiagnostics(true);
              }}
              disabled={isActionRunning}
              className="px-2.5 py-1 text-[11px] font-medium rounded-lg bg-secondary hover:bg-accent disabled:opacity-50 flex items-center gap-1"
            >
              <Stethoscope size={10} /> Diagnostics
            </button>

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

          {/* Diagnostics panel */}
          {showDiagnostics && diagnosticsResult && (
            <div className="space-y-1">
              <button
                type="button"
                onClick={() => setShowDiagnostics(false)}
                className="text-[10px] text-muted-foreground/60 hover:text-muted-foreground flex items-center gap-1"
              >
                <ChevronUp size={10} /> Hide diagnostics
              </button>
              <DiagnosticsPanel result={diagnosticsResult} />
            </div>
          )}

          {/* Log viewer */}
          <div className="space-y-1">
            <button
              type="button"
              onClick={toggleLogs}
              className="text-[10px] text-muted-foreground/60 hover:text-muted-foreground flex items-center gap-1"
            >
              <FileText size={10} />
              {showLogs ? (
                <>
                  <ChevronUp size={10} /> Hide logs
                </>
              ) : (
                <>
                  <ChevronDown size={10} /> View logs
                </>
              )}
            </button>
            {showLogs && (
              <div className="space-y-1.5">
                <div className="flex items-center gap-2">
                  <div className="flex rounded-lg overflow-hidden border border-border">
                    {([20, 50, 100] as const).map((n) => (
                      <button
                        key={n}
                        type="button"
                        onClick={() => {
                          setLogLines(n);
                          setLogs("");
                          setTimeout(handleLoadLogs, 0);
                        }}
                        className={`px-2 py-0.5 text-[10px] transition-colors ${
                          logLines === n
                            ? "bg-secondary text-foreground"
                            : "bg-transparent text-muted-foreground hover:bg-secondary/50"
                        }`}
                      >
                        {n}
                      </button>
                    ))}
                  </div>
                  <button
                    type="button"
                    onClick={handleLoadLogs}
                    disabled={logsLoading}
                    className="text-muted-foreground/60 hover:text-muted-foreground transition-colors"
                    title="Refresh logs"
                  >
                    {logsLoading ? (
                      <Loader2 size={11} className="animate-spin" />
                    ) : (
                      <RefreshCw size={11} />
                    )}
                  </button>
                </div>
                <pre className="text-[10px] font-mono bg-black/20 dark:bg-black/40 rounded-lg p-2 overflow-auto max-h-48 whitespace-pre-wrap text-foreground/70">
                  {logsLoading ? "Loading..." : logs || "(no output)"}
                </pre>
              </div>
            )}
          </div>
        </div>
      )}
    </div>
  );
}
