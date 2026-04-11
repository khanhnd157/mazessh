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

export interface ConnectionTestResult {
  success: boolean;
  output: string;
  profile_name: string;
}

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
