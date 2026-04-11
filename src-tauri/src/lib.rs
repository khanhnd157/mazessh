mod commands;
mod error;
mod models;
mod services;
mod state;

use services::profile_service;
use state::AppState;
use tauri::{
    menu::{MenuBuilder, MenuItemBuilder},
    tray::TrayIconBuilder,
    Manager, WindowEvent,
};

#[tauri::command]
fn update_tray_tooltip(app: tauri::AppHandle, tooltip: String) {
    if let Some(tray) = app.tray_by_id("main-tray") {
        let _ = tray.set_tooltip(Some(&tooltip));
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Load persisted data
    let profiles = profile_service::load_profiles().unwrap_or_default();
    let active_id = profile_service::load_active_profile_id().unwrap_or(None);

    let app_state = AppState::from_persisted(profiles, active_id);

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

            // Set initial tooltip based on active profile
            let initial_tooltip = {
                let state = app.state::<AppState>();
                let inner = state.inner.lock().unwrap();
                match &inner.active_profile_id {
                    Some(id) => {
                        if let Some(profile) = inner.profiles.iter().find(|p| p.id == *id) {
                            format!("Maze SSH - {}", profile.name)
                        } else {
                            "Maze SSH - No active profile".to_string()
                        }
                    }
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

            Ok(())
        })
        .on_window_event(|window, event| {
            // Minimize to tray on close instead of quitting
            if let WindowEvent::CloseRequested { api, .. } = event {
                let _ = window.hide();
                api.prevent_close();
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::profiles::get_profiles,
            commands::profiles::get_profile,
            commands::profiles::create_profile,
            commands::profiles::update_profile,
            commands::profiles::delete_profile,
            commands::scanner::scan_ssh_keys,
            commands::switch::activate_profile,
            commands::switch::deactivate_profile,
            commands::switch::get_active_profile,
            commands::ssh_config::preview_ssh_config,
            commands::ssh_config::write_ssh_config,
            commands::ssh_config::backup_ssh_config,
            commands::git::get_git_ssh_command,
            commands::git::test_ssh_connection,
            update_tray_tooltip,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Maze SSH");
}
