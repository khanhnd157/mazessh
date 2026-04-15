import { invoke } from "@tauri-apps/api/core";
import type {
  ActivationResult,
  AuditEntry,
  BinaryUpdateStatus,
  BinaryVersion,
  BootstrapAllResult,
  BridgeHistoryEvent,
  BridgeOverview,
  BridgeProvider,
  ConfigBackup,
  ConnectionTestResult,
  CreateProfileInput,
  CreateRepoMappingInput,
  DetectedKey,
  DiagnosticsResult,
  DistroBridgeStatus,
  GenerateKeyRequest,
  GitConfigScope,
  GitIdentityInfo,
  ImportKeyRequest,
  KeyFingerprint,
  KeyHealthReport,
  LockStateResponse,
  MigrationPreview,
  MigrationReport,
  NamedPipeEntry,
  ProfileSummary,
  ProviderStatus,
  RelayMode,
  RepoMapping,
  RepoMappingSummary,
  SecuritySettings,
  ShellInjection,
  SshHostTestResult,
  SshKeyItem,
  SshKeyItemSummary,
  SshProfile,
  UpdateKeyRequest,
  UpdateProfileInput,
  VaultStateResponse,
  WslDistro,
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
  checkAllKeysHealth: () =>
    invoke<KeyHealthReport[]>("check_all_keys_health"),
  readPublicKey: (id: string) =>
    invoke<string>("read_public_key", { id }),

  // Bridge
  getBridgeOverview: () =>
    invoke<BridgeOverview>("get_bridge_overview"),
  listWslDistros: () =>
    invoke<WslDistro[]>("list_wsl_distros"),
  bootstrapBridge: (distro: string, relayMode?: RelayMode) =>
    invoke<DistroBridgeStatus>("bootstrap_bridge", { distro, relayMode: relayMode ?? null }),
  teardownBridge: (distro: string) =>
    invoke<void>("teardown_bridge", { distro }),
  startBridgeRelay: (distro: string) =>
    invoke<void>("start_bridge_relay", { distro }),
  stopBridgeRelay: (distro: string) =>
    invoke<void>("stop_bridge_relay", { distro }),
  restartBridgeRelay: (distro: string) =>
    invoke<void>("restart_bridge_relay", { distro }),
  getDistroBridgeStatus: (distro: string) =>
    invoke<DistroBridgeStatus>("get_distro_bridge_status", { distro }),
  setBridgeEnabled: (distro: string, enabled: boolean) =>
    invoke<void>("set_bridge_enabled", { distro, enabled }),
  listBridgeProviders: () =>
    invoke<ProviderStatus[]>("list_bridge_providers"),
  setDistroProvider: (distro: string, provider: BridgeProvider) =>
    invoke<void>("set_distro_provider", { distro, provider }),
  getRecommendedProvider: () =>
    invoke<BridgeProvider | null>("get_recommended_provider"),
  setAgentForwarding: (distro: string, enabled: boolean) =>
    invoke<void>("set_agent_forwarding", { distro, enabled }),

  // Bridge Phase 4
  runBridgeDiagnostics: (distro: string) =>
    invoke<DiagnosticsResult>("run_bridge_diagnostics", { distro }),
  getRelayLogs: (distro: string, lines: number) =>
    invoke<string>("get_relay_logs", { distro, lines }),
  getRelayBinaryVersions: () =>
    invoke<BinaryVersion>("get_relay_binary_versions"),
  downloadRelayBinary: (binary: string) =>
    invoke<void>("download_relay_binary", { binary }),

  // Bridge Phase 5
  setAutoRestart: (distro: string, enabled: boolean) =>
    invoke<void>("set_auto_restart", { distro, enabled }),
  checkRelayBinaryUpdates: () =>
    invoke<BinaryUpdateStatus[]>("check_relay_binary_updates"),
  setDistroSocketPath: (distro: string, socketPath: string) =>
    invoke<void>("set_distro_socket_path", { distro, socketPath }),

  // Bridge Phase 6
  resetWatchdogRestartCount: (distro: string) =>
    invoke<void>("reset_watchdog_restart_count", { distro }),
  runDiagnosticFix: (distro: string, cmd: string) =>
    invoke<string>("run_diagnostic_fix", { distro, cmd }),
  scanWindowsNamedPipes: () =>
    invoke<NamedPipeEntry[]>("scan_windows_named_pipes"),
  exportBridgeConfig: () =>
    invoke<string>("export_bridge_config"),
  importBridgeConfig: (json: string) =>
    invoke<number>("import_bridge_config", { json }),
  bootstrapAllDistros: () =>
    invoke<BootstrapAllResult[]>("bootstrap_all_distros"),

  // Bridge Phase 7
  refreshRelayScript: (distro: string) =>
    invoke<void>("refresh_relay_script", { distro }),
  getShellInjections: (distro: string) =>
    invoke<ShellInjection[]>("get_shell_injections", { distro }),
  removeShellInjection: (distro: string, rcFile: string) =>
    invoke<void>("remove_shell_injection", { distro, rcFile }),
  testSshViaBridge: (distro: string, host: string, user: string, port: number) =>
    invoke<SshHostTestResult>("test_ssh_via_bridge", { distro, host, user, port }),

  // Phase 8
  getBridgeHistory: (distro: string, limit: number) =>
    invoke<BridgeHistoryEvent[]>("get_bridge_history", { distro, limit }),
  setDistroMaxRestarts: (distro: string, maxRestarts: number) =>
    invoke<void>("set_distro_max_restarts", { distro, maxRestarts }),
  previewWindowsSshHost: (distro: string) =>
    invoke<string>("preview_windows_ssh_host", { distro }),
  upsertWindowsSshHost: (distro: string) =>
    invoke<void>("upsert_windows_ssh_host", { distro }),
  removeWindowsSshHost: (distro: string) =>
    invoke<void>("remove_windows_ssh_host", { distro }),

  // ── Vault ──
  vaultGetState: () => invoke<VaultStateResponse>("vault_get_state"),
  vaultInit: (passphrase: string) => invoke<void>("vault_init", { passphrase }),
  vaultUnlock: (passphrase: string) => invoke<void>("vault_unlock", { passphrase }),
  vaultLock: () => invoke<void>("vault_lock"),
  vaultChangePassphrase: (oldPassphrase: string, newPassphrase: string) =>
    invoke<void>("vault_change_passphrase", { oldPassphrase, newPassphrase }),
  vaultGenerateKey: (request: GenerateKeyRequest) =>
    invoke<SshKeyItem>("vault_generate_key", { request }),
  vaultImportKey: (request: ImportKeyRequest) =>
    invoke<SshKeyItem>("vault_import_key", { request }),
  vaultListKeys: () => invoke<SshKeyItemSummary[]>("vault_list_keys"),
  vaultGetKey: (id: string) => invoke<SshKeyItem>("vault_get_key", { id }),
  vaultUpdateKey: (id: string, request: UpdateKeyRequest) =>
    invoke<SshKeyItem>("vault_update_key", { id, request }),
  vaultDeleteKey: (id: string) => invoke<void>("vault_delete_key", { id }),
  vaultArchiveKey: (id: string) => invoke<void>("vault_archive_key", { id }),
  vaultExportPublicKey: (id: string) => invoke<string>("vault_export_public_key", { id }),
  vaultExportPrivateKey: (id: string) => invoke<string>("vault_export_private_key", { id }),

  // ── Migration ──
  getMigrationPreview: () => invoke<MigrationPreview>("get_migration_preview"),
  migrateProfilesToVault: (profileIds: string[]) =>
    invoke<MigrationReport>("migrate_profiles_to_vault", { profileIds }),
  deleteOriginalKeyFile: (keyPath: string) =>
    invoke<void>("delete_original_key_file", { keyPath }),

  // ── Agent ──
  getAgentPipePath: () => invoke<string>("get_agent_pipe_path"),
  testAgentConnection: () => invoke<string>("test_agent_connection"),

  // ── Consent ──
  respondToConsent: (consentId: string, approved: boolean, selectedKeyId: string, allowMode: string) =>
    invoke<void>("respond_to_consent", { consentId, approved, selectedKeyId, allowMode }),
  getPendingConsent: () =>
    invoke<{ consent_id: string; key_id: string; key_name: string; process_name: string; host: string } | null>("get_pending_consent"),

  // ── Policy Rules ──
  listPolicyRules: () =>
    invoke<{ key_id: string; key_name: string; rule_type: string; created_at: string }[]>("list_policy_rules"),
  deletePolicyRule: (keyId: string) =>
    invoke<void>("delete_policy_rule", { keyId }),
  clearAllPolicyRules: () =>
    invoke<void>("clear_all_policy_rules"),
};
