import { useState } from "react";
import { Lock, AlertCircle } from "lucide-react";
import { toast } from "sonner";
import { commands } from "@/lib/tauri-commands";

export function VaultPassphraseChange() {
  const [oldPass, setOldPass] = useState("");
  const [newPass, setNewPass] = useState("");
  const [confirmPass, setConfirmPass] = useState("");
  const [error, setError] = useState("");
  const [loading, setLoading] = useState(false);
  const [expanded, setExpanded] = useState(false);

  const handleChange = async () => {
    setError("");
    if (newPass.length < 4) {
      setError("New passphrase must be at least 4 characters");
      return;
    }
    if (newPass !== confirmPass) {
      setError("Passphrases do not match");
      return;
    }
    setLoading(true);
    try {
      await commands.vaultChangePassphrase(oldPass, newPass);
      toast.success("Vault passphrase changed");
      setOldPass("");
      setNewPass("");
      setConfirmPass("");
      setExpanded(false);
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  };

  if (!expanded) {
    return (
      <button
        type="button"
        onClick={() => setExpanded(true)}
        className="flex items-center gap-1.5 px-3 py-1.5 text-xs font-medium rounded-lg bg-secondary hover:bg-accent transition-colors"
      >
        <Lock size={12} />
        Change Vault Passphrase
      </button>
    );
  }

  return (
    <div className="space-y-2.5 p-3 rounded-lg bg-secondary/40 border border-border">
      <h4 className="text-xs font-medium flex items-center gap-1.5">
        <Lock size={12} className="text-muted-foreground" />
        Change Vault Passphrase
      </h4>

      {error && (
        <div className="flex items-center gap-1.5 text-destructive text-xs animate-fade-in">
          <AlertCircle size={12} />
          {error}
        </div>
      )}

      <input
        type="password"
        value={oldPass}
        onChange={(e) => { setOldPass(e.target.value); setError(""); }}
        placeholder="Current passphrase"
        className="w-full px-3 py-2 rounded-lg bg-secondary border border-border text-sm focus:outline-none focus:ring-2 focus:ring-ring"
      />
      <input
        type="password"
        value={newPass}
        onChange={(e) => { setNewPass(e.target.value); setError(""); }}
        placeholder="New passphrase (4+ chars)"
        className="w-full px-3 py-2 rounded-lg bg-secondary border border-border text-sm focus:outline-none focus:ring-2 focus:ring-ring"
      />
      <input
        type="password"
        value={confirmPass}
        onChange={(e) => { setConfirmPass(e.target.value); setError(""); }}
        placeholder="Confirm new passphrase"
        className="w-full px-3 py-2 rounded-lg bg-secondary border border-border text-sm focus:outline-none focus:ring-2 focus:ring-ring"
      />

      <div className="flex gap-2">
        <button
          type="button"
          onClick={() => { setExpanded(false); setError(""); setOldPass(""); setNewPass(""); setConfirmPass(""); }}
          className="px-3 py-1.5 text-xs font-medium rounded-lg bg-secondary hover:bg-accent transition-colors"
        >
          Cancel
        </button>
        <button
          type="button"
          onClick={handleChange}
          disabled={loading || !oldPass || !newPass || !confirmPass}
          className="px-3 py-1.5 text-xs font-medium rounded-lg bg-primary text-primary-foreground hover:bg-primary/90 transition-colors disabled:opacity-30"
        >
          {loading ? "Changing..." : "Change Passphrase"}
        </button>
      </div>
    </div>
  );
}
