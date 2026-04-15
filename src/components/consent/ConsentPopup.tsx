import { useState, useEffect, useRef } from "react";
import { Shield, Lock, Unlock, AlertCircle, Clock } from "lucide-react";
import { commands } from "@/lib/tauri-commands";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import type { VaultStateResponse, SshKeyItemSummary } from "@/types";

type AllowMode = "once" | "session" | "always";

interface PendingConsent {
  consent_id: string;
  key_id: string;
  key_name: string;
  process_name: string;
  host: string;
}

export function ConsentPopup() {
  const [vaultState, setVaultState] = useState<VaultStateResponse | null>(null);
  const [passphrase, setPassphrase] = useState("");
  const [unlockError, setUnlockError] = useState("");
  const [unlocking, setUnlocking] = useState(false);

  const [pending, setPending] = useState<PendingConsent | null>(null);
  const [keys, setKeys] = useState<SshKeyItemSummary[]>([]);
  const [selectedKeyId, setSelectedKeyId] = useState("");
  const [allowMode, setAllowMode] = useState<AllowMode>("once");
  const [countdown, setCountdown] = useState(60);
  const [submitting, setSubmitting] = useState(false);
  const inputRef = useRef<HTMLInputElement>(null);

  // Fetch vault state and pending consent on mount
  useEffect(() => {
    commands.vaultGetState().then(setVaultState).catch(() => {});
    fetchPending();

    // Listen for new consent requests
    const unlisten = listen<{ consent_id: string }>("consent-request", () => {
      fetchPending();
    });

    return () => { unlisten.then((fn) => fn()); };
  }, []);

  const fetchPending = async () => {
    try {
      const p = await commands.getPendingConsent();
      if (p) {
        setPending(p);
        setSelectedKeyId(p.key_id);
      }
    } catch {
      // ignore
    }
  };

  // Fetch keys when vault is unlocked
  useEffect(() => {
    if (vaultState?.unlocked) {
      commands.vaultListKeys().then(setKeys).catch(() => {});
      fetchPending();
    }
  }, [vaultState?.unlocked]);

  // Auto-focus passphrase input
  useEffect(() => {
    if (vaultState && !vaultState.unlocked) {
      setTimeout(() => inputRef.current?.focus(), 100);
    }
  }, [vaultState]);

  // 60s countdown timer
  useEffect(() => {
    if (countdown <= 0) {
      handleDeny();
      return;
    }
    const timer = setInterval(() => setCountdown((c) => c - 1), 1000);
    return () => clearInterval(timer);
  }, [countdown]);

  const handleUnlock = async () => {
    setUnlocking(true);
    setUnlockError("");
    try {
      await commands.vaultUnlock(passphrase);
      const state = await commands.vaultGetState();
      setVaultState(state);
    } catch (e) {
      setUnlockError(String(e));
    } finally {
      setUnlocking(false);
    }
  };

  const handleAllow = async () => {
    if (!pending) return;
    setSubmitting(true);
    try {
      await commands.respondToConsent(pending.consent_id, true, selectedKeyId, allowMode);
      getCurrentWindow().close();
    } catch {
      setSubmitting(false);
    }
  };

  const handleDeny = async () => {
    try {
      if (pending) {
        await commands.respondToConsent(pending.consent_id, false, "", "once");
      }
      getCurrentWindow().close();
    } catch {
      // ignore close errors
    }
  };

  const appWindow = getCurrentWindow();

  return (
    <div className="h-screen flex flex-col bg-background select-none">
      {/* Mini titlebar */}
      <div className="flex items-center justify-between h-8 px-3 shrink-0 titlebar-bg" data-tauri-drag-region>
        <div className="flex items-center gap-1.5" data-tauri-drag-region>
          <Shield size={12} className="text-primary" />
          <span className="text-[10px] font-medium text-muted-foreground">SSH Signing Request</span>
        </div>
        <div className="flex items-center gap-1">
          <div className="flex items-center gap-1 text-[10px] text-warning">
            <Clock size={10} />
            {countdown}s
          </div>
          <button
            type="button"
            onClick={() => appWindow.close()}
            title="Close"
            className="p-1 rounded hover:bg-secondary transition-colors text-muted-foreground/50 hover:text-foreground"
          >
            <span className="text-xs leading-none">&times;</span>
          </button>
        </div>
      </div>

      <div className="flex-1 overflow-y-auto p-4">
        {/* Vault locked — show unlock first */}
        {vaultState && !vaultState.unlocked ? (
          <div className="space-y-4">
            <div className="text-center mb-4">
              <div className="w-10 h-10 rounded-xl bg-primary/8 flex items-center justify-center mx-auto mb-2">
                <Lock size={18} className="text-primary/60" />
              </div>
              <h2 className="text-sm font-semibold">Vault Locked</h2>
              <p className="text-[11px] text-muted-foreground/60 mt-0.5">
                Enter PIN to authorize this request
              </p>
            </div>

            {unlockError && (
              <div className="flex items-center justify-center gap-1.5 text-destructive text-xs animate-fade-in">
                <AlertCircle size={12} />
                {unlockError}
              </div>
            )}

            <input
              ref={inputRef}
              type="password"
              value={passphrase}
              onChange={(e) => { setPassphrase(e.target.value); setUnlockError(""); }}
              onKeyDown={(e) => { if (e.key === "Enter" && passphrase) handleUnlock(); }}
              placeholder="PIN / Passphrase"
              className="w-full px-4 py-2.5 rounded-xl bg-secondary border border-border text-center text-sm tracking-[0.15em] focus:outline-none focus:ring-2 focus:ring-ring placeholder:tracking-normal placeholder:text-muted-foreground/30"
            />
            <button
              type="button"
              onClick={handleUnlock}
              disabled={unlocking || !passphrase}
              className="w-full flex items-center justify-center gap-2 px-4 py-2.5 rounded-xl bg-primary text-primary-foreground font-medium text-sm hover:bg-primary/90 transition-colors disabled:opacity-30"
            >
              <Unlock size={14} />
              {unlocking ? "Unlocking..." : "Unlock & Continue"}
            </button>
            <button
              type="button"
              onClick={handleDeny}
              className="w-full text-center text-xs text-muted-foreground/50 hover:text-muted-foreground transition-colors"
            >
              Deny request
            </button>
          </div>
        ) : pending ? (
          /* Vault unlocked + pending request — show consent form */
          <div className="space-y-4">
            {/* Request info */}
            <div className="rounded-lg bg-secondary/50 p-3 space-y-1.5">
              <div className="flex items-center justify-between">
                <span className="text-[10px] font-medium text-muted-foreground/70 uppercase">Application</span>
                <span className="text-xs font-medium">{pending.process_name}</span>
              </div>
              <div className="flex items-center justify-between">
                <span className="text-[10px] font-medium text-muted-foreground/70 uppercase">Host</span>
                <span className="text-xs font-medium text-primary">{pending.host}</span>
              </div>
              <div className="flex items-center justify-between">
                <span className="text-[10px] font-medium text-muted-foreground/70 uppercase">Requested Key</span>
                <span className="text-xs font-medium">{pending.key_name}</span>
              </div>
            </div>

            {/* Key selection */}
            {keys.length > 0 && (
              <div className="space-y-1.5">
                <h4 className="text-[10px] font-medium text-muted-foreground/70 uppercase">Select SSH Key</h4>
                {keys
                  .filter((k) => k.state === "active")
                  .map((key) => (
                    <label
                      key={key.id}
                      className={`flex items-center gap-2.5 p-2.5 rounded-lg border cursor-pointer transition-all ${
                        selectedKeyId === key.id
                          ? "bg-primary/8 border-primary/25"
                          : "bg-secondary/30 border-border hover:bg-accent/50"
                      }`}
                    >
                      <input
                        type="radio"
                        name="key"
                        checked={selectedKeyId === key.id}
                        onChange={() => setSelectedKeyId(key.id)}
                        className="accent-primary"
                      />
                      <div className="min-w-0 flex-1">
                        <div className="text-xs font-medium truncate">{key.name}</div>
                        <div className="text-[10px] text-muted-foreground/50 font-mono truncate">{key.fingerprint}</div>
                      </div>
                      <span className="text-[8px] px-1 py-0.5 rounded bg-primary/10 text-primary font-medium shrink-0">
                        {key.algorithm === "ed25519" ? "Ed25519" : "RSA"}
                      </span>
                    </label>
                  ))}
              </div>
            )}

            {/* Allow mode */}
            <div className="flex gap-1.5">
              {(["once", "session", "always"] as const).map((mode) => (
                <button
                  key={mode}
                  type="button"
                  onClick={() => setAllowMode(mode)}
                  className={`flex-1 py-1.5 text-[10px] font-medium rounded-lg border transition-colors ${
                    allowMode === mode
                      ? "bg-primary/15 border-primary/30 text-primary"
                      : "bg-secondary border-border text-muted-foreground hover:text-foreground"
                  }`}
                >
                  {mode === "once" ? "Once" : mode === "session" ? "Session" : "Always"}
                </button>
              ))}
            </div>

            {/* Action buttons */}
            <div className="flex gap-2">
              <button
                type="button"
                onClick={handleDeny}
                className="flex-1 px-3 py-2 text-xs font-medium rounded-lg bg-secondary hover:bg-accent transition-colors"
              >
                Deny
              </button>
              <button
                type="button"
                onClick={handleAllow}
                disabled={submitting || !selectedKeyId}
                className="flex-1 px-3 py-2 text-xs font-medium rounded-lg bg-primary text-primary-foreground hover:bg-primary/90 transition-colors disabled:opacity-30"
              >
                {submitting ? "Allowing..." : "Allow"}
              </button>
            </div>
          </div>
        ) : (
          /* Vault unlocked but no pending request */
          <div className="flex items-center justify-center h-full">
            <div className="text-center">
              <Shield size={20} className="text-primary/40 mx-auto mb-2" />
              <p className="text-xs text-muted-foreground/60">
                Waiting for SSH signing request...
              </p>
            </div>
          </div>
        )}
      </div>
    </div>
  );
}
