import { useState, useEffect, useMemo } from "react";
import { Plus, KeyRound, Search, Radio, X } from "lucide-react";
import { toast } from "sonner";
import { commands } from "@/lib/tauri-commands";
import { useVaultStore } from "@/stores/vaultStore";
import { VaultSetupPrompt } from "./VaultSetupPrompt";
import { VaultUnlockPrompt } from "./VaultUnlockPrompt";
import { GenerateKeyDialog } from "./GenerateKeyDialog";
import { ImportKeyDialog } from "./ImportKeyDialog";
import { KeyDetailSheet } from "./KeyDetailSheet";
import { MigrationWizard } from "@/components/migration/MigrationWizard";
import { VaultPassphraseChange } from "./VaultPassphraseChange";
import { KeyCard } from "./KeyCard";
import type { KeyState } from "@/types";

const KEY_FILTER_TABS = ["all", "active", "archived"] as const;

export function KeyVaultList() {
  const vaultState = useVaultStore((s) => s.vaultState);
  const keys = useVaultStore((s) => s.keys);
  const keysLoading = useVaultStore((s) => s.keysLoading);
  const selectedKeyId = useVaultStore((s) => s.selectedKeyId);
  const selectedKey = useVaultStore((s) => s.selectedKey);
  const fetchKeys = useVaultStore((s) => s.fetchKeys);
  const selectKey = useVaultStore((s) => s.selectKey);
  const [search, setSearch] = useState("");
  const [filterState, setFilterState] = useState<"all" | KeyState>("all");
  const [showGenerate, setShowGenerate] = useState(false);
  const [showImport, setShowImport] = useState(false);
  const [showMigration, setShowMigration] = useState(false);
  const [agentPipe, setAgentPipe] = useState<string | null>(null);

  useEffect(() => {
    if (vaultState?.unlocked) {
      fetchKeys();
      commands.getAgentPipePath().then(setAgentPipe).catch(() => {});
    }
  }, [vaultState?.unlocked, fetchKeys]);

  const filtered = useMemo(() => {
    const q = search.toLowerCase();
    return keys.filter((k) => {
      if (filterState !== "all" && k.state !== filterState) return false;
      if (q) return k.name.toLowerCase().includes(q) || k.fingerprint.toLowerCase().includes(q);
      return true;
    });
  }, [keys, filterState, search]);

  // Vault not initialized
  if (vaultState && !vaultState.initialized) {
    return <VaultSetupPrompt />;
  }

  // Vault locked
  if (vaultState && !vaultState.unlocked) {
    return <VaultUnlockPrompt />;
  }

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
            <Search size={13} className="absolute left-2.5 top-1/2 -translate-y-1/2 text-muted-foreground/40 pointer-events-none" />
            <input
              value={search}
              onChange={(e) => setSearch(e.target.value)}
              placeholder="Search..."
              aria-label="Search keys"
              className="pl-7 pr-7 py-1.5 text-xs rounded-lg bg-secondary border border-border w-48 focus:outline-none focus:ring-2 focus:ring-ring"
            />
            {search && (
              <button
                type="button"
                onClick={() => setSearch("")}
                aria-label="Clear search"
                className="absolute right-2 top-1/2 -translate-y-1/2 text-muted-foreground/40 hover:text-muted-foreground transition-colors"
              >
                <X size={12} />
              </button>
            )}
          </div>
          <div className="flex rounded-lg border border-border overflow-hidden">
            {KEY_FILTER_TABS.map((s) => (
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
            onClick={() => setShowImport(true)}
            className="flex items-center gap-1.5 px-3 py-1.5 text-xs font-medium rounded-lg bg-secondary hover:bg-accent transition-colors"
          >
            Import
          </button>
          <button
            type="button"
            onClick={() => setShowMigration(true)}
            className="flex items-center gap-1.5 px-3 py-1.5 text-xs font-medium rounded-lg bg-secondary hover:bg-accent transition-colors"
          >
            Migrate
          </button>
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

      {/* Agent status */}
      {agentPipe && (
        <div className="flex items-center gap-2 px-3 py-1.5 rounded-lg bg-success/5 border border-success/15">
          <Radio size={12} className="text-success animate-pulse" />
          <span className="text-[11px] text-success font-medium">Agent running</span>
          <span className="text-[10px] text-muted-foreground/50 font-mono flex-1">{agentPipe}</span>
          <button
            type="button"
            onClick={async () => {
              try {
                const result = await commands.testAgentConnection();
                toast.success("Agent Test", { description: result });
              } catch (e) {
                toast.error("Agent Test Failed", { description: String(e) });
              }
            }}
            className="text-[10px] px-2 py-0.5 rounded bg-success/15 text-success font-medium hover:bg-success/25 transition-colors"
          >
            Test
          </button>
        </div>
      )}

      {/* Key grid */}
      {keysLoading ? (
        <div className="grid grid-cols-1 md:grid-cols-2 xl:grid-cols-3 gap-3" aria-hidden="true">
          {[1, 2, 3].map((i) => (
            <div key={i} className="rounded-lg p-3 border bg-secondary/50">
              <div className="flex items-start justify-between gap-2">
                <div className="space-y-2 flex-1">
                  <div className="flex items-center gap-1.5">
                    <div className="h-3.5 w-28 rounded bg-muted animate-pulse" />
                    <div className="h-4 w-14 rounded bg-muted/60 animate-pulse" />
                  </div>
                  <div className="h-2.5 w-44 rounded bg-muted/40 animate-pulse" />
                </div>
                <div className="w-2 h-2 rounded-full bg-muted animate-pulse mt-1.5" />
              </div>
              <div className="h-2.5 w-24 rounded bg-muted/30 animate-pulse mt-3" />
            </div>
          ))}
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
            {search ? (
              <button
                type="button"
                onClick={() => setSearch("")}
                className="text-xs text-primary hover:underline"
              >
                Clear search
              </button>
            ) : (
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
              onSelect={selectKey}
            />
          ))}
        </div>
      )}

      {/* Vault actions */}
      <div className="flex items-center gap-3 pt-2 border-t border-border/50">
        <VaultPassphraseChange />
        <button
          type="button"
          onClick={() => useVaultStore.getState().lockVault()}
          className="flex items-center gap-1.5 px-3 py-1.5 text-xs font-medium rounded-lg bg-secondary hover:bg-accent transition-colors text-muted-foreground"
        >
          Lock Vault
        </button>
      </div>

      {/* Dialogs */}
      {showGenerate && <GenerateKeyDialog onClose={() => setShowGenerate(false)} />}
      {showImport && <ImportKeyDialog onClose={() => setShowImport(false)} />}
      {showMigration && <MigrationWizard onClose={() => setShowMigration(false)} />}
      {selectedKey && (
        <KeyDetailSheet keyItem={selectedKey} onClose={() => selectKey(null)} />
      )}
    </div>
  );
}
