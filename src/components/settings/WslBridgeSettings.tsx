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
  ArrowUpCircle,
  Settings2,
  Upload,
  ClipboardCopy,
  Layers,
  Terminal,
} from "lucide-react";
import { toast } from "sonner";
import { commands } from "@/lib/tauri-commands";
import { useBridgeStore } from "@/stores/bridgeStore";
import type { BinaryUpdateStatus, BootstrapAllResult, BridgeProvider, DistroBridgeStatus, ProviderStatus, RelayMode, ShellInjection } from "@/types";
import { ConfirmDialog } from "@/components/ui/ConfirmDialog";
import { ProviderSelector } from "./ProviderSelector";
import { DiagnosticsPanel } from "./DiagnosticsPanel";

const MAX_RESTARTS_DEFAULT = 5;

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
    updateStatuses,
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
    setAutoRestart,
    checkUpdates,
    setSocketPath,
    resetRestartCount,
    exportConfig,
    importConfig,
    bootstrapAll,
    shellInjections,
    refreshRelayScript,
    fetchShellInjections,
    removeShellInjection,
  } = useBridgeStore();
  const [actionLoading, setActionLoading] = useState<string | null>(null);
  const [teardownTarget, setTeardownTarget] = useState<string | null>(null);
  const [bootstrapAllResults, setBootstrapAllResults] = useState<BootstrapAllResult[] | null>(null);
  const [showImportModal, setShowImportModal] = useState(false);
  const [importJson, setImportJson] = useState("");

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

  const handleBootstrapAll = async () => {
    setActionLoading("bootstrap-all");
    setBootstrapAllResults(null);
    try {
      const results = await bootstrapAll();
      setBootstrapAllResults(results);
      const succeeded = results.filter((r) => r.success).length;
      if (succeeded === results.length) {
        toast.success(`${succeeded}/${results.length} distros bootstrapped`);
      } else {
        toast.warning(`${succeeded}/${results.length} distros bootstrapped`);
      }
    } catch (err) {
      toast.error("Bootstrap all failed", { description: String(err) });
    } finally {
      setActionLoading(null);
    }
  };

  const handleExport = async () => {
    try {
      const json = await exportConfig();
      await navigator.clipboard.writeText(json);
      toast.success("Bridge config copied to clipboard");
    } catch (err) {
      toast.error("Export failed", { description: String(err) });
    }
  };

  const handleImport = async () => {
    try {
      const count = await importConfig(importJson);
      setShowImportModal(false);
      setImportJson("");
      toast.success(`${count} distro${count !== 1 ? "s" : ""} imported`);
    } catch (err) {
      toast.error("Import failed", { description: String(err) });
    }
  };

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
        updateStatuses={updateStatuses}
        onRefresh={() => fetchOverview()}
        onDownload={(binary) =>
          handleAction(`Downloading ${binary}`, () => downloadBinary(binary))
        }
        onCheckUpdates={() =>
          handleAction("Checking for updates", () => checkUpdates())
        }
        onExport={handleExport}
        onImport={() => setShowImportModal(true)}
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
            <>
              {/* Setup All button — shown when multiple distros need bootstrapping */}
              {overview.distros.filter((d) => d.distro_running && !d.relay_installed).length > 1 && (
                <div className="flex items-center justify-between">
                  <span className="text-xs text-muted-foreground">
                    {overview.distros.filter((d) => d.distro_running && !d.relay_installed).length} distros ready to setup
                  </span>
                  <button
                    type="button"
                    onClick={handleBootstrapAll}
                    disabled={actionLoading !== null}
                    className="px-3 py-1.5 text-xs font-medium rounded-lg bg-primary text-primary-foreground hover:bg-primary/90 disabled:opacity-50 flex items-center gap-1.5"
                  >
                    {actionLoading === "bootstrap-all" ? (
                      <Loader2 size={12} className="animate-spin" />
                    ) : (
                      <Layers size={12} />
                    )}
                    Setup All
                  </button>
                </div>
              )}

              {/* Bootstrap-all results summary */}
              {bootstrapAllResults && (
                <div className="rounded-xl border bg-card p-3 space-y-1">
                  <p className="text-xs font-medium">Bootstrap results</p>
                  {bootstrapAllResults.map((r) => (
                    <div key={r.distro} className="flex items-center gap-2 text-xs">
                      {r.success ? (
                        <CheckCircle size={12} className="text-success shrink-0" />
                      ) : (
                        <XCircle size={12} className="text-destructive shrink-0" />
                      )}
                      <span>{r.distro}</span>
                      {r.error && <span className="text-[10px] text-muted-foreground/70 truncate">{r.error}</span>}
                    </div>
                  ))}
                </div>
              )}

              {overview.distros.map((distro) => (
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
                  onAutoRestartChange={(enabled) =>
                    handleAction(
                      enabled ? "Auto-restart enabled" : "Auto-restart disabled",
                      () => setAutoRestart(distro.distro_name, enabled),
                    )
                  }
                  onSocketPathChange={(path) =>
                    handleAction("Socket path saved", () => setSocketPath(distro.distro_name, path))
                  }
                  onRunDiagnostics={() =>
                    handleAction(`Diagnostics for ${distro.distro_name}`, () => runDiagnostics(distro.distro_name))
                  }
                  onResetRestartCount={() =>
                    handleAction("Watchdog reset", () => resetRestartCount(distro.distro_name))
                  }
                  onRefreshScript={() =>
                    handleAction("Relay script updated and restarted", () => refreshRelayScript(distro.distro_name))
                  }
                  shellInjections={shellInjections[distro.distro_name] ?? null}
                  onFetchShellInjections={() => fetchShellInjections(distro.distro_name)}
                  onRemoveShellInjection={(rcFile) =>
                    handleAction(`Removed injection from ${rcFile}`, () => removeShellInjection(distro.distro_name, rcFile))
                  }
                  onRefresh={() => fetchOverview()}
                />
              ))}
            </>
          )}
        </>
      )}

      {/* Import modal */}
      {showImportModal && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/50">
          <div className="bg-card rounded-xl border p-4 w-full max-w-md space-y-3 shadow-xl">
            <p className="text-sm font-medium">Import Bridge Config</p>
            <p className="text-xs text-muted-foreground">Paste the JSON from a previous export. Existing distro entries will be replaced.</p>
            <textarea
              value={importJson}
              onChange={(e) => setImportJson(e.target.value)}
              rows={8}
              placeholder='{"distros": [...]}'
              className="w-full px-2 py-1.5 text-[10px] font-mono rounded-lg bg-secondary border border-border focus:outline-none focus:ring-1 focus:ring-primary resize-none"
            />
            <div className="flex justify-end gap-2">
              <button
                type="button"
                onClick={() => { setShowImportModal(false); setImportJson(""); }}
                className="px-3 py-1.5 text-xs rounded-lg bg-secondary hover:bg-accent"
              >
                Cancel
              </button>
              <button
                type="button"
                onClick={handleImport}
                disabled={!importJson.trim()}
                className="px-3 py-1.5 text-xs font-medium rounded-lg bg-primary text-primary-foreground hover:bg-primary/90 disabled:opacity-50"
              >
                Import
              </button>
            </div>
          </div>
        </div>
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
  updateStatuses,
  onRefresh,
  onDownload,
  onCheckUpdates,
  onExport,
  onImport,
}: {
  overview: ReturnType<typeof useBridgeStore>["overview"];
  providers: ProviderStatus[];
  loading: boolean;
  binaryVersions: ReturnType<typeof useBridgeStore>["binaryVersions"];
  downloadProgress: Record<string, number>;
  updateStatuses: BinaryUpdateStatus[];
  onRefresh: () => void;
  onDownload: (binary: string) => void;
  onCheckUpdates: () => void;
  onExport: () => void;
  onImport: () => void;
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
              <div className="flex items-center justify-between">
                <span className="text-[10px] font-medium text-muted-foreground uppercase tracking-wider">
                  Relay Binaries
                </span>
                {overview.relay_binaries.some((b) => b.installed) && (
                  <button
                    type="button"
                    onClick={onCheckUpdates}
                    className="text-[10px] text-primary hover:underline flex items-center gap-0.5"
                  >
                    <RefreshCw size={9} />
                    Check for updates
                  </button>
                )}
              </div>
              {overview.relay_binaries.map((b) => {
                const binaryKey = b.binary === "Npiperelay" ? "npiperelay" : "wsl-ssh-pageant";
                const versionKey = b.binary === "Npiperelay" ? "npiperelay" : "wsl_ssh_pageant";
                const progress = downloadProgress[binaryKey];
                const isDownloading = progress !== undefined && progress < 100;
                const updateStatus = updateStatuses.find((u) => u.binary === versionKey);
                const hasUpdate = updateStatus?.update_available ?? false;
                const installedVersion = binaryVersions
                  ? b.binary === "Npiperelay"
                    ? binaryVersions.npiperelay
                    : binaryVersions.wsl_ssh_pageant
                  : null;

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
                      {b.installed && installedVersion && (
                        <span className="text-[10px] text-muted-foreground/50">{installedVersion}</span>
                      )}
                      {hasUpdate && (
                        <span className="flex items-center gap-0.5 text-[10px] px-1.5 py-0.5 rounded-full bg-warning/15 text-warning font-medium">
                          <ArrowUpCircle size={9} />
                          Update available
                        </span>
                      )}
                      {(b.installed && hasUpdate) && (
                        <button
                          type="button"
                          onClick={() => onDownload(binaryKey)}
                          className="ml-auto px-2 py-0.5 text-[10px] font-medium rounded-md bg-warning text-warning-foreground hover:bg-warning/90 flex items-center gap-1"
                        >
                          <Download size={9} />
                          Update
                        </button>
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
                    {updateStatus?.latest_version && !hasUpdate && b.installed && (
                      <p className="ml-5 text-[9px] text-muted-foreground/50 mt-0.5">
                        Latest: {updateStatus.latest_version}
                      </p>
                    )}
                  </div>
                );
              })}
            </div>
          )}
        </div>
      )}

      {/* Export / Import footer */}
      <div className="border-t border-border/50 pt-2 flex items-center justify-end gap-2">
        <button
          type="button"
          onClick={onExport}
          className="flex items-center gap-1 text-[10px] text-muted-foreground hover:text-foreground transition-colors"
        >
          <ClipboardCopy size={10} />
          Export config
        </button>
        <button
          type="button"
          onClick={onImport}
          className="flex items-center gap-1 text-[10px] text-muted-foreground hover:text-foreground transition-colors"
        >
          <Upload size={10} />
          Import config
        </button>
      </div>
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
  onAutoRestartChange,
  onSocketPathChange,
  onRunDiagnostics,
  onResetRestartCount,
  onRefreshScript,
  shellInjections,
  onFetchShellInjections,
  onRemoveShellInjection,
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
  onAutoRestartChange: (enabled: boolean) => void;
  onSocketPathChange: (path: string) => void;
  onRunDiagnostics: () => void;
  onResetRestartCount: () => void;
  onRefreshScript: () => void;
  shellInjections: ShellInjection[] | null;
  onFetchShellInjections: () => void;
  onRemoveShellInjection: (rcFile: string) => void;
  onRefresh: () => void;
}) {
  const isActionRunning = actionLoading !== null;
  const [relayMode, setRelayMode] = useState<RelayMode>("systemd");
  const [showDiagnostics, setShowDiagnostics] = useState(false);
  const [showShellEnv, setShowShellEnv] = useState(false);
  const [expandedRcFile, setExpandedRcFile] = useState<string | null>(null);
  const [showLogs, setShowLogs] = useState(false);
  const [logs, setLogs] = useState<string>("");
  const [logLines, setLogLines] = useState<number>(50);
  const [logsLoading, setLogsLoading] = useState(false);
  const [showAdvanced, setShowAdvanced] = useState(false);
  const [socketPathInput, setSocketPathInput] = useState(distro.socket_path || "/tmp/maze-ssh-agent.sock");

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

          {/* Advanced: custom socket path */}
          <div className="border-t border-border/50 pt-2 mt-1">
            <button
              type="button"
              onClick={() => setShowAdvanced(!showAdvanced)}
              className="flex items-center gap-1 text-[10px] text-muted-foreground/60 hover:text-muted-foreground"
            >
              <Settings2 size={10} />
              Advanced
              {showAdvanced ? <ChevronUp size={10} /> : <ChevronDown size={10} />}
            </button>
            {showAdvanced && (
              <div className="mt-2 space-y-1.5">
                <label className="text-[10px] text-muted-foreground">Unix socket path</label>
                <div className="flex items-center gap-2">
                  <input
                    type="text"
                    value={socketPathInput}
                    onChange={(e) => setSocketPathInput(e.target.value)}
                    placeholder="/tmp/maze-ssh-agent.sock"
                    className="flex-1 px-2 py-1 text-[10px] font-mono rounded-lg bg-secondary border border-border focus:outline-none focus:ring-1 focus:ring-primary"
                  />
                  <button
                    type="button"
                    onClick={() => onSocketPathChange(socketPathInput)}
                    disabled={isActionRunning || !socketPathInput}
                    className="px-2 py-1 text-[10px] font-medium rounded-lg bg-secondary hover:bg-accent disabled:opacity-50"
                  >
                    Save
                  </button>
                </div>
                <p className="text-[9px] text-muted-foreground/50">
                  Must start with /tmp/ or /run/user/. Requires re-bootstrapping after change.
                </p>
              </div>
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

          {/* Auto-restart toggle */}
          <div className="flex items-center justify-between py-1">
            <div className="flex-1">
              <span className="text-xs text-muted-foreground">Auto-restart relay if it stops</span>
              {distro.relay_mode === "daemon" && (
                <p className="text-[10px] text-muted-foreground/50 mt-0.5">
                  Daemon mode self-starts via .bashrc
                </p>
              )}
            </div>
            <button
              type="button"
              onClick={() => onAutoRestartChange(!distro.auto_restart)}
              disabled={isActionRunning || distro.relay_mode === "daemon"}
              aria-label="Toggle auto-restart"
              className={`w-9 h-5 rounded-full transition-colors relative shrink-0 ${
                distro.auto_restart && distro.relay_mode !== "daemon" ? "bg-primary" : "bg-secondary"
              }`}
            >
              <div
                className={`absolute top-0.5 w-4 h-4 rounded-full bg-white shadow transition-transform ${
                  distro.auto_restart && distro.relay_mode !== "daemon" ? "translate-x-4" : "translate-x-0.5"
                }`}
              />
            </button>
          </div>

          {/* Relay script stale banner */}
          {distro.relay_script_stale && (
            <div className="flex items-center justify-between py-1.5 px-2.5 rounded-lg bg-warning/10 border border-warning/20">
              <div className="flex items-center gap-1.5 text-xs text-warning">
                <AlertTriangle size={11} />
                Relay script outdated — config changed since bootstrap
              </div>
              <button
                type="button"
                onClick={onRefreshScript}
                disabled={isActionRunning}
                className="flex items-center gap-1 text-[10px] font-medium text-warning hover:text-warning/80 disabled:opacity-50"
              >
                <ArrowUpCircle size={10} />
                Refresh Script
              </button>
            </div>
          )}

          {/* Watchdog paused badge */}
          {distro.auto_restart && distro.relay_mode !== "daemon" && distro.watchdog_restart_count >= MAX_RESTARTS_DEFAULT && (
            <div className="flex items-center justify-between py-1 px-2 rounded-lg bg-warning/10 border border-warning/20">
              <div className="flex items-center gap-1.5 text-xs text-warning">
                <AlertTriangle size={11} />
                Auto-restart paused ({distro.watchdog_restart_count}/{MAX_RESTARTS_DEFAULT} attempts)
              </div>
              <button
                type="button"
                onClick={onResetRestartCount}
                disabled={actionLoading !== null}
                className="text-[10px] text-warning hover:text-warning/80 underline"
              >
                Reset
              </button>
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
              <DiagnosticsPanel
                result={diagnosticsResult}
                distro={distro.distro_name}
                onRerunDiagnostics={onRunDiagnostics}
              />
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

          {/* Shell Env viewer */}
          <div className="space-y-1">
            <button
              type="button"
              onClick={() => {
                const next = !showShellEnv;
                setShowShellEnv(next);
                if (next && shellInjections === null) {
                  onFetchShellInjections();
                }
              }}
              className="text-[10px] text-muted-foreground/60 hover:text-muted-foreground flex items-center gap-1"
            >
              <Terminal size={10} />
              {showShellEnv ? (
                <>
                  <ChevronUp size={10} /> Hide shell env
                </>
              ) : (
                <>
                  <ChevronDown size={10} /> View shell env
                </>
              )}
            </button>
            {showShellEnv && (
              <div className="space-y-1.5">
                {shellInjections === null ? (
                  <div className="flex items-center gap-1 text-[10px] text-muted-foreground">
                    <Loader2 size={10} className="animate-spin" />
                    Loading...
                  </div>
                ) : shellInjections.length === 0 ? (
                  <p className="text-[10px] text-muted-foreground/60">No shell injections found.</p>
                ) : (
                  <div className="space-y-1">
                    {shellInjections.map((inj) => (
                      <div key={inj.rc_file} className="rounded-lg border border-border/50 overflow-hidden">
                        <div className="flex items-center gap-2 px-2 py-1.5 bg-secondary/50">
                          <span className="text-[10px] font-mono text-foreground/80 flex-1">{inj.rc_file}</span>
                          <span className="text-[9px] px-1.5 py-0.5 rounded-full bg-muted text-muted-foreground">
                            {inj.shell}
                          </span>
                          {inj.has_forward_block && (
                            <span className="text-[9px] px-1.5 py-0.5 rounded-full bg-warning/15 text-warning">
                              ForwardAgent
                            </span>
                          )}
                          {inj.injected_block !== null && (
                            <button
                              type="button"
                              onClick={() => setExpandedRcFile(expandedRcFile === inj.rc_file ? null : inj.rc_file)}
                              className="text-muted-foreground/60 hover:text-muted-foreground"
                              title="Toggle block"
                            >
                              {expandedRcFile === inj.rc_file ? <ChevronUp size={10} /> : <ChevronDown size={10} />}
                            </button>
                          )}
                          <button
                            type="button"
                            onClick={() => onRemoveShellInjection(inj.rc_file)}
                            disabled={isActionRunning}
                            className="text-[9px] px-1.5 py-0.5 rounded-md text-destructive hover:bg-destructive/10 disabled:opacity-50 font-medium"
                          >
                            Remove
                          </button>
                        </div>
                        {expandedRcFile === inj.rc_file && inj.injected_block !== null && (
                          <pre className="text-[9px] font-mono bg-black/20 dark:bg-black/40 p-2 overflow-auto max-h-32 whitespace-pre-wrap text-foreground/60">
                            {inj.injected_block}
                          </pre>
                        )}
                        {inj.injected_block === null && (
                          <p className="text-[9px] text-muted-foreground/50 px-2 py-1">No Maze SSH block found</p>
                        )}
                      </div>
                    ))}
                    <p className="text-[9px] text-muted-foreground/40 pt-0.5">
                      "Remove" surgically strips only the Maze SSH block. Use "Remove bridge" to clean all injections at once.
                    </p>
                  </div>
                )}
              </div>
            )}
          </div>
        </div>
      )}
    </div>
  );
}
