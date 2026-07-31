#![allow(dead_code)]

mod app_state;
mod application;
mod commands;
mod domain;
mod infrastructure;
mod menu_bar;
mod platform;
mod shortcuts;

use app_state::AppState;
use infrastructure::db::Database;
use infrastructure::logging;
use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Emitter, Manager, WindowEvent,
};
use tauri_plugin_autostart::{MacosLauncher, ManagerExt};

fn resolve_db_path(app: &tauri::AppHandle) -> Result<std::path::PathBuf, String> {
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("resolve app data dir: {e}"))?;
    std::fs::create_dir_all(&dir).map_err(|e| format!("create app data dir: {e}"))?;
    Ok(dir.join("workbench.db"))
}

fn resolve_backup_dir(app: &tauri::AppHandle) -> Result<std::path::PathBuf, String> {
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("resolve app data dir: {e}"))?
        .join("backups");
    std::fs::create_dir_all(&dir).map_err(|e| format!("create backup dir: {e}"))?;
    Ok(dir)
}

fn resolve_assets_dir(app: &tauri::AppHandle) -> Result<std::path::PathBuf, String> {
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("resolve app data dir: {e}"))?
        .join("assets");
    std::fs::create_dir_all(&dir).map_err(|e| format!("create assets dir: {e}"))?;
    Ok(dir)
}

fn setup_tray(app: &tauri::AppHandle) -> tauri::Result<()> {
    let capture_enabled = app
        .try_state::<AppState>()
        .and_then(|state| state.settings.get().ok())
        .map(|s| s.clipboard_capture_enabled)
        .unwrap_or(true);
    let pause_label = if capture_enabled {
        "暂停剪切板记录"
    } else {
        "恢复剪切板记录"
    };

    let show_main = MenuItem::with_id(app, "show_main", "打开主窗口", true, None::<&str>)?;
    let quick_capture = MenuItem::with_id(app, "quick_capture", "快速记录", true, None::<&str>)?;
    let clipboard = MenuItem::with_id(app, "clipboard", "剪切板历史…", true, None::<&str>)?;
    let toggle_clipboard =
        MenuItem::with_id(app, "toggle_clipboard", pause_label, true, None::<&str>)?;
    let settings = MenuItem::with_id(app, "settings", "设置", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;

    let menu = Menu::with_items(
        app,
        &[
            &show_main,
            &quick_capture,
            &clipboard,
            &toggle_clipboard,
            &settings,
            &quit,
        ],
    )?;

    let toggle_item = toggle_clipboard.clone();
    let _tray = TrayIconBuilder::new()
        .icon(app.default_window_icon().unwrap().clone())
        .menu(&menu)
        .show_menu_on_left_click(cfg!(target_os = "macos"))
        .on_menu_event(move |app, event| match event.id.as_ref() {
            "show_main" => {
                let _ = commands::window_show_main(app.clone());
            }
            "quick_capture" => {
                let _ = commands::window_show_quick(app.clone(), Some("capture".into()));
            }
            "clipboard" => {
                let _ = commands::window_show_quick(app.clone(), Some("clip".into()));
            }
            "toggle_clipboard" => {
                if let Some(state) = app.try_state::<AppState>() {
                    if let Ok(current) = state.settings.get() {
                        let enabled = !current.clipboard_capture_enabled;
                        let mut next = current;
                        next.clipboard_capture_enabled = enabled;
                        if state.settings.save(&next).is_ok() {
                            let label = if enabled {
                                "暂停剪切板记录"
                            } else {
                                "恢复剪切板记录"
                            };
                            let _ = toggle_item.set_text(label);
                            let _ = app.emit(
                                "domain://changed",
                                serde_json::json!({
                                    "entityType": "settings",
                                    "entityId": "clipboard_capture",
                                    "change": if enabled { "resumed" } else { "paused" },
                                    "revision": 0,
                                }),
                            );
                        }
                    }
                }
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
            let backup_dir = resolve_backup_dir(app.handle())?;
            let assets_dir = resolve_assets_dir(app.handle())?;
            tracing::info!(path = %db_path.display(), "opening database");
            let db = Database::open_with_backup_dir(db_path, Some(backup_dir.clone()))
                .map_err(|e| e.to_string())?;
            let state = AppState::new(db, backup_dir, assets_dir)?;
            let settings = state.settings.get().unwrap_or_default();
            let reminders = state.reminders.clone();
            let clipboard = state.clipboard.clone();
            let backups = state.backups.clone();

            if settings.auto_backup_on_launch {
                match backups.create("launch") {
                    Ok(_) => {
                        let _ = backups.rotate(settings.backup_retention_count as usize);
                    }
                    Err(err) => {
                        tracing::error!(error = %err, "automatic launch backup failed");
                        let _ = app.emit(
                            "backup://failed",
                            serde_json::json!({
                                "message": format!("自动备份失败：{err}"),
                            }),
                        );
                    }
                }
            }

            // Sync autostart with persisted preference.
            let autostart = app.autolaunch();
            let _ = if settings.launch_at_login {
                autostart.enable()
            } else {
                autostart.disable()
            };

            app.manage(state);

            setup_tray(app.handle())?;
            menu_bar::setup_app_menu(app.handle())?;
            let shortcut_result = shortcuts::apply_shortcuts(app.handle());
            if !shortcut_result.ok {
                tracing::warn!(errors = ?shortcut_result.errors, "some global shortcuts failed to register");
            }
            hide_on_close(app.handle(), "main");
            hide_on_close(app.handle(), "quick");
            application::scheduler::start(app.handle().clone(), reminders);
            application::clipboard_poller::start(app.handle().clone(), clipboard);

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::app_health,
            commands::settings_get,
            commands::settings_save,
            commands::settings_reset_shortcuts,
            commands::shortcuts_apply,
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
            commands::task_smart_list,
            commands::task_postpone,
            commands::nl_parse_capture,
            commands::template_list,
            commands::template_create,
            commands::template_delete,
            commands::template_preview,
            commands::template_apply,
            commands::memory_create,
            commands::memory_update,
            commands::memory_get,
            commands::memory_query,
            commands::memory_delete,
            commands::memory_convert_to_task,
            commands::search_query,
            commands::clipboard_query,
            commands::clipboard_get,
            commands::clipboard_set_favorite,
            commands::clipboard_copy,
            commands::asset_read_thumb,
            commands::clipboard_delete,
            commands::clipboard_clear_non_favorites,
            commands::clipboard_convert_to_task,
            commands::clipboard_convert_to_memory,
            commands::clipboard_set_capture_enabled,
            commands::backup_create,
            commands::backup_list,
            commands::backup_status,
            commands::backup_restore,
            commands::data_export,
            commands::data_import,
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
