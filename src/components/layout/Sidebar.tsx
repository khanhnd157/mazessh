import { ProfileList } from "@/components/profiles/ProfileList";

export function Sidebar() {
  return (
    <div className="w-60 border-r flex flex-col bg-card/20">
      <ProfileList />
    </div>
  );
}
