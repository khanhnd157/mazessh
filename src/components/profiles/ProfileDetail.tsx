import { useState } from "react";
import { useAppStore } from "@/stores/appStore";
import { useProfileStore } from "@/stores/profileStore";
import { useLogStore } from "@/stores/logStore";
import type { ConnectionTestResult, SshProfile } from "@/types";
import { getProviderLabel } from "@/types";

interface ProfileDetailProps {
  profile: SshProfile;
}

export function ProfileDetail({ profile }: ProfileDetailProps) {
  const { activateProfile, activeProfile, testConnection } = useAppStore();
  const { deleteProfile, fetchProfiles } = useProfileStore();
  const { addLog } = useLogStore();
  const [testResult, setTestResult] = useState<ConnectionTestResult | null>(null);
  const [testing, setTesting] = useState(false);
  const [deleting, setDeleting] = useState(false);

  const isActive = activeProfile?.id === profile.id;

  const handleActivate = async () => {
    try {
      const result = await activateProfile(profile.id);
      await fetchProfiles();
      addLog({
        action: "activate",
        detail: `Activated "${result.profile_name}"`,
        level: "info",
      });
    } catch (err) {
      addLog({
        action: "activate",
        detail: `Failed: ${err}`,
        level: "error",
      });
    }
  };

  const handleTest = async () => {
    setTesting(true);
    setTestResult(null);
    try {
      const result = await testConnection(profile.id);
      setTestResult(result);
      addLog({
        action: "test",
        detail: `Connection test ${result.success ? "passed" : "failed"} for "${result.profile_name}"`,
        level: result.success ? "info" : "warn",
      });
    } catch (err) {
      addLog({
        action: "test",
        detail: `Test error: ${err}`,
        level: "error",
      });
    } finally {
      setTesting(false);
    }
  };

  const handleDelete = async () => {
    if (!confirm(`Delete profile "${profile.name}"?`)) return;
    setDeleting(true);
    try {
      await deleteProfile(profile.id);
      addLog({
        action: "delete",
        detail: `Deleted profile "${profile.name}"`,
        level: "info",
      });
    } catch (err) {
      addLog({
        action: "delete",
        detail: `Failed: ${err}`,
        level: "error",
      });
    } finally {
      setDeleting(false);
    }
  };

  return (
    <div className="space-y-6">
      <div className="flex items-start justify-between">
        <div>
          <h2 className="text-2xl font-bold">{profile.name}</h2>
          <p className="text-muted-foreground mt-1">
            {getProviderLabel(profile.provider)} · {profile.email}
          </p>
        </div>
        <div className="flex gap-2">
          {!isActive && (
            <button
              onClick={handleActivate}
              className="px-4 py-2 text-sm rounded-md bg-primary text-primary-foreground hover:bg-primary/90 transition-colors"
            >
              Activate
            </button>
          )}
          {isActive && (
            <span className="px-4 py-2 text-sm rounded-md bg-green-500/20 text-green-400 font-medium">
              Active
            </span>
          )}
        </div>
      </div>

      <div className="grid grid-cols-2 gap-4">
        <InfoField label="Host Alias" value={profile.host_alias} mono />
        <InfoField label="Hostname" value={profile.hostname} mono />
        <InfoField label="SSH User" value={profile.ssh_user || "git"} mono />
        <InfoField label="Port" value={String(profile.port || 22)} mono />
        <InfoField label="Git Username" value={profile.git_username} />
        <InfoField label="Key Type" value={profile.has_passphrase ? "With passphrase" : "No passphrase"} />
      </div>

      <div>
        <h3 className="text-sm font-medium text-muted-foreground mb-2">SSH Key</h3>
        <div className="p-3 rounded-md bg-secondary font-mono text-sm break-all">
          {profile.private_key_path}
        </div>
      </div>

      <div className="flex gap-2">
        <button
          onClick={handleTest}
          disabled={testing}
          className="px-4 py-2 text-sm rounded-md bg-secondary hover:bg-secondary/80 transition-colors disabled:opacity-50"
        >
          {testing ? "Testing..." : "Test Connection"}
        </button>
        <button
          onClick={handleDelete}
          disabled={deleting}
          className="px-4 py-2 text-sm rounded-md bg-destructive/10 text-destructive hover:bg-destructive/20 transition-colors disabled:opacity-50"
        >
          {deleting ? "Deleting..." : "Delete"}
        </button>
      </div>

      {testResult && (
        <div
          className={`p-4 rounded-md text-sm ${
            testResult.success
              ? "bg-green-500/10 border border-green-500/20"
              : "bg-destructive/10 border border-destructive/20"
          }`}
        >
          <p className="font-medium mb-1">
            {testResult.success ? "Connection successful" : "Connection failed"}
          </p>
          <pre className="text-xs font-mono whitespace-pre-wrap text-muted-foreground">
            {testResult.output}
          </pre>
        </div>
      )}

      <div className="text-xs text-muted-foreground">
        Created: {new Date(profile.created_at).toLocaleString()} ·
        Updated: {new Date(profile.updated_at).toLocaleString()}
      </div>
    </div>
  );
}

function InfoField({ label, value, mono }: { label: string; value: string; mono?: boolean }) {
  return (
    <div>
      <p className="text-xs text-muted-foreground mb-0.5">{label}</p>
      <p className={`text-sm ${mono ? "font-mono" : ""}`}>{value}</p>
    </div>
  );
}
