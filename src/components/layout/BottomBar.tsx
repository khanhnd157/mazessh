import { User } from "lucide-react";
import { useAppStore } from "@/stores/appStore";
import { ActivityLog } from "@/components/logs/ActivityLog";

export function BottomBar() {
  const { currentGitIdentity } = useAppStore();

  return (
    <div className="h-40 shrink-0 border-t bg-card/50 flex flex-col">
      <ActivityLog />
      {/* Git identity status bar */}
      <div className="flex items-center gap-2 px-4 py-1 border-t text-[10px] text-muted-foreground/60 shrink-0">
        <User size={10} />
        <span>
          {currentGitIdentity
            ? `Git: ${currentGitIdentity.user_name} <${currentGitIdentity.user_email}>`
            : "Git: not configured"}
        </span>
      </div>
    </div>
  );
}
