import { useState, useCallback, useEffect } from "react";
import { Upload, X } from "lucide-react";
import { toast } from "sonner";
import { useVaultStore } from "@/stores/vaultStore";

interface Props {
  onClose: () => void;
}

export function ImportKeyDialog({ onClose }: Props) {
  const { importKey } = useVaultStore();
  const [name, setName] = useState("");
  const [pemData, setPemData] = useState("");
  const [hasPassphrase, setHasPassphrase] = useState(false);
  const [passphrase, setPassphrase] = useState("");
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
    if (!name || !pemData) return;
    setError(null);
    setSubmitting(true);
    try {
      const item = await importKey({
        name,
        private_key_pem: pemData,
        comment: null,
        source_passphrase: hasPassphrase ? passphrase : null,
        allow_private_export: true,
      });
      await navigator.clipboard.writeText(item.public_key_openssh);
      toast.success(`Key "${name}" imported`, { description: "Public key copied to clipboard" });
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
        className="bg-card border rounded-xl shadow-2xl shadow-black/30 w-[460px] overflow-hidden animate-fade-in"
      >
        {/* Header */}
        <div className="flex items-start gap-3 px-5 pt-5 pb-3">
          <div className="w-9 h-9 rounded-lg bg-primary/15 flex items-center justify-center shrink-0 mt-0.5">
            <Upload size={18} className="text-primary" />
          </div>
          <div className="flex-1 min-w-0">
            <h3 className="text-sm font-semibold leading-tight">Import SSH Key</h3>
            <p className="text-xs text-muted-foreground mt-1">Import an existing private key into the vault</p>
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
              placeholder="e.g. Work GitHub Key"
              required
              className="w-full px-3 py-2 rounded-lg bg-secondary border border-border text-sm focus:outline-none focus:ring-2 focus:ring-ring"
            />
          </div>

          <div>
            <label className="text-xs font-medium text-muted-foreground mb-1 block">Private Key (PEM)</label>
            <textarea
              value={pemData}
              onChange={(e) => setPemData(e.target.value)}
              placeholder={"-----BEGIN OPENSSH PRIVATE KEY-----\n..."}
              rows={6}
              className="w-full px-3 py-2 rounded-lg bg-secondary border border-border text-xs font-mono focus:outline-none focus:ring-2 focus:ring-ring resize-none"
            />
          </div>

          <div className="space-y-2">
            <label className="flex items-center gap-2 cursor-pointer">
              <input
                type="checkbox"
                checked={hasPassphrase}
                onChange={(e) => setHasPassphrase(e.target.checked)}
                className="accent-primary"
              />
              <span className="text-xs text-muted-foreground">Key is passphrase-protected</span>
            </label>
            {hasPassphrase && (
              <input
                type="password"
                value={passphrase}
                onChange={(e) => setPassphrase(e.target.value)}
                placeholder="Key passphrase"
                className="w-full px-3 py-2 rounded-lg bg-secondary border border-border text-sm focus:outline-none focus:ring-2 focus:ring-ring"
              />
            )}
          </div>
        </div>

        {/* Footer */}
        <div className="flex justify-end gap-2 px-5 py-3.5 border-t bg-secondary/30">
          <button type="button" onClick={onClose} className="px-3.5 py-1.5 text-xs font-medium rounded-lg bg-secondary hover:bg-accent transition-colors">
            Cancel
          </button>
          <button
            type="submit"
            disabled={submitting || !name || !pemData}
            className="px-3.5 py-1.5 text-xs font-medium rounded-lg bg-primary text-primary-foreground hover:bg-primary/90 transition-colors disabled:opacity-30"
          >
            {submitting ? "Importing..." : "Import Key"}
          </button>
        </div>
      </form>
    </div>
  );
}
