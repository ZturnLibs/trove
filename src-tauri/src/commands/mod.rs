use crate::app_state::AppState;
use crate::application::smoke_notes::SmokeNote;
use crate::application::tasks::TaskCounts;
use crate::domain::{
    AppError, CreateReminderInput, CreateTaskInput, EntityId, RecurrenceRule, Reminder,
    ReminderOccurrence, SnoozePreset, Tag, Task, TaskList, TaskQuery, TodayTasks,
    UpdateReminderInput, UpdateTaskInput,
};
use crate::infrastructure::db::DbHealth;
use crate::infrastructure::settings::AppSettings;
use crate::platform::{detect_capabilities, PlatformCapabilities};
use serde::Serialize;
use tauri::{AppHandle, Emitter, State};

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct DomainChangeEvent {
    pub entity_type: String,
    pub entity_id: String,
    pub change: String,
    pub revision: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppHealth {
    pub ok: bool,
    pub app_version: String,
    pub database: DbHealth,
    pub capabilities: PlatformCapabilities,
}

fn emit_task_change(app: &AppHandle, task: &Task, change: &str) {
    let _ = app.emit(
        "domain://changed",
        DomainChangeEvent {
            entity_type: "task".into(),
            entity_id: task.id.to_string(),
            change: change.into(),
            revision: task.revision,
        },
    );
}

#[tauri::command]
pub fn app_health(state: State<'_, AppState>) -> Result<AppHealth, AppError> {
    let database = state
        .db
        .health_check()
        .map_err(|e| AppError::retryable("db_unavailable", e.to_string()))?;

    Ok(AppHealth {
        ok: true,
        app_version: env!("CARGO_PKG_VERSION").into(),
        database,
        capabilities: detect_capabilities(),
    })
}

#[tauri::command]
pub fn settings_get(state: State<'_, AppState>) -> Result<AppSettings, AppError> {
    state.settings.get().map_err(Into::into)
}

#[tauri::command]
pub fn settings_save(
    state: State<'_, AppState>,
    settings: AppSettings,
) -> Result<AppSettings, AppError> {
    state.settings.save(&settings)?;
    Ok(settings)
}

#[tauri::command]
pub fn smoke_note_create(
    state: State<'_, AppState>,
    body: String,
) -> Result<SmokeNote, AppError> {
    state.smoke_notes.create(body).map_err(Into::into)
}

#[tauri::command]
pub fn smoke_note_list(state: State<'_, AppState>) -> Result<Vec<SmokeNote>, AppError> {
    state.smoke_notes.list_active().map_err(Into::into)
}

#[tauri::command]
pub fn smoke_note_delete(state: State<'_, AppState>, id: EntityId) -> Result<(), AppError> {
    state.smoke_notes.soft_delete(id).map_err(Into::into)
}

#[tauri::command]
pub fn task_list_lists(state: State<'_, AppState>) -> Result<Vec<TaskList>, AppError> {
    state.tasks.list_lists().map_err(Into::into)
}

#[tauri::command]
pub fn task_list_create(state: State<'_, AppState>, name: String) -> Result<TaskList, AppError> {
    state.tasks.create_list(name).map_err(Into::into)
}

#[tauri::command]
pub fn task_create(
    app: AppHandle,
    state: State<'_, AppState>,
    input: CreateTaskInput,
) -> Result<Task, AppError> {
    let task = state.tasks.create_task(input)?;
    emit_task_change(&app, &task, "created");
    Ok(task)
}

#[tauri::command]
pub fn task_update(
    app: AppHandle,
    state: State<'_, AppState>,
    input: UpdateTaskInput,
) -> Result<Task, AppError> {
    let task = state.tasks.update_task(input)?;
    emit_task_change(&app, &task, "updated");
    Ok(task)
}

#[tauri::command]
pub fn task_get(state: State<'_, AppState>, id: EntityId) -> Result<Task, AppError> {
    state.tasks.get_task(id).map_err(Into::into)
}

#[tauri::command]
pub fn task_query(state: State<'_, AppState>, query: TaskQuery) -> Result<Vec<Task>, AppError> {
    state.tasks.query_tasks(query).map_err(Into::into)
}

#[tauri::command]
pub fn task_today(state: State<'_, AppState>) -> Result<TodayTasks, AppError> {
    let mut today = state.tasks.today_tasks()?;
    today.reminders_today = state.reminders.today_items()?;
    Ok(today)
}

#[tauri::command]
pub fn task_create_recurring(
    app: AppHandle,
    state: State<'_, AppState>,
    input: CreateTaskInput,
    recurrence: RecurrenceRule,
) -> Result<Task, AppError> {
    let task = state.tasks.create_recurring_task(input, recurrence)?;
    emit_task_change(&app, &task, "created");
    Ok(task)
}

#[tauri::command]
pub fn task_skip(
    app: AppHandle,
    state: State<'_, AppState>,
    id: EntityId,
) -> Result<Task, AppError> {
    let task = state.tasks.skip_task_instance(id)?;
    emit_task_change(&app, &task, "updated");
    Ok(task)
}

#[tauri::command]
pub fn reminder_create(
    app: AppHandle,
    state: State<'_, AppState>,
    input: CreateReminderInput,
) -> Result<Reminder, AppError> {
    let reminder = state.reminders.create(input)?;
    let _ = app.emit(
        "domain://changed",
        DomainChangeEvent {
            entity_type: "reminder".into(),
            entity_id: reminder.id.to_string(),
            change: "created".into(),
            revision: reminder.revision,
        },
    );
    Ok(reminder)
}

#[tauri::command]
pub fn reminder_update(
    app: AppHandle,
    state: State<'_, AppState>,
    input: UpdateReminderInput,
) -> Result<Reminder, AppError> {
    let reminder = state.reminders.update(input)?;
    let _ = app.emit(
        "domain://changed",
        DomainChangeEvent {
            entity_type: "reminder".into(),
            entity_id: reminder.id.to_string(),
            change: "updated".into(),
            revision: reminder.revision,
        },
    );
    Ok(reminder)
}

#[tauri::command]
pub fn reminder_delete(
    app: AppHandle,
    state: State<'_, AppState>,
    id: EntityId,
) -> Result<(), AppError> {
    state.reminders.delete(id)?;
    let _ = app.emit(
        "domain://changed",
        DomainChangeEvent {
            entity_type: "reminder".into(),
            entity_id: id.to_string(),
            change: "deleted".into(),
            revision: 0,
        },
    );
    Ok(())
}

#[tauri::command]
pub fn reminder_list_for_task(
    state: State<'_, AppState>,
    task_id: EntityId,
) -> Result<Vec<Reminder>, AppError> {
    state.reminders.list_for_task(task_id).map_err(Into::into)
}

#[tauri::command]
pub fn reminder_complete(
    app: AppHandle,
    state: State<'_, AppState>,
    occurrence_id: EntityId,
) -> Result<ReminderOccurrence, AppError> {
    let occ = state.reminders.complete_occurrence(occurrence_id)?;
    if let Some(task_id) = occ.task_id {
        let _ = state.tasks.complete_task(task_id);
    }
    let _ = app.emit(
        "domain://changed",
        DomainChangeEvent {
            entity_type: "reminder".into(),
            entity_id: occ.id.to_string(),
            change: "updated".into(),
            revision: occ.revision,
        },
    );
    Ok(occ)
}

#[tauri::command]
pub fn reminder_snooze(
    app: AppHandle,
    state: State<'_, AppState>,
    occurrence_id: EntityId,
    preset: SnoozePreset,
) -> Result<ReminderOccurrence, AppError> {
    let occ = state.reminders.snooze_occurrence(occurrence_id, preset)?;
    let _ = app.emit(
        "domain://changed",
        DomainChangeEvent {
            entity_type: "reminder".into(),
            entity_id: occ.id.to_string(),
            change: "updated".into(),
            revision: occ.revision,
        },
    );
    Ok(occ)
}

#[tauri::command]
pub fn task_complete(
    app: AppHandle,
    state: State<'_, AppState>,
    id: EntityId,
) -> Result<Task, AppError> {
    let task = state.tasks.complete_task(id)?;
    emit_task_change(&app, &task, "updated");
    Ok(task)
}

#[tauri::command]
pub fn task_uncomplete(
    app: AppHandle,
    state: State<'_, AppState>,
    id: EntityId,
) -> Result<Task, AppError> {
    let task = state.tasks.uncomplete_task(id)?;
    emit_task_change(&app, &task, "updated");
    Ok(task)
}

#[tauri::command]
pub fn task_archive(
    app: AppHandle,
    state: State<'_, AppState>,
    id: EntityId,
) -> Result<Task, AppError> {
    let task = state.tasks.archive_task(id)?;
    emit_task_change(&app, &task, "updated");
    Ok(task)
}

#[tauri::command]
pub fn task_delete(
    app: AppHandle,
    state: State<'_, AppState>,
    id: EntityId,
) -> Result<(), AppError> {
    state.tasks.delete_task(id)?;
    let _ = app.emit(
        "domain://changed",
        DomainChangeEvent {
            entity_type: "task".into(),
            entity_id: id.to_string(),
            change: "deleted".into(),
            revision: 0,
        },
    );
    Ok(())
}

#[tauri::command]
pub fn task_reorder(
    app: AppHandle,
    state: State<'_, AppState>,
    ordered_ids: Vec<EntityId>,
) -> Result<(), AppError> {
    state.tasks.reorder_tasks(ordered_ids.clone())?;
    if let Some(id) = ordered_ids.first() {
        let _ = app.emit(
            "domain://changed",
            DomainChangeEvent {
                entity_type: "task".into(),
                entity_id: id.to_string(),
                change: "updated".into(),
                revision: 0,
            },
        );
    }
    Ok(())
}

#[tauri::command]
pub fn task_list_tags(state: State<'_, AppState>) -> Result<Vec<Tag>, AppError> {
    state.tasks.list_tags().map_err(Into::into)
}

#[tauri::command]
pub fn task_counts(state: State<'_, AppState>) -> Result<TaskCounts, AppError> {
    state.tasks.counts().map_err(Into::into)
}

#[tauri::command]
pub fn window_show_main(app: tauri::AppHandle) -> Result<(), AppError> {
    use tauri::Manager;
    if let Some(window) = app.get_webview_window("main") {
        window
            .show()
            .map_err(|e| AppError::new("window_error", e.to_string()))?;
        window
            .set_focus()
            .map_err(|e| AppError::new("window_error", e.to_string()))?;
    }
    Ok(())
}

#[tauri::command]
pub fn window_show_quick(
    app: tauri::AppHandle,
    mode: Option<String>,
) -> Result<(), AppError> {
    use tauri::Manager;

    if let Some(window) = app.get_webview_window("quick") {
        let mode = mode.unwrap_or_else(|| "capture".into());
        let _ = window.emit("quick://set-mode", mode);
        window
            .show()
            .map_err(|e| AppError::new("window_error", e.to_string()))?;
        window
            .set_focus()
            .map_err(|e| AppError::new("window_error", e.to_string()))?;
    }
    Ok(())
}

#[tauri::command]
pub fn window_hide_quick(app: tauri::AppHandle) -> Result<(), AppError> {
    use tauri::Manager;
    if let Some(window) = app.get_webview_window("quick") {
        window
            .hide()
            .map_err(|e| AppError::new("window_error", e.to_string()))?;
    }
    Ok(())
}

#[tauri::command]
pub fn app_quit(app: tauri::AppHandle) {
    app.exit(0);
}
