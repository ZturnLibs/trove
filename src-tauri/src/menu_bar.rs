use crate::app_state::AppState;
use crate::commands;
use crate::domain::{CreateMemoryInput, CreateReminderInput, CreateTaskInput, SearchEntityType};
use chrono::Local;
use tauri::{
    menu::{Menu, MenuItem, PredefinedMenuItem, Submenu},
    AppHandle, Emitter, Manager,
};

fn show_and_navigate(app: &AppHandle, path: &str) {
    let _ = commands::window_show_main(app.clone());
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.emit("main://navigate", path);
    }
}

fn hide_focused_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("quick") {
        if window.is_visible().unwrap_or(false) {
            let _ = window.hide();
            return;
        }
    }
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.hide();
    }
}

fn emit_domain(app: &AppHandle, entity_type: &str, entity_id: &str, change: &str, revision: i64) {
    let _ = app.emit(
        "domain://changed",
        serde_json::json!({
            "entityType": entity_type,
            "entityId": entity_id,
            "change": change,
            "revision": revision,
        }),
    );
}

fn menu_new_task(app: &AppHandle) {
    let Some(state) = app.try_state::<AppState>() else {
        return;
    };
    match state.tasks.create_task(CreateTaskInput {
        title: "新任务".into(),
        notes: None,
        priority: None,
        list_id: None,
        due_date: None,
        due_time: None,
        tag_names: None,
    }) {
        Ok(task) => {
            let _ = state
                .search
                .upsert(SearchEntityType::Task, task.id, &task.title, &task.notes);
            emit_domain(app, "task", &task.id.to_string(), "created", task.revision);
            show_and_navigate(app, "/inbox");
        }
        Err(err) => tracing::warn!(error = %err, "menu new task failed"),
    }
}

fn menu_new_reminder(app: &AppHandle) {
    let Some(state) = app.try_state::<AppState>() else {
        return;
    };
    let today = Local::now().date_naive();
    let fire_at = format!("{today}T09:00:00");
    let timezone = iana_time_zone::get_timezone().unwrap_or_else(|_| "Asia/Shanghai".to_string());
    match state.reminders.create(CreateReminderInput {
        title: "新提醒".into(),
        notes: None,
        task_id: None,
        fire_at,
        recurrence: None,
        timezone: Some(timezone),
        end_at: None,
    }) {
        Ok(reminder) => {
            let _ = state.search.upsert(
                SearchEntityType::Reminder,
                reminder.id,
                &reminder.title,
                &reminder.notes,
            );
            emit_domain(
                app,
                "reminder",
                &reminder.id.to_string(),
                "created",
                reminder.revision,
            );
            show_and_navigate(app, "/today");
        }
        Err(err) => tracing::warn!(error = %err, "menu new reminder failed"),
    }
}

fn menu_new_memory(app: &AppHandle) {
    let Some(state) = app.try_state::<AppState>() else {
        return;
    };
    match state.memories.create(CreateMemoryInput {
        title: "新记忆".into(),
        body: Some(String::new()),
        pinned: None,
        quick_insert: None,
        trigger_word: None,
        tag_names: None,
    }) {
        Ok(memory) => {
            let _ = state.search.upsert(
                SearchEntityType::Memory,
                memory.id,
                &memory.title,
                &memory.body,
            );
            emit_domain(
                app,
                "memory",
                &memory.id.to_string(),
                "created",
                memory.revision,
            );
            show_and_navigate(app, "/memory");
        }
        Err(err) => tracing::warn!(error = %err, "menu new memory failed"),
    }
}

pub fn setup_app_menu(app: &AppHandle) -> tauri::Result<()> {
    let new_task = MenuItem::with_id(
        app,
        "menu.file.new_task",
        "新建任务",
        true,
        Some("CmdOrCtrl+N"),
    )?;
    let new_reminder = MenuItem::with_id(
        app,
        "menu.file.new_reminder",
        "新建提醒",
        true,
        Some("CmdOrCtrl+Shift+N"),
    )?;
    let new_memory = MenuItem::with_id(
        app,
        "menu.file.new_memory",
        "新建记忆",
        true,
        Some("Alt+CmdOrCtrl+N"),
    )?;
    let quick_capture = MenuItem::with_id(
        app,
        "menu.file.quick_capture",
        "在快捷窗口中记录…",
        true,
        None::<&str>,
    )?;
    let close_window = MenuItem::with_id(
        app,
        "menu.file.close_window",
        "关闭窗口",
        true,
        Some("CmdOrCtrl+W"),
    )?;

    #[cfg(not(target_os = "macos"))]
    let settings_file = MenuItem::with_id(
        app,
        "menu.file.settings",
        "设置…",
        true,
        Some("CmdOrCtrl+,"),
    )?;
    #[cfg(not(target_os = "macos"))]
    let quit_file = MenuItem::with_id(app, "menu.file.quit", "退出", true, Some("CmdOrCtrl+Q"))?;

    #[cfg(target_os = "macos")]
    let file_menu = Submenu::with_items(
        app,
        "文件",
        true,
        &[
            &new_task,
            &new_reminder,
            &new_memory,
            &quick_capture,
            &PredefinedMenuItem::separator(app)?,
            &close_window,
        ],
    )?;
    #[cfg(not(target_os = "macos"))]
    let file_menu = Submenu::with_items(
        app,
        "文件",
        true,
        &[
            &new_task,
            &new_reminder,
            &new_memory,
            &quick_capture,
            &PredefinedMenuItem::separator(app)?,
            &settings_file,
            &PredefinedMenuItem::separator(app)?,
            &close_window,
            &quit_file,
        ],
    )?;

    let universal_search = MenuItem::with_id(
        app,
        "menu.edit.universal_search",
        "统一搜索…",
        true,
        None::<&str>,
    )?;
    let edit_menu = Submenu::with_items(
        app,
        "编辑",
        true,
        &[
            &PredefinedMenuItem::undo(app, Some("撤销"))?,
            &PredefinedMenuItem::redo(app, Some("重做"))?,
            &PredefinedMenuItem::separator(app)?,
            &PredefinedMenuItem::cut(app, Some("剪切"))?,
            &PredefinedMenuItem::copy(app, Some("复制"))?,
            &PredefinedMenuItem::paste(app, Some("粘贴"))?,
            &PredefinedMenuItem::select_all(app, Some("全选"))?,
            &PredefinedMenuItem::separator(app)?,
            &universal_search,
        ],
    )?;

    let view_today = MenuItem::with_id(app, "menu.view.today", "今日", true, Some("CmdOrCtrl+1"))?;
    let view_inbox =
        MenuItem::with_id(app, "menu.view.inbox", "收件箱", true, Some("CmdOrCtrl+2"))?;
    let view_tasks = MenuItem::with_id(app, "menu.view.tasks", "任务", true, Some("CmdOrCtrl+3"))?;
    let view_memory =
        MenuItem::with_id(app, "menu.view.memory", "记忆", true, Some("CmdOrCtrl+4"))?;
    let view_clipboard = MenuItem::with_id(
        app,
        "menu.view.clipboard",
        "剪切板",
        true,
        Some("CmdOrCtrl+5"),
    )?;
    let show_main =
        MenuItem::with_id(app, "menu.view.show_main", "显示主窗口", true, None::<&str>)?;
    let show_quick = MenuItem::with_id(
        app,
        "menu.view.show_quick",
        "显示快捷窗口",
        true,
        None::<&str>,
    )?;

    let view_menu = Submenu::with_items(
        app,
        "显示",
        true,
        &[
            &view_today,
            &view_inbox,
            &view_tasks,
            &view_memory,
            &view_clipboard,
            &PredefinedMenuItem::separator(app)?,
            &show_main,
            &show_quick,
        ],
    )?;

    let go_settings = MenuItem::with_id(app, "menu.go.settings", "设置", true, None::<&str>)?;
    let go_shortcuts =
        MenuItem::with_id(app, "menu.go.shortcuts", "快捷键设置", true, None::<&str>)?;
    let go_menu = Submenu::with_items(app, "转到", true, &[&go_settings, &go_shortcuts])?;

    let window_main = MenuItem::with_id(app, "menu.window.main", "主窗口", true, None::<&str>)?;
    let window_quick = MenuItem::with_id(app, "menu.window.quick", "快捷窗口", true, None::<&str>)?;
    let window_menu = Submenu::with_items(
        app,
        "窗口",
        true,
        &[
            &PredefinedMenuItem::minimize(app, Some("最小化"))?,
            &window_main,
            &window_quick,
        ],
    )?;

    let help_shortcuts =
        MenuItem::with_id(app, "menu.help.shortcuts", "快捷键一览", true, None::<&str>)?;
    let help_privacy = MenuItem::with_id(
        app,
        "menu.help.privacy",
        "隐私与数据说明",
        true,
        None::<&str>,
    )?;
    let help_about = MenuItem::with_id(app, "menu.help.about", "关于 Trove", true, None::<&str>)?;
    let help_menu = Submenu::with_items(
        app,
        "帮助",
        true,
        &[&help_shortcuts, &help_privacy, &help_about],
    )?;

    #[cfg(target_os = "macos")]
    let menu = {
        use tauri::menu::AboutMetadata;
        let about_meta = AboutMetadata {
            name: Some("Trove".to_string()),
            version: Some(env!("CARGO_PKG_VERSION").to_string()),
            short_version: Some(env!("CARGO_PKG_VERSION").to_string()),
            comments: Some("本地优先的个人工作台：任务、提醒、记忆与剪切板".to_string()),
            copyright: Some("© 2026 Trove".to_string()),
            // macOS 原生 About 面板不会自动读取 bundle 图标；需显式传入当前应用图标。
            icon: app.default_window_icon().cloned(),
            ..Default::default()
        };
        let about = PredefinedMenuItem::about(app, Some("关于 Trove"), Some(about_meta))?;
        let settings =
            MenuItem::with_id(app, "menu.app.settings", "设置…", true, Some("CmdOrCtrl+,"))?;
        let app_menu = Submenu::with_items(
            app,
            "Trove",
            true,
            &[
                &about,
                &PredefinedMenuItem::separator(app)?,
                &settings,
                &PredefinedMenuItem::separator(app)?,
                &PredefinedMenuItem::services(app, None)?,
                &PredefinedMenuItem::separator(app)?,
                &PredefinedMenuItem::hide(app, Some("隐藏 Trove"))?,
                &PredefinedMenuItem::hide_others(app, Some("隐藏其他"))?,
                &PredefinedMenuItem::show_all(app, Some("显示全部"))?,
                &PredefinedMenuItem::separator(app)?,
                &PredefinedMenuItem::quit(app, Some("退出 Trove"))?,
            ],
        )?;
        Menu::with_items(
            app,
            &[
                &app_menu,
                &file_menu,
                &edit_menu,
                &view_menu,
                &go_menu,
                &window_menu,
                &help_menu,
            ],
        )?
    };

    #[cfg(not(target_os = "macos"))]
    let menu = Menu::with_items(
        app,
        &[
            &file_menu,
            &edit_menu,
            &view_menu,
            &go_menu,
            &window_menu,
            &help_menu,
        ],
    )?;

    app.set_menu(menu)?;
    app.on_menu_event(|app, event| {
        handle_menu_event(&app, event.id().as_ref());
    });

    Ok(())
}

fn handle_menu_event(app: &AppHandle, id: &str) {
    match id {
        "menu.file.new_task" => menu_new_task(app),
        "menu.file.new_reminder" => menu_new_reminder(app),
        "menu.file.new_memory" => menu_new_memory(app),
        "menu.file.quick_capture" => {
            let _ = commands::window_show_quick(app.clone(), Some("capture".into()));
        }
        "menu.file.close_window" => hide_focused_window(app),
        "menu.file.settings" | "menu.app.settings" | "menu.go.settings" => {
            show_and_navigate(app, "/settings");
        }
        "menu.file.quit" => app.exit(0),
        "menu.edit.universal_search" => {
            let _ = commands::window_show_quick(app.clone(), Some("search".into()));
        }
        "menu.view.today" => show_and_navigate(app, "/today"),
        "menu.view.inbox" => show_and_navigate(app, "/inbox"),
        "menu.view.tasks" => show_and_navigate(app, "/tasks"),
        "menu.view.memory" => show_and_navigate(app, "/memory"),
        "menu.view.clipboard" => show_and_navigate(app, "/clipboard"),
        "menu.view.show_main" | "menu.window.main" => {
            let _ = commands::window_show_main(app.clone());
        }
        "menu.view.show_quick" | "menu.window.quick" => {
            let _ = commands::window_show_quick(app.clone(), None);
        }
        "menu.go.shortcuts" | "menu.help.shortcuts" | "menu.help.privacy" => {
            show_and_navigate(app, "/settings");
        }
        "menu.help.about" => {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.emit("menu://about", ());
            }
        }
        _ => {}
    }
}
