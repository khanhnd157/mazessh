import { User } from "lucide-react";
import { useAppStore } from "@/stores/appStore";
import { ActivityLog } from "@/components/logs/ActivityLog";

export function BottomBar() {
  const currentGitIdentity = useAppStore((s) => s.currentGitIdentity);

  return (
    <div className="h-36 shrink-0 border-t bg-card/20 flex flex-col">
      <ActivityLog />
      <div className="flex items-center gap-1.5 px-3 py-1 border-t text-[10px] text-muted-foreground/40 shrink-0">
        <User size={9} />
        <span>
          {currentGitIdentity
            ? `${currentGitIdentity.user_name} <${currentGitIdentity.user_email}>`
            : "Git identity not configured"}
        </span>
      </div>
    </div>
  );
}
