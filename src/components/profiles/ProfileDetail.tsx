import { useState, useEffect, useCallback } from "react";
import { ConfirmDialog } from "@/components/ui/ConfirmDialog";
import { useConfirm } from "@/hooks/useConfirm";
import {
  Zap,
  Trash2,
  Pencil,
  Plug,
  Loader2,
  Copy,
  Server,
  AtSign,
  Globe,
  Hash,
  User,
  KeyRound,
  CheckCircle2,
  XCircle,
  FolderGit2,
  Plus,
} from "lucide-react";
import { ProfileForm } from "./ProfileForm";
import { AddRepoMappingDialog } from "@/components/repos/AddRepoMappingDialog";
import { toast } from "sonner";
import { useAppStore } from "@/stores/appStore";
import { useProfileStore } from "@/stores/profileStore";
import { useLogStore } from "@/stores/logStore";
import { commands } from "@/lib/tauri-commands";
import type { ConnectionTestResult, KeyFingerprint, RepoMappingSummary, SshProfile } from "@/types";
import { getProviderLabel } from "@/types";
import { ProviderIcon } from "./ProviderIcon";

interface ProfileDetailProps {
  profile: SshProfile;
}

export function ProfileDetail({ profile }: ProfileDetailProps) {
  const { activateProfile, activeProfile, testConnection } = useAppStore();
  const { deleteProfile, fetchProfiles, selectProfile } = useProfileStore();
  const { addLog } = useLogStore();
  const [testResult, setTestResult] = useState<ConnectionTestResult | null>(null);
  const [testing, setTesting] = useState(false);
  const [deleting, setDeleting] = useState(false);
  const [showEdit, setShowEdit] = useState(false);
  const [showAddRepo, setShowAddRepo] = useState(false);
  const [profileMappings, setProfileMappings] = useState<RepoMappingSummary[]>([]);
  const [fingerprint, setFingerprint] = useState<KeyFingerprint | null>(null);

  // Reset state when switching profiles
  useEffect(() => {
    setTestResult(null);
    setShowEdit(false);
    setShowAddRepo(false);
    setFingerprint(null);
    commands.getRepoMappingsForProfile(profile.id).then(setProfileMappings);
    commands.getKeyFingerprint(profile.id).then(setFingerprint).catch(() => {});
  }, [profile.id]);

  const { confirmProps, confirm } = useConfirm();
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
      toast.success(`Activated "${result.profile_name}"`);
    } catch (err) {
      addLog({ action: "activate", detail: `Failed: ${err}`, level: "error" });
      toast.error("Activation failed", { description: String(err) });
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
        detail: `Connection ${result.success ? "passed" : "failed"} for "${result.profile_name}"`,
        level: result.success ? "info" : "warn",
      });
      if (result.success) {
        toast.success("Connection successful");
      } else {
        toast.error("Connection failed");
      }
    } catch (err) {
      addLog({ action: "test", detail: `Test error: ${err}`, level: "error" });
      toast.error("Test error", { description: String(err) });
    } finally {
      setTesting(false);
    }
  };

  const handleDelete = useCallback(async () => {
    const ok = await confirm({
      title: `Delete "${profile.name}"?`,
      description: "This will remove the profile, its repo mappings, and passphrase from keyring. This action cannot be undone.",
      confirmLabel: "Delete",
      variant: "danger",
    });
    if (!ok) return;
    setDeleting(true);
    try {
      await deleteProfile(profile.id);
      addLog({ action: "delete", detail: `Deleted "${profile.name}"`, level: "info" });
      toast.success(`Deleted "${profile.name}"`);
    } catch (err) {
      addLog({ action: "delete", detail: `Failed: ${err}`, level: "error" });
      toast.error("Delete failed");
    } finally {
      setDeleting(false);
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [profile.id, profile.name, confirm]);

  const copyKeyPath = () => {
    navigator.clipboard.writeText(String(profile.private_key_path));
    toast.info("Key path copied");
  };

  const copyPublicKey = async () => {
    try {
      const pubKey = await commands.readPublicKey(profile.id);
      await navigator.clipboard.writeText(pubKey);
      toast.success("Public key copied to clipboard");
    } catch {
      toast.error("Could not read public key");
    }
  };

  return (
    <div className="space-y-6 max-w-2xl">
      {/* Header */}
      <div className="flex items-start justify-between gap-4">
        <div className="flex items-center gap-3">
          <div className="w-10 h-10 rounded-xl bg-primary/10 flex items-center justify-center">
            <ProviderIcon provider={profile.provider} size={22} />
          </div>
          <div>
            <h2 className="text-lg font-semibold">{profile.name}</h2>
            <p className="text-sm text-muted-foreground">
              {getProviderLabel(profile.provider)} · {profile.email}
            </p>
          </div>
        </div>
        {isActive ? (
          <span className="flex items-center gap-1.5 px-3 py-1.5 text-xs rounded-lg bg-success/15 text-success font-medium ring-1 ring-success/20">
            <CheckCircle2 size={13} />
            Active
          </span>
        ) : (
          <button
            type="button"
            onClick={handleActivate}
            className="flex items-center gap-1.5 px-4 py-2 text-xs font-medium rounded-lg bg-primary text-primary-foreground hover:bg-primary/90 transition-colors"
          >
            <Zap size={13} />
            Activate
          </button>
        )}
      </div>

      {/* Info Grid */}
      <div className="grid grid-cols-2 gap-3">
        <InfoField icon={<Server size={13} />} label="Host Alias" value={profile.host_alias} mono />
        <InfoField icon={<Globe size={13} />} label="Hostname" value={profile.hostname} mono />
        <InfoField icon={<User size={13} />} label="SSH User" value={profile.ssh_user || "git"} mono />
        <InfoField icon={<Hash size={13} />} label="Port" value={String(profile.port || 22)} mono />
        <InfoField icon={<AtSign size={13} />} label="Git Username" value={profile.git_username} />
        <InfoField
          icon={<KeyRound size={13} />}
          label="Key Type"
          value={profile.has_passphrase ? "With passphrase" : "No passphrase"}
        />
      </div>

      {/* Key Path */}
      <div>
        <div className="flex items-center gap-2 mb-1.5">
          <p className="text-[11px] font-medium text-muted-foreground uppercase tracking-wider">
            SSH Private Key
          </p>
          {profile.vault_key_id && (
            <span className="text-[9px] px-1.5 py-0.5 rounded bg-primary/15 text-primary font-medium">
              Vault Managed
            </span>
          )}
        </div>
        <div className="flex items-center gap-2 p-3 rounded-lg bg-secondary group">
          <code className="text-sm font-mono break-all flex-1">{String(profile.private_key_path)}</code>
          <button
            type="button"
            onClick={copyKeyPath}
            title="Copy path"
            className="p-1.5 rounded-md text-muted-foreground hover:text-foreground hover:bg-accent opacity-0 group-hover:opacity-100 transition-all shrink-0"
          >
            <Copy size={14} />
          </button>
        </div>
      </div>

      {/* Key Fingerprint + Copy Public Key */}
      {fingerprint && (
        <div className="flex items-center gap-3 px-3 py-2 rounded-lg bg-secondary/30 text-xs font-mono text-muted-foreground">
          <span className="text-primary/70">{fingerprint.key_type}</span>
          <span>{fingerprint.hash}</span>
          <span className="text-muted-foreground/50">{fingerprint.bits} bits</span>
          <div className="flex-1" />
          <button
            type="button"
            onClick={copyPublicKey}
            title="Copy public key to clipboard"
            className="text-[10px] font-sans font-medium px-2 py-0.5 rounded bg-primary/10 text-primary hover:bg-primary/20 transition-colors"
          >
            Copy Public Key
          </button>
        </div>
      )}

      {/* Actions */}
      <div className="flex items-center gap-2">
        <button
          type="button"
          onClick={handleTest}
          disabled={testing}
          className="flex items-center gap-1.5 px-3.5 py-2 text-xs font-medium rounded-lg bg-secondary hover:bg-accent transition-colors disabled:opacity-50"
        >
          {testing ? <Loader2 size={13} className="animate-spin" /> : <Plug size={13} />}
          {testing ? "Testing..." : "Test Connection"}
        </button>
        <button
          type="button"
          onClick={() => setShowEdit(true)}
          className="flex items-center gap-1.5 px-3.5 py-2 text-xs font-medium rounded-lg bg-secondary hover:bg-accent transition-colors"
        >
          <Pencil size={13} />
          Edit
        </button>
        <div className="flex-1" />
        <button
          type="button"
          onClick={handleDelete}
          disabled={deleting}
          className="flex items-center gap-1.5 px-3.5 py-2 text-xs font-medium rounded-lg text-destructive/70 hover:text-destructive hover:bg-destructive/10 transition-colors disabled:opacity-50"
        >
          <Trash2 size={13} />
          {deleting ? "Deleting..." : "Delete"}
        </button>
      </div>

      {/* Edit modal */}
      {showEdit && (
        <ProfileForm
          editProfile={profile}
          onClose={async () => {
            setShowEdit(false);
            await fetchProfiles();
            await selectProfile(profile.id);
          }}
        />
      )}

      {/* Test Result */}
      {testResult && (
        <div
          className={`rounded-lg overflow-hidden border ${
            testResult.success ? "border-success/20" : "border-destructive/20"
          }`}
        >
          <div
            className={`flex items-center gap-2 px-4 py-2.5 text-sm font-medium ${
              testResult.success ? "bg-success/10 text-success" : "bg-destructive/10 text-destructive"
            }`}
          >
            {testResult.success ? <CheckCircle2 size={15} /> : <XCircle size={15} />}
            {testResult.success ? "Connection successful" : "Connection failed"}
          </div>
          <pre className="px-4 py-3 text-xs font-mono whitespace-pre-wrap text-muted-foreground bg-secondary/50 max-h-32 overflow-y-auto">
            {testResult.output}
          </pre>
        </div>
      )}

      {/* Mapped Repos */}
      <div>
        <div className="flex items-center justify-between mb-2">
          <p className="text-[11px] font-medium text-muted-foreground uppercase tracking-wider flex items-center gap-1.5">
            <FolderGit2 size={12} />
            Mapped Repositories
            {profileMappings.length > 0 && (
              <span className="text-muted-foreground/50">{profileMappings.length}</span>
            )}
          </p>
          <button
            type="button"
            onClick={() => setShowAddRepo(true)}
            className="flex items-center gap-1 text-[11px] text-primary hover:underline"
          >
            <Plus size={11} />
            Map Repo
          </button>
        </div>
        {profileMappings.length === 0 ? (
          <p className="text-xs text-muted-foreground/50 py-2">
            No repositories mapped to this profile
          </p>
        ) : (
          <div className="space-y-1">
            {profileMappings.map((m) => (
              <div
                key={m.id}
                className="group flex items-center gap-2 px-3 py-2 rounded-lg bg-secondary/30 text-xs"
              >
                <FolderGit2 size={13} className="text-muted-foreground shrink-0" />
                <span className="font-medium">{m.repo_name}</span>
                <span className="text-muted-foreground font-mono truncate flex-1">
                  {m.repo_path}
                </span>
                <span className={`text-[9px] px-1.5 py-0.5 rounded shrink-0 ${
                  m.git_config_scope === "local"
                    ? "bg-primary/15 text-primary"
                    : "bg-muted text-muted-foreground"
                }`}>
                  {m.git_config_scope}
                </span>
                <button
                  type="button"
                  onClick={async () => {
                    await commands.deleteRepoMapping(m.id);
                    setProfileMappings((prev) => prev.filter((p) => p.id !== m.id));
                    toast.success(`Removed ${m.repo_name}`);
                  }}
                  title="Remove"
                  className="text-muted-foreground/30 hover:text-destructive opacity-0 group-hover:opacity-100 transition-all shrink-0"
                >
                  <Trash2 size={12} />
                </button>
              </div>
            ))}
          </div>
        )}
      </div>

      {/* Add repo mapping modal */}
      {showAddRepo && (
        <AddRepoMappingDialog
          preselectedProfileId={profile.id}
          onClose={async () => {
            setShowAddRepo(false);
            const updated = await commands.getRepoMappingsForProfile(profile.id);
            setProfileMappings(updated);
          }}
        />
      )}

      {/* Metadata */}
      <div className="text-[11px] text-muted-foreground/60 pt-2 border-t">
        Created {new Date(profile.created_at).toLocaleString()} · Updated{" "}
        {new Date(profile.updated_at).toLocaleString()}
      </div>

      <ConfirmDialog {...confirmProps} />
    </div>
  );
}

function InfoField({
  icon,
  label,
  value,
  mono,
}: {
  icon: React.ReactNode;
  label: string;
  value: string;
  mono?: boolean;
}) {
  return (
    <div className="p-3 rounded-lg bg-secondary/50">
      <div className="flex items-center gap-1.5 mb-1">
        <span className="text-muted-foreground">{icon}</span>
        <p className="text-[10px] font-medium text-muted-foreground uppercase tracking-wider">{label}</p>
      </div>
      <p className={`text-sm ${mono ? "font-mono" : ""}`}>{value}</p>
    </div>
  );
}
