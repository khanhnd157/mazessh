import { useEffect, useRef } from "react";
import { listen } from "@tauri-apps/api/event";
import { Toaster, toast } from "sonner";
import { KeyRound, FileCode2, FolderGit2, Settings } from "lucide-react";
import { useProfileStore } from "@/stores/profileStore";
import { useAppStore } from "@/stores/appStore";
import { useLogStore } from "@/stores/logStore";
import { useThemeStore } from "@/stores/themeStore";
import { useSecurityStore } from "@/stores/securityStore";
import { useUiStore } from "@/stores/uiStore";
import { useInactivityTracker } from "@/hooks/useInactivityTracker";
import { useKeyboardShortcuts } from "@/hooks/useKeyboardShortcuts";
import { TitleBar } from "@/components/layout/TitleBar";
import { Sidebar } from "@/components/layout/Sidebar";
import { MainPanel } from "@/components/layout/MainPanel";
import { BottomBar } from "@/components/layout/BottomBar";
import { ConfigPreview } from "@/components/ssh-config/ConfigPreview";
import { RepoMappingList } from "@/components/repos/RepoMappingList";
import { LockScreen } from "@/components/security/LockScreen";
import { SecuritySettingsPanel } from "@/components/settings/SecuritySettings";
import type { AgentStatusEvent } from "@/types";

function App() {
  const theme = useThemeStore((s) => s.theme);
  const isLocked = useSecurityStore((s) => s.isLocked);
  const initialized = useSecurityStore((s) => s.initialized);
  const activeTab = useUiStore((s) => s.activeTab);
  const setActiveTab = useUiStore((s) => s.setActiveTab);
  const didInit = useRef(false);

  useInactivityTracker();
  useKeyboardShortcuts();

  // One-time initialization
  useEffect(() => {
    if (didInit.current) return;
    didInit.current = true;

    useSecurityStore.getState().fetchLockState();
    useSecurityStore.getState().fetchSettings();
  }, []);

  // Fetch data once unlocked
  useEffect(() => {
    if (!isLocked && initialized) {
      useProfileStore.getState().fetchProfiles();
      useAppStore.getState().fetchActiveProfile();
      useAppStore.getState().fetchGitIdentity();
    }
  }, [isLocked, initialized]);

  // Event listeners — stable, no store deps
  useEffect(() => {
    const unlistenAgent = listen<AgentStatusEvent>("agent-status", (event) => {
      useLogStore.getState().addLog({
        action: "agent",
        detail: event.payload.status,
        level: event.payload.success ? "info" : "warn",
      });
      if (event.payload.success) {
        toast.success("SSH Agent", { description: event.payload.status });
      } else {
        toast.warning("SSH Agent", { description: event.payload.status });
      }
      useAppStore.getState().fetchGitIdentity();
    });

    const unlistenLock = listen("lock-state-changed", (event) => {
      const payload = event.payload as { is_locked: boolean };
      useSecurityStore.getState().setLocked(payload.is_locked);
      if (payload.is_locked) {
        toast.info("App locked");
      }
    });

    const unlistenExpiry = listen("agent-expired", (event) => {
      const payload = event.payload as { message: string };
      useLogStore.getState().addLog({ action: "agent", detail: payload.message, level: "warn" });
      toast.warning("Agent keys expired", { description: payload.message });
      useAppStore.getState().fetchActiveProfile();
    });

    return () => {
      unlistenAgent.then((fn) => fn());
      unlistenLock.then((fn) => fn());
      unlistenExpiry.then((fn) => fn());
    };
  }, []);

  return (
    <div className="h-screen flex flex-col overflow-hidden bg-background">
      <Toaster
        position="top-right"
        richColors
        theme={theme}
        toastOptions={{ duration: 3000 }}
      />

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
          {/* All panels stay mounted; active panel fades in, others hidden */}
          <div className="flex-1 min-h-0 overflow-hidden relative">
            <TabPanel active={activeTab === "profiles"}>
              <MainPanel />
            </TabPanel>
            <TabPanel active={activeTab === "repos"} scrollable>
              <RepoMappingList />
            </TabPanel>
            <TabPanel active={activeTab === "config"} scrollable>
              <ConfigPreview />
            </TabPanel>
            <TabPanel active={activeTab === "settings"} scrollable>
              <SecuritySettingsPanel />
            </TabPanel>
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
      className={`relative flex items-center gap-1.5 px-4 py-2.5 text-sm font-medium transition-all duration-150 ${
        active ? "text-foreground" : "text-muted-foreground hover:text-foreground"
      }`}
    >
      {icon}
      {label}
      {/* Animated underline indicator */}
      <span
        className={`absolute bottom-0 left-2 right-2 h-0.5 rounded-full bg-primary transition-all duration-200 ease-out ${
          active ? "opacity-100 scale-x-100" : "opacity-0 scale-x-0"
        }`}
      />
    </button>
  );
}

function TabPanel({
  active,
  scrollable,
  children,
}: {
  active: boolean;
  scrollable?: boolean;
  children: React.ReactNode;
}) {
  return (
    <div
      className={`absolute inset-0 transition-opacity duration-150 ease-in-out ${
        active
          ? "opacity-100 z-10 pointer-events-auto"
          : "opacity-0 z-0 pointer-events-none"
      } ${scrollable ? "p-6 overflow-y-auto" : ""}`}
    >
      {children}
    </div>
  );
}

export default App;
