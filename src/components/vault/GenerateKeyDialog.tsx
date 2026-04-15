import { useState, useCallback, useEffect } from "react";
import { KeyRound, X } from "lucide-react";
import { toast } from "sonner";
import { useVaultStore } from "@/stores/vaultStore";
import type { KeyAlgorithm } from "@/types";

interface Props {
  onClose: () => void;
}

export function GenerateKeyDialog({ onClose }: Props) {
  const { generateKey } = useVaultStore();
  const [name, setName] = useState("");
  const [algorithm, setAlgorithm] = useState<KeyAlgorithm>("ed25519");
  const [comment, setComment] = useState("");
  const [allowedHosts, setAllowedHosts] = useState("");
  const [submitting, setSubmitting] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const handleKeyDown = useCallback(
    (e: KeyboardEvent) => { if (e.key === "Escape") onClose(); },
    [onClose],
  );
  useEffect(() => {
    document.addEventListener("keydown", handleKeyDown);
    return () => document.removeEventListener("keydown", handleKeyDown);
  }, [handleKeyDown]);

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setError(null);
    setSubmitting(true);
    try {
      const hosts = allowedHosts.split(",").map((h) => h.trim()).filter(Boolean);
      const item = await generateKey({
        name,
        algorithm,
        comment: comment || null,
        allowed_hosts: hosts.length > 0 ? hosts : undefined,
      });
      await navigator.clipboard.writeText(item.public_key_openssh);
      toast.success(`Key "${name}" generated`, { description: "Public key copied to clipboard" });
      onClose();
    } catch (err) {
      setError(String(err));
    } finally {
      setSubmitting(false);
    }
  };

  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center bg-black/50 backdrop-blur-sm"
      onClick={(e) => { if (e.target === e.currentTarget) onClose(); }}
    >
      <form
        onSubmit={handleSubmit}
        className="bg-card border rounded-xl shadow-2xl shadow-black/30 w-[420px] overflow-hidden animate-fade-in"
      >
        {/* Header */}
        <div className="flex items-start gap-3 px-5 pt-5 pb-3">
          <div className="w-9 h-9 rounded-lg bg-primary/15 flex items-center justify-center shrink-0 mt-0.5">
            <KeyRound size={18} className="text-primary" />
          </div>
          <div className="flex-1 min-w-0">
            <h3 className="text-sm font-semibold leading-tight">Generate SSH Key</h3>
            <p className="text-xs text-muted-foreground mt-1">Create a new SSH key pair stored in the vault</p>
          </div>
          <button type="button" onClick={onClose} title="Close" className="p-1 rounded-md text-muted-foreground/50 hover:text-foreground hover:bg-secondary transition-colors shrink-0 -mt-0.5">
            <X size={14} />
          </button>
        </div>

        {/* Body */}
        <div className="px-5 pb-4 space-y-3">
          {error && (
            <div className="text-xs text-destructive bg-destructive/10 rounded-lg px-3 py-2 animate-fade-in">
              {error}
            </div>
          )}

          <div>
            <label className="text-xs font-medium text-muted-foreground mb-1 block">Name</label>
            <input
              autoFocus
              value={name}
              onChange={(e) => setName(e.target.value)}
              placeholder="e.g. GitHub Personal"
              required
              className="w-full px-3 py-2 rounded-lg bg-secondary border border-border text-sm focus:outline-none focus:ring-2 focus:ring-ring"
            />
          </div>

          <div>
            <label className="text-xs font-medium text-muted-foreground mb-1.5 block">Algorithm</label>
            <div className="flex gap-2">
              {(["ed25519", "rsa4096"] as const).map((alg) => (
                <button
                  key={alg}
                  type="button"
                  onClick={() => setAlgorithm(alg)}
                  className={`flex-1 px-3 py-2 rounded-lg text-xs font-medium transition-colors border ${
                    algorithm === alg
                      ? "bg-primary/15 border-primary/30 text-primary"
                      : "bg-secondary border-border text-muted-foreground hover:text-foreground"
                  }`}
                >
                  {alg === "ed25519" ? "Ed25519" : "RSA 4096"}
                </button>
              ))}
            </div>
          </div>

          <div>
            <label className="text-xs font-medium text-muted-foreground mb-1 block">Comment (optional)</label>
            <input
              value={comment}
              onChange={(e) => setComment(e.target.value)}
              placeholder="user@hostname"
              className="w-full px-3 py-2 rounded-lg bg-secondary border border-border text-sm focus:outline-none focus:ring-2 focus:ring-ring"
            />
          </div>

          <div>
            <label className="text-xs font-medium text-muted-foreground mb-1 block">Allowed Hosts (optional)</label>
            <input
              value={allowedHosts}
              onChange={(e) => setAllowedHosts(e.target.value)}
              placeholder="github.com, gitlab.com"
              className="w-full px-3 py-2 rounded-lg bg-secondary border border-border text-sm focus:outline-none focus:ring-2 focus:ring-ring"
            />
            <p className="text-[10px] text-muted-foreground/40 mt-1">
              Comma-separated. Leave empty to allow all hosts.
            </p>
          </div>
        </div>

        {/* Footer */}
        <div className="flex justify-end gap-2 px-5 py-3.5 border-t bg-secondary/30">
          <button type="button" onClick={onClose} className="px-3.5 py-1.5 text-xs font-medium rounded-lg bg-secondary hover:bg-accent transition-colors">
            Cancel
          </button>
          <button
            type="submit"
            disabled={submitting || !name}
            className="px-3.5 py-1.5 text-xs font-medium rounded-lg bg-primary text-primary-foreground hover:bg-primary/90 transition-colors disabled:opacity-30"
          >
            {submitting ? "Generating..." : "Generate Key"}
          </button>
        </div>
      </form>
    </div>
  );
}
