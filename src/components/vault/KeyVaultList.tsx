import { useState, useEffect } from "react";
import { Plus, KeyRound, Search } from "lucide-react";
import { useVaultStore } from "@/stores/vaultStore";
import { VaultSetupPrompt } from "./VaultSetupPrompt";
import { VaultUnlockPrompt } from "./VaultUnlockPrompt";
import { GenerateKeyDialog } from "./GenerateKeyDialog";
import { KeyDetailSheet } from "./KeyDetailSheet";
import type { KeyState, SshKeyItemSummary } from "@/types";

export function KeyVaultList() {
  const { vaultState, keys, keysLoading, selectedKeyId, selectedKey, fetchKeys, selectKey } = useVaultStore();
  const [search, setSearch] = useState("");
  const [filterState, setFilterState] = useState<"all" | KeyState>("all");
  const [showGenerate, setShowGenerate] = useState(false);

  useEffect(() => {
    if (vaultState?.unlocked) {
      fetchKeys();
    }
  }, [vaultState?.unlocked, fetchKeys]);

  // Vault not initialized
  if (vaultState && !vaultState.initialized) {
    return <VaultSetupPrompt />;
  }

  // Vault locked
  if (vaultState && !vaultState.unlocked) {
    return <VaultUnlockPrompt />;
  }

  const filtered = keys.filter((k) => {
    if (filterState !== "all" && k.state !== filterState) return false;
    if (search) {
      const q = search.toLowerCase();
      return k.name.toLowerCase().includes(q) || k.fingerprint.toLowerCase().includes(q);
    }
    return true;
  });

  return (
    <div className="space-y-4">
      {/* Top bar */}
      <div className="flex items-center justify-between gap-3">
        <div className="flex items-center gap-2">
          <h2 className="text-sm font-semibold">Key Vault</h2>
          <span className="text-[10px] px-1.5 py-0.5 rounded bg-secondary text-muted-foreground font-medium">
            {keys.length} key{keys.length !== 1 ? "s" : ""}
          </span>
        </div>
        <div className="flex items-center gap-2">
          <div className="relative">
            <Search size={13} className="absolute left-2.5 top-1/2 -translate-y-1/2 text-muted-foreground/40" />
            <input
              value={search}
              onChange={(e) => setSearch(e.target.value)}
              placeholder="Search..."
              className="pl-7 pr-3 py-1.5 text-xs rounded-lg bg-secondary border border-border w-48 focus:outline-none focus:ring-2 focus:ring-ring"
            />
          </div>
          <div className="flex rounded-lg border border-border overflow-hidden">
            {(["all", "active", "archived"] as const).map((s) => (
              <button
                key={s}
                type="button"
                onClick={() => setFilterState(s)}
                className={`px-2.5 py-1 text-[10px] font-medium transition-colors ${
                  filterState === s ? "bg-primary/15 text-primary" : "text-muted-foreground hover:text-foreground"
                }`}
              >
                {s.charAt(0).toUpperCase() + s.slice(1)}
              </button>
            ))}
          </div>
          <button
            type="button"
            onClick={() => setShowGenerate(true)}
            className="flex items-center gap-1.5 px-3 py-1.5 text-xs font-medium rounded-lg bg-primary text-primary-foreground hover:bg-primary/90 transition-colors"
          >
            <Plus size={13} />
            Generate Key
          </button>
        </div>
      </div>

      {/* Key grid */}
      {keysLoading ? (
        <div className="flex items-center justify-center py-16">
          <div className="text-xs text-muted-foreground/50">Loading keys...</div>
        </div>
      ) : filtered.length === 0 ? (
        <div className="flex items-center justify-center py-16">
          <div className="text-center">
            <div className="w-12 h-12 rounded-2xl bg-primary/6 flex items-center justify-center mx-auto mb-3">
              <KeyRound size={20} className="text-primary/30" />
            </div>
            <p className="text-sm text-muted-foreground/50 mb-2">
              {search ? "No keys match your search" : "No keys yet"}
            </p>
            {!search && (
              <button
                type="button"
                onClick={() => setShowGenerate(true)}
                className="text-xs text-primary hover:underline"
              >
                Generate your first key
              </button>
            )}
          </div>
        </div>
      ) : (
        <div className="grid grid-cols-1 md:grid-cols-2 xl:grid-cols-3 gap-3">
          {filtered.map((key) => (
            <KeyCard
              key={key.id}
              item={key}
              selected={selectedKeyId === key.id}
              onClick={() => selectKey(key.id)}
            />
          ))}
        </div>
      )}

      {/* Dialogs */}
      {showGenerate && <GenerateKeyDialog onClose={() => setShowGenerate(false)} />}
      {selectedKey && (
        <KeyDetailSheet keyItem={selectedKey} onClose={() => selectKey(null)} />
      )}
    </div>
  );
}

function KeyCard({ item, selected, onClick }: { item: SshKeyItemSummary; selected: boolean; onClick: () => void }) {
  return (
    <button
      type="button"
      onClick={onClick}
      className={`w-full text-left rounded-lg p-3 transition-all duration-100 border ${
        selected
          ? "bg-primary/8 border-primary/25 ring-1 ring-primary/20"
          : "bg-secondary/50 border-border hover:bg-accent/50"
      }`}
    >
      <div className="flex items-start justify-between gap-2">
        <div className="min-w-0">
          <div className="flex items-center gap-1.5 mb-1">
            <span className="font-medium text-[12.5px] truncate">{item.name}</span>
            <span className="text-[9px] px-1.5 py-0.5 rounded bg-primary/15 text-primary font-medium shrink-0">
              {item.algorithm === "ed25519" ? "Ed25519" : "RSA"}
            </span>
          </div>
          <div className="font-mono text-[10.5px] text-muted-foreground/60 truncate">
            {item.fingerprint}
          </div>
        </div>
        <div className={`w-2 h-2 rounded-full shrink-0 mt-1.5 ${
          item.state === "active" ? "bg-success" : "bg-muted-foreground/30"
        }`} />
      </div>
      <div className="text-[10px] text-muted-foreground/40 mt-2">
        Created {new Date(item.created_at).toLocaleDateString()}
      </div>
    </button>
  );
}
