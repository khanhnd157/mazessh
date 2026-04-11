import { useState, useEffect } from "react";
import { commands } from "@/lib/tauri-commands";
import { useLogStore } from "@/stores/logStore";

export function ConfigPreview() {
  const [preview, setPreview] = useState<string>("");
  const [loading, setLoading] = useState(false);
  const { addLog } = useLogStore();

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

  useEffect(() => {
    loadPreview();
  }, []);

  const handleWrite = async () => {
    if (!confirm("Write SSH config? A backup will be created first.")) return;
    try {
      const backupPath = await commands.backupSshConfig().catch(() => null);
      await commands.writeSshConfig();
      addLog({
        action: "config",
        detail: `SSH config written${backupPath ? ` (backup: ${backupPath})` : ""}`,
        level: "info",
      });
    } catch (err) {
      addLog({
        action: "config",
        detail: `Failed to write config: ${err}`,
        level: "error",
      });
    }
  };

  return (
    <div className="space-y-3">
      <div className="flex items-center justify-between">
        <h3 className="text-sm font-semibold text-muted-foreground uppercase tracking-wider">
          SSH Config Preview
        </h3>
        <div className="flex gap-2">
          <button
            onClick={loadPreview}
            disabled={loading}
            className="px-3 py-1.5 text-xs rounded-md bg-secondary hover:bg-secondary/80 transition-colors"
          >
            Refresh
          </button>
          <button
            onClick={handleWrite}
            className="px-3 py-1.5 text-xs rounded-md bg-primary text-primary-foreground hover:bg-primary/90 transition-colors"
          >
            Write to ~/.ssh/config
          </button>
        </div>
      </div>
      <pre className="p-4 rounded-md bg-secondary text-sm font-mono whitespace-pre-wrap overflow-auto max-h-64">
        {loading ? "Loading..." : preview || "No profiles configured yet."}
      </pre>
    </div>
  );
}
