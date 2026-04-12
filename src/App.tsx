import { useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { Toaster, toast } from "sonner";
import { KeyRound, FileCode2, FolderGit2, Settings } from "lucide-react";
import { useProfileStore } from "@/stores/profileStore";
import { useAppStore } from "@/stores/appStore";
import { useLogStore } from "@/stores/logStore";
import { useThemeStore } from "@/stores/themeStore";
import { useSecurityStore } from "@/stores/securityStore";
import { useInactivityTracker } from "@/hooks/useInactivityTracker";
import { TitleBar } from "@/components/layout/TitleBar";
import { Sidebar } from "@/components/layout/Sidebar";
import { MainPanel } from "@/components/layout/MainPanel";
import { BottomBar } from "@/components/layout/BottomBar";
import { ConfigPreview } from "@/components/ssh-config/ConfigPreview";
import { RepoMappingList } from "@/components/repos/RepoMappingList";
import { LockScreen } from "@/components/security/LockScreen";
import { SecuritySettingsPanel } from "@/components/settings/SecuritySettings";
import type { AgentStatusEvent } from "@/types";

type Tab = "profiles" | "config" | "repos" | "settings";

function App() {
  const { fetchProfiles } = useProfileStore();
  const { fetchActiveProfile, fetchGitIdentity } = useAppStore();
  const { addLog } = useLogStore();
  const { theme } = useThemeStore();
  const { isLocked, initialized, fetchLockState, setLocked, fetchSettings } = useSecurityStore();
  const [activeTab, setActiveTab] = useState<Tab>("profiles");

  // Track user activity for auto-lock
  useInactivityTracker();

  // Initialize
  useEffect(() => {
    fetchLockState();
    fetchSettings();
  }, [fetchLockState, fetchSettings]);

  // Only fetch data once unlocked
  useEffect(() => {
    if (!isLocked && initialized) {
      fetchProfiles();
      fetchActiveProfile();
      fetchGitIdentity();
    }
  }, [isLocked, initialized, fetchProfiles, fetchActiveProfile, fetchGitIdentity]);

  // Listen for backend events
  useEffect(() => {
    const unlistenAgent = listen<AgentStatusEvent>("agent-status", (event) => {
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
      fetchGitIdentity();
    });

    const unlistenLock = listen("lock-state-changed", (event) => {
      const payload = event.payload as { is_locked: boolean };
      setLocked(payload.is_locked);
      if (payload.is_locked) {
        toast.info("App locked");
      }
    });

    const unlistenExpiry = listen("agent-expired", (event) => {
      const payload = event.payload as { message: string };
      addLog({ action: "agent", detail: payload.message, level: "warn" });
      toast.warning("Agent keys expired", { description: payload.message });
      fetchActiveProfile();
    });

    return () => {
      unlistenAgent.then((fn) => fn());
      unlistenLock.then((fn) => fn());
      unlistenExpiry.then((fn) => fn());
    };
  }, [addLog, fetchGitIdentity, fetchActiveProfile, setLocked]);

  return (
    <div className="h-screen flex flex-col overflow-hidden bg-background">
      <Toaster
        position="top-right"
        richColors
        theme={theme}
        toastOptions={{ duration: 3000 }}
      />

      {/* Lock screen overlay */}
      {isLocked && initialized && <LockScreen />}

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
              icon={<FolderGit2 size={14} />}
              label="Repo Mappings"
              active={activeTab === "repos"}
              onClick={() => setActiveTab("repos")}
            />
            <TabButton
              icon={<FileCode2 size={14} />}
              label="SSH Config"
              active={activeTab === "config"}
              onClick={() => setActiveTab("config")}
            />
            <TabButton
              icon={<Settings size={14} />}
              label="Settings"
              active={activeTab === "settings"}
              onClick={() => setActiveTab("settings")}
            />
          </div>
          <div className="flex-1 min-h-0 overflow-hidden">
            {activeTab === "profiles" && <MainPanel />}
            {activeTab === "repos" && (
              <div className="p-6 overflow-y-auto h-full">
                <RepoMappingList />
              </div>
            )}
            {activeTab === "config" && (
              <div className="p-6 overflow-y-auto h-full">
                <ConfigPreview />
              </div>
            )}
            {activeTab === "settings" && (
              <div className="p-6 overflow-y-auto h-full">
                <SecuritySettingsPanel />
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
