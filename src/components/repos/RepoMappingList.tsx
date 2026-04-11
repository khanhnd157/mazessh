import { useEffect, useState } from "react";
import { Plus, FolderGit2 } from "lucide-react";
import { useRepoMappingStore } from "@/stores/repoMappingStore";
import { RepoMappingCard } from "./RepoMappingCard";
import { AddRepoMappingDialog } from "./AddRepoMappingDialog";

export function RepoMappingList() {
  const { mappings, loading, fetchMappings } = useRepoMappingStore();
  const [showAdd, setShowAdd] = useState(false);

  useEffect(() => {
    fetchMappings();
  }, [fetchMappings]);

  return (
    <div className="space-y-4 max-w-3xl">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-2">
          <FolderGit2 size={16} className="text-primary" />
          <h3 className="text-sm font-semibold">Repository Mappings</h3>
          {mappings.length > 0 && (
            <span className="text-[10px] text-muted-foreground/60">{mappings.length}</span>
          )}
        </div>
        <button
          type="button"
          onClick={() => setShowAdd(true)}
          className="flex items-center gap-1.5 px-3 py-1.5 text-xs font-medium rounded-lg bg-primary text-primary-foreground hover:bg-primary/90 transition-colors"
        >
          <Plus size={13} />
          Add Mapping
        </button>
      </div>

      {/* Description */}
      <p className="text-xs text-muted-foreground">
        Map repositories to SSH profiles. When you use auto-switch on a mapped repo, the correct
        profile activates and git identity is configured automatically.
      </p>

      {/* List */}
      {loading && mappings.length === 0 && (
        <p className="text-sm text-muted-foreground py-4">Loading...</p>
      )}

      {!loading && mappings.length === 0 && (
        <div className="text-center py-10">
          <div className="w-12 h-12 rounded-2xl bg-primary/8 flex items-center justify-center mx-auto mb-3">
            <FolderGit2 size={22} className="text-primary/50" />
          </div>
          <p className="text-sm text-foreground/60">No repo mappings yet</p>
          <p className="text-xs text-muted-foreground mt-1">
            Map a repository to a profile to enable auto-switching
          </p>
          <button
            type="button"
            onClick={() => setShowAdd(true)}
            className="mt-3 text-xs text-primary hover:underline"
          >
            Add your first mapping
          </button>
        </div>
      )}

      {mappings.length > 0 && (
        <div className="space-y-2">
          {mappings.map((m) => (
            <RepoMappingCard key={m.id} mapping={m} />
          ))}
        </div>
      )}

      {showAdd && <AddRepoMappingDialog onClose={() => setShowAdd(false)} />}
    </div>
  );
}
