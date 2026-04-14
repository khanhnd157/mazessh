import { CheckCircle, XCircle, Key, AlertCircle } from "lucide-react";
import type { DiagnosticsResult } from "@/types";

export function DiagnosticsPanel({ result }: { result: DiagnosticsResult }) {
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
            </div>
            {step.detail && (
              <p className="text-[10px] text-muted-foreground/60 ml-[21px] truncate">{step.detail}</p>
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
    </div>
  );
}
