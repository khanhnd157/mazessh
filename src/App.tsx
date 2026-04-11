import { useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { Toaster, toast } from "sonner";
import { KeyRound, FileCode2 } from "lucide-react";
import { useProfileStore } from "@/stores/profileStore";
import { useAppStore } from "@/stores/appStore";
import { useLogStore } from "@/stores/logStore";
import { useThemeStore } from "@/stores/themeStore";
import { TitleBar } from "@/components/layout/TitleBar";
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
  const { theme } = useThemeStore();
  const [activeTab, setActiveTab] = useState<Tab>("profiles");

  useEffect(() => {
    fetchProfiles();
    fetchActiveProfile();
  }, [fetchProfiles, fetchActiveProfile]);

  useEffect(() => {
    const unlisten = listen<AgentStatusEvent>("agent-status", (event) => {
      addLog({
        action: "agent",
        detail: event.payload.status,
        level: event.payload.success ? "info" : "warn",
      });
      if (event.payload.success) {
        toast.success("SSH Agent", { description: event.payload.status });
      } else {
        toast.warning("SSH Agent", { description: event.payload.status });
      }
    });
    return () => {
      unlisten.then((fn) => fn());
    };
  }, [addLog]);

  return (
    <div className="h-screen flex flex-col overflow-hidden bg-background">
      <Toaster
        position="top-right"
        richColors
        theme={theme}
        toastOptions={{ duration: 3000 }}
      />
      <TitleBar />
      <div className="flex-1 min-h-0 flex overflow-hidden">
        <Sidebar />
        <div className="flex-1 min-h-0 flex flex-col overflow-hidden">
          <div className="flex border-b bg-card/30 h-10 shrink-0">
            <TabButton
              icon={<KeyRound size={14} />}
              label="Profiles"
              active={activeTab === "profiles"}
              onClick={() => setActiveTab("profiles")}
            />
            <TabButton
              icon={<FileCode2 size={14} />}
              label="SSH Config"
              active={activeTab === "config"}
              onClick={() => setActiveTab("config")}
            />
          </div>
          <div className="flex-1 min-h-0 overflow-hidden">
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

function TabButton({
  icon,
  label,
  active,
  onClick,
}: {
  icon: React.ReactNode;
  label: string;
  active: boolean;
  onClick: () => void;
}) {
  return (
    <button
      type="button"
      onClick={onClick}
      className={`flex items-center gap-1.5 px-4 py-2.5 text-sm font-medium transition-colors ${
        active
          ? "border-b-2 border-primary text-foreground"
          : "text-muted-foreground hover:text-foreground"
      }`}
    >
      {icon}
      {label}
    </button>
  );
}

export default App;
