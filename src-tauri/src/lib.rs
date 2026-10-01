mod commands;
mod db;
mod laya;
mod models;
mod pipeline;

use std::sync::Mutex;

use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_autostart::MacosLauncher;
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};

use pipeline::AppState;

pub const EVENT_CAPTURE_OPEN: &str = "capture:open";

fn setup_tray(app: &AppHandle) -> tauri::Result<()> {
    let open = MenuItem::with_id(app, "open", "Open Dashboard", true, None::<&str>)?;
    let capture = MenuItem::with_id(app, "capture", "Quick Capture", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&open, &capture, &quit])?;

    TrayIconBuilder::with_id("tray")
        .icon(app.default_window_icon().unwrap().clone())
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id().as_ref() {
            "open" => pipeline::show_main_window(app),
            "capture" => pipeline::show_capture_window(app),
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                pipeline::show_main_window(tray.app_handle());
            }
        })
        .build(app)?;
    Ok(())
}

fn setup_shortcut(app: &AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    app.global_shortcut().on_shortcut(
        Shortcut::new(Some(Modifiers::CONTROL), Code::Backquote),
        |app, _shortcut, event| {
            if event.state() == ShortcutState::Pressed {
                eprintln!("hotkey fired");
                let _ = app.emit(EVENT_CAPTURE_OPEN, ());
                pipeline::show_capture_window(app);
            }
        },
    )?;
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_autostart::init(
            MacosLauncher::LaunchAgent,
            Some(vec![]),
        ))
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .setup(|app| {
            // manage state before anything else — the config-defined webview
            // can start invoking while setup is still running
            let db_path = app
                .path()
                .app_data_dir()
                .unwrap_or_default()
                .join("learngraph.db");
            let db = db::Db::open(&db_path).expect("open sqlite");
            app.manage(AppState {
                db: Mutex::new(db),
                db_path,
            });

            setup_tray(app.handle())?;
            if let Err(e) = setup_shortcut(app.handle()) {
                eprintln!("global shortcut registration failed: {e}");
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::capture_event,
            commands::review_event,
            commands::list_domains,
            commands::add_domain,
            commands::update_domain,
            commands::delete_domain,
            commands::add_subdomain,
            commands::delete_subdomain,
            commands::list_events,
            commands::daily_counts,
            commands::reclassify_event,
            commands::delete_event,
            commands::get_settings,
            commands::set_setting,
            commands::daemon_health,
            commands::hide_capture_window,
            commands::diag,
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app_handle, event| {
            if let tauri::RunEvent::Ready = event {
                let db_path = app_handle.state::<AppState>().db_path.clone();
                pipeline::spawn_retry_loop(app_handle.clone(), db_path);
            }
        });
}