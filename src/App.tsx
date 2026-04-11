import { useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { useProfileStore } from "@/stores/profileStore";
import { useAppStore } from "@/stores/appStore";
import { useLogStore } from "@/stores/logStore";
import { TopBar } from "@/components/layout/TopBar";
import { Sidebar } from "@/components/layout/Sidebar";
import { MainPanel } from "@/components/layout/MainPanel";
import { BottomBar } from "@/components/layout/BottomBar";
import { ConfigPreview } from "@/components/ssh-config/ConfigPreview";
import type { AgentStatusEvent } from "@/types";

type Tab = "profiles" | "config";

function App() {
  const { fetchProfiles } = useProfileStore();
  const { fetchActiveProfile } = useAppStore();
  const { addLog } = useLogStore();
  const [activeTab, setActiveTab] = useState<Tab>("profiles");

  useEffect(() => {
    fetchProfiles();
    fetchActiveProfile();
  }, [fetchProfiles, fetchActiveProfile]);

  // Listen for background agent-status events from Rust
  useEffect(() => {
    const unlisten = listen<AgentStatusEvent>("agent-status", (event) => {
      addLog({
        action: "agent",
        detail: event.payload.status,
        level: event.payload.success ? "info" : "warn",
      });
    });
    return () => {
      unlisten.then((fn) => fn());
    };
  }, [addLog]);

  return (
    <div className="h-screen flex flex-col overflow-hidden">
      <TopBar />
      <div className="flex-1 flex overflow-hidden">
        <Sidebar />
        <div className="flex-1 flex flex-col overflow-hidden">
          <div className="flex border-b">
            <TabButton
              label="Profiles"
              active={activeTab === "profiles"}
              onClick={() => setActiveTab("profiles")}
            />
            <TabButton
              label="SSH Config"
              active={activeTab === "config"}
              onClick={() => setActiveTab("config")}
            />
          </div>
          <div className="flex-1 overflow-hidden">
            {activeTab === "profiles" && <MainPanel />}
            {activeTab === "config" && (
              <div className="p-6 overflow-y-auto h-full">
                <ConfigPreview />
              </div>
            )}
          </div>
        </div>
      </div>
      <BottomBar />
    </div>
  );
}

function TabButton({ label, active, onClick }: { label: string; active: boolean; onClick: () => void }) {
  return (
    <button
      type="button"
      onClick={onClick}
      className={`px-4 py-2.5 text-sm font-medium transition-colors ${
        active
          ? "border-b-2 border-primary text-foreground"
          : "text-muted-foreground hover:text-foreground"
      }`}
    >
      {label}
    </button>
  );
}

export default App;
