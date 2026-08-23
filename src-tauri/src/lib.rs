#![allow(dead_code)]

mod app_state;
pub mod application;
pub mod cli_protocol;
mod commands;
pub mod domain;
pub mod infrastructure;
mod menu_bar;
mod platform;
mod shortcuts;

use app_state::AppState;
use infrastructure::db::Database;
use infrastructure::logging;
use std::sync::atomic::{AtomicI64, AtomicU64, Ordering};
use std::sync::Mutex;
use tauri::{
    image::Image,
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Emitter, Manager, PhysicalPosition, PhysicalSize, Rect, WindowEvent,
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

fn load_tray_icon_png(path: &std::path::Path) -> tauri::Result<Image<'static>> {
    let bytes = std::fs::read(path)
        .map_err(|e| tauri::Error::AssetNotFound(format!("read {}: {e}", path.display())))?;
    let img = ::image::load_from_memory(&bytes)
        .map_err(|e| tauri::Error::AssetNotFound(format!("decode {}: {e}", path.display())))?;
    let rgba = img.to_rgba8();
    let (width, height) = rgba.dimensions();
    Ok(Image::new_owned(rgba.into_raw(), width, height))
}

fn resolve_tray_icon(app: &tauri::AppHandle) -> tauri::Result<Image<'static>> {
    #[cfg(target_os = "macos")]
    {
        let base = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("icons");
        let hi = base.join("tray-icon@2x.png");
        let path = if hi.exists() {
            hi
        } else {
            base.join("tray-icon.png")
        };
        if path.exists() {
            return load_tray_icon_png(&path);
        }
    }
    app.default_window_icon()
        .cloned()
        .map(|icon| icon.to_owned())
        .ok_or_else(|| tauri::Error::AssetNotFound("tray icon".into()))
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
    let tray_icon = resolve_tray_icon(app)?;
    let mut tray_builder = TrayIconBuilder::with_id("main").icon(tray_icon);
    #[cfg(target_os = "macos")]
    {
        tray_builder = tray_builder.icon_as_template(true);
    }
    let _tray = tray_builder
        .menu(&menu)
        .show_menu_on_left_click(false)
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
            // Cache the tray-icon rect so the global-shortcut path (which has
            // no click event) can still anchor the popover to the tray.
            if let Ok(mut guard) = LAST_TRAY_RECT.lock() {
                let rect = match &event {
                    TrayIconEvent::Click { rect, .. }
                    | TrayIconEvent::DoubleClick { rect, .. }
                    | TrayIconEvent::Enter { rect, .. }
                    | TrayIconEvent::Move { rect, .. }
                    | TrayIconEvent::Leave { rect, .. } => Some(*rect),
                    _ => None,
                };
                *guard = rect;
            }
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                rect,
                ..
            } = event
            {
                let app = tray.app_handle();
                toggle_tray_today_from_tray(&app, &rect);
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

/// Guards against a persisted window state that restores the main window to an
/// oversized or off-screen frame (e.g. a corrupted .window-state.json).
fn clamp_main_window(app: &tauri::AppHandle) {
    use tauri::{LogicalSize, Manager};
    let Some(window) = app.get_webview_window("main") else {
        return;
    };
    let Ok(Some(monitor)) = window.current_monitor() else {
        return;
    };
    let area = monitor.work_area();
    let Ok(size) = window.outer_size() else {
        return;
    };
    if size.width > area.size.width.saturating_mul(2)
        || size.height > area.size.height.saturating_mul(2)
    {
        tracing::warn!(size = ?size, area = ?area, "resetting oversized main window");
        if window.set_size(LogicalSize::new(1080.0, 720.0)).is_err() {
            return;
        }
    }
    let Ok(pos) = window.outer_position() else {
        return;
    };
    let Ok(size) = window.outer_size() else {
        return;
    };
    let center_x = pos.x + (size.width as i32) / 2;
    let center_y = pos.y + (size.height as i32) / 2;
    let on_screen = window
        .available_monitors()
        .map(|mons| {
            mons.iter().any(|m| {
                let r = m.work_area();
                center_x >= r.position.x
                    && center_x <= r.position.x + r.size.width as i32
                    && center_y >= r.position.y
                    && center_y <= r.position.y + r.size.height as i32
            })
        })
        .unwrap_or(false);
    if !on_screen {
        tracing::warn!(pos = ?pos, "centering off-screen main window");
        let _ = window.center();
    }
}

// --- 托盘锚定弹层（quick / tray-today popover） ---

const POPOVER_GAP: f64 = 6.0;
const POPOVER_SHOW_GRACE_MS: i64 = 150;
const POPOVER_BLUR_HIDE_DELAY_MS: u64 = 120;

static QUICK_LAST_SHOWN_MS: AtomicI64 = AtomicI64::new(0);
static QUICK_BLUR_TOKEN: AtomicU64 = AtomicU64::new(0);
static TRAY_TODAY_LAST_SHOWN_MS: AtomicI64 = AtomicI64::new(0);
static TRAY_TODAY_BLUR_TOKEN: AtomicU64 = AtomicU64::new(0);
static LAST_TRAY_RECT: Mutex<Option<Rect>> = Mutex::new(None);

fn now_ms() -> i64 {
    chrono::Local::now().timestamp_millis()
}

fn hide_popover(app: &tauri::AppHandle, label: &str) {
    if let Some(window) = app.get_webview_window(label) {
        let _ = window.hide();
    }
}

fn hide_sibling_popovers(app: &tauri::AppHandle, except: &str) {
    for label in ["quick", "tray-today"] {
        if label != except {
            hide_popover(app, label);
        }
    }
}

fn position_popover_at_tray(
    app: &tauri::AppHandle,
    label: &str,
    rect: &Rect,
) -> tauri::Result<()> {
    let Some(window) = app.get_webview_window(label) else {
        return Ok(());
    };
    let scale = window.scale_factor().unwrap_or(1.0);
    let icon_pos = rect.position.to_physical::<f64>(scale);
    let icon_size = rect.size.to_physical::<f64>(scale);
    let icon_cx = icon_pos.x + icon_size.width / 2.0;
    let icon_bottom = icon_pos.y + icon_size.height;

    let panel = window.outer_size()?;
    let (pw, ph) = (panel.width as f64, panel.height as f64);

    let monitor = window
        .monitor_from_point(icon_cx, icon_pos.y)?
        .or_else(|| window.current_monitor().ok().flatten());
    let (x, y) = match monitor.as_ref().map(|m| m.work_area()) {
        Some(area) => {
            let (ax, ay) = (area.position.x as f64, area.position.y as f64);
            let (aw, ah) = (area.size.width as f64, area.size.height as f64);
            let x = (icon_cx - pw / 2.0).clamp(ax, (ax + aw - pw).max(ax));
            let mut y = if cfg!(target_os = "macos") {
                icon_bottom + POPOVER_GAP
            } else {
                icon_pos.y - ph - POPOVER_GAP
            };
            if y < ay {
                y = icon_bottom + POPOVER_GAP;
            }
            let y = y.clamp(ay, (ay + ah - ph).max(ay));
            (x, y)
        }
        None => (icon_cx - pw / 2.0, icon_bottom + POPOVER_GAP),
    };
    window.set_position(PhysicalPosition::new(x.round() as i32, y.round() as i32))?;
    Ok(())
}

fn default_center_rect(app: &tauri::AppHandle, label: &str) -> Rect {
    let (w, h) = if label == "tray-today" {
        (380.0_f64, 520.0_f64)
    } else {
        (560.0_f64, 420.0_f64)
    };
    if let Some(window) = app.get_webview_window(label) {
        if let Ok(Some(monitor)) = window.current_monitor() {
            let area = monitor.work_area();
            let cx = area.position.x as f64 + area.size.width as f64 / 2.0;
            let top = area.position.y as f64 + area.size.height as f64 / 3.0;
            return Rect {
                position: PhysicalPosition::new(cx - w / 2.0, top).into(),
                size: PhysicalSize::new(w, h).into(),
            };
        }
    }
    Rect::default()
}

fn tray_rect_or_default(app: &tauri::AppHandle, label: &str) -> Rect {
    LAST_TRAY_RECT
        .lock()
        .ok()
        .and_then(|guard| *guard)
        .unwrap_or_else(|| default_center_rect(app, label))
}

pub fn show_quick_anchored(app: &tauri::AppHandle) -> tauri::Result<()> {
    hide_sibling_popovers(app, "quick");
    let rect = tray_rect_or_default(app, "quick");
    position_popover_at_tray(app, "quick", &rect)?;
    QUICK_LAST_SHOWN_MS.store(now_ms(), Ordering::SeqCst);
    Ok(())
}

fn toggle_quick_from_tray(app: &tauri::AppHandle, rect: &Rect) {
    QUICK_BLUR_TOKEN.fetch_add(1, Ordering::SeqCst);
    hide_sibling_popovers(app, "quick");
    let Some(window) = app.get_webview_window("quick") else {
        return;
    };
    if window.is_visible().unwrap_or(false) {
        let _ = window.hide();
        return;
    }
    let _ = position_popover_at_tray(app, "quick", rect);
    let _ = window.emit("quick://set-mode", "capture");
    let _ = window.show();
    let _ = window.set_focus();
    QUICK_LAST_SHOWN_MS.store(now_ms(), Ordering::SeqCst);
}

fn toggle_tray_today_from_tray(app: &tauri::AppHandle, rect: &Rect) {
    TRAY_TODAY_BLUR_TOKEN.fetch_add(1, Ordering::SeqCst);
    hide_sibling_popovers(app, "tray-today");
    let Some(window) = app.get_webview_window("tray-today") else {
        return;
    };
    if window.is_visible().unwrap_or(false) {
        let _ = window.hide();
        return;
    }
    let _ = position_popover_at_tray(app, "tray-today", rect);
    let _ = window.emit("tray-today://refresh", ());
    let _ = window.show();
    let _ = window.set_focus();
    TRAY_TODAY_LAST_SHOWN_MS.store(now_ms(), Ordering::SeqCst);
}

fn setup_popover_blur_hide(
    app: &tauri::AppHandle,
    label: &str,
    last_shown: &'static AtomicI64,
    blur_token: &'static AtomicU64,
) {
    let Some(window) = app.get_webview_window(label) else {
        return;
    };
    let app = app.clone();
    let label = label.to_string();
    window.on_window_event(move |event| {
        if !matches!(event, WindowEvent::Focused(false)) {
            return;
        }
        if now_ms() - last_shown.load(Ordering::SeqCst) < POPOVER_SHOW_GRACE_MS {
            return;
        }
        let token = blur_token.fetch_add(1, Ordering::SeqCst) + 1;
        let app = app.clone();
        let label = label.clone();
        std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(
                POPOVER_BLUR_HIDE_DELAY_MS,
            ));
            if blur_token.load(Ordering::SeqCst) != token {
                return;
            }
            hide_popover(&app, &label);
        });
    });
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    logging::init_logging();

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(
            tauri_plugin_window_state::Builder::default()
                // The quick popover is transient and positioned on every show;
                // exclude it so a stale saved geometry (e.g. from when it had a
                // title bar) doesn't override the configured 560×420.
                .with_denylist(&["quick", "tray-today"])
                .build(),
        )
        .plugin(tauri_plugin_autostart::init(
            MacosLauncher::LaunchAgent,
            None,
        ))
        .plugin(tauri_plugin_single_instance::init(|app, argv, _cwd| {
            let mut saw_action = false;
            for arg in argv {
                if arg.starts_with("trove://") {
                    application::url_scheme::handle_trove_url(app, &arg);
                    saw_action = true;
                } else if arg.starts_with(cli_protocol::TROVE_ACTION_PREFIX) {
                    cli_protocol::handle_cli_dispatch(app, &arg);
                    saw_action = true;
                }
            }
            if !saw_action {
                let _ = commands::window_show_main(app.clone());
            }
        }))
        .plugin(tauri_plugin_deep_link::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .setup(|app| {
            let db_path = resolve_db_path(app.handle())?;
            let backup_dir = resolve_backup_dir(app.handle())?;
            let assets_dir = resolve_assets_dir(app.handle())?;
            let data_dir = app
                .path()
                .app_data_dir()
                .map_err(|e| e.to_string())?;
            std::fs::create_dir_all(&data_dir).map_err(|e| e.to_string())?;
            tracing::info!(path = %db_path.display(), "opening database");
            let db = Database::open_with_backup_dir(db_path, Some(backup_dir.clone()))
                .map_err(|e| e.to_string())?;
            let state = AppState::new(db, backup_dir, assets_dir, data_dir)?;
            let settings = state.settings.get().unwrap_or_default();
            let scheduler_state = state.clone();
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

            let handle = app.handle().clone();
            for arg in std::env::args().skip(1) {
                if cli_protocol::is_cli_action_arg(&arg) {
                    cli_protocol::handle_cli_dispatch(&handle, &arg);
                }
            }

            clamp_main_window(app.handle());
            // The window-state plugin applies the restored geometry after setup,
            // so also re-check once the restore has taken effect.
            {
                let handle = app.handle().clone();
                std::thread::spawn(move || {
                    std::thread::sleep(std::time::Duration::from_millis(600));
                    let target = handle.clone();
                    let _ = handle.run_on_main_thread(move || {
                        clamp_main_window(&target);
                    });
                });
            }

            setup_tray(app.handle())?;
            menu_bar::setup_app_menu(app.handle())?;
            let shortcut_result = shortcuts::apply_shortcuts(app.handle());
            if !shortcut_result.ok {
                tracing::warn!(errors = ?shortcut_result.errors, "some global shortcuts failed to register");
            }
            hide_on_close(app.handle(), "main");
            hide_on_close(app.handle(), "quick");
            hide_on_close(app.handle(), "tray-today");
            setup_popover_blur_hide(app.handle(), "quick", &QUICK_LAST_SHOWN_MS, &QUICK_BLUR_TOKEN);
            setup_popover_blur_hide(
                app.handle(),
                "tray-today",
                &TRAY_TODAY_LAST_SHOWN_MS,
                &TRAY_TODAY_BLUR_TOKEN,
            );
            application::scheduler::start(app.handle().clone(), scheduler_state);
            application::clipboard_poller::start(app.handle().clone(), clipboard);

            #[cfg(desktop)]
            {
                use tauri_plugin_deep_link::DeepLinkExt;

                if let Ok(Some(urls)) = app.deep_link().get_current() {
                    let handle = app.handle().clone();
                    for url in urls {
                        application::url_scheme::handle_trove_url(&handle, url.as_ref());
                    }
                }

                let handle = app.handle().clone();
                app.deep_link().on_open_url(move |event| {
                    for url in event.urls() {
                        application::url_scheme::handle_trove_url(&handle, url.as_ref());
                    }
                });

                #[cfg(any(target_os = "linux", target_os = "windows"))]
                {
                    let _ = app.deep_link().register_all();
                }
            }

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
            commands::task_list_update,
            commands::task_list_todo_count,
            commands::task_list_delete,
            commands::task_list_undo_delete,
            commands::task_create,
            commands::task_create_recurring,
            commands::task_update,
            commands::task_get,
            commands::task_query,
            commands::task_today,
            commands::today_sort_suggestions,
            commands::today_set_smart_sort_enabled,
            commands::task_complete,
            commands::task_uncomplete,
            commands::task_archive,
            commands::task_unarchive,
            commands::task_delete,
            commands::task_skip,
            commands::task_reorder,
            commands::task_list_tags,
            commands::task_counts,
            commands::task_smart_list,
            commands::task_postpone,
            commands::task_set_defer,
            commands::task_set_waiting,
            commands::task_clear_waiting,
            commands::daily_focus_add,
            commands::daily_focus_remove,
            commands::daily_focus_reorder,
            commands::daily_focus_carry,
            commands::focus_start,
            commands::focus_end,
            commands::focus_active,
            commands::focus_list,
            commands::daily_wrap_snapshot,
            commands::daily_wrap_start,
            commands::daily_wrap_complete,
            commands::daily_wrap_completed_for_date,
            commands::weekly_review_snapshot,
            commands::weekly_review_start,
            commands::weekly_review_complete,
            commands::weekly_review_last_completed,
            commands::health_dashboard_snapshot,
            commands::nl_parse_capture,
            commands::template_list,
            commands::template_create,
            commands::template_delete,
            commands::template_preview,
            commands::template_apply,
            commands::saved_view_create,
            commands::saved_view_list,
            commands::saved_view_delete,
            commands::memory_create,
            commands::memory_update,
            commands::memory_get,
            commands::memory_query,
            commands::memory_delete,
            commands::memory_convert_to_task,
            commands::memory_wikilink_pending,
            commands::memory_resolve_wikilinks,
            commands::memory_backlinks,
            commands::memory_related,
            commands::memory_link_mention,
            commands::search_query,
            commands::entity_link_create,
            commands::entity_link_remove,
            commands::entity_link_list,
            commands::entity_link_assets,
            commands::clipboard_query,
            commands::clipboard_list_source_apps,
            commands::clipboard_get,
            commands::clipboard_set_favorite,
            commands::clipboard_copy,
            commands::asset_read_thumb,
            commands::clipboard_delete,
            commands::clipboard_clear_non_favorites,
            commands::clipboard_convert_to_task,
            commands::clipboard_convert_to_memory,
            commands::clipboard_smart_context,
            commands::clipboard_link_to_task,
            commands::clipboard_set_capture_enabled,
            commands::clipboard_set_smart_actions_enabled,
            commands::storage_run_assets_gc,
            commands::capture_region_screenshot,
            commands::file_ref_pick_and_attach,
            commands::file_ref_list_for_entity,
            commands::file_ref_open,
            commands::file_ref_reveal,
            commands::file_ref_relink,
            commands::backup_create,
            commands::backup_list,
            commands::backup_status,
            commands::backup_restore,
            commands::data_export,
            commands::data_import,
            commands::csv_export_tasks,
            commands::csv_preview_tasks,
            commands::csv_import_tasks,
            commands::csv_import_batches,
            commands::csv_undo_import,
            commands::reminder_create,
            commands::reminder_update,
            commands::reminder_delete,
            commands::reminder_list_for_task,
            commands::reminder_list_all,
            commands::reminder_complete,
            commands::reminder_snooze,
            commands::window_show_main,
            commands::window_show_quick,
            commands::window_hide_quick,
            commands::url_scheme_handle,
            commands::workbench_action_dispatch,
            commands::automation_list,
            commands::automation_create,
            commands::automation_update,
            commands::automation_delete,
            commands::automation_set_enabled,
            commands::automation_runs_list,
            commands::automation_dry_run,
            commands::ai_provider_key_status,
            commands::ai_provider_key_set,
            commands::ai_provider_key_clear,
            commands::ai_provider_probe,
            commands::ai_suggestion_list,
            commands::ai_suggestion_decide,
            commands::ai_suggestion_clear,
            commands::ai_extract_request,
            commands::ai_suggestion_apply,
            commands::ai_weekly_summary_request,
            commands::ai_related_request,
            commands::ai_related_confirm,
            commands::ai_related_reject_item,
            commands::ai_daily_suggest_request,
            commands::ai_daily_suggest_skip,
            commands::ai_daily_suggest_accept,
            commands::task_checklist_list,
            commands::task_checklist_add,
            commands::task_checklist_update,
            commands::task_checklist_delete,
            commands::task_checklist_reorder,
            commands::app_quit,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
