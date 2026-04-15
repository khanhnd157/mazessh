import { useState } from "react";
import { X, Copy, Check, Archive, Trash2, KeyRound } from "lucide-react";
import { toast } from "sonner";
import { useVaultStore } from "@/stores/vaultStore";
import { commands } from "@/lib/tauri-commands";
import { ConfirmDialog } from "@/components/ui/ConfirmDialog";
import type { SshKeyItem } from "@/types";

type SubTab = "overview" | "public_key" | "security";

interface Props {
  keyItem: SshKeyItem;
  onClose: () => void;
}

export function KeyDetailSheet({ keyItem, onClose }: Props) {
  const { archiveKey, deleteKey } = useVaultStore();
  const [activeTab, setActiveTab] = useState<SubTab>("overview");
  const [showDelete, setShowDelete] = useState(false);
  const [copied, setCopied] = useState(false);

  const copyPubKey = async () => {
    await navigator.clipboard.writeText(keyItem.public_key_openssh);
    setCopied(true);
    toast.success("Public key copied");
    setTimeout(() => setCopied(false), 2000);
  };

  const handleArchive = async () => {
    await archiveKey(keyItem.id);
    toast.success(`Key "${keyItem.name}" archived`);
    onClose();
  };

  const handleDelete = async () => {
    await deleteKey(keyItem.id);
    toast.success(`Key "${keyItem.name}" deleted`);
    onClose();
  };

  const handleExportPrivate = async () => {
    try {
      const pem = await commands.vaultExportPrivateKey(keyItem.id);
      await navigator.clipboard.writeText(pem);
      toast.success("Private key copied to clipboard");
    } catch (e) {
      toast.error("Export failed", { description: String(e) });
    }
  };

  const tabs: { id: SubTab; label: string }[] = [
    { id: "overview", label: "Overview" },
    { id: "public_key", label: "Public Key" },
    { id: "security", label: "Security" },
  ];

  return (
    <>
      {/* Backdrop */}
      <div className="fixed inset-0 bg-black/30 z-40" onClick={onClose} />

      {/* Sheet */}
      <div className="fixed top-0 right-0 bottom-0 w-[420px] z-50 bg-card border-l shadow-2xl animate-slide-in-right flex flex-col">
        {/* Header */}
        <div className="flex items-center gap-3 px-5 py-4 border-b shrink-0">
          <div className="w-9 h-9 rounded-lg bg-primary/15 flex items-center justify-center shrink-0">
            <KeyRound size={18} className="text-primary" />
          </div>
          <div className="flex-1 min-w-0">
            <h3 className="text-sm font-semibold truncate">{keyItem.name}</h3>
            <div className="flex items-center gap-2 mt-0.5">
              <span className="text-[9px] px-1.5 py-0.5 rounded bg-primary/15 text-primary font-medium">
                {keyItem.algorithm === "ed25519" ? "Ed25519" : "RSA 4096"}
              </span>
              <span className={`text-[9px] px-1.5 py-0.5 rounded font-medium ${
                keyItem.state === "active"
                  ? "bg-success/15 text-success"
                  : "bg-muted-foreground/15 text-muted-foreground"
              }`}>
                {keyItem.state}
              </span>
            </div>
          </div>
          <button type="button" onClick={onClose} className="p-1.5 rounded-md text-muted-foreground/50 hover:text-foreground hover:bg-secondary transition-colors">
            <X size={16} />
          </button>
        </div>

        {/* Sub-tabs */}
        <div className="flex border-b px-5 shrink-0">
          {tabs.map((t) => (
            <button
              key={t.id}
              type="button"
              onClick={() => setActiveTab(t.id)}
              className={`relative px-3 py-2.5 text-xs font-medium transition-colors ${
                activeTab === t.id ? "text-foreground" : "text-muted-foreground hover:text-foreground"
              }`}
            >
              {t.label}
              <span className={`absolute bottom-0 left-1 right-1 h-0.5 rounded-full bg-primary transition-all ${
                activeTab === t.id ? "opacity-100" : "opacity-0"
              }`} />
            </button>
          ))}
        </div>

        {/* Content */}
        <div className="flex-1 overflow-y-auto p-5">
          {activeTab === "overview" && (
            <div className="space-y-4">
              <InfoRow label="Algorithm" value={keyItem.algorithm === "ed25519" ? "Ed25519" : "RSA 4096"} />
              <InfoRow label="Fingerprint" value={keyItem.fingerprint} mono copyable />
              <InfoRow label="Comment" value={keyItem.comment || "—"} />
              <InfoRow label="Created" value={new Date(keyItem.created_at).toLocaleDateString()} />
              <InfoRow label="Updated" value={new Date(keyItem.updated_at).toLocaleDateString()} />

              <div className="pt-3 border-t flex gap-2">
                {keyItem.state === "active" && (
                  <button type="button" onClick={handleArchive} className="flex items-center gap-1.5 px-3 py-1.5 text-xs font-medium rounded-lg bg-secondary hover:bg-accent transition-colors">
                    <Archive size={12} /> Archive
                  </button>
                )}
                <button type="button" onClick={() => setShowDelete(true)} className="flex items-center gap-1.5 px-3 py-1.5 text-xs font-medium rounded-lg bg-destructive/10 text-destructive hover:bg-destructive/20 transition-colors">
                  <Trash2 size={12} /> Delete
                </button>
              </div>
            </div>
          )}

          {activeTab === "public_key" && (
            <div className="space-y-3">
              <pre className="bg-secondary rounded-lg p-4 font-mono text-xs break-all whitespace-pre-wrap select-all">
                {keyItem.public_key_openssh}
              </pre>
              <button
                type="button"
                onClick={copyPubKey}
                className="flex items-center gap-1.5 px-3 py-1.5 text-xs font-medium rounded-lg bg-primary text-primary-foreground hover:bg-primary/90 transition-colors"
              >
                {copied ? <Check size={12} /> : <Copy size={12} />}
                {copied ? "Copied" : "Copy to Clipboard"}
              </button>
            </div>
          )}

          {activeTab === "security" && (
            <div className="space-y-4">
              <div>
                <h4 className="text-xs font-medium text-muted-foreground mb-1">Export Policy</h4>
                <p className="text-sm">
                  Private key export: {keyItem.export_policy.allow_private_export ? (
                    <span className="text-warning">Allowed</span>
                  ) : (
                    <span className="text-success">Denied</span>
                  )}
                </p>
              </div>

              {keyItem.export_policy.allow_private_export && (
                <div className="pt-3 border-t">
                  <button
                    type="button"
                    onClick={handleExportPrivate}
                    className="flex items-center gap-1.5 px-3 py-1.5 text-xs font-medium rounded-lg bg-warning/15 text-warning hover:bg-warning/25 transition-colors"
                  >
                    Export Private Key
                  </button>
                  <p className="text-[10px] text-muted-foreground/50 mt-1.5">
                    Copies decrypted private key PEM to clipboard
                  </p>
                </div>
              )}
            </div>
          )}
        </div>
      </div>

      <ConfirmDialog
        open={showDelete}
        title={`Delete "${keyItem.name}"?`}
        description="This will permanently delete the key from the vault. This cannot be undone."
        confirmLabel="Delete"
        variant="danger"
        onConfirm={handleDelete}
        onCancel={() => setShowDelete(false)}
      />
    </>
  );
}

function InfoRow({ label, value, mono, copyable }: { label: string; value: string; mono?: boolean; copyable?: boolean }) {
  const handleCopy = () => {
    navigator.clipboard.writeText(value);
    toast.success(`${label} copied`);
  };

  return (
    <div>
      <dt className="text-[10px] font-medium text-muted-foreground/70 uppercase tracking-wider">{label}</dt>
      <dd className={`text-sm mt-0.5 ${mono ? "font-mono text-xs break-all" : ""}`}>
        {value}
        {copyable && (
          <button type="button" onClick={handleCopy} className="ml-2 inline-flex text-muted-foreground/40 hover:text-foreground transition-colors">
            <Copy size={11} />
          </button>
        )}
      </dd>
    </div>
  );
}
