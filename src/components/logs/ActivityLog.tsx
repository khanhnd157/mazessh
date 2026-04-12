import { Trash2, Info, AlertTriangle, XCircle } from "lucide-react";
import { useLogStore } from "@/stores/logStore";

const levelConfig = {
  info: { color: "text-blue-400", icon: Info },
  warn: { color: "text-amber-400", icon: AlertTriangle },
  error: { color: "text-red-400", icon: XCircle },
};

export function ActivityLog() {
  const { logs, clearLogs } = useLogStore();

  return (
    <div className="h-full flex flex-col">
      <div className="flex items-center justify-between px-4 py-2 border-b bg-card/30">
        <h3 className="text-[10px] font-semibold text-muted-foreground uppercase tracking-wider">
          Activity
          {logs.length > 0 && (
            <span className="ml-1.5 text-muted-foreground/40">{logs.length}</span>
          )}
        </h3>
        {logs.length > 0 && (
          <button
            type="button"
            onClick={clearLogs}
            title="Clear logs"
            className="p-1 rounded text-muted-foreground/50 hover:text-muted-foreground transition-colors"
          >
            <Trash2 size={11} />
          </button>
        )}
      </div>
      <div className="flex-1 overflow-y-auto px-4 py-1.5 font-mono text-[11px] leading-[1.6]">
        {logs.length === 0 && (
          <p className="text-muted-foreground/40 py-2">No activity yet</p>
        )}
        {logs.map((log) => {
          const config = levelConfig[log.level];
          const Icon = config.icon;
          return (
            <div key={log.id} className="flex items-start gap-2 py-0.5 group">
              <span className="text-muted-foreground/40 shrink-0 tabular-nums">
                {new Date(log.timestamp).toLocaleTimeString()}
              </span>
              <Icon size={11} className={`${config.color} shrink-0 mt-0.5`} />
              <span className={`${config.color} shrink-0`}>{log.action}</span>
              <span className="text-foreground/70 truncate">{log.detail}</span>
            </div>
          );
        })}
      </div>
    </div>
  );
}
