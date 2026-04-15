import { useState, useRef, useEffect } from "react";
import { Shield, AlertCircle } from "lucide-react";
import { useVaultStore } from "@/stores/vaultStore";
import { toast } from "sonner";

export function VaultSetupPrompt() {
  const { initVault, loading } = useVaultStore();
  const [passphrase, setPassphrase] = useState("");
  const [confirm, setConfirm] = useState("");
  const [error, setError] = useState("");
  const [shake, setShake] = useState(false);
  const inputRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    const t = setTimeout(() => inputRef.current?.focus(), 100);
    return () => clearTimeout(t);
  }, []);

  const triggerShake = () => {
    setShake(true);
    const t = setTimeout(() => setShake(false), 500);
    return () => clearTimeout(t);
  };

  const handleSetup = async () => {
    if (passphrase.length < 4) {
      setError("Passphrase must be at least 4 characters");
      triggerShake();
      return;
    }
    if (passphrase !== confirm) {
      setError("Passphrases do not match");
      triggerShake();
      return;
    }
    try {
      await initVault(passphrase);
      toast.success("Vault initialized");
    } catch (e) {
      setError(String(e));
      triggerShake();
    }
  };

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === "Enter" && passphrase && confirm) {
      handleSetup();
    }
  };

  return (
    <div className="flex items-center justify-center h-full">
      <div className={`w-80 text-center ${shake ? "animate-shake" : ""}`}>
        <div className="w-14 h-14 rounded-2xl bg-primary/8 flex items-center justify-center mx-auto mb-4">
          <Shield size={24} className="text-primary/60" />
        </div>
        <h2 className="text-base font-semibold mb-0.5">Set Up Key Vault</h2>
        <p className="text-xs text-muted-foreground/60 mb-5">
          Create a passphrase to encrypt your SSH keys. This can be the same as your app PIN.
        </p>

        {error && (
          <div className="flex items-center justify-center gap-1.5 text-destructive text-xs mb-3 animate-fade-in">
            <AlertCircle size={12} />
            {error}
          </div>
        )}

        <div className="space-y-2.5">
          <input
            ref={inputRef}
            type="password"
            value={passphrase}
            onChange={(e) => { setPassphrase(e.target.value); setError(""); }}
            onKeyDown={handleKeyDown}
            placeholder="Create passphrase (4+ chars)"
            className="w-full px-4 py-2.5 rounded-xl bg-secondary border border-border text-center text-sm tracking-[0.15em] focus:outline-none focus:ring-2 focus:ring-ring focus:border-transparent placeholder:tracking-normal placeholder:text-muted-foreground/30"
          />
          <input
            type="password"
            value={confirm}
            onChange={(e) => { setConfirm(e.target.value); setError(""); }}
            onKeyDown={handleKeyDown}
            placeholder="Confirm passphrase"
            className="w-full px-4 py-2.5 rounded-xl bg-secondary border border-border text-center text-sm tracking-[0.15em] focus:outline-none focus:ring-2 focus:ring-ring focus:border-transparent placeholder:tracking-normal placeholder:text-muted-foreground/30"
          />
          <button
            type="button"
            onClick={handleSetup}
            disabled={loading || !passphrase || !confirm}
            className="w-full flex items-center justify-center gap-2 px-4 py-2.5 rounded-xl bg-primary text-primary-foreground font-medium text-sm hover:bg-primary/90 transition-colors disabled:opacity-30"
          >
            <Shield size={14} />
            {loading ? "Initializing..." : "Initialize Vault"}
          </button>
        </div>
      </div>
    </div>
  );
}
