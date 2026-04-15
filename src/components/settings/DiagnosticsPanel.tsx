import { useState } from "react";
import { CheckCircle, XCircle, Key, AlertCircle, Terminal, Loader2, Wifi } from "lucide-react";
import type { DiagnosticsResult, SshHostTestResult } from "@/types";
import { useBridgeStore } from "@/stores/bridgeStore";
import { commands } from "@/lib/tauri-commands";

export function DiagnosticsPanel({
  result,
  distro,
  onRerunDiagnostics,
}: {
  result: DiagnosticsResult;
  distro: string;
  onRerunDiagnostics?: () => void;
}) {
  const { runFix } = useBridgeStore();
  const [fixLoading, setFixLoading] = useState<number | null>(null);
  const [fixResults, setFixResults] = useState<Record<number, { ok: boolean; output: string }>>({});

  // SSH host test state
  const [sshTarget, setSshTarget] = useState("git@github.com");
  const [sshPort, setSshPort] = useState(22);
  const [sshTestLoading, setSshTestLoading] = useState(false);
  const [sshTestResult, setSshTestResult] = useState<SshHostTestResult | null>(null);

  const handleSshTest = async () => {
    const atIdx = sshTarget.lastIndexOf("@");
    const user = atIdx >= 0 ? sshTarget.slice(0, atIdx) : "git";
    const host = atIdx >= 0 ? sshTarget.slice(atIdx + 1) : sshTarget;
    setSshTestLoading(true);
    setSshTestResult(null);
    try {
      const result = await commands.testSshViaBridge(distro, host, user, sshPort);
      setSshTestResult(result);
    } catch (err) {
      setSshTestResult({
        command: "",
        output: String(err),
        connected: false,
        authenticated: false,
        exit_code: -1,
      });
    } finally {
      setSshTestLoading(false);
    }
  };

  const allPassed = result.steps.every((s) => s.passed);
  const showSshTest = allPassed || result.keys_visible.length > 0;

  const handleFix = async (idx: number, cmd: string) => {
    setFixLoading(idx);
    try {
      const output = await runFix(distro, cmd);
      setFixResults((prev) => ({ ...prev, [idx]: { ok: true, output: output || "(done)" } }));
      onRerunDiagnostics?.();
    } catch (err) {
      setFixResults((prev) => ({ ...prev, [idx]: { ok: false, output: String(err) } }));
    } finally {
      setFixLoading(null);
    }
  };

  return (
    <div className="space-y-3 mt-2">
      {/* Steps */}
      <div className="rounded-lg border bg-secondary/20 p-3 space-y-2">
        {result.steps.map((step, i) => (
          <div key={i} className="space-y-0.5">
            <div className="flex items-center gap-2 text-xs">
              {step.passed ? (
                <CheckCircle size={13} className="text-success shrink-0" />
              ) : (
                <XCircle size={13} className="text-destructive shrink-0" />
              )}
              <span className={step.passed ? "text-foreground" : "text-muted-foreground"}>{step.name}</span>

              {/* Quick-fix button for failing steps with a remediation command */}
              {!step.passed && step.remediation_cmd && (
                <button
                  type="button"
                  onClick={() => handleFix(i, step.remediation_cmd!)}
                  disabled={fixLoading !== null}
                  className="ml-auto flex items-center gap-1 px-2 py-0.5 text-[10px] font-medium rounded-md bg-secondary hover:bg-accent disabled:opacity-50"
                  title={step.remediation_cmd}
                >
                  {fixLoading === i ? (
                    <Loader2 size={9} className="animate-spin" />
                  ) : (
                    <Terminal size={9} />
                  )}
                  Run Fix
                </button>
              )}
            </div>

            {step.detail && (
              <p className="text-[10px] text-muted-foreground/60 ml-[21px] truncate">{step.detail}</p>
            )}

            {/* Fix result inline */}
            {fixResults[i] && (
              <div
                className={`ml-[21px] mt-0.5 text-[10px] font-mono rounded px-1.5 py-0.5 ${
                  fixResults[i].ok
                    ? "bg-success/10 text-success"
                    : "bg-destructive/10 text-destructive"
                }`}
              >
                {fixResults[i].output}
              </div>
            )}
          </div>
        ))}
      </div>

      {/* Keys visible */}
      {result.keys_visible.length > 0 && (
        <div className="space-y-1">
          <div className="flex items-center gap-1.5 text-[10px] font-medium text-muted-foreground uppercase tracking-wider">
            <Key size={10} />
            Keys visible through bridge
          </div>
          <div className="rounded-lg bg-secondary/30 p-2 space-y-0.5">
            {result.keys_visible.map((key, i) => (
              <p key={i} className="text-[10px] font-mono text-foreground/70 truncate">{key}</p>
            ))}
          </div>
        </div>
      )}

      {/* Suggestions */}
      {result.suggestions.length > 0 && (
        <div className="space-y-1">
          {result.suggestions.map((s, i) => (
            <div key={i} className="flex items-start gap-1.5 text-[11px] text-warning bg-warning/10 rounded-lg px-2.5 py-1.5">
              <AlertCircle size={12} className="mt-0.5 shrink-0" />
              <span>{s}</span>
            </div>
          ))}
        </div>
      )}

      {/* SSH host connectivity test */}
      {showSshTest && (
        <div className="space-y-2 pt-1 border-t border-border/50">
          <div className="flex items-center gap-1.5 text-[10px] font-medium text-muted-foreground uppercase tracking-wider">
            <Wifi size={10} />
            Test SSH Connection
          </div>
          <div className="flex items-center gap-2">
            <input
              type="text"
              value={sshTarget}
              onChange={(e) => setSshTarget(e.target.value)}
              placeholder="git@github.com"
              className="flex-1 px-2 py-1 text-[10px] font-mono rounded-lg bg-secondary border border-border focus:outline-none focus:ring-1 focus:ring-primary"
            />
            <input
              type="number"
              value={sshPort}
              onChange={(e) => setSshPort(Number(e.target.value))}
              min={1}
              max={65535}
              title="Port"
              placeholder="22"
              className="w-16 px-2 py-1 text-[10px] font-mono rounded-lg bg-secondary border border-border focus:outline-none focus:ring-1 focus:ring-primary"
            />
            <button
              type="button"
              onClick={handleSshTest}
              disabled={sshTestLoading || !sshTarget.trim()}
              className="px-2.5 py-1 text-[10px] font-medium rounded-lg bg-secondary hover:bg-accent disabled:opacity-50 flex items-center gap-1"
            >
              {sshTestLoading ? <Loader2 size={10} className="animate-spin" /> : <Wifi size={10} />}
              Test
            </button>
          </div>
          {sshTestResult && (
            <div className="space-y-1.5">
              <div className="flex items-center gap-2 text-[10px]">
                <span
                  className={`flex items-center gap-1 px-1.5 py-0.5 rounded-full font-medium ${
                    sshTestResult.connected
                      ? "bg-success/15 text-success"
                      : "bg-destructive/15 text-destructive"
                  }`}
                >
                  {sshTestResult.connected ? <CheckCircle size={9} /> : <XCircle size={9} />}
                  {sshTestResult.connected ? "Connected" : "Not connected"}
                </span>
                <span
                  className={`flex items-center gap-1 px-1.5 py-0.5 rounded-full font-medium ${
                    sshTestResult.authenticated
                      ? "bg-success/15 text-success"
                      : "bg-muted text-muted-foreground"
                  }`}
                >
                  {sshTestResult.authenticated ? <CheckCircle size={9} /> : <XCircle size={9} />}
                  {sshTestResult.authenticated ? "Authenticated" : "Not authenticated"}
                </span>
              </div>
              {sshTestResult.output && (
                <pre className="text-[9px] font-mono bg-black/20 dark:bg-black/40 rounded-lg p-2 overflow-auto max-h-24 whitespace-pre-wrap text-foreground/60">
                  {sshTestResult.output}
                </pre>
              )}
              {sshTestResult.exit_code !== -1 && (
                <p className="text-[9px] text-muted-foreground/50">Exit code: {sshTestResult.exit_code}</p>
              )}
            </div>
          )}
        </div>
      )}
    </div>
  );
}
