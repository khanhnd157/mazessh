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
    inputRef.current?.focus();
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
      className="fixed inset-0 z-[100] bg-background/98 backdrop-blur-2xl flex flex-col"
      onKeyDown={(e) => {
        // Prevent all keyboard shortcuts from reaching the app behind
        if (e.key !== "Tab") e.stopPropagation();
      }}
    >
      {/* Mini titlebar for window controls only */}
      <div className="flex justify-end h-9 shrink-0" data-tauri-drag-region>
        <button
          type="button"
          onClick={() => appWindow.minimize()}
          className="h-full w-11.5 flex items-center justify-center text-muted-foreground/40 hover:text-foreground hover:bg-foreground/5"
        >
          <Minus size={15} strokeWidth={1} />
        </button>
        <button
          type="button"
          onClick={() => appWindow.hide()}
          className="h-full w-11.5 flex items-center justify-center text-muted-foreground/40 hover:bg-[#c42b1c] hover:text-white"
        >
          <X size={15} strokeWidth={1.5} />
        </button>
      </div>

      {/* Center content */}
      <div className="flex-1 flex items-center justify-center">
        <div className={`w-80 text-center ${shake ? "animate-shake" : ""}`}>
          {/* Logo */}
          <div className="w-16 h-16 rounded-2xl bg-primary/10 flex items-center justify-center mx-auto mb-5">
            <Lock size={28} className="text-primary" />
          </div>

          <h1 className="text-lg font-semibold mb-1">Maze SSH</h1>
          <p className="text-sm text-muted-foreground mb-6">
            {isSetup ? "Set up a PIN to protect your profiles" : "Enter your PIN to unlock"}
          </p>

          {error && (
            <div className="flex items-center justify-center gap-1.5 text-destructive text-xs mb-4">
              <AlertCircle size={13} />
              {error}
            </div>
          )}

          {/* PIN Input */}
          <div className="space-y-3">
            <input
              ref={inputRef}
              type="password"
              value={pin}
              onChange={(e) => setPin(e.target.value)}
              onKeyDown={handleKeyDown}
              placeholder={isSetup ? "Create PIN (4+ chars)" : "Enter PIN"}
              autoFocus
              className="w-full px-4 py-3 rounded-xl bg-secondary border border-transparent text-center text-sm tracking-widest focus:outline-none focus:ring-2 focus:ring-ring focus:border-ring placeholder:tracking-normal placeholder:text-muted-foreground/40"
            />

            {isSetup && (
              <input
                type="password"
                value={confirmPin}
                onChange={(e) => setConfirmPin(e.target.value)}
                onKeyDown={handleKeyDown}
                placeholder="Confirm PIN"
                className="w-full px-4 py-3 rounded-xl bg-secondary border border-transparent text-center text-sm tracking-widest focus:outline-none focus:ring-2 focus:ring-ring focus:border-ring placeholder:tracking-normal placeholder:text-muted-foreground/40"
              />
            )}

            <button
              type="button"
              onClick={isSetup ? handleSetup : handleUnlock}
              disabled={loading || !pin || (isSetup && !confirmPin)}
              className="w-full flex items-center justify-center gap-2 px-4 py-3 rounded-xl bg-primary text-primary-foreground font-medium text-sm hover:bg-primary/90 transition-colors disabled:opacity-40"
            >
              <Unlock size={15} />
              {loading ? "Please wait..." : isSetup ? "Set PIN & Continue" : "Unlock"}
            </button>

            {isSetup && (
              <button
                type="button"
                onClick={() => {
                  // Skip setup — dismiss lock without PIN
                  useSecurityStore.getState().setLocked(false);
                }}
                className="text-xs text-muted-foreground hover:text-foreground transition-colors"
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
