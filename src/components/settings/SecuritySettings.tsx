import { useState, useEffect } from "react";
import { Shield, Lock, Clock, KeyRound, AlertCircle, Check, Download, Upload, HeartPulse } from "lucide-react";
import { toast } from "sonner";
import { commands } from "@/lib/tauri-commands";
import { useSecurityStore } from "@/stores/securityStore";
import { useProfileStore } from "@/stores/profileStore";
import type { SecuritySettings as SecuritySettingsType, KeyHealthReport } from "@/types";
import { AuditLogViewer } from "./AuditLogViewer";

const TIMEOUT_OPTIONS = [
  { value: null, label: "Disabled" },
  { value: 5, label: "5 minutes" },
  { value: 15, label: "15 minutes" },
  { value: 30, label: "30 minutes" },
  { value: 60, label: "60 minutes" },
];

const AGENT_TIMEOUT_OPTIONS = [
  ...TIMEOUT_OPTIONS,
  { value: 120, label: "120 minutes" },
];

export function SecuritySettingsPanel() {
  const { pinIsSet, settings, fetchSettings, updateSettings, setupPin, changePin, removePin } =
    useSecurityStore();
  const [showAuditLog, setShowAuditLog] = useState(false);
  const [pinAction, setPinAction] = useState<"setup" | "change" | "remove" | null>(null);
  const [pinInput, setPinInput] = useState("");
  const [pinConfirm, setPinConfirm] = useState("");
  const [pinOld, setPinOld] = useState("");
  const [pinError, setPinError] = useState("");
  const [pinLoading, setPinLoading] = useState(false);

  useEffect(() => {
    fetchSettings();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const handleUpdateSetting = async (partial: Partial<SecuritySettingsType>) => {
    if (!settings) return;
    const updated = { ...settings, ...partial };
    try {
      await updateSettings(updated);
      toast.success("Settings updated");
    } catch (err) {
      toast.error("Failed to update settings", { description: String(err) });
    }
  };

  const handlePinSubmit = async () => {
    setPinError("");
    setPinLoading(true);
    try {
      if (pinAction === "setup") {
        if (pinInput.length < 4) {
          setPinError("PIN must be at least 4 characters");
          return;
        }
        if (pinInput !== pinConfirm) {
          setPinError("PINs do not match");
          return;
        }
        await setupPin(pinInput);
        toast.success("PIN configured");
      } else if (pinAction === "change") {
        if (pinInput.length < 4) {
          setPinError("New PIN must be at least 4 characters");
          return;
        }
        await changePin(pinOld, pinInput);
        toast.success("PIN changed");
      } else if (pinAction === "remove") {
        await removePin(pinInput);
        toast.success("PIN removed");
      }
      setPinAction(null);
      setPinInput("");
      setPinConfirm("");
      setPinOld("");
    } catch (err) {
      setPinError(String(err));
    } finally {
      setPinLoading(false);
    }
  };

  if (showAuditLog) {
    return (
      <div className="space-y-4 max-w-3xl">
        <button
          type="button"
          onClick={() => setShowAuditLog(false)}
          className="text-xs text-primary hover:underline"
        >
          Back to Settings
        </button>
        <AuditLogViewer />
      </div>
    );
  }

  return (
    <div className="space-y-6 max-w-2xl">
      <div className="flex items-center gap-2">
        <Shield size={16} className="text-primary" />
        <h3 className="text-sm font-semibold">Security Settings</h3>
      </div>

      {/* Section 1: PIN Protection */}
      <div className="rounded-xl border bg-card p-4 space-y-3">
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-2">
            <Lock size={14} className="text-muted-foreground" />
            <span className="text-sm font-medium">PIN Protection</span>
          </div>
          <span
            className={`text-[10px] px-2 py-0.5 rounded-full font-medium ${
              pinIsSet
                ? "bg-success/15 text-success"
                : "bg-muted text-muted-foreground"
            }`}
          >
            {pinIsSet ? "Enabled" : "Disabled"}
          </span>
        </div>
        <p className="text-xs text-muted-foreground">
          Protect your SSH profiles with a PIN. The app will lock and require your PIN to access.
        </p>

        {pinAction ? (
          <div className="space-y-2 p-3 rounded-lg bg-secondary/50">
            {pinError && (
              <div className="flex items-center gap-1.5 text-destructive text-xs">
                <AlertCircle size={12} /> {pinError}
              </div>
            )}
            {pinAction === "change" && (
              <input
                type="password"
                value={pinOld}
                onChange={(e) => setPinOld(e.target.value)}
                placeholder="Current PIN"
                className="w-full px-3 py-2 rounded-lg bg-secondary border border-transparent text-sm focus:outline-none focus:ring-1 focus:ring-ring"
              />
            )}
            <input
              type="password"
              value={pinInput}
              onChange={(e) => setPinInput(e.target.value)}
              placeholder={pinAction === "remove" ? "Enter PIN to confirm" : "New PIN (4+ chars)"}
              autoFocus
              className="w-full px-3 py-2 rounded-lg bg-secondary border border-transparent text-sm focus:outline-none focus:ring-1 focus:ring-ring"
            />
            {pinAction === "setup" && (
              <input
                type="password"
                value={pinConfirm}
                onChange={(e) => setPinConfirm(e.target.value)}
                placeholder="Confirm PIN"
                className="w-full px-3 py-2 rounded-lg bg-secondary border border-transparent text-sm focus:outline-none focus:ring-1 focus:ring-ring"
              />
            )}
            <div className="flex gap-2">
              <button
                type="button"
                onClick={handlePinSubmit}
                disabled={pinLoading}
                className="px-3 py-1.5 text-xs font-medium rounded-lg bg-primary text-primary-foreground hover:bg-primary/90 disabled:opacity-50"
              >
                {pinLoading ? "..." : "Confirm"}
              </button>
              <button
                type="button"
                onClick={() => {
                  setPinAction(null);
                  setPinInput("");
                  setPinConfirm("");
                  setPinOld("");
                  setPinError("");
                }}
                className="px-3 py-1.5 text-xs rounded-lg bg-secondary hover:bg-secondary/80"
              >
                Cancel
              </button>
            </div>
          </div>
        ) : (
          <div className="flex gap-2">
            {!pinIsSet ? (
              <button
                type="button"
                onClick={() => setPinAction("setup")}
                className="px-3 py-1.5 text-xs font-medium rounded-lg bg-primary text-primary-foreground hover:bg-primary/90"
              >
                Set PIN
              </button>
            ) : (
              <>
                <button
                  type="button"
                  onClick={() => setPinAction("change")}
                  className="px-3 py-1.5 text-xs font-medium rounded-lg bg-secondary hover:bg-accent"
                >
                  Change PIN
                </button>
                <button
                  type="button"
                  onClick={() => setPinAction("remove")}
                  className="px-3 py-1.5 text-xs font-medium rounded-lg text-destructive/70 hover:text-destructive hover:bg-destructive/10"
                >
                  Remove PIN
                </button>
              </>
            )}
          </div>
        )}
      </div>

      {/* Section 2: Auto-Lock */}
      <div className={`rounded-xl border bg-card p-4 space-y-3 ${!pinIsSet ? "opacity-50 pointer-events-none" : ""}`}>
        <div className="flex items-center gap-2">
          <Clock size={14} className="text-muted-foreground" />
          <span className="text-sm font-medium">Auto-Lock</span>
        </div>

        <div className="flex items-center justify-between">
          <span className="text-xs text-muted-foreground">Lock after inactivity</span>
          <select
            value={settings?.auto_lock_timeout_minutes ?? ""}
            onChange={(e) =>
              handleUpdateSetting({
                auto_lock_timeout_minutes: e.target.value ? Number(e.target.value) : null,
              })
            }
            className="px-2 py-1 text-xs rounded-lg bg-secondary border-none focus:ring-1 focus:ring-ring"
          >
            {TIMEOUT_OPTIONS.map((o) => (
              <option key={String(o.value)} value={o.value ?? ""}>
                {o.label}
              </option>
            ))}
          </select>
        </div>

        <div className="flex items-center justify-between">
          <span className="text-xs text-muted-foreground">Lock when minimized to tray</span>
          <button
            type="button"
            onClick={() =>
              handleUpdateSetting({ lock_on_minimize: !settings?.lock_on_minimize })
            }
            className={`w-9 h-5 rounded-full transition-colors relative ${
              settings?.lock_on_minimize ? "bg-primary" : "bg-secondary"
            }`}
          >
            <div
              className={`absolute top-0.5 w-4 h-4 rounded-full bg-white transition-transform ${
                settings?.lock_on_minimize ? "translate-x-4" : "translate-x-0.5"
              }`}
            />
          </button>
        </div>
      </div>

      {/* Section 3: Agent Key Timeout */}
      <div className="rounded-xl border bg-card p-4 space-y-3">
        <div className="flex items-center gap-2">
          <KeyRound size={14} className="text-muted-foreground" />
          <span className="text-sm font-medium">Agent Key Timeout</span>
        </div>
        <p className="text-xs text-muted-foreground">
          Automatically clear SSH keys from the agent after a period. Independent of app lock.
        </p>

        <div className="flex items-center justify-between">
          <span className="text-xs text-muted-foreground">Clear keys after</span>
          <select
            value={settings?.agent_key_timeout_minutes ?? ""}
            onChange={(e) =>
              handleUpdateSetting({
                agent_key_timeout_minutes: e.target.value ? Number(e.target.value) : null,
              })
            }
            className="px-2 py-1 text-xs rounded-lg bg-secondary border-none focus:ring-1 focus:ring-ring"
          >
            {AGENT_TIMEOUT_OPTIONS.map((o) => (
              <option key={String(o.value)} value={o.value ?? ""}>
                {o.label}
              </option>
            ))}
          </select>
        </div>
      </div>

      {/* Section 4: Export / Import */}
      <div className="rounded-xl border bg-card p-4 space-y-3">
        <div className="flex items-center gap-2">
          <Download size={14} className="text-muted-foreground" />
          <span className="text-sm font-medium">Export / Import Profiles</span>
        </div>
        <p className="text-xs text-muted-foreground">
          Export all profiles as JSON for backup or migration. Import merges profiles (skips duplicates by name).
        </p>
        <div className="flex gap-2">
          <button
            type="button"
            onClick={async () => {
              try {
                const json = await commands.exportProfiles();
                await navigator.clipboard.writeText(json);
                toast.success("Profiles copied to clipboard");
              } catch (err) {
                toast.error("Export failed", { description: String(err) });
              }
            }}
            className="flex items-center gap-1.5 px-3 py-1.5 text-xs font-medium rounded-lg bg-secondary hover:bg-accent"
          >
            <Download size={12} />
            Export to Clipboard
          </button>
          <button
            type="button"
            onClick={async () => {
              try {
                const json = await navigator.clipboard.readText();
                const count = await commands.importProfiles(json);
                useProfileStore.getState().fetchProfiles();
                toast.success(`Imported ${count} profile(s)`);
              } catch (err) {
                toast.error("Import failed", { description: String(err) });
              }
            }}
            className="flex items-center gap-1.5 px-3 py-1.5 text-xs font-medium rounded-lg bg-secondary hover:bg-accent"
          >
            <Upload size={12} />
            Import from Clipboard
          </button>
        </div>
      </div>

      {/* Section 5: Key Health Check */}
      <KeyHealthSection />

      {/* Section 6: Audit Log */}
      <div className="rounded-xl border bg-card p-4 space-y-3">
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-2">
            <Check size={14} className="text-muted-foreground" />
            <span className="text-sm font-medium">Audit Log</span>
          </div>
          <button
            type="button"
            onClick={() => setShowAuditLog(true)}
            className="px-3 py-1.5 text-xs font-medium rounded-lg bg-secondary hover:bg-accent"
          >
            View Log
          </button>
        </div>
        <p className="text-xs text-muted-foreground">
          All security-sensitive actions are logged for review.
        </p>
      </div>
    </div>
  );
}

function KeyHealthSection() {
  const [reports, setReports] = useState<KeyHealthReport[]>([]);
  const [loading, setLoading] = useState(false);
  const [ran, setRan] = useState(false);

  const runCheck = async () => {
    setLoading(true);
    try {
      const result = await commands.checkAllKeysHealth();
      setReports(result);
      setRan(true);
    } catch (err) {
      toast.error("Health check failed", { description: String(err) });
    } finally {
      setLoading(false);
    }
  };

  const totalIssues = reports.reduce((sum, r) => sum + r.issues.length, 0);
  const criticalCount = reports.reduce(
    (sum, r) => sum + r.issues.filter((i) => i.severity === "critical").length,
    0,
  );

  return (
    <div className="rounded-xl border bg-card p-4 space-y-3">
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-2">
          <HeartPulse size={14} className="text-muted-foreground" />
          <span className="text-sm font-medium">Key Health Check</span>
        </div>
        <button
          type="button"
          onClick={runCheck}
          disabled={loading}
          className="px-3 py-1.5 text-xs font-medium rounded-lg bg-secondary hover:bg-accent disabled:opacity-50"
        >
          {loading ? "Checking..." : "Run Check"}
        </button>
      </div>
      <p className="text-xs text-muted-foreground">
        Analyze all SSH keys for algorithm strength, missing files, and security issues.
      </p>

      {ran && (
        <div className="space-y-2 pt-1">
          {/* Summary */}
          <div className="flex items-center gap-3 text-xs">
            <span className="text-muted-foreground">{reports.length} profiles checked</span>
            {totalIssues === 0 ? (
              <span className="text-success font-medium">All keys healthy</span>
            ) : (
              <>
                {criticalCount > 0 && (
                  <span className="text-destructive font-medium">{criticalCount} critical</span>
                )}
                <span className="text-warning font-medium">{totalIssues} issue(s)</span>
              </>
            )}
          </div>

          {/* Per-profile results */}
          {reports.map((r) => (
            <div key={r.profile_name} className="p-2.5 rounded-lg bg-secondary/40 space-y-1">
              <div className="flex items-center justify-between">
                <span className="text-xs font-medium">{r.profile_name}</span>
                <span className="text-[10px] text-muted-foreground font-mono">
                  {r.key_type} {r.bits > 0 ? `${r.bits}b` : ""}
                </span>
              </div>
              {r.issues.length === 0 ? (
                <div className="flex items-center gap-1 text-[11px] text-success">
                  <Check size={11} /> No issues
                </div>
              ) : (
                r.issues.map((issue, i) => (
                  <div
                    key={i}
                    className={`flex items-start gap-1.5 text-[11px] ${
                      issue.severity === "critical"
                        ? "text-destructive"
                        : issue.severity === "warning"
                          ? "text-warning"
                          : "text-muted-foreground"
                    }`}
                  >
                    <AlertCircle size={11} className="mt-0.5 shrink-0" />
                    {issue.message}
                  </div>
                ))
              )}
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
