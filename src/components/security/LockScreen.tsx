import { useState, useRef, useEffect } from "react";
import { Lock, Minus, X, Unlock, AlertCircle } from "lucide-react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { useSecurityStore } from "@/stores/securityStore";

export function LockScreen() {
  const { unlock, pinIsSet, setupPin } = useSecurityStore();
  const [pin, setPin] = useState("");
  const [confirmPin, setConfirmPin] = useState("");
  const [error, setError] = useState("");
  const [loading, setLoading] = useState(false);
  const [isSetup, setIsSetup] = useState(!pinIsSet);
  const [shake, setShake] = useState(false);
  const inputRef = useRef<HTMLInputElement>(null);
  const appWindow = getCurrentWindow();

  useEffect(() => {
    setTimeout(() => inputRef.current?.focus(), 100);
  }, []);

  const triggerShake = () => {
    setShake(true);
    setTimeout(() => setShake(false), 500);
  };

  const handleUnlock = async () => {
    if (!pin) return;
    setLoading(true);
    setError("");
    try {
      const valid = await unlock(pin);
      if (!valid) {
        setError("Incorrect PIN");
        setPin("");
        triggerShake();
        inputRef.current?.focus();
      }
    } catch (err) {
      setError(String(err));
    } finally {
      setLoading(false);
    }
  };

  const handleSetup = async () => {
    if (pin.length < 4) {
      setError("PIN must be at least 4 characters");
      return;
    }
    if (pin !== confirmPin) {
      setError("PINs do not match");
      setConfirmPin("");
      triggerShake();
      return;
    }
    setLoading(true);
    setError("");
    try {
      await setupPin(pin);
      setIsSetup(false);
    } catch (err) {
      setError(String(err));
    } finally {
      setLoading(false);
    }
  };

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === "Enter") {
      if (isSetup) handleSetup();
      else handleUnlock();
    }
  };

  return (
    <div
      className="fixed inset-0 z-100 bg-background flex flex-col"
      onKeyDown={(e) => {
        if (e.key !== "Tab") e.stopPropagation();
      }}
    >
      {/* Mini titlebar */}
      <div className="titlebar-bg flex justify-end h-9 shrink-0" data-tauri-drag-region>
        <button
          type="button"
          onClick={() => appWindow.minimize()}
          title="Minimize"
          className="h-full w-11.5 flex items-center justify-center text-muted-foreground/40 hover:text-foreground hover:bg-foreground/5"
        >
          <Minus size={15} strokeWidth={1} />
        </button>
        <button
          type="button"
          onClick={() => appWindow.hide()}
          title="Close"
          className="h-full w-11.5 flex items-center justify-center text-muted-foreground/40 hover:bg-[#c42b1c] hover:text-white"
        >
          <X size={15} strokeWidth={1.5} />
        </button>
      </div>

      {/* Center */}
      <div className="flex-1 flex items-center justify-center">
        <div className={`w-72 text-center ${shake ? "animate-shake" : ""}`}>
          {/* Logo */}
          <img src="/logo.png" alt="Maze SSH" className="w-14 h-14 rounded-2xl mx-auto mb-4 opacity-80" />

          <h1 className="text-base font-semibold mb-0.5">Maze SSH</h1>
          <p className="text-xs text-muted-foreground/60 mb-5">
            {isSetup ? "Set up a PIN to secure your profiles" : "Enter PIN to continue"}
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
              value={pin}
              onChange={(e) => setPin(e.target.value)}
              onKeyDown={handleKeyDown}
              placeholder={isSetup ? "Create PIN (4+ chars)" : "PIN"}
              autoFocus
              className="w-full px-4 py-2.5 rounded-xl bg-secondary border border-border text-center text-sm tracking-[0.2em] focus:outline-none focus:ring-2 focus:ring-ring focus:border-transparent placeholder:tracking-normal placeholder:text-muted-foreground/30"
            />

            {isSetup && (
              <input
                type="password"
                value={confirmPin}
                onChange={(e) => setConfirmPin(e.target.value)}
                onKeyDown={handleKeyDown}
                placeholder="Confirm PIN"
                className="w-full px-4 py-2.5 rounded-xl bg-secondary border border-border text-center text-sm tracking-[0.2em] focus:outline-none focus:ring-2 focus:ring-ring focus:border-transparent placeholder:tracking-normal placeholder:text-muted-foreground/30"
              />
            )}

            <button
              type="button"
              onClick={isSetup ? handleSetup : handleUnlock}
              disabled={loading || !pin || (isSetup && !confirmPin)}
              className="w-full flex items-center justify-center gap-2 px-4 py-2.5 rounded-xl bg-primary text-primary-foreground font-medium text-sm hover:bg-primary/90 transition-colors disabled:opacity-30"
            >
              {isSetup ? (
                <>
                  <Lock size={14} />
                  {loading ? "Setting up..." : "Set PIN"}
                </>
              ) : (
                <>
                  <Unlock size={14} />
                  {loading ? "Verifying..." : "Unlock"}
                </>
              )}
            </button>

            {isSetup && (
              <button
                type="button"
                onClick={() => useSecurityStore.getState().setLocked(false)}
                className="text-[11px] text-muted-foreground/40 hover:text-muted-foreground/70 transition-colors"
              >
                Skip for now
              </button>
            )}
          </div>
        </div>
      </div>
    </div>
  );
}
