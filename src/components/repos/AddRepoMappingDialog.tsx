import { useState, useEffect, useCallback } from "react";
import { X, FolderGit2, AlertCircle, FolderSearch, CheckCircle2 } from "lucide-react";
import { toast } from "sonner";
import { commands } from "@/lib/tauri-commands";
import { useRepoMappingStore } from "@/stores/repoMappingStore";
import { useProfileStore } from "@/stores/profileStore";
import { useLogStore } from "@/stores/logStore";
import { ProviderIcon } from "@/components/profiles/ProviderIcon";
import type { GitConfigScope } from "@/types";

interface AddRepoMappingDialogProps {
  onClose: () => void;
  preselectedProfileId?: string;
}

export function AddRepoMappingDialog({ onClose, preselectedProfileId }: AddRepoMappingDialogProps) {
  const createMapping = useRepoMappingStore((s) => s.createMapping);
  const profiles = useProfileStore((s) => s.profiles);
  const addLog = useLogStore((s) => s.addLog);

  const [repoPath, setRepoPath] = useState("");
  const [resolvedPath, setResolvedPath] = useState<string | null>(null);
  const [validating, setValidating] = useState(false);
  const [profileId, setProfileId] = useState(preselectedProfileId ?? profiles[0]?.id ?? "");
  const [scope, setScope] = useState<GitConfigScope>("local");
  const [submitting, setSubmitting] = useState(false);
  const [error, setError] = useState<string | null>(null);

  // Escape to close
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

  // Validate repo path when it changes
  useEffect(() => {
    if (!repoPath.trim()) {
      setResolvedPath(null);
      return;
    }
    const timer = setTimeout(async () => {
      setValidating(true);
      try {
        const resolved = await commands.resolveRepoPath(repoPath.trim());
        setResolvedPath(resolved);
      } catch {
        setResolvedPath(null);
      } finally {
        setValidating(false);
      }
    }, 400);
    return () => clearTimeout(timer);
  }, [repoPath]);

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!resolvedPath || !profileId) return;
    setError(null);
    setSubmitting(true);

    try {
      const mapping = await createMapping({
        repo_path: resolvedPath,
        profile_id: profileId,
        git_config_scope: scope,
      });
      addLog({
        action: "map",
        detail: `Mapped ${mapping.repo_name} → ${profiles.find((p) => p.id === profileId)?.name}`,
        level: "info",
      });
      toast.success(`Mapped ${mapping.repo_name}`);
      onClose();
    } catch (err) {
      setError(String(err));
      toast.error("Failed to create mapping", { description: String(err) });
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
      <div role="dialog" aria-modal="true" aria-label="Map repository" className="bg-card border rounded-xl shadow-2xl shadow-black/40 w-120 max-h-[85vh] overflow-hidden flex flex-col">
        {/* Header */}
        <div className="flex items-center justify-between px-5 py-3.5 border-b">
          <div className="flex items-center gap-2">
            <FolderGit2 size={16} className="text-primary" />
            <h3 className="text-sm font-semibold">Add Repo Mapping</h3>
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

          {/* Repo Path */}
          <div>
            <label className="block text-xs font-medium text-muted-foreground mb-1.5">
              Repository Path
            </label>
            <div className="relative">
              <input
                value={repoPath}
                onChange={(e) => setRepoPath(e.target.value)}
                placeholder="C:\Users\you\projects\my-repo"
                required
                autoFocus
                className="w-full px-3 py-2 pr-8 rounded-lg bg-secondary border border-transparent text-sm font-mono focus:outline-none focus:ring-1 focus:ring-ring focus:border-ring placeholder:text-muted-foreground/40"
              />
              <div className="absolute right-2.5 top-1/2 -translate-y-1/2">
                {validating && <FolderSearch size={14} className="text-muted-foreground animate-pulse" />}
                {!validating && resolvedPath && <CheckCircle2 size={14} className="text-success" />}
                {!validating && repoPath && !resolvedPath && <AlertCircle size={14} className="text-destructive/60" />}
              </div>
            </div>
            {resolvedPath && (
              <p className="text-[11px] text-success mt-1 font-mono">
                Git root: {resolvedPath}
              </p>
            )}
            {repoPath && !validating && !resolvedPath && (
              <p className="text-[11px] text-destructive/60 mt-1">
                Not a git repository
              </p>
            )}
          </div>

          {/* Profile */}
          <div>
            <label className="block text-xs font-medium text-muted-foreground mb-1.5">
              Profile
            </label>
            <div className="space-y-1">
              {profiles.map((p) => (
                <button
                  key={p.id}
                  type="button"
                  onClick={() => setProfileId(p.id)}
                  className={`w-full text-left flex items-center gap-2.5 px-3 py-2 rounded-lg text-sm transition-all ${
                    profileId === p.id
                      ? "bg-primary/10 ring-1 ring-primary/25"
                      : "bg-secondary/50 hover:bg-secondary"
                  }`}
                >
                  <ProviderIcon provider={p.provider} size={15} />
                  <span className="font-medium">{p.name}</span>
                  <span className="text-[11px] text-muted-foreground">{p.email}</span>
                </button>
              ))}
            </div>
          </div>

          {/* Scope */}
          <div>
            <label className="block text-xs font-medium text-muted-foreground mb-1.5">
              Git Config Scope
            </label>
            <div className="flex gap-2">
              <button
                type="button"
                onClick={() => setScope("local")}
                className={`flex-1 px-3 py-2 rounded-lg text-xs font-medium text-center transition-all ${
                  scope === "local"
                    ? "bg-primary/15 text-primary ring-1 ring-primary/30"
                    : "bg-secondary text-muted-foreground hover:text-foreground"
                }`}
              >
                Local (repo only)
              </button>
              <button
                type="button"
                onClick={() => setScope("global")}
                className={`flex-1 px-3 py-2 rounded-lg text-xs font-medium text-center transition-all ${
                  scope === "global"
                    ? "bg-primary/15 text-primary ring-1 ring-primary/30"
                    : "bg-secondary text-muted-foreground hover:text-foreground"
                }`}
              >
                Global
              </button>
            </div>
            <p className="text-[10px] text-muted-foreground mt-1.5">
              {scope === "local"
                ? "Sets git user.name/email only for this repository"
                : "Sets git user.name/email globally when auto-switching"}
            </p>
          </div>
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
            disabled={submitting || !resolvedPath || !profileId}
            onClick={handleSubmit}
            className="px-4 py-2 text-xs font-medium rounded-lg bg-primary text-primary-foreground hover:bg-primary/90 transition-colors disabled:opacity-40 disabled:cursor-not-allowed"
          >
            {submitting ? "Creating..." : "Create Mapping"}
          </button>
        </div>
      </div>
    </div>
  );
}
