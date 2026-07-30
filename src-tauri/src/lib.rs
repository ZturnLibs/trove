#![allow(dead_code)]

mod app_state;
mod application;
mod commands;
mod domain;
mod infrastructure;
mod platform;

use app_state::AppState;
use infrastructure::db::Database;
use infrastructure::logging;
use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Emitter, Manager, WindowEvent,
};
use tauri_plugin_autostart::MacosLauncher;

fn resolve_db_path(app: &tauri::AppHandle) -> Result<std::path::PathBuf, String> {
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("resolve app data dir: {e}"))?;
    std::fs::create_dir_all(&dir).map_err(|e| format!("create app data dir: {e}"))?;
    Ok(dir.join("workbench.db"))
}

fn setup_tray(app: &tauri::AppHandle) -> tauri::Result<()> {
    let show_main = MenuItem::with_id(app, "show_main", "打开主窗口", true, None::<&str>)?;
    let quick_capture = MenuItem::with_id(app, "quick_capture", "快速记录", true, None::<&str>)?;
    let settings = MenuItem::with_id(app, "settings", "设置", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;

    let menu = Menu::with_items(app, &[&show_main, &quick_capture, &settings, &quit])?;

    let _tray = TrayIconBuilder::new()
        .icon(app.default_window_icon().unwrap().clone())
        .menu(&menu)
        .show_menu_on_left_click(cfg!(target_os = "macos"))
        .on_menu_event(|app, event| match event.id.as_ref() {
            "show_main" => {
                let _ = commands::window_show_main(app.clone());
            }
            "quick_capture" => {
                let _ = commands::window_show_quick(app.clone(), Some("capture".into()));
            }
            "settings" => {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.set_focus();
                    let _ = window.emit("main://navigate", "/settings");
                }
            }
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
                #[cfg(not(target_os = "macos"))]
                {
                    let app = tray.app_handle();
                    let _ = commands::window_show_main(app.clone());
                }
                #[cfg(target_os = "macos")]
                {
                    let _ = tray;
                }
            }
        })
        .build(app)?;

    Ok(())
}

fn register_default_shortcuts(app: &tauri::AppHandle) {
    use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};

    #[cfg(target_os = "macos")]
    let shortcuts = [
        (
            "capture",
            Shortcut::new(Some(Modifiers::SUPER | Modifiers::SHIFT), Code::Space),
        ),
        (
            "search",
            Shortcut::new(Some(Modifiers::SUPER | Modifiers::SHIFT), Code::KeyF),
        ),
        (
            "clipboard",
            Shortcut::new(Some(Modifiers::SUPER | Modifiers::SHIFT), Code::KeyV),
        ),
        (
            "main",
            Shortcut::new(Some(Modifiers::SUPER | Modifiers::SHIFT), Code::KeyA),
        ),
    ];

    #[cfg(not(target_os = "macos"))]
    let shortcuts = [
        (
            "capture",
            Shortcut::new(Some(Modifiers::CONTROL | Modifiers::SHIFT), Code::Space),
        ),
        (
            "search",
            Shortcut::new(Some(Modifiers::CONTROL | Modifiers::SHIFT), Code::KeyF),
        ),
        (
            "clipboard",
            Shortcut::new(Some(Modifiers::CONTROL | Modifiers::SHIFT), Code::KeyV),
        ),
        (
            "main",
            Shortcut::new(Some(Modifiers::CONTROL | Modifiers::SHIFT), Code::KeyA),
        ),
    ];

    for (kind, shortcut) in shortcuts {
        let kind = kind.to_string();
        if let Err(err) = app.global_shortcut().on_shortcut(shortcut, move |app, _shortcut, event| {
            if event.state != ShortcutState::Pressed {
                return;
            }
            match kind.as_str() {
                "capture" => {
                    let _ = commands::window_show_quick(app.clone(), Some("capture".into()));
                }
                "search" => {
                    let _ = commands::window_show_quick(app.clone(), Some("search".into()));
                }
                "clipboard" => {
                    let _ = commands::window_show_quick(app.clone(), Some("clip".into()));
                }
                "main" => {
                    let _ = commands::window_show_main(app.clone());
                }
                _ => {}
            }
        }) {
            tracing::warn!(error = %err, "failed to register global shortcut; remapping will be available in settings");
        }
    }
}

fn hide_on_close(app: &tauri::AppHandle, label: &str) {
    if let Some(window) = app.get_webview_window(label) {
        let handle = app.clone();
        let label = label.to_string();
        window.on_window_event(move |event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                if let Some(window) = handle.get_webview_window(&label) {
                    let _ = window.hide();
                }
            }
        });
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    logging::init_logging();

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_window_state::Builder::default().build())
        .plugin(tauri_plugin_autostart::init(
            MacosLauncher::LaunchAgent,
            None,
        ))
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            let _ = commands::window_show_main(app.clone());
        }))
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .setup(|app| {
            let db_path = resolve_db_path(app.handle())?;
            tracing::info!(path = %db_path.display(), "opening database");
            let db = Database::open(db_path).map_err(|e| e.to_string())?;
            let state = AppState::new(db)?;
            let _ = state.settings.get();
            let reminders = state.reminders.clone();
            app.manage(state);

            setup_tray(app.handle())?;
            register_default_shortcuts(app.handle());
            hide_on_close(app.handle(), "main");
            hide_on_close(app.handle(), "quick");
            application::scheduler::start(app.handle().clone(), reminders);

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::app_health,
            commands::settings_get,
            commands::settings_save,
            commands::smoke_note_create,
            commands::smoke_note_list,
            commands::smoke_note_delete,
            commands::task_list_lists,
            commands::task_list_create,
            commands::task_create,
            commands::task_create_recurring,
            commands::task_update,
            commands::task_get,
            commands::task_query,
            commands::task_today,
            commands::task_complete,
            commands::task_uncomplete,
            commands::task_archive,
            commands::task_delete,
            commands::task_skip,
            commands::task_reorder,
            commands::task_list_tags,
            commands::task_counts,
            commands::reminder_create,
            commands::reminder_update,
            commands::reminder_delete,
            commands::reminder_list_for_task,
            commands::reminder_complete,
            commands::reminder_snooze,
            commands::window_show_main,
            commands::window_show_quick,
            commands::window_hide_quick,
            commands::app_quit,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
