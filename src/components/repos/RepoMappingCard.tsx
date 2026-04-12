import { FolderGit2, Trash2, GitBranch } from "lucide-react";
import { toast } from "sonner";
import { commands } from "@/lib/tauri-commands";
import { useRepoMappingStore } from "@/stores/repoMappingStore";
import { useLogStore } from "@/stores/logStore";
import { ProviderIcon } from "@/components/profiles/ProviderIcon";
import type { RepoMappingSummary } from "@/types";
import { useProfileStore } from "@/stores/profileStore";

interface RepoMappingCardProps {
  mapping: RepoMappingSummary;
}

export function RepoMappingCard({ mapping }: RepoMappingCardProps) {
  const { deleteMapping } = useRepoMappingStore();
  const { profiles } = useProfileStore();
  const { addLog } = useLogStore();

  const profile = profiles.find((p) => p.id === mapping.profile_id);

  const handleDelete = async () => {
    if (!confirm(`Remove mapping for "${mapping.repo_name}"?`)) return;
    try {
      await deleteMapping(mapping.id);
      addLog({ action: "unmap", detail: `Removed mapping for ${mapping.repo_name}`, level: "info" });
      toast.success(`Mapping removed: ${mapping.repo_name}`);
    } catch (err) {
      toast.error("Failed to remove mapping", { description: String(err) });
    }
  };

  return (
    <div className="group flex items-center gap-3 px-4 py-3 rounded-lg bg-secondary/30 hover:bg-secondary/50 transition-colors">
      <div className="w-9 h-9 rounded-lg bg-primary/10 flex items-center justify-center shrink-0">
        <FolderGit2 size={18} className="text-primary/70" />
      </div>

      <div className="min-w-0 flex-1">
        <div className="flex items-center gap-2">
          <span className="text-sm font-medium truncate">{mapping.repo_name}</span>
          <span className={`text-[10px] px-1.5 py-0.5 rounded font-medium ${
            mapping.git_config_scope === "local"
              ? "bg-primary/15 text-primary"
              : "bg-muted text-muted-foreground"
          }`}>
            {mapping.git_config_scope === "local" ? "Local" : "Global"}
          </span>
        </div>
        <p className="text-[11px] text-muted-foreground font-mono truncate mt-0.5">
          {mapping.repo_path}
        </p>
      </div>

      <div className="flex items-center gap-2 shrink-0">
        {profile && (
          <div className="flex items-center gap-1.5 px-2 py-1 rounded-md bg-accent/50">
            <ProviderIcon provider={profile.provider} size={13} />
            <span className="text-[11px] font-medium">{mapping.profile_name}</span>
          </div>
        )}
        <button
          type="button"
          onClick={async () => {
            try {
              const path = await commands.generateGitHook(mapping.repo_path);
              addLog({ action: "hook", detail: `Pre-push hook installed: ${path}`, level: "info" });
              toast.success("Git hook installed", { description: "pre-push identity validation" });
            } catch (err) {
              toast.error("Hook failed", { description: String(err) });
            }
          }}
          title="Install pre-push hook"
          className="p-1.5 rounded-md text-muted-foreground/40 hover:text-primary hover:bg-primary/10 opacity-0 group-hover:opacity-100 transition-all"
        >
          <GitBranch size={13} />
        </button>
        <button
          type="button"
          onClick={handleDelete}
          title="Remove mapping"
          className="p-1.5 rounded-md text-muted-foreground/40 hover:text-destructive hover:bg-destructive/10 opacity-0 group-hover:opacity-100 transition-all"
        >
          <Trash2 size={13} />
        </button>
      </div>
    </div>
  );
}
