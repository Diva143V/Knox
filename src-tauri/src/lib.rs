//! Knox Desktop Tauri application library.

pub mod commands;

use knox::core::config::KnoxConfig;
use knox::storage::db::KnoxDb;
use std::sync::Arc;
use tauri::{
    Manager, WindowEvent,
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
};
use tokio::sync::Mutex;

/// Shared state available to all Tauri command invocations.
pub struct AppState {
    pub db: Arc<Mutex<KnoxDb>>,
    pub config: Arc<Mutex<KnoxConfig>>,
}

/// Initializes the SQLite database or exits cleanly on fatal error.
fn init_db(kg_path: &std::path::Path) -> KnoxDb {
    match KnoxDb::open(kg_path) {
        Ok(db) => db,
        Err(e) => {
            let msg = format!("Failed to initialize Knox SQLite database: {e}");
            eprintln!("{msg}");
            let _ = std::fs::write("knox-desktop-error.log", &msg);
            std::process::exit(1);
        }
    }
}

/// Configures the system tray icon, menu items, and event handlers.
fn setup_tray(app: &mut tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    let show_item = MenuItem::with_id(app, "show", "Show Knox", true, None::<&str>)?;
    let quit_item = MenuItem::with_id(app, "quit", "Quit Knox", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&show_item, &quit_item])?;

    let mut builder = TrayIconBuilder::new()
        .menu(&menu)
        .tooltip("Knox 🛡️ Knowledge Graph");

    // Explicitly embed official 32x32 Knox logo for system tray
    if let Ok(tray_icon) = tauri::image::Image::from_bytes(include_bytes!("../icons/32x32.png")) {
        builder = builder.icon(tray_icon);
    } else if let Some(icon) = app.default_window_icon().cloned() {
        builder = builder.icon(icon);
    }

    // Set high-res 128x128 Knox logo for main window & Windows taskbar
    if let (Some(window), Ok(win_icon)) = (
        app.get_webview_window("main"),
        tauri::image::Image::from_bytes(include_bytes!("../icons/128x128.png")),
    ) {
        let _ = window.set_icon(win_icon);
    }

    let _ = builder
        .on_menu_event(|app, event| match event.id.as_ref() {
            "show" => {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.set_focus();
                }
            }
            "quit" => {
                app.exit(0);
            }
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                let app = tray.app_handle();
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.set_focus();
                }
            }
        })
        .build(app);

    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.set_focus();
    }

    Ok(())
}

/// Prevents window destruction on close request, hiding to system tray instead.
fn handle_window_event(window: &tauri::Window, event: &WindowEvent) {
    if let WindowEvent::CloseRequested { api, .. } = event {
        api.prevent_close();
        let _ = window.hide();
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let config = KnoxConfig::from_env();
    let db = init_db(&config.kg_path);

    let builder = tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
                let _ = window.unminimize();
                let _ = window.set_focus();
            }
        }))
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_shell::init())
        .manage(AppState {
            db: Arc::new(Mutex::new(db)),
            config: Arc::new(Mutex::new(config)),
        })
        .invoke_handler(tauri::generate_handler![
            commands::status::get_status,
            commands::query::query_actions,
            commands::query::get_action,
            commands::graph::get_graph_data,
            commands::ingest::ingest_transcript,
            commands::ingest::ingest_raw_transcript,
            commands::ingest::preview_transcript,
            commands::ingest::ingest_git_history,
            commands::ingest::detect_local_transcripts,
            commands::workspace::init_project,
            commands::workspace::get_projects,
            commands::workspace::install_hook,
            commands::workspace::uninstall_hook,
            commands::workspace::get_knox_md,
            commands::workspace::get_hook_status,
            commands::scan::scan_staged,
            commands::sessions::start_session,
            commands::sessions::end_session,
            commands::sessions::get_sessions,
            commands::config::get_config,
            commands::config::update_config,
            commands::dialog::open_file_dialog,
            commands::dialog::open_folder_dialog,
            commands::mcp::detect_mcp_configs,
            commands::mcp::deep_scan_mcp_configs,
            commands::mcp::inspect_custom_mcp_config,
            commands::mcp::inject_mcp_config,
            commands::mcp::remove_mcp_config,
        ])
        .setup(setup_tray)
        .on_window_event(handle_window_event);

    if let Err(e) = builder.run(tauri::generate_context!()) {
        let msg = format!("Error while running Knox Tauri application: {e}");
        eprintln!("{msg}");
        let _ = std::fs::write("knox-desktop-error.log", &msg);
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_app_state_initialization() {
        let config = KnoxConfig::default();
        let db = KnoxDb::open_memory().unwrap();
        let state = AppState {
            db: Arc::new(Mutex::new(db)),
            config: Arc::new(Mutex::new(config)),
        };
        assert!(state.config.try_lock().unwrap().session_timeout.as_secs() > 0);
    }

    #[test]
    fn test_embedded_icons() {
        let tray_bytes = include_bytes!("../icons/32x32.png");
        let tray_img = tauri::image::Image::from_bytes(tray_bytes).unwrap();
        assert_eq!(tray_img.width(), 32);
        assert_eq!(tray_img.height(), 32);

        let win_bytes = include_bytes!("../icons/128x128.png");
        let win_img = tauri::image::Image::from_bytes(win_bytes).unwrap();
        assert_eq!(win_img.width(), 128);
        assert_eq!(win_img.height(), 128);
    }
}
