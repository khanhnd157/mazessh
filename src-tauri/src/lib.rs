#[cfg(feature = "desktop")]
mod commands;
pub mod error;
pub mod models;
pub mod services;
pub mod state;

#[cfg(feature = "desktop")]
use std::time::Duration;

#[cfg(feature = "desktop")]
use services::{lock_service, profile_service, repo_mapping_service, session_service, settings_service};
#[cfg(feature = "desktop")]
use state::AppState;
#[cfg(feature = "desktop")]
use tauri::{
    menu::{MenuBuilder, MenuItemBuilder},
    tray::TrayIconBuilder,
    Manager, WindowEvent,
};

#[cfg(feature = "desktop")]
#[tauri::command]
fn update_tray_tooltip(app: tauri::AppHandle, tooltip: String) {
    if let Some(tray) = app.tray_by_id("main-tray") {
        let _ = tray.set_tooltip(Some(&tooltip));
    }
}

#[cfg(feature = "desktop")]
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Load persisted data
    let profiles = profile_service::load_profiles().unwrap_or_default();
    let active_id = profile_service::load_active_profile_id().unwrap_or(None);
    let repo_mappings = repo_mapping_service::load_mappings().unwrap_or_default();
    let settings = settings_service::load_settings();
    let pin_is_set = lock_service::is_pin_configured();

    let app_state = AppState::from_persisted(profiles, active_id, repo_mappings, settings, pin_is_set);

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_process::init())
        .manage(app_state)
        .setup(|app| {
            // Build tray menu
            let show = MenuItemBuilder::with_id("show", "Show Maze SSH").build(app)?;
            let quit = MenuItemBuilder::with_id("quit", "Quit").build(app)?;
            let menu = MenuBuilder::new(app).items(&[&show, &quit]).build()?;

            // Set initial tooltip
            let initial_tooltip = {
                let state = app.state::<AppState>();
                let inner = state.inner.lock().unwrap();
                match &inner.active_profile_id {
                    Some(id) => inner
                        .profiles
                        .iter()
                        .find(|p| p.id == *id)
                        .map(|p| format!("Maze SSH - {}", p.name))
                        .unwrap_or_else(|| "Maze SSH - No active profile".to_string()),
                    None => "Maze SSH - No active profile".to_string(),
                }
            };

            let _tray = TrayIconBuilder::with_id("main-tray")
                .icon(app.default_window_icon().unwrap().clone())
                .tooltip(&initial_tooltip)
                .menu(&menu)
                .on_menu_event(|app, event| match event.id().as_ref() {
                    "show" => {
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.unminimize();
                            let _ = window.set_focus();
                        }
                    }
                    "quit" => {
                        app.exit(0);
                    }
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let tauri::tray::TrayIconEvent::Click {
                        button: tauri::tray::MouseButton::Left,
                        button_state: tauri::tray::MouseButtonState::Up,
                        ..
                    } = event
                    {
                        let app = tray.app_handle();
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.unminimize();
                            let _ = window.set_focus();
                        }
                    }
                })
                .build(app)?;

            // Start background security timer (15s interval)
            let timer_handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                loop {
                    tokio::time::sleep(Duration::from_secs(15)).await;
                    session_service::check_inactivity_and_lock(&timer_handle);
                    session_service::check_agent_expiry(&timer_handle);
                }
            });

            Ok(())
        })
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                // Lock on minimize if setting enabled
                let app = window.app_handle();
                let state = app.state::<AppState>();
                let security = state.security.lock().unwrap();
                if security.settings.lock_on_minimize && security.pin_is_set && !security.is_locked {
                    drop(security);
                    let _ = commands::security::do_lock(app);
                } else {
                    drop(security);
                }
                let _ = window.hide();
                api.prevent_close();
            }
        })
        .invoke_handler(tauri::generate_handler![
            // Profiles
            commands::profiles::get_profiles,
            commands::profiles::get_profile,
            commands::profiles::create_profile,
            commands::profiles::update_profile,
            commands::profiles::delete_profile,
            // Scanner
            commands::scanner::scan_ssh_keys,
            // Switch
            commands::switch::activate_profile,
            commands::switch::deactivate_profile,
            commands::switch::get_active_profile,
            // SSH Config
            commands::ssh_config::preview_ssh_config,
            commands::ssh_config::write_ssh_config,
            commands::ssh_config::backup_ssh_config,
            commands::ssh_config::list_config_backups,
            commands::ssh_config::rollback_ssh_config,
            commands::ssh_config::read_current_ssh_config,
            // Git
            commands::git::get_git_ssh_command,
            commands::git::test_ssh_connection,
            // Repo Mappings
            commands::repo_mappings::get_repo_mappings,
            commands::repo_mappings::get_repo_mappings_for_profile,
            commands::repo_mappings::create_repo_mapping,
            commands::repo_mappings::delete_repo_mapping,
            commands::repo_mappings::update_repo_mapping_scope,
            // Git Identity
            commands::git_identity::get_current_git_identity,
            commands::git_identity::get_repo_git_identity,
            commands::git_identity::sync_git_identity,
            // Repo Detection
            commands::repo_detection::resolve_repo_path,
            commands::repo_detection::check_repo_mapping,
            commands::repo_detection::auto_switch_for_repo,
            // Security
            commands::security::setup_pin,
            commands::security::verify_pin,
            commands::security::change_pin,
            commands::security::remove_pin,
            commands::security::lock_app,
            commands::security::get_lock_state,
            commands::security::get_security_settings,
            commands::security::update_security_settings,
            commands::security::get_audit_logs,
            commands::security::get_agent_time_remaining,
            commands::security::touch_activity,
            // Hooks
            commands::hooks::generate_git_hook,
            commands::hooks::remove_git_hook,
            // Advanced
            commands::advanced::export_profiles,
            commands::advanced::import_profiles,
            commands::advanced::get_key_fingerprint,
            // Tray
            update_tray_tooltip,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Maze SSH");
}
