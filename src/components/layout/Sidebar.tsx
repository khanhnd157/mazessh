import { ProfileList } from "@/components/profiles/ProfileList";

export function Sidebar() {
  return (
    <div className="w-72 border-r flex flex-col bg-card/50">
      <ProfileList />
    </div>
  );
}
