import { ProfileList } from "@/components/profiles/ProfileList";

export function Sidebar() {
  return (
    <div className="w-64 border-r flex flex-col bg-card/30">
      <ProfileList />
    </div>
  );
}
