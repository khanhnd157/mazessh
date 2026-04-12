import { useEffect, useState } from "react";
import { Shield, RefreshCw } from "lucide-react";
import { commands } from "@/lib/tauri-commands";
import type { AuditEntry } from "@/types";

export function AuditLogViewer() {
  const [logs, setLogs] = useState<AuditEntry[]>([]);
  const [loading, setLoading] = useState(false);
  const [offset, setOffset] = useState(0);
  const [hasMore, setHasMore] = useState(true);
  const PAGE_SIZE = 50;

  const loadLogs = async (reset = false) => {
    setLoading(true);
    try {
      const newOffset = reset ? 0 : offset;
      const entries = await commands.getAuditLogs(PAGE_SIZE, newOffset);
      if (reset) {
        setLogs(entries);
        setOffset(PAGE_SIZE);
      } else {
        setLogs((prev) => [...prev, ...entries]);
        setOffset(newOffset + PAGE_SIZE);
      }
      setHasMore(entries.length === PAGE_SIZE);
    } catch {
      // May fail if locked
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    loadLogs(true);
  }, []);

  return (
    <div className="space-y-4">
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-2">
          <Shield size={16} className="text-primary" />
          <h3 className="text-sm font-semibold">Audit Log</h3>
        </div>
        <button
          type="button"
          onClick={() => loadLogs(true)}
          disabled={loading}
          className="flex items-center gap-1.5 px-3 py-1.5 text-xs font-medium rounded-lg bg-secondary hover:bg-accent disabled:opacity-50"
        >
          <RefreshCw size={12} className={loading ? "animate-spin" : ""} />
          Refresh
        </button>
      </div>

      {logs.length === 0 && !loading && (
        <p className="text-sm text-muted-foreground py-4">No audit entries yet.</p>
      )}

      {logs.length > 0 && (
        <div className="rounded-xl border overflow-hidden">
          <table className="w-full text-xs">
            <thead>
              <tr className="bg-secondary/50 text-muted-foreground">
                <th className="text-left px-3 py-2 font-medium">Time</th>
                <th className="text-left px-3 py-2 font-medium">Action</th>
                <th className="text-left px-3 py-2 font-medium">Profile</th>
                <th className="text-left px-3 py-2 font-medium">Result</th>
              </tr>
            </thead>
            <tbody>
              {logs.map((log, i) => (
                <tr key={i} className="border-t hover:bg-accent/30">
                  <td className="px-3 py-2 text-muted-foreground font-mono whitespace-nowrap">
                    {new Date(log.timestamp).toLocaleString()}
                  </td>
                  <td className="px-3 py-2">
                    <span className="px-1.5 py-0.5 rounded bg-primary/10 text-primary font-medium">
                      {log.action}
                    </span>
                  </td>
                  <td className="px-3 py-2 text-muted-foreground">
                    {log.profile_name ?? "—"}
                  </td>
                  <td className="px-3 py-2 truncate max-w-48">{log.result}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}

      {hasMore && logs.length > 0 && (
        <button
          type="button"
          onClick={() => loadLogs(false)}
          disabled={loading}
          className="text-xs text-primary hover:underline disabled:opacity-50"
        >
          {loading ? "Loading..." : "Load more"}
        </button>
      )}
    </div>
  );
}
