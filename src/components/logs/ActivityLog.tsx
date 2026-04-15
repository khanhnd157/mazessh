import { Trash2, Info, AlertTriangle, XCircle } from "lucide-react";
import { useLogStore } from "@/stores/logStore";

const levelConfig = {
  info: { color: "text-primary/70", icon: Info },
  warn: { color: "text-warning", icon: AlertTriangle },
  error: { color: "text-destructive", icon: XCircle },
};

export function ActivityLog() {
  const { logs, clearLogs } = useLogStore();

  return (
    <div className="flex-1 min-h-0 flex flex-col">
      <div className="flex items-center justify-between px-3 py-1.5 border-b shrink-0">
        <h3 className="text-[10px] font-semibold text-muted-foreground/50 uppercase tracking-wider">
          Activity
          {logs.length > 0 && <span className="ml-1 opacity-50">{logs.length}</span>}
        </h3>
        {logs.length > 0 && (
          <button
            type="button"
            onClick={clearLogs}
            title="Clear"
            className="p-0.5 rounded text-muted-foreground/30 hover:text-muted-foreground/60 transition-colors"
          >
            <Trash2 size={10} />
          </button>
        )}
      </div>
      <div className="flex-1 min-h-0 overflow-y-auto px-3 py-1 font-mono text-[10.5px] leading-relaxed">
        {logs.length === 0 && (
          <p className="text-muted-foreground/30 py-2">No activity</p>
        )}
        {logs.map((log) => {
          const config = levelConfig[log.level];
          const Icon = config.icon;
          return (
            <div key={log.id} className="flex items-start gap-1.5 py-px">
              <span className="text-muted-foreground/30 shrink-0 tabular-nums">
                {new Date(log.timestamp).toLocaleTimeString(undefined, { hour: "2-digit", minute: "2-digit", second: "2-digit" })}
              </span>
              <Icon size={10} className={`${config.color} shrink-0 mt-px`} />
              <span className={`${config.color} shrink-0 font-medium`}>{log.action}</span>
              <span className="text-foreground/50 truncate">{log.detail}</span>
            </div>
          );
        })}
      </div>
    </div>
  );
}
