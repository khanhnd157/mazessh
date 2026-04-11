import { invoke } from "@tauri-apps/api/core";
import type {
  ActivationResult,
  ConnectionTestResult,
  CreateProfileInput,
  DetectedKey,
  ProfileSummary,
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
};
