import { useState, useEffect } from "react";
import { useProfileStore } from "@/stores/profileStore";
import { useLogStore } from "@/stores/logStore";
import type { CreateProfileInput, DetectedKey, Provider } from "@/types";
import { getProviderHostname } from "@/types";

interface ProfileFormProps {
  onClose: () => void;
}

const providers: { value: Provider; label: string }[] = [
  { value: "github", label: "GitHub" },
  { value: "gitlab", label: "GitLab" },
  { value: "gitea", label: "Gitea" },
  { value: "bitbucket", label: "Bitbucket" },
];

export function ProfileForm({ onClose }: ProfileFormProps) {
  const { createProfile, scanKeys, detectedKeys } = useProfileStore();
  const { addLog } = useLogStore();

  const [name, setName] = useState("");
  const [provider, setProvider] = useState<Provider>("github");
  const [email, setEmail] = useState("");
  const [gitUsername, setGitUsername] = useState("");
  const [privateKeyPath, setPrivateKeyPath] = useState("");
  const [hostAlias, setHostAlias] = useState("");
  const [hostname, setHostname] = useState("github.com");
  const [hasPassphrase, setHasPassphrase] = useState(false);
  const [submitting, setSubmitting] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    scanKeys();
  }, [scanKeys]);

  useEffect(() => {
    const host = getProviderHostname(provider);
    if (host) setHostname(host);
  }, [provider]);

  useEffect(() => {
    if (name) {
      setHostAlias(name.toLowerCase().replace(/\s+/g, "-"));
    }
  }, [name]);

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
      addLog({
        action: "create",
        detail: `Created profile "${name}"`,
        level: "info",
      });
      onClose();
    } catch (err) {
      setError(String(err));
    } finally {
      setSubmitting(false);
    }
  };

  return (
    <div className="fixed inset-0 bg-black/60 flex items-center justify-center z-50">
      <div className="bg-card border rounded-lg shadow-xl w-[500px] max-h-[90vh] overflow-y-auto">
        <div className="flex items-center justify-between px-6 py-4 border-b">
          <h3 className="text-lg font-semibold">New SSH Profile</h3>
          <button onClick={onClose} className="text-muted-foreground hover:text-foreground">
            ✕
          </button>
        </div>
        <form onSubmit={handleSubmit} className="p-6 space-y-4">
          {error && (
            <div className="p-3 rounded-md bg-destructive/10 text-destructive text-sm">
              {error}
            </div>
          )}

          <div>
            <label className="block text-sm font-medium mb-1.5">Profile Name</label>
            <input
              value={name}
              onChange={(e) => setName(e.target.value)}
              placeholder="e.g. Work GitHub"
              required
              className="w-full px-3 py-2 rounded-md bg-input border text-sm focus:outline-none focus:ring-2 focus:ring-ring"
            />
          </div>

          <div>
            <label className="block text-sm font-medium mb-1.5">Provider</label>
            <select
              value={typeof provider === "string" ? provider : "github"}
              onChange={(e) => setProvider(e.target.value as Provider)}
              className="w-full px-3 py-2 rounded-md bg-input border text-sm focus:outline-none focus:ring-2 focus:ring-ring"
            >
              {providers.map((p) => (
                <option key={typeof p.value === "string" ? p.value : "custom"} value={typeof p.value === "string" ? p.value : "custom"}>
                  {p.label}
                </option>
              ))}
            </select>
          </div>

          <div className="grid grid-cols-2 gap-3">
            <div>
              <label className="block text-sm font-medium mb-1.5">Email</label>
              <input
                value={email}
                onChange={(e) => setEmail(e.target.value)}
                placeholder="user@example.com"
                required
                className="w-full px-3 py-2 rounded-md bg-input border text-sm focus:outline-none focus:ring-2 focus:ring-ring"
              />
            </div>
            <div>
              <label className="block text-sm font-medium mb-1.5">Git Username</label>
              <input
                value={gitUsername}
                onChange={(e) => setGitUsername(e.target.value)}
                placeholder="username"
                required
                className="w-full px-3 py-2 rounded-md bg-input border text-sm focus:outline-none focus:ring-2 focus:ring-ring"
              />
            </div>
          </div>

          <div>
            <label className="block text-sm font-medium mb-1.5">SSH Private Key</label>
            <input
              value={privateKeyPath}
              onChange={(e) => setPrivateKeyPath(e.target.value)}
              placeholder="~/.ssh/id_ed25519"
              required
              className="w-full px-3 py-2 rounded-md bg-input border text-sm focus:outline-none focus:ring-2 focus:ring-ring"
            />
            {detectedKeys.length > 0 && (
              <div className="mt-2 space-y-1">
                <p className="text-xs text-muted-foreground">Detected keys:</p>
                {detectedKeys.map((key) => (
                  <button
                    key={key.private_key_path}
                    type="button"
                    onClick={() => selectDetectedKey(key)}
                    className={`w-full text-left px-2 py-1.5 text-xs rounded border transition-colors ${
                      privateKeyPath === key.private_key_path
                        ? "border-primary bg-primary/10"
                        : "hover:bg-accent"
                    }`}
                  >
                    <span className="font-mono">{key.private_key_path}</span>
                    <span className="text-muted-foreground ml-2">
                      ({key.key_type}) {key.comment}
                    </span>
                  </button>
                ))}
              </div>
            )}
          </div>

          <div className="grid grid-cols-2 gap-3">
            <div>
              <label className="block text-sm font-medium mb-1.5">Host Alias</label>
              <input
                value={hostAlias}
                onChange={(e) => setHostAlias(e.target.value)}
                placeholder="github-work"
                required
                className="w-full px-3 py-2 rounded-md bg-input border text-sm focus:outline-none focus:ring-2 focus:ring-ring"
              />
            </div>
            <div>
              <label className="block text-sm font-medium mb-1.5">Hostname</label>
              <input
                value={hostname}
                onChange={(e) => setHostname(e.target.value)}
                placeholder="github.com"
                required
                className="w-full px-3 py-2 rounded-md bg-input border text-sm focus:outline-none focus:ring-2 focus:ring-ring"
              />
            </div>
          </div>

          <div className="flex items-center gap-2">
            <input
              type="checkbox"
              id="hasPassphrase"
              checked={hasPassphrase}
              onChange={(e) => setHasPassphrase(e.target.checked)}
              className="rounded"
            />
            <label htmlFor="hasPassphrase" className="text-sm">
              Key has passphrase
            </label>
          </div>

          <div className="flex justify-end gap-2 pt-2">
            <button
              type="button"
              onClick={onClose}
              className="px-4 py-2 text-sm rounded-md bg-secondary hover:bg-secondary/80 transition-colors"
            >
              Cancel
            </button>
            <button
              type="submit"
              disabled={submitting}
              className="px-4 py-2 text-sm rounded-md bg-primary text-primary-foreground hover:bg-primary/90 transition-colors disabled:opacity-50"
            >
              {submitting ? "Creating..." : "Create Profile"}
            </button>
          </div>
        </form>
      </div>
    </div>
  );
}
