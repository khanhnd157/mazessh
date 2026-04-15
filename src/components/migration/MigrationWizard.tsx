import { useState } from "react";
import { X, ArrowRight, ArrowLeft, Check, AlertTriangle, XCircle, Trash2 } from "lucide-react";
import { toast } from "sonner";
import { useVaultStore } from "@/stores/vaultStore";
import { commands } from "@/lib/tauri-commands";
import type { MigrationPreview, MigrationReport } from "@/types";

interface Props {
  onClose: () => void;
}

type Step = 1 | 2 | 3 | 4;

export function MigrationWizard({ onClose }: Props) {
  const { migrateProfiles } = useVaultStore();
  const [step, setStep] = useState<Step>(1);
  const [preview, setPreview] = useState<MigrationPreview | null>(null);
  const [selectedIds, setSelectedIds] = useState<string[]>([]);
  const [report, setReport] = useState<MigrationReport | null>(null);
  const [loading, setLoading] = useState(false);

  const handleScan = async () => {
    setLoading(true);
    try {
      const p = await useVaultStore.getState().getMigrationPreview();
      setPreview(p);
      setSelectedIds(p.eligible.map((e) => e.profile_id));
      setStep(2);
    } catch (err) {
      toast.error("Scan failed", { description: String(err) });
    } finally {
      setLoading(false);
    }
  };

  const handleMigrate = async () => {
    setStep(3);
    setLoading(true);
    try {
      const r = await migrateProfiles(selectedIds);
      setReport(r);
      setStep(4);
    } catch (err) {
      toast.error("Migration failed", { description: String(err) });
      setStep(2);
    } finally {
      setLoading(false);
    }
  };

  const handleDeleteOriginal = async (keyPath: string) => {
    try {
      await commands.deleteOriginalKeyFile(keyPath);
      toast.success("Key file deleted");
    } catch (err) {
      toast.error("Delete failed", { description: String(err) });
    }
  };

  const toggleSelect = (id: string) => {
    setSelectedIds((prev) =>
      prev.includes(id) ? prev.filter((x) => x !== id) : [...prev, id],
    );
  };

  const stepLabels = ["Intro", "Select", "Migrate", "Done"];

  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center bg-black/50 backdrop-blur-sm"
      onClick={(e) => { if (e.target === e.currentTarget) onClose(); }}
    >
      <div className="bg-card border rounded-xl shadow-2xl shadow-black/30 w-[560px] max-h-[80vh] overflow-hidden flex flex-col animate-fade-in">
        {/* Header */}
        <div className="flex items-center justify-between px-5 pt-5 pb-3 shrink-0">
          <div>
            <h3 className="text-sm font-semibold">Migrate Profiles to Vault</h3>
            <div className="flex items-center gap-1.5 mt-1.5">
              {stepLabels.map((label, i) => (
                <div key={label} className="flex items-center gap-1.5">
                  <div className={`w-5 h-5 rounded-full flex items-center justify-center text-[9px] font-bold ${
                    i + 1 <= step
                      ? "bg-primary text-primary-foreground"
                      : "bg-secondary text-muted-foreground"
                  }`}>
                    {i + 1 < step ? <Check size={10} /> : i + 1}
                  </div>
                  {i < stepLabels.length - 1 && (
                    <div className={`w-6 h-px ${i + 1 < step ? "bg-primary" : "bg-border"}`} />
                  )}
                </div>
              ))}
            </div>
          </div>
          <button type="button" onClick={onClose} className="p-1 rounded-md text-muted-foreground/50 hover:text-foreground hover:bg-secondary transition-colors">
            <X size={14} />
          </button>
        </div>

        {/* Content */}
        <div className="flex-1 overflow-y-auto px-5 pb-4">
          {/* Step 1: Intro */}
          {step === 1 && (
            <div className="space-y-3 py-4">
              <p className="text-sm text-muted-foreground leading-relaxed">
                Move your existing SSH keys into the encrypted vault. Your profiles will be updated to reference vault-managed keys.
              </p>
              <ul className="text-xs text-muted-foreground/80 space-y-1.5 list-disc pl-4">
                <li>Private keys will be encrypted with AES-256-GCM</li>
                <li>Original key files will NOT be deleted automatically</li>
                <li>You can choose to delete originals after migration</li>
                <li>Profiles already using vault keys will be skipped</li>
              </ul>
            </div>
          )}

          {/* Step 2: Select profiles */}
          {step === 2 && preview && (
            <div className="space-y-3">
              {preview.eligible.length > 0 && (
                <div>
                  <div className="flex items-center justify-between mb-2">
                    <h4 className="text-xs font-medium text-muted-foreground">
                      Eligible ({preview.eligible.length})
                    </h4>
                    <button
                      type="button"
                      onClick={() => {
                        if (selectedIds.length === preview.eligible.length) {
                          setSelectedIds([]);
                        } else {
                          setSelectedIds(preview.eligible.map((e) => e.profile_id));
                        }
                      }}
                      className="text-[10px] text-primary hover:underline"
                    >
                      {selectedIds.length === preview.eligible.length ? "Deselect All" : "Select All"}
                    </button>
                  </div>
                  <div className="space-y-1.5">
                    {preview.eligible.map((e) => (
                      <label
                        key={e.profile_id}
                        className={`flex items-center gap-2.5 p-2.5 rounded-lg border cursor-pointer transition-all ${
                          selectedIds.includes(e.profile_id)
                            ? "bg-primary/8 border-primary/25"
                            : "bg-secondary/30 border-border hover:bg-accent/50"
                        }`}
                      >
                        <input
                          type="checkbox"
                          checked={selectedIds.includes(e.profile_id)}
                          onChange={() => toggleSelect(e.profile_id)}
                          className="accent-primary"
                        />
                        <div className="flex-1 min-w-0">
                          <div className="text-xs font-medium">{e.profile_name}</div>
                          <div className="text-[10px] text-muted-foreground/50 truncate font-mono">{e.key_path}</div>
                        </div>
                        <span className="text-[9px] px-1.5 py-0.5 rounded bg-primary/15 text-primary font-medium shrink-0">
                          {e.algorithm}
                        </span>
                      </label>
                    ))}
                  </div>
                </div>
              )}

              {preview.skipped.length > 0 && (
                <div>
                  <h4 className="text-xs font-medium text-muted-foreground/60 mb-2">
                    Skipped ({preview.skipped.length})
                  </h4>
                  <div className="space-y-1">
                    {preview.skipped.map((s) => (
                      <div key={s.profile_id} className="flex items-center gap-2 p-2 rounded-lg bg-secondary/20 text-muted-foreground/50">
                        <AlertTriangle size={11} className="shrink-0" />
                        <span className="text-[11px]">{s.profile_name}: {s.reason}</span>
                      </div>
                    ))}
                  </div>
                </div>
              )}

              {preview.eligible.length === 0 && (
                <div className="text-center py-8">
                  <Check size={20} className="text-success mx-auto mb-2" />
                  <p className="text-sm text-muted-foreground">All profiles are already migrated or have no key files.</p>
                </div>
              )}
            </div>
          )}

          {/* Step 3: Migrating */}
          {step === 3 && (
            <div className="flex items-center justify-center py-12">
              <div className="text-center">
                <div className="w-8 h-8 border-2 border-primary/30 border-t-primary rounded-full animate-spin mx-auto mb-3" />
                <p className="text-sm text-muted-foreground">
                  Migrating {selectedIds.length} profile{selectedIds.length !== 1 ? "s" : ""}...
                </p>
              </div>
            </div>
          )}

          {/* Step 4: Results */}
          {step === 4 && report && (
            <div className="space-y-3">
              {report.succeeded.length > 0 && (
                <div>
                  <h4 className="text-xs font-medium text-success mb-2 flex items-center gap-1">
                    <Check size={12} /> Migrated ({report.succeeded.length})
                  </h4>
                  <div className="space-y-1.5">
                    {report.succeeded.map((s) => (
                      <div key={s.profile_id} className="flex items-center justify-between p-2 rounded-lg bg-success/5 border border-success/15">
                        <span className="text-xs font-medium">{s.profile_name}</span>
                        <button
                          type="button"
                          onClick={() => {
                            const eligible = preview?.eligible.find((e) => e.profile_id === s.profile_id);
                            if (eligible) handleDeleteOriginal(eligible.key_path);
                          }}
                          className="flex items-center gap-1 text-[10px] text-destructive/70 hover:text-destructive transition-colors"
                        >
                          <Trash2 size={10} /> Delete original
                        </button>
                      </div>
                    ))}
                  </div>
                </div>
              )}

              {report.skipped.length > 0 && (
                <div>
                  <h4 className="text-xs font-medium text-warning mb-2 flex items-center gap-1">
                    <AlertTriangle size={12} /> Skipped ({report.skipped.length})
                  </h4>
                  {report.skipped.map((s) => (
                    <div key={s.profile_id} className="text-[11px] text-muted-foreground/60 p-1.5">
                      {s.profile_name}: {s.reason}
                    </div>
                  ))}
                </div>
              )}

              {report.failed.length > 0 && (
                <div>
                  <h4 className="text-xs font-medium text-destructive mb-2 flex items-center gap-1">
                    <XCircle size={12} /> Failed ({report.failed.length})
                  </h4>
                  {report.failed.map((f) => (
                    <div key={f.profile_id} className="text-[11px] text-destructive/80 p-1.5 bg-destructive/5 rounded">
                      {f.profile_name}: {f.error}
                    </div>
                  ))}
                </div>
              )}
            </div>
          )}
        </div>

        {/* Footer */}
        <div className="flex justify-between px-5 py-3.5 border-t bg-secondary/30 shrink-0">
          <div>
            {step === 2 && (
              <button type="button" onClick={() => setStep(1)} className="flex items-center gap-1 px-3 py-1.5 text-xs font-medium rounded-lg bg-secondary hover:bg-accent transition-colors">
                <ArrowLeft size={12} /> Back
              </button>
            )}
          </div>
          <div className="flex gap-2">
            <button type="button" onClick={onClose} className="px-3.5 py-1.5 text-xs font-medium rounded-lg bg-secondary hover:bg-accent transition-colors">
              {step === 4 ? "Done" : "Cancel"}
            </button>
            {step === 1 && (
              <button
                type="button"
                onClick={handleScan}
                disabled={loading}
                className="flex items-center gap-1 px-3.5 py-1.5 text-xs font-medium rounded-lg bg-primary text-primary-foreground hover:bg-primary/90 transition-colors disabled:opacity-30"
              >
                {loading ? "Scanning..." : "Scan Profiles"} <ArrowRight size={12} />
              </button>
            )}
            {step === 2 && preview && preview.eligible.length > 0 && (
              <button
                type="button"
                onClick={handleMigrate}
                disabled={selectedIds.length === 0}
                className="flex items-center gap-1 px-3.5 py-1.5 text-xs font-medium rounded-lg bg-primary text-primary-foreground hover:bg-primary/90 transition-colors disabled:opacity-30"
              >
                Migrate {selectedIds.length} key{selectedIds.length !== 1 ? "s" : ""} <ArrowRight size={12} />
              </button>
            )}
          </div>
        </div>
      </div>
    </div>
  );
}
