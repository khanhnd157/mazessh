import { memo } from "react";
import type { SshKeyItemSummary } from "@/types";

interface Props {
  item: SshKeyItemSummary;
  selected: boolean;
  onSelect: (id: string) => void;
}

function formatCreatedAt(iso: string): string {
  const d = new Date(iso);
  return `Created ${Number.isNaN(d.getTime()) ? "—" : d.toLocaleDateString()}`;
}

export const KeyCard = memo(function KeyCard({ item, selected, onSelect }: Props) {
  return (
    <button
      type="button"
      onClick={() => onSelect(item.id)}
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
          {item.allowed_hosts.length > 0 && (
            <div className="flex flex-wrap gap-1 mt-1.5">
              {item.allowed_hosts.map((host) => (
                <span key={host} className="text-[8px] px-1 py-0.5 rounded bg-accent text-muted-foreground font-medium">
                  {host}
                </span>
              ))}
            </div>
          )}
        </div>
        <div className={`w-2 h-2 rounded-full shrink-0 mt-1.5 ${
          item.state === "active" ? "bg-success" : "bg-muted-foreground/30"
        }`} />
      </div>
      <div className="text-[10px] text-muted-foreground/40 mt-2">
        {formatCreatedAt(item.created_at)}
      </div>
    </button>
  );
});
