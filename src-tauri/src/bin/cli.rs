use clap::{Parser, Subcommand};
use colored::Colorize;

use maze_ssh_lib::services::{
    config_engine, git_identity_service, key_scanner, profile_service, repo_detection_service,
    repo_mapping_service, ssh_engine,
};

#[derive(Parser)]
#[command(
    name = "maze-ssh",
    about = "SSH Identity Orchestrator for Git Workflows",
    version
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// List all SSH profiles
    List,

    /// Activate a profile by name
    Use {
        /// Profile name (or --auto to detect from current directory)
        name: Option<String>,
        /// Auto-detect profile from current directory's repo mapping
        #[arg(long)]
        auto: bool,
    },

    /// Show the currently active profile
    Current,

    /// Deactivate the current profile
    Off,

    /// Show agent status, active key, and git identity
    Status,

    /// Test SSH connection for a profile
    Test {
        /// Profile name (defaults to active profile)
        name: Option<String>,
    },

    /// SSH config management
    Config {
        #[command(subcommand)]
        action: ConfigAction,
    },

    /// Export all profiles as JSON to stdout
    Export,

    /// Import profiles from a JSON file
    Import {
        /// Path to JSON file
        file: String,
    },
}

#[derive(Subcommand)]
enum ConfigAction {
    /// Preview the generated SSH config
    Preview,
    /// Write SSH config to ~/.ssh/config
    Write,
    /// List config backups
    Backups,
}

fn main() {
    let cli = Cli::parse();

    let result = match cli.command {
        Commands::List => cmd_list(),
        Commands::Use { name, auto } => {
            if auto {
                cmd_use_auto()
            } else if let Some(name) = name {
                cmd_use(&name)
            } else {
                eprintln!("{}", "Error: provide a profile name or --auto".red());
                std::process::exit(1);
            }
        }
        Commands::Current => cmd_current(),
        Commands::Off => cmd_off(),
        Commands::Status => cmd_status(),
        Commands::Test { name } => cmd_test(name),
        Commands::Config { action } => match action {
            ConfigAction::Preview => cmd_config_preview(),
            ConfigAction::Write => cmd_config_write(),
            ConfigAction::Backups => cmd_config_backups(),
        },
        Commands::Export => cmd_export(),
        Commands::Import { file } => cmd_import(&file),
    };

    if let Err(e) = result {
        eprintln!("{} {}", "Error:".red().bold(), e);
        std::process::exit(1);
    }
}

fn cmd_list() -> Result<(), String> {
    let profiles = profile_service::load_profiles().map_err(|e| e.to_string())?;
    let active_id = profile_service::load_active_profile_id().map_err(|e| e.to_string())?;

    if profiles.is_empty() {
        println!("{}", "No profiles configured.".dimmed());
        println!("Create profiles in the Maze SSH desktop app.");
        return Ok(());
    }

    println!("{}", "SSH Profiles".bold());
    println!("{}", "─".repeat(60).dimmed());

    for p in &profiles {
        let active = active_id.as_ref() == Some(&p.id);
        let marker = if active {
            "●".green().to_string()
        } else {
            "○".dimmed().to_string()
        };
        let name = if active {
            p.name.green().bold().to_string()
        } else {
            p.name.normal().to_string()
        };
        let provider = format!("{}", p.provider).dimmed();
        let email = p.email.dimmed();

        println!("  {} {} {} {}", marker, name, provider, email);
        println!(
            "    {} {} {}",
            "key:".dimmed(),
            p.private_key_path.to_string_lossy().dimmed(),
            if active { "[ACTIVE]".green().bold().to_string() } else { String::new() }
        );
    }

    Ok(())
}

fn cmd_use(name: &str) -> Result<(), String> {
    let profiles = profile_service::load_profiles().map_err(|e| e.to_string())?;
    let profile = profiles
        .iter()
        .find(|p| p.name.to_lowercase() == name.to_lowercase())
        .ok_or_else(|| format!("Profile '{}' not found", name))?;

    println!("{} Activating {}...", "→".blue(), profile.name.bold());

    // Save active ID
    profile_service::save_active_profile_id(Some(&profile.id)).map_err(|e| e.to_string())?;

    // Write env file
    ssh_engine::write_env_file(profile).map_err(|e| e.to_string())?;

    // Set user env var
    let _ = ssh_engine::set_user_env_git_ssh_command(profile);

    // SSH agent
    match ssh_engine::ensure_agent_running() {
        Ok(true) => {
            match ssh_engine::agent_switch_key(&profile.private_key_path.to_string_lossy()) {
                Ok(_) => println!("  {} Key loaded into ssh-agent", "✓".green()),
                Err(e) => println!("  {} ssh-add failed: {}", "✗".red(), e),
            }
        }
        Ok(false) => println!("  {} Could not start ssh-agent", "!".yellow()),
        Err(e) => println!("  {} Agent error: {}", "!".yellow(), e),
    }

    // Git identity
    match git_identity_service::set_git_identity_global(&profile.git_username, &profile.email) {
        Ok(()) => println!(
            "  {} Git identity: {} <{}>",
            "✓".green(),
            profile.git_username,
            profile.email
        ),
        Err(e) => println!("  {} Git identity failed: {}", "✗".red(), e),
    }

    println!(
        "\n{} {} is now active.",
        "✓".green().bold(),
        profile.name.green().bold()
    );

    Ok(())
}

fn cmd_use_auto() -> Result<(), String> {
    let cwd = std::env::current_dir().map_err(|e| e.to_string())?;

    let git_root = repo_detection_service::find_git_root(&cwd)
        .ok_or_else(|| "Not inside a git repository".to_string())?;

    let mappings = repo_mapping_service::load_mappings().map_err(|e| e.to_string())?;
    let mapping = repo_detection_service::lookup_mapping(&git_root, &mappings)
        .ok_or_else(|| {
            format!(
                "No mapping found for {}. Create one in the Maze SSH app.",
                git_root.display()
            )
        })?;

    let profiles = profile_service::load_profiles().map_err(|e| e.to_string())?;
    let profile = profiles
        .iter()
        .find(|p| p.id == mapping.profile_id)
        .ok_or_else(|| "Mapped profile not found".to_string())?;

    println!(
        "{} Detected repo: {} → {}",
        "→".blue(),
        mapping.repo_name.bold(),
        profile.name.green().bold()
    );

    cmd_use(&profile.name)
}

fn cmd_current() -> Result<(), String> {
    let active_id = profile_service::load_active_profile_id().map_err(|e| e.to_string())?;

    match active_id {
        None => {
            println!("{}", "No active profile.".dimmed());
        }
        Some(id) => {
            let profiles = profile_service::load_profiles().map_err(|e| e.to_string())?;
            match profiles.iter().find(|p| p.id == id) {
                Some(p) => {
                    println!("{} {}", "Active:".green().bold(), p.name.bold());
                    println!("  Provider: {}", p.provider);
                    println!("  Email:    {}", p.email);
                    println!("  Key:      {}", p.private_key_path.to_string_lossy());
                    println!("  Alias:    {}", p.host_alias);
                }
                None => println!("{}", "Active profile not found in profiles.".red()),
            }
        }
    }

    Ok(())
}

fn cmd_off() -> Result<(), String> {
    profile_service::save_active_profile_id(None).map_err(|e| e.to_string())?;
    ssh_engine::clear_env_file().map_err(|e| e.to_string())?;
    let _ = ssh_engine::clear_user_env_git_ssh_command();
    let _ = ssh_engine::agent_clear_keys();

    println!("{} Profile deactivated. Agent keys cleared.", "✓".green());
    Ok(())
}

fn cmd_status() -> Result<(), String> {
    println!("{}", "Maze SSH Status".bold());
    println!("{}", "─".repeat(50).dimmed());

    // Active profile
    let active_id = profile_service::load_active_profile_id().map_err(|e| e.to_string())?;
    let profiles = profile_service::load_profiles().map_err(|e| e.to_string())?;

    match &active_id {
        Some(id) => match profiles.iter().find(|p| p.id == *id) {
            Some(p) => println!(
                "  {} {} ({})",
                "Profile:".bold(),
                p.name.green(),
                p.provider
            ),
            None => println!("  {} {}", "Profile:".bold(), "unknown".red()),
        },
        None => println!("  {} {}", "Profile:".bold(), "none".dimmed()),
    }

    // Agent keys
    match ssh_engine::agent_list_keys() {
        Ok(keys) => {
            if keys.contains("no identities") || keys.is_empty() {
                println!("  {} {}", "Agent:".bold(), "no keys loaded".dimmed());
            } else {
                for line in keys.lines() {
                    println!("  {} {}", "Agent:".bold(), line);
                }
            }
        }
        Err(e) => println!("  {} {}", "Agent:".bold(), e.red()),
    }

    // Git identity
    match git_identity_service::get_git_identity_global() {
        Ok(info) => println!(
            "  {} {} <{}>",
            "Git:".bold(),
            info.user_name,
            info.user_email
        ),
        Err(_) => println!("  {} {}", "Git:".bold(), "not configured".dimmed()),
    }

    // Profiles count
    println!("  {} {}", "Profiles:".bold(), profiles.len());

    // Repo mappings
    let mappings = repo_mapping_service::load_mappings().unwrap_or_default();
    println!("  {} {}", "Mappings:".bold(), mappings.len());

    Ok(())
}

fn cmd_test(name: Option<String>) -> Result<(), String> {
    let profiles = profile_service::load_profiles().map_err(|e| e.to_string())?;

    let profile = if let Some(name) = name {
        profiles
            .iter()
            .find(|p| p.name.to_lowercase() == name.to_lowercase())
            .ok_or_else(|| format!("Profile '{}' not found", name))?
    } else {
        let active_id = profile_service::load_active_profile_id().map_err(|e| e.to_string())?;
        let id = active_id.ok_or("No active profile. Specify a name or activate one.")?;
        profiles
            .iter()
            .find(|p| p.id == id)
            .ok_or("Active profile not found")?
    };

    println!(
        "{} Testing connection for {}...",
        "→".blue(),
        profile.name.bold()
    );

    let key_path = profile.private_key_path.to_string_lossy();
    let hostname = &profile.hostname;

    let output = std::process::Command::new("ssh")
        .args([
            "-T",
            "-i",
            &key_path,
            "-o",
            "IdentitiesOnly=yes",
            "-o",
            "StrictHostKeyChecking=accept-new",
            "-o",
            "ConnectTimeout=10",
            &format!("git@{}", hostname),
        ])
        .output()
        .map_err(|e| format!("Failed to run ssh: {}", e))?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let combined = format!("{}{}", stdout, stderr);

    let success = combined.contains("successfully authenticated")
        || combined.contains("Welcome to GitLab")
        || combined.contains("Welcome to Gitea")
        || combined.contains("Hi ")
        || output.status.success();

    if success {
        println!("  {} {}", "✓".green().bold(), combined.trim());
    } else {
        println!("  {} {}", "✗".red().bold(), combined.trim());
    }

    Ok(())
}

fn cmd_config_preview() -> Result<(), String> {
    let profiles = profile_service::load_profiles().map_err(|e| e.to_string())?;
    let preview = config_engine::preview_config(&profiles);
    println!("{}", preview);
    Ok(())
}

fn cmd_config_write() -> Result<(), String> {
    let profiles = profile_service::load_profiles().map_err(|e| e.to_string())?;

    // Backup first
    match config_engine::backup_config() {
        Ok(path) => println!("  {} Backup: {}", "✓".green(), path.dimmed()),
        Err(_) => println!("  {} No existing config to backup", "–".dimmed()),
    }

    config_engine::write_config(&profiles).map_err(|e| e.to_string())?;
    println!("  {} SSH config written to ~/.ssh/config", "✓".green().bold());

    Ok(())
}

fn cmd_config_backups() -> Result<(), String> {
    let backups = config_engine::list_backups().map_err(|e| e.to_string())?;

    if backups.is_empty() {
        println!("{}", "No backups found.".dimmed());
        return Ok(());
    }

    println!("{}", "SSH Config Backups".bold());
    println!("{}", "─".repeat(60).dimmed());

    for b in &backups {
        println!(
            "  {} ({}, {:.1} KB)",
            b.filename,
            b.created_at.dimmed(),
            b.size as f64 / 1024.0
        );
    }

    Ok(())
}

fn cmd_export() -> Result<(), String> {
    let profiles = profile_service::load_profiles().map_err(|e| e.to_string())?;
    let json = serde_json::to_string_pretty(&profiles).map_err(|e| e.to_string())?;
    println!("{}", json);
    Ok(())
}

fn cmd_import(file: &str) -> Result<(), String> {
    let content = std::fs::read_to_string(file).map_err(|e| format!("Cannot read {}: {}", file, e))?;
    let imported: Vec<maze_ssh_lib::models::profile::SshProfile> =
        serde_json::from_str(&content).map_err(|e| format!("Invalid JSON: {}", e))?;

    let mut profiles = profile_service::load_profiles().map_err(|e| e.to_string())?;
    let mut count = 0u32;

    for mut p in imported {
        if profiles.iter().any(|existing| existing.name == p.name) {
            println!("  {} Skipping '{}' (already exists)", "–".dimmed(), p.name);
            continue;
        }
        p.id = uuid::Uuid::new_v4().to_string();
        let now = chrono::Utc::now().to_rfc3339();
        p.created_at = now.clone();
        p.updated_at = now;
        println!("  {} Imported '{}'", "✓".green(), p.name);
        profiles.push(p);
        count += 1;
    }

    profile_service::save_profiles(&profiles).map_err(|e| e.to_string())?;
    println!("\n{} {} profile(s) imported.", "✓".green().bold(), count);

    Ok(())
}

fn cmd_scan() -> Result<(), String> {
    let keys = key_scanner::scan_ssh_keys().map_err(|e| e.to_string())?;

    if keys.is_empty() {
        println!("{}", "No SSH keys found in ~/.ssh".dimmed());
        return Ok(());
    }

    println!("{}", "Detected SSH Keys".bold());
    println!("{}", "─".repeat(60).dimmed());

    for k in &keys {
        println!("  {} {} {}", k.key_type.cyan(), k.private_key_path, k.comment.dimmed());
    }

    Ok(())
}
