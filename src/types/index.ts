export type Provider = "github" | "gitlab" | "gitea" | "bitbucket" | { custom: string };

export interface SshProfile {
  id: string;
  name: string;
  provider: Provider;
  email: string;
  git_username: string;
  private_key_path: string;
  public_key_path: string;
  host_alias: string;
  hostname: string;
  port: number | null;
  ssh_user: string | null;
  has_passphrase: boolean;
  created_at: string;
  updated_at: string;
  vault_key_id?: string | null;
}

export interface ProfileSummary {
  id: string;
  name: string;
  provider: Provider;
  email: string;
  is_active: boolean;
}

export interface CreateProfileInput {
  name: string;
  provider: Provider;
  email: string;
  git_username: string;
  private_key_path: string;
  host_alias: string;
  hostname: string;
  port: number | null;
  ssh_user: string | null;
  has_passphrase: boolean;
}

export interface UpdateProfileInput {
  name?: string;
  provider?: Provider;
  email?: string;
  git_username?: string;
  host_alias?: string;
  hostname?: string;
  port?: number;
  ssh_user?: string;
}

export interface DetectedKey {
  private_key_path: string;
  public_key_path: string;
  key_type: string;
  comment: string;
}

export interface ActivationResult {
  profile_name: string;
  git_ssh_command: string;
}

export interface AgentStatusEvent {
  status: string;
  success: boolean;
}

export interface ConnectionTestResult {
  success: boolean;
  output: string;
  profile_name: string;
}

// ── M2: Repo Mapping Types ──

export type GitConfigScope = "local" | "global";

export interface RepoMapping {
  id: string;
  repo_path: string;
  repo_name: string;
  profile_id: string;
  git_config_scope: GitConfigScope;
  created_at: string;
  updated_at: string;
}

export interface RepoMappingSummary {
  id: string;
  repo_path: string;
  repo_name: string;
  profile_id: string;
  profile_name: string;
  git_config_scope: GitConfigScope;
}

export interface CreateRepoMappingInput {
  repo_path: string;
  profile_id: string;
  git_config_scope: GitConfigScope;
}

export interface GitIdentityInfo {
  user_name: string;
  user_email: string;
  scope: string;
}

// ── Vault Types ──

export type KeyAlgorithm = "ed25519" | "rsa4096";
export type KeyState = "active" | "archived";
export type AgentMode = "file_system" | "vault";
export type VaultUnlockMode = "same_as_pin" | "separate_passphrase";

export interface VaultStateResponse {
  initialized: boolean;
  unlocked: boolean;
  key_count: number;
}

export interface ExportPolicy {
  allow_private_export: boolean;
}

export interface SshKeyItem {
  id: string;
  name: string;
  algorithm: KeyAlgorithm;
  public_key_openssh: string;
  fingerprint: string;
  state: KeyState;
  export_policy: ExportPolicy;
  comment: string;
  allowed_hosts: string[];
  created_at: string;
  updated_at: string;
}

export interface SshKeyItemSummary {
  id: string;
  name: string;
  algorithm: KeyAlgorithm;
  fingerprint: string;
  state: KeyState;
  allowed_hosts: string[];
  created_at: string;
}

export interface GenerateKeyRequest {
  name: string;
  algorithm: KeyAlgorithm;
  comment?: string | null;
  allow_private_export?: boolean | null;
  allowed_hosts?: string[];
}

export interface ImportKeyRequest {
  name: string;
  private_key_pem: string;
  comment?: string | null;
  source_passphrase?: string | null;
  allow_private_export?: boolean | null;
}

export interface UpdateKeyRequest {
  name?: string | null;
  comment?: string | null;
  allow_private_export?: boolean | null;
}

export interface MigrationPreview {
  eligible: { profile_id: string; profile_name: string; key_path: string; algorithm: string }[];
  skipped: { profile_id: string; profile_name: string; reason: string }[];
}

export interface MigrationReport {
  succeeded: { profile_id: string; profile_name: string; vault_key_id: string }[];
  skipped: { profile_id: string; profile_name: string; reason: string }[];
  failed: { profile_id: string; profile_name: string; error: string }[];
}

// ── M3: Security Types ──

export interface SecuritySettings {
  auto_lock_timeout_minutes: number | null;
  agent_key_timeout_minutes: number | null;
  lock_on_minimize: boolean;
  vault_unlock_mode: VaultUnlockMode;
  agent_mode: AgentMode;
}

export interface LockStateResponse {
  is_locked: boolean;
  pin_is_set: boolean;
}

export interface AuditEntry {
  timestamp: string;
  action: string;
  profile_name: string | null;
  result: string;
  distro?: string | null;
  provider?: string | null;
}

// ── M4: Advanced Types ──

export interface ConfigBackup {
  filename: string;
  path: string;
  size: number;
  created_at: string;
}

export interface KeyFingerprint {
  bits: string;
  hash: string;
  comment: string;
  key_type: string;
}

export interface KeyHealthReport {
  profile_name: string;
  key_type: string;
  bits: number;
  has_public_key: boolean;
  has_passphrase: boolean;
  is_hardware_key: boolean;
  issues: KeyHealthIssue[];
}

export interface KeyHealthIssue {
  severity: "critical" | "warning" | "info";
  message: string;
}

// ── WSL Bridge Types ──

export type BridgeProviderType = "windows-open-ssh" | "one-password" | "pageant" | "custom";

export type RelayMode = "systemd" | "daemon";

export interface BridgeProvider {
  type: BridgeProviderType;
  pipe_path?: string;
}

export interface ProviderStatus {
  provider: BridgeProvider;
  display_name: string;
  available: boolean;
  error: string | null;
}

export interface RelayBinaryStatus {
  binary: "Npiperelay" | "WslSshPageant";
  installed: boolean;
  path: string;
}

export interface WslDistro {
  name: string;
  state: string;
  version: number;
  is_default: boolean;
}

export interface ShellProfile {
  shell: string;    // "bash" | "zsh" | "fish"
  rc_file: string;  // "~/.bashrc" | "~/.zshrc" | "~/.config/fish/config.fish"
  is_installed: boolean;
}

export interface DistroBridgeStatus {
  distro_name: string;
  wsl_version: number;
  distro_running: boolean;
  enabled: boolean;
  provider: BridgeProvider;
  relay_installed: boolean;
  service_active: boolean;
  socket_exists: boolean;
  agent_reachable: boolean;
  allow_agent_forwarding: boolean;
  socat_installed: boolean;
  systemd_available: boolean;
  relay_mode: RelayMode;
  auto_restart: boolean;
  watchdog_restart_count: number;
  relay_script_stale: boolean;
  max_restarts: number;
  detected_shells: ShellProfile[];
  socket_path: string;
  error: string | null;
}

export interface ShellInjection {
  shell: string;
  rc_file: string;
  injected_block: string | null;
  has_forward_block: boolean;
}

export interface SshHostTestResult {
  command: string;
  output: string;
  connected: boolean;
  authenticated: boolean;
  exit_code: number;
}

// ── Phase 8: Health history ring buffer ──

export type BridgeHistoryEventKind =
  | "bridgeStarted"
  | "bridgeStopped"
  | "watchdogRestart"
  | "watchdogPaused"
  | "relayRefreshed"
  | "bridgeBootstrapped"
  | "bridgeTeardown";

export interface BridgeHistoryEvent {
  timestamp: string;
  event: BridgeHistoryEventKind;
  detail: string | null;
}

export interface DiagnosticsStep {
  name: string;
  passed: boolean;
  detail: string | null;
  remediation_cmd: string | null;
}

export interface DiagnosticsResult {
  distro: string;
  steps: DiagnosticsStep[];
  keys_visible: string[];
  suggestions: string[];
}

export interface BinaryVersion {
  npiperelay?: string;
  wsl_ssh_pageant?: string;
}

export interface DownloadProgress {
  binary: string;
  percent: number;
  status: "downloading" | "done" | "error";
}

export interface BinaryUpdateStatus {
  binary: string;                    // "npiperelay" | "wsl_ssh_pageant"
  installed_version: string | null;
  latest_version: string | null;
  update_available: boolean;
}

export interface RelayRestartedEvent {
  distro: string;
}

export interface RelayRestartFailedEvent {
  distro: string;
  count: number;
}

export interface NamedPipeEntry {
  path: string;
  display: string;
}

export interface BootstrapAllResult {
  distro: string;
  success: boolean;
  error: string | null;
}

export interface BridgeOverview {
  wsl_available: boolean;
  npiperelay_installed: boolean;
  windows_agent_running: boolean;
  provider_statuses: ProviderStatus[];
  relay_binaries: RelayBinaryStatus[];
  distros: DistroBridgeStatus[];
}

// ── Helpers ──

export function getProviderLabel(provider: Provider): string {
  if (typeof provider === "string") {
    return { github: "GitHub", gitlab: "GitLab", gitea: "Gitea", bitbucket: "Bitbucket" }[provider] ?? provider;
  }
  return provider.custom;
}

export function getProviderHostname(provider: Provider): string {
  if (typeof provider === "string") {
    return { github: "github.com", gitlab: "gitlab.com", gitea: "", bitbucket: "bitbucket.org" }[provider] ?? "";
  }
  return "";
}
