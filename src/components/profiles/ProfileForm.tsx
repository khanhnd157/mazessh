import { useState, useEffect, useCallback } from "react";
import { X, KeyRound, AlertCircle, Pencil } from "lucide-react";
import { toast } from "sonner";
import { useProfileStore } from "@/stores/profileStore";
import { useLogStore } from "@/stores/logStore";
import type { CreateProfileInput, DetectedKey, Provider, SshProfile } from "@/types";
import { getProviderHostname } from "@/types";
import { ProviderIcon } from "./ProviderIcon";

interface ProfileFormProps {
  onClose: () => void;
  editProfile?: SshProfile | null;
}

const providers: { value: Provider; label: string }[] = [
  { value: "github", label: "GitHub" },
  { value: "gitlab", label: "GitLab" },
  { value: "gitea", label: "Gitea" },
  { value: "bitbucket", label: "Bitbucket" },
];

export function ProfileForm({ onClose, editProfile }: ProfileFormProps) {
  const { createProfile, updateProfile, scanKeys, detectedKeys } = useProfileStore();
  const { addLog } = useLogStore();

  const isEdit = !!editProfile;

  const [name, setName] = useState(editProfile?.name ?? "");
  const [provider, setProvider] = useState<Provider>(editProfile?.provider ?? "github");
  const [email, setEmail] = useState(editProfile?.email ?? "");
  const [gitUsername, setGitUsername] = useState(editProfile?.git_username ?? "");
  const [privateKeyPath, setPrivateKeyPath] = useState(
    editProfile ? String(editProfile.private_key_path) : "",
  );
  const [hostAlias, setHostAlias] = useState(editProfile?.host_alias ?? "");
  const [hostname, setHostname] = useState(editProfile?.hostname ?? "github.com");
  const [hasPassphrase, setHasPassphrase] = useState(editProfile?.has_passphrase ?? false);
  const [submitting, setSubmitting] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!isEdit) scanKeys();
  }, [scanKeys, isEdit]);

  // Auto-fill hostname from provider (only in create mode)
  useEffect(() => {
    if (isEdit) return;
    const host = getProviderHostname(provider);
    if (host) setHostname(host);
  }, [provider, isEdit]);

  // Auto-fill host alias from name (only in create mode)
  useEffect(() => {
    if (isEdit) return;
    if (name) {
      setHostAlias(name.toLowerCase().replace(/\s+/g, "-"));
    }
  }, [name, isEdit]);

  const handleKeyDown = useCallback(
    (e: KeyboardEvent) => {
      if (e.key === "Escape") onClose();
    },
    [onClose],
  );
  useEffect(() => {
    document.addEventListener("keydown", handleKeyDown);
    return () => document.removeEventListener("keydown", handleKeyDown);
  }, [handleKeyDown]);

  const selectDetectedKey = (key: DetectedKey) => {
    setPrivateKeyPath(key.private_key_path);
    if (key.comment && key.comment.includes("@")) {
      setEmail(key.comment);
    }
  };

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setError(null);
    setSubmitting(true);

    try {
      if (isEdit && editProfile) {
        await updateProfile(editProfile.id, {
          name,
          provider,
          email,
          git_username: gitUsername,
          host_alias: hostAlias,
          hostname,
        });
        addLog({ action: "update", detail: `Updated profile "${name}"`, level: "info" });
        toast.success(`Profile "${name}" updated`);
      } else {
        const input: CreateProfileInput = {
          name,
          provider,
          email,
          git_username: gitUsername,
          private_key_path: privateKeyPath,
          host_alias: hostAlias,
          hostname,
          port: null,
          ssh_user: null,
          has_passphrase: hasPassphrase,
        };
        await createProfile(input);
        addLog({ action: "create", detail: `Created profile "${name}"`, level: "info" });
        toast.success(`Profile "${name}" created`);
      }
      onClose();
    } catch (err) {
      setError(String(err));
      toast.error(isEdit ? "Failed to update" : "Failed to create", {
        description: String(err),
      });
    } finally {
      setSubmitting(false);
    }
  };

  return (
    <div
      className="fixed inset-0 bg-black/60 backdrop-blur-sm flex items-center justify-center z-50"
      onClick={(e) => {
        if (e.target === e.currentTarget) onClose();
      }}
    >
      <div role="dialog" aria-modal="true" aria-label={isEdit ? "Edit profile" : "Create profile"} className="bg-card border rounded-xl shadow-2xl shadow-black/40 w-120 max-h-[85vh] overflow-hidden flex flex-col">
        {/* Header */}
        <div className="flex items-center justify-between px-5 py-3.5 border-b">
          <div className="flex items-center gap-2">
            {isEdit ? (
              <Pencil size={16} className="text-primary" />
            ) : (
              <KeyRound size={16} className="text-primary" />
            )}
            <h3 className="text-sm font-semibold">
              {isEdit ? "Edit Profile" : "New SSH Profile"}
            </h3>
          </div>
          <button
            type="button"
            onClick={onClose}
            title="Close"
            className="p-1 rounded-md text-muted-foreground hover:text-foreground hover:bg-secondary transition-colors"
          >
            <X size={16} />
          </button>
        </div>

        {/* Form */}
        <form onSubmit={handleSubmit} className="flex-1 overflow-y-auto p-5 space-y-4">
          {error && (
            <div className="flex items-start gap-2 p-3 rounded-lg bg-destructive/10 text-destructive text-sm">
              <AlertCircle size={16} className="shrink-0 mt-0.5" />
              <span>{error}</span>
            </div>
          )}

          {/* Name */}
          <div>
            <label className="block text-xs font-medium text-muted-foreground mb-1.5">
              Profile Name
            </label>
            <input
              value={name}
              onChange={(e) => setName(e.target.value)}
              placeholder="e.g. Work GitHub"
              required
              autoFocus
              className="w-full px-3 py-2 rounded-lg bg-secondary border border-transparent text-sm focus:outline-none focus:ring-1 focus:ring-ring focus:border-ring placeholder:text-muted-foreground/40"
            />
          </div>

          {/* Provider */}
          <div>
            <label className="block text-xs font-medium text-muted-foreground mb-1.5">
              Provider
            </label>
            <div className="flex gap-2">
              {providers.map((p) => {
                const isSelected =
                  typeof provider === "string" &&
                  typeof p.value === "string" &&
                  provider === p.value;
                return (
                  <button
                    key={typeof p.value === "string" ? p.value : "custom"}
                    type="button"
                    onClick={() => setProvider(p.value)}
                    className={`flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-xs font-medium transition-all ${
                      isSelected
                        ? "bg-primary/15 text-primary ring-1 ring-primary/30"
                        : "bg-secondary text-muted-foreground hover:text-foreground hover:bg-secondary/80"
                    }`}
                  >
                    <ProviderIcon provider={p.value} size={14} />
                    {p.label}
                  </button>
                );
              })}
            </div>
          </div>

          {/* Email + Git Username */}
          <div className="grid grid-cols-2 gap-3">
            <div>
              <label className="block text-xs font-medium text-muted-foreground mb-1.5">
                Email
              </label>
              <input
                value={email}
                onChange={(e) => setEmail(e.target.value)}
                placeholder="user@example.com"
                required
                className="w-full px-3 py-2 rounded-lg bg-secondary border border-transparent text-sm focus:outline-none focus:ring-1 focus:ring-ring focus:border-ring placeholder:text-muted-foreground/40"
              />
            </div>
            <div>
              <label className="block text-xs font-medium text-muted-foreground mb-1.5">
                Git Username
              </label>
              <input
                value={gitUsername}
                onChange={(e) => setGitUsername(e.target.value)}
                placeholder="username"
                required
                className="w-full px-3 py-2 rounded-lg bg-secondary border border-transparent text-sm focus:outline-none focus:ring-1 focus:ring-ring focus:border-ring placeholder:text-muted-foreground/40"
              />
            </div>
          </div>

          {/* SSH Key (only in create mode) */}
          {!isEdit && (
            <div>
              <label className="block text-xs font-medium text-muted-foreground mb-1.5">
                SSH Private Key
              </label>
              <input
                value={privateKeyPath}
                onChange={(e) => setPrivateKeyPath(e.target.value)}
                placeholder="C:\Users\you\.ssh\id_ed25519"
                required
                className="w-full px-3 py-2 rounded-lg bg-secondary border border-transparent text-sm font-mono focus:outline-none focus:ring-1 focus:ring-ring focus:border-ring placeholder:text-muted-foreground/40"
              />
              {detectedKeys.length > 0 && (
                <div className="mt-2 space-y-1">
                  <p className="text-[10px] text-muted-foreground font-medium uppercase tracking-wider">
                    Detected Keys
                  </p>
                  {detectedKeys.map((key) => (
                    <button
                      key={key.private_key_path}
                      type="button"
                      onClick={() => selectDetectedKey(key)}
                      className={`w-full text-left px-2.5 py-2 text-xs rounded-lg transition-all flex items-center gap-2 ${
                        privateKeyPath === key.private_key_path
                          ? "bg-primary/10 ring-1 ring-primary/25"
                          : "bg-secondary/50 hover:bg-secondary"
                      }`}
                    >
                      <KeyRound size={12} className="text-muted-foreground shrink-0" />
                      <div className="min-w-0">
                        <span className="font-mono truncate block">
                          {key.private_key_path}
                        </span>
                        <span className="text-muted-foreground">
                          {key.key_type} {key.comment && `· ${key.comment}`}
                        </span>
                      </div>
                    </button>
                  ))}
                </div>
              )}
            </div>
          )}

          {/* Host Alias + Hostname */}
          <div className="grid grid-cols-2 gap-3">
            <div>
              <label className="block text-xs font-medium text-muted-foreground mb-1.5">
                Host Alias
              </label>
              <input
                value={hostAlias}
                onChange={(e) => setHostAlias(e.target.value)}
                placeholder="github-work"
                required
                className="w-full px-3 py-2 rounded-lg bg-secondary border border-transparent text-sm font-mono focus:outline-none focus:ring-1 focus:ring-ring focus:border-ring placeholder:text-muted-foreground/40"
              />
            </div>
            <div>
              <label className="block text-xs font-medium text-muted-foreground mb-1.5">
                Hostname
              </label>
              <input
                value={hostname}
                onChange={(e) => setHostname(e.target.value)}
                placeholder="github.com"
                required
                className="w-full px-3 py-2 rounded-lg bg-secondary border border-transparent text-sm font-mono focus:outline-none focus:ring-1 focus:ring-ring focus:border-ring placeholder:text-muted-foreground/40"
              />
            </div>
          </div>

          {/* Passphrase (only in create mode) */}
          {!isEdit && (
            <label className="flex items-center gap-2 cursor-pointer">
              <input
                type="checkbox"
                checked={hasPassphrase}
                onChange={(e) => setHasPassphrase(e.target.checked)}
                className="rounded border-muted-foreground"
              />
              <span className="text-xs text-muted-foreground">Key has passphrase</span>
            </label>
          )}
        </form>

        {/* Footer */}
        <div className="flex justify-end gap-2 px-5 py-3.5 border-t bg-card">
          <button
            type="button"
            onClick={onClose}
            className="px-4 py-2 text-xs font-medium rounded-lg bg-secondary hover:bg-secondary/80 transition-colors"
          >
            Cancel
          </button>
          <button
            type="submit"
            disabled={submitting || !name || !email || (!isEdit && !privateKeyPath)}
            onClick={handleSubmit}
            className="px-4 py-2 text-xs font-medium rounded-lg bg-primary text-primary-foreground hover:bg-primary/90 transition-colors disabled:opacity-40 disabled:cursor-not-allowed"
          >
            {submitting
              ? isEdit
                ? "Saving..."
                : "Creating..."
              : isEdit
                ? "Save Changes"
                : "Create Profile"}
          </button>
        </div>
      </div>
    </div>
  );
}
