import { invoke } from "@tauri-apps/api/core";
import type {
  ActivationResult,
  AuditEntry,
  ConfigBackup,
  ConnectionTestResult,
  CreateProfileInput,
  CreateRepoMappingInput,
  DetectedKey,
  GitConfigScope,
  GitIdentityInfo,
  KeyFingerprint,
  LockStateResponse,
  ProfileSummary,
  RepoMapping,
  RepoMappingSummary,
  SecuritySettings,
  SshProfile,
  UpdateProfileInput,
} from "@/types";

export const commands = {
  // Profiles
  getProfiles: () => invoke<ProfileSummary[]>("get_profiles"),
  getProfile: (id: string) => invoke<SshProfile>("get_profile", { id }),
  createProfile: (input: CreateProfileInput) =>
    invoke<SshProfile>("create_profile", { input }),
  updateProfile: (id: string, input: UpdateProfileInput) =>
    invoke<SshProfile>("update_profile", { id, input }),
  deleteProfile: (id: string) => invoke<void>("delete_profile", { id }),

  // Scanner
  scanSshKeys: () => invoke<DetectedKey[]>("scan_ssh_keys"),

  // Switch
  activateProfile: (id: string) =>
    invoke<ActivationResult>("activate_profile", { id }),
  deactivateProfile: () => invoke<void>("deactivate_profile"),
  getActiveProfile: () =>
    invoke<ProfileSummary | null>("get_active_profile"),

  // SSH Config
  previewSshConfig: () => invoke<string>("preview_ssh_config"),
  writeSshConfig: () => invoke<void>("write_ssh_config"),
  backupSshConfig: () => invoke<string>("backup_ssh_config"),

  // Git
  getGitSshCommand: (id: string) =>
    invoke<string>("get_git_ssh_command", { id }),
  testSshConnection: (id: string) =>
    invoke<ConnectionTestResult>("test_ssh_connection", { id }),

  // Repo Mappings
  getRepoMappings: () => invoke<RepoMappingSummary[]>("get_repo_mappings"),
  getRepoMappingsForProfile: (profileId: string) =>
    invoke<RepoMappingSummary[]>("get_repo_mappings_for_profile", { profileId }),
  createRepoMapping: (input: CreateRepoMappingInput) =>
    invoke<RepoMapping>("create_repo_mapping", { input }),
  deleteRepoMapping: (id: string) =>
    invoke<void>("delete_repo_mapping", { id }),
  updateRepoMappingScope: (id: string, scope: GitConfigScope) =>
    invoke<RepoMapping>("update_repo_mapping_scope", { id, scope }),

  // Git Identity
  getCurrentGitIdentity: () =>
    invoke<GitIdentityInfo>("get_current_git_identity"),
  getRepoGitIdentity: (repoPath: string) =>
    invoke<GitIdentityInfo>("get_repo_git_identity", { repoPath }),
  syncGitIdentity: (profileId: string, repoPath: string | null, scope: GitConfigScope) =>
    invoke<void>("sync_git_identity", { profileId, repoPath, scope }),

  // Repo Detection
  resolveRepoPath: (path: string) =>
    invoke<string | null>("resolve_repo_path", { path }),
  checkRepoMapping: (path: string) =>
    invoke<RepoMappingSummary | null>("check_repo_mapping", { path }),
  autoSwitchForRepo: (path: string) =>
    invoke<ActivationResult | null>("auto_switch_for_repo", { path }),

  // Security
  setupPin: (pin: string) => invoke<void>("setup_pin", { pin }),
  verifyPin: (pin: string) => invoke<boolean>("verify_pin", { pin }),
  changePin: (oldPin: string, newPin: string) =>
    invoke<void>("change_pin", { oldPin, newPin }),
  removePin: (pin: string) => invoke<void>("remove_pin", { pin }),
  lockApp: () => invoke<void>("lock_app"),
  getLockState: () => invoke<LockStateResponse>("get_lock_state"),
  getSecuritySettings: () => invoke<SecuritySettings>("get_security_settings"),
  updateSecuritySettings: (settings: SecuritySettings) =>
    invoke<void>("update_security_settings", { settings }),
  getAuditLogs: (limit: number, offset: number, actionFilter?: string) =>
    invoke<AuditEntry[]>("get_audit_logs", { limit, offset, actionFilter: actionFilter ?? null }),
  getAgentTimeRemaining: () => invoke<number | null>("get_agent_time_remaining"),
  touchActivity: () => invoke<void>("touch_activity"),

  // SSH Config (M4)
  listConfigBackups: () => invoke<ConfigBackup[]>("list_config_backups"),
  rollbackSshConfig: (backupPath: string) =>
    invoke<void>("rollback_ssh_config", { backupPath }),
  readCurrentSshConfig: () => invoke<string>("read_current_ssh_config"),

  // Hooks
  generateGitHook: (repoPath: string) =>
    invoke<string>("generate_git_hook", { repoPath }),
  removeGitHook: (repoPath: string) =>
    invoke<void>("remove_git_hook", { repoPath }),

  // Advanced
  exportProfiles: () => invoke<string>("export_profiles"),
  importProfiles: (json: string) => invoke<number>("import_profiles", { json }),
  getKeyFingerprint: (id: string) =>
    invoke<KeyFingerprint>("get_key_fingerprint", { id }),
};
