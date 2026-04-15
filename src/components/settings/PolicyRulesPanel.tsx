import { useState, useEffect } from "react";
import { Shield, Trash2, X } from "lucide-react";
import { toast } from "sonner";
import { commands } from "@/lib/tauri-commands";

interface PolicyRule {
  key_id: string;
  key_name: string;
  rule_type: string;
  created_at: string;
}

export function PolicyRulesPanel() {
  const [rules, setRules] = useState<PolicyRule[]>([]);
  const [loading, setLoading] = useState(false);

  const fetchRules = async () => {
    setLoading(true);
    try {
      const r = await commands.listPolicyRules();
      setRules(r);
    } catch {
      // may fail if locked
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    fetchRules();
  }, []);

  const handleDelete = async (keyId: string) => {
    try {
      await commands.deletePolicyRule(keyId);
      toast.success("Rule removed");
      fetchRules();
    } catch (e) {
      toast.error("Failed to remove rule", { description: String(e) });
    }
  };

  const handleClearAll = async () => {
    try {
      await commands.clearAllPolicyRules();
      toast.success("All rules cleared");
      setRules([]);
    } catch (e) {
      toast.error("Failed to clear rules", { description: String(e) });
    }
  };

  return (
    <div className="rounded-xl border bg-card p-4 space-y-3">
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-2">
          <Shield size={14} className="text-muted-foreground" />
          <span className="text-sm font-medium">Agent Policy Rules</span>
        </div>
        {rules.length > 0 && (
          <button
            type="button"
            onClick={handleClearAll}
            className="flex items-center gap-1 px-2.5 py-1 text-[10px] font-medium rounded-lg text-destructive/70 hover:text-destructive hover:bg-destructive/10 transition-colors"
          >
            <X size={10} /> Clear All
          </button>
        )}
      </div>
      <p className="text-xs text-muted-foreground">
        Keys with "Always Allow" rules skip the consent popup. Session rules are cleared when the app locks.
      </p>

      {loading ? (
        <div className="text-xs text-muted-foreground/50 py-2">Loading...</div>
      ) : rules.length === 0 ? (
        <div className="text-xs text-muted-foreground/40 py-2">
          No policy rules set. Rules are created when you approve a signing request.
        </div>
      ) : (
        <div className="space-y-1.5">
          {rules.map((rule) => (
            <div
              key={rule.key_id}
              className="flex items-center justify-between p-2.5 rounded-lg bg-secondary/40"
            >
              <div className="min-w-0 flex-1">
                <div className="text-xs font-medium">{rule.key_name}</div>
                <div className="flex items-center gap-2 mt-0.5">
                  <span className="text-[9px] px-1.5 py-0.5 rounded bg-success/15 text-success font-medium">
                    Always Allow
                  </span>
                  <span className="text-[10px] text-muted-foreground/40">
                    Since {new Date(rule.created_at).toLocaleDateString()}
                  </span>
                </div>
              </div>
              <button
                type="button"
                onClick={() => handleDelete(rule.key_id)}
                title="Remove rule"
                className="p-1.5 rounded-md text-muted-foreground/30 hover:text-destructive hover:bg-destructive/10 transition-colors"
              >
                <Trash2 size={12} />
              </button>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
