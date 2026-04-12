import { useState, useEffect } from "react";
import {
  RefreshCw, Download, FileCode2, Loader2, History, RotateCcw, Eye,
} from "lucide-react";
import { toast } from "sonner";
import { commands } from "@/lib/tauri-commands";
import { useLogStore } from "@/stores/logStore";
import { ConfirmDialog } from "@/components/ui/ConfirmDialog";
import { useConfirm } from "@/hooks/useConfirm";
import type { ConfigBackup } from "@/types";

type View = "preview" | "current" | "backups";

export function ConfigPreview() {
  const [preview, setPreview] = useState("");
  const [currentConfig, setCurrentConfig] = useState("");
  const [backups, setBackups] = useState<ConfigBackup[]>([]);
  const [loading, setLoading] = useState(false);
  const [writing, setWriting] = useState(false);
  const [view, setView] = useState<View>("preview");
  const { addLog } = useLogStore();
  const { confirmProps, confirm } = useConfirm();

  const loadPreview = async () => {
    setLoading(true);
    try {
      const config = await commands.previewSshConfig();
      setPreview(config);
    } catch (err) {
      setPreview(`Error: ${err}`);
    } finally {
      setLoading(false);
    }
  };

  const loadCurrent = async () => {
    try {
      const config = await commands.readCurrentSshConfig();
      setCurrentConfig(config || "(empty — no ~/.ssh/config file)");
    } catch (err) {
      setCurrentConfig(`Error: ${err}`);
    }
  };

  const loadBackups = async () => {
    try {
      const list = await commands.listConfigBackups();
      setBackups(list);
    } catch {
      setBackups([]);
    }
  };

  useEffect(() => {
    loadPreview();
    loadCurrent();
    loadBackups();
  }, []);

  const handleWrite = async () => {
    const ok = await confirm({
      title: "Write SSH config?",
      description: "A backup of your current ~/.ssh/config will be created before writing the new configuration.",
      confirmLabel: "Write Config",
      variant: "warning",
    });
    if (!ok) return;
    setWriting(true);
    try {
      const backupPath = await commands.backupSshConfig().catch(() => null);
      await commands.writeSshConfig();
      addLog({
        action: "config",
        detail: `SSH config written${backupPath ? ` (backup: ${backupPath})` : ""}`,
        level: "info",
      });
      toast.success("SSH config written");
      loadCurrent();
      loadBackups();
    } catch (err) {
      toast.error("Failed to write config", { description: String(err) });
    } finally {
      setWriting(false);
    }
  };

  const handleRollback = async (backup: ConfigBackup) => {
    const ok = await confirm({
      title: "Rollback SSH config?",
      description: `Restore from "${backup.filename}"? Your current config will be backed up automatically before restoring.`,
      confirmLabel: "Rollback",
      variant: "warning",
    });
    if (!ok) return;
    try {
      await commands.rollbackSshConfig(backup.path);
      addLog({ action: "rollback", detail: `Restored ${backup.filename}`, level: "info" });
      toast.success(`Restored ${backup.filename}`);
      loadCurrent();
      loadBackups();
    } catch (err) {
      toast.error("Rollback failed", { description: String(err) });
    }
  };

  return (
    <div className="space-y-4 max-w-3xl">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-2">
          <FileCode2 size={16} className="text-primary" />
          <h3 className="text-sm font-semibold">SSH Config</h3>
        </div>
        <div className="flex gap-2">
          <button
            type="button"
            onClick={() => { setView("preview"); loadPreview(); }}
            className={`flex items-center gap-1.5 px-3 py-1.5 text-xs font-medium rounded-lg transition-colors ${
              view === "preview" ? "bg-primary/15 text-primary" : "bg-secondary hover:bg-accent"
            }`}
          >
            <Eye size={12} />
            Preview
          </button>
          <button
            type="button"
            onClick={() => { setView("current"); loadCurrent(); }}
            className={`flex items-center gap-1.5 px-3 py-1.5 text-xs font-medium rounded-lg transition-colors ${
              view === "current" ? "bg-primary/15 text-primary" : "bg-secondary hover:bg-accent"
            }`}
          >
            <FileCode2 size={12} />
            Current
          </button>
          <button
            type="button"
            onClick={() => { setView("backups"); loadBackups(); }}
            className={`flex items-center gap-1.5 px-3 py-1.5 text-xs font-medium rounded-lg transition-colors ${
              view === "backups" ? "bg-primary/15 text-primary" : "bg-secondary hover:bg-accent"
            }`}
          >
            <History size={12} />
            Backups
            {backups.length > 0 && (
              <span className="text-[10px] text-muted-foreground">{backups.length}</span>
            )}
          </button>
        </div>
      </div>

      {/* Preview View */}
      {view === "preview" && (
        <>
          <div className="flex gap-2 justify-end">
            <button
              type="button"
              onClick={loadPreview}
              disabled={loading}
              className="flex items-center gap-1.5 px-3 py-1.5 text-xs font-medium rounded-lg bg-secondary hover:bg-accent disabled:opacity-50"
            >
              <RefreshCw size={12} className={loading ? "animate-spin" : ""} />
              Refresh
            </button>
            <button
              type="button"
              onClick={handleWrite}
              disabled={writing}
              className="flex items-center gap-1.5 px-3 py-1.5 text-xs font-medium rounded-lg bg-primary text-primary-foreground hover:bg-primary/90 disabled:opacity-50"
            >
              {writing ? <Loader2 size={12} className="animate-spin" /> : <Download size={12} />}
              {writing ? "Writing..." : "Write Config"}
            </button>
          </div>
          <CodeBlock
            title="# Generated by Maze SSH — will be written to ~/.ssh/config"
            content={preview}
            loading={loading}
          />
        </>
      )}

      {/* Current Config View */}
      {view === "current" && (
        <CodeBlock
          title="~/.ssh/config (current)"
          content={currentConfig}
          loading={false}
        />
      )}

      {/* Backups View */}
      {view === "backups" && (
        <div className="space-y-2">
          {backups.length === 0 && (
            <p className="text-sm text-muted-foreground py-4">
              No backups yet. Backups are created automatically when writing config.
            </p>
          )}
          {backups.map((b) => (
            <div
              key={b.filename}
              className="flex items-center justify-between px-4 py-3 rounded-lg bg-secondary/30 hover:bg-secondary/50 transition-colors"
            >
              <div>
                <p className="text-sm font-mono">{b.filename}</p>
                <p className="text-[11px] text-muted-foreground">
                  {b.created_at} · {(b.size / 1024).toFixed(1)} KB
                </p>
              </div>
              <button
                type="button"
                onClick={() => handleRollback(b)}
                className="flex items-center gap-1.5 px-3 py-1.5 text-xs font-medium rounded-lg bg-secondary hover:bg-accent transition-colors"
              >
                <RotateCcw size={12} />
                Rollback
              </button>
            </div>
          ))}
        </div>
      )}

      <ConfirmDialog {...confirmProps} />
    </div>
  );
}

function CodeBlock({ title, content, loading }: { title: string; content: string; loading: boolean }) {
  return (
    <div className="rounded-lg border overflow-hidden">
      <div className="px-4 py-2 bg-secondary/50 border-b">
        <span className="text-[10px] text-muted-foreground font-mono">{title}</span>
      </div>
      <pre className="p-4 text-sm font-mono whitespace-pre-wrap overflow-auto max-h-96 bg-card leading-relaxed">
        {loading ? (
          <span className="text-muted-foreground flex items-center gap-2">
            <Loader2 size={14} className="animate-spin" /> Loading...
          </span>
        ) : (
          content || <span className="text-muted-foreground">Empty</span>
        )}
      </pre>
    </div>
  );
}
