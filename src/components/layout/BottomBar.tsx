import { ActivityLog } from "@/components/logs/ActivityLog";

export function BottomBar() {
  return (
    <div className="h-40 border-t bg-card/50">
      <ActivityLog />
    </div>
  );
}
