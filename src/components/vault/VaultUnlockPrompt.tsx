import { useState, useRef, useEffect } from "react";
import { Lock, Unlock, AlertCircle } from "lucide-react";
import { useVaultStore } from "@/stores/vaultStore";

export function VaultUnlockPrompt() {
  const unlockVault = useVaultStore((s) => s.unlockVault);
  const loading = useVaultStore((s) => s.loading);
  const [passphrase, setPassphrase] = useState("");
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

  const handleUnlock = async () => {
    try {
      setError("");
      await unlockVault(passphrase);
    } catch (e) {
      setError(String(e));
      triggerShake();
    }
  };

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === "Enter" && passphrase) {
      handleUnlock();
    }
  };

  return (
    <div className="flex items-center justify-center h-full">
      <div className={`w-72 text-center ${shake ? "animate-shake" : ""}`}>
        <div className="w-14 h-14 rounded-2xl bg-primary/8 flex items-center justify-center mx-auto mb-4">
          <Lock size={24} className="text-primary/60" />
        </div>
        <h2 className="text-base font-semibold mb-0.5">Vault Locked</h2>
        <p className="text-xs text-muted-foreground/60 mb-5">
          Enter your vault passphrase to access SSH keys
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
            placeholder="Vault passphrase"
            className="w-full px-4 py-2.5 rounded-xl bg-secondary border border-border text-center text-sm tracking-[0.15em] focus:outline-none focus:ring-2 focus:ring-ring focus:border-transparent placeholder:tracking-normal placeholder:text-muted-foreground/30"
          />
          <button
            type="button"
            onClick={handleUnlock}
            disabled={loading || !passphrase}
            className="w-full flex items-center justify-center gap-2 px-4 py-2.5 rounded-xl bg-primary text-primary-foreground font-medium text-sm hover:bg-primary/90 transition-colors disabled:opacity-30"
          >
            <Unlock size={14} />
            {loading ? "Unlocking..." : "Unlock Vault"}
          </button>
        </div>
      </div>
    </div>
  );
}
