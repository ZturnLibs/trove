use crate::app_state::AppState;
use crate::application::backup::{BackupInfo, BackupStatus};
use crate::application::data_port::ImportResult;
use crate::application::saved_views::{CreateSavedViewInput, SavedView};
use crate::application::smoke_notes::SmokeNote;
use crate::application::tasks::TaskCounts;
use crate::application::templates::{
    CreateTemplateInput, ItemTemplate, TemplateKind, TemplatePreview,
};
use crate::domain::{
    parse_capture, AppError, ClipboardItem, ClipboardKind, ClipboardQuery,
    ConvertMemoryToTaskResult, CreateMemoryInput, CreateReminderInput, CreateTaskInput, EntityId,
    EntityLink, LinkInput, Memory, MemoryQuery, PagedResult, ParsedCapture, RecurrenceRule, Reminder,
    ReminderOccurrence, SearchEntityType, SearchQuery, SearchResults, SmartListKind, SnoozePreset,
    Tag, Task, TaskList, TaskQuery, TodayTasks, UpdateMemoryInput, UpdateReminderInput,
    UpdateTaskInput,
};
use crate::infrastructure::db::DbHealth;
use crate::infrastructure::settings::{AppSettings, ShortcutSettings};
use crate::platform::{detect_capabilities, PlatformCapabilities};
use serde::Serialize;
use tauri::{image::Image, AppHandle, Emitter, State};
use tauri_plugin_autostart::ManagerExt;
use tauri_plugin_clipboard_manager::ClipboardExt;

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
    pub backup: BackupStatus,
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

fn index_task(state: &AppState, task: &Task) {
    let _ = state
        .search
        .upsert(SearchEntityType::Task, task.id, &task.title, &task.notes);
}

fn index_reminder(state: &AppState, reminder: &Reminder) {
    let _ = state.search.upsert(
        SearchEntityType::Reminder,
        reminder.id,
        &reminder.title,
        &reminder.notes,
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
        backup: state.backups.status(),
    })
}

#[tauri::command]
pub fn settings_get(state: State<'_, AppState>) -> Result<AppSettings, AppError> {
    state.settings.get().map_err(Into::into)
}

#[tauri::command]
pub fn settings_save(
    app: AppHandle,
    state: State<'_, AppState>,
    settings: AppSettings,
) -> Result<AppSettings, AppError> {
    crate::domain::validate_clipboard_settings(&crate::domain::ClipboardCaptureSettings {
        enabled: settings.clipboard_capture_enabled,
        retention_days: settings.clipboard_retention_days,
        max_items: settings.clipboard_max_items,
        excluded_apps: settings.clipboard_excluded_apps.clone(),
    })?;
    let previous = state.settings.get().unwrap_or_default();
    state.settings.save(&settings)?;

    // Sync launch-at-login with OS.
    let autostart = app.autolaunch();
    let result = if settings.launch_at_login {
        autostart.enable()
    } else {
        autostart.disable()
    };
    if let Err(err) = result {
        tracing::warn!(error = %err, "failed to update launch at login");
    }

    if previous.shortcuts.quick_capture != settings.shortcuts.quick_capture
        || previous.shortcuts.search != settings.shortcuts.search
        || previous.shortcuts.clipboard != settings.shortcuts.clipboard
        || previous.shortcuts.focus_main != settings.shortcuts.focus_main
    {
        let apply = crate::shortcuts::apply_shortcuts(&app);
        if !apply.ok {
            return Err(AppError::new(
                "shortcut_register_failed",
                apply.errors.join("；"),
            ));
        }
    }

    Ok(settings)
}

#[tauri::command]
pub fn settings_reset_shortcuts(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<AppSettings, AppError> {
    let mut settings = state.settings.get()?;
    settings.shortcuts = ShortcutSettings::default();
    state.settings.save(&settings)?;
    let apply = crate::shortcuts::apply_shortcuts(&app);
    if !apply.ok {
        return Err(AppError::new(
            "shortcut_register_failed",
            apply.errors.join("；"),
        ));
    }
    Ok(settings)
}

#[tauri::command]
pub fn shortcuts_apply(app: AppHandle) -> Result<crate::shortcuts::ShortcutApplyResult, AppError> {
    Ok(crate::shortcuts::apply_shortcuts(&app))
}

#[tauri::command]
pub fn smoke_note_create(state: State<'_, AppState>, body: String) -> Result<SmokeNote, AppError> {
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
    index_task(&state, &task);
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
    index_task(&state, &task);
    emit_task_change(&app, &task, "updated");
    Ok(task)
}

#[tauri::command]
pub fn task_get(state: State<'_, AppState>, id: EntityId) -> Result<Task, AppError> {
    state.tasks.get_task(id).map_err(Into::into)
}

#[tauri::command]
pub fn task_query(
    state: State<'_, AppState>,
    query: TaskQuery,
) -> Result<PagedResult<Task>, AppError> {
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
    index_task(&state, &task);
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
    index_reminder(&state, &reminder);
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
    index_reminder(&state, &reminder);
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
    let _ = state.search.remove(SearchEntityType::Reminder, id);
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
pub fn reminder_list_all(state: State<'_, AppState>) -> Result<Vec<Reminder>, AppError> {
    state.reminders.list_all().map_err(Into::into)
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
pub fn task_unarchive(
    app: AppHandle,
    state: State<'_, AppState>,
    id: EntityId,
) -> Result<Task, AppError> {
    let task = state.tasks.unarchive_task(id)?;
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
    let _ = state.search.remove(SearchEntityType::Task, id);
    let _ = state.links.purge_for_source("task", id);
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
pub fn task_smart_list(
    state: State<'_, AppState>,
    kind: SmartListKind,
    limit: Option<i64>,
    offset: Option<i64>,
) -> Result<PagedResult<Task>, AppError> {
    state
        .tasks
        .smart_list(kind, limit, offset)
        .map_err(Into::into)
}

#[tauri::command]
pub fn task_postpone(
    app: AppHandle,
    state: State<'_, AppState>,
    id: EntityId,
    days: i64,
) -> Result<Task, AppError> {
    let task = state.tasks.postpone_task(id, days)?;
    index_task(&state, &task);
    emit_task_change(&app, &task, "updated");
    Ok(task)
}

#[tauri::command]
pub fn nl_parse_capture(text: String) -> ParsedCapture {
    let timezone = iana_time_zone::get_timezone().unwrap_or_else(|_| "Asia/Shanghai".to_string());
    parse_capture(&text, &timezone)
}

#[tauri::command]
pub fn template_list(state: State<'_, AppState>) -> Result<Vec<ItemTemplate>, AppError> {
    state.templates.list().map_err(Into::into)
}

#[tauri::command]
pub fn template_create(
    state: State<'_, AppState>,
    input: CreateTemplateInput,
) -> Result<ItemTemplate, AppError> {
    state.templates.create(input).map_err(Into::into)
}

#[tauri::command]
pub fn template_delete(state: State<'_, AppState>, id: EntityId) -> Result<(), AppError> {
    state.templates.delete(id).map_err(Into::into)
}

#[tauri::command]
pub fn template_preview(
    state: State<'_, AppState>,
    id: EntityId,
) -> Result<TemplatePreview, AppError> {
    state.templates.preview(id).map_err(Into::into)
}

#[tauri::command]
pub fn saved_view_create(
    state: State<'_, AppState>,
    input: CreateSavedViewInput,
) -> Result<SavedView, AppError> {
    state.saved_views.create(input).map_err(Into::into)
}

#[tauri::command]
pub fn saved_view_list(state: State<'_, AppState>) -> Result<Vec<SavedView>, AppError> {
    state.saved_views.list().map_err(Into::into)
}

#[tauri::command]
pub fn saved_view_delete(state: State<'_, AppState>, id: EntityId) -> Result<(), AppError> {
    state.saved_views.delete(id).map_err(Into::into)
}

#[tauri::command]
pub fn template_apply(
    app: AppHandle,
    state: State<'_, AppState>,
    id: EntityId,
) -> Result<serde_json::Value, AppError> {
    let preview = state.templates.preview(id)?;
    match preview.kind {
        TemplateKind::Task => {
            let input = state.templates.to_task_input(id)?;
            let task = if let Some(recurrence) = preview.recurrence.clone() {
                state.tasks.create_recurring_task(input, recurrence)?
            } else {
                state.tasks.create_task(input)?
            };
            index_task(&state, &task);
            emit_task_change(&app, &task, "created");
            Ok(serde_json::json!({ "kind": "task", "id": task.id.to_string() }))
        }
        TemplateKind::Reminder => {
            let input = state.templates.to_reminder_input(id)?;
            let reminder = state.reminders.create(input)?;
            index_reminder(&state, &reminder);
            let _ = app.emit(
                "domain://changed",
                DomainChangeEvent {
                    entity_type: "reminder".into(),
                    entity_id: reminder.id.to_string(),
                    change: "created".into(),
                    revision: reminder.revision,
                },
            );
            Ok(serde_json::json!({ "kind": "reminder", "id": reminder.id.to_string() }))
        }
        TemplateKind::Memory => {
            let input = state.templates.to_memory_input(id)?;
            let memory = state.memories.create(input)?;
            let _ = app.emit(
                "domain://changed",
                DomainChangeEvent {
                    entity_type: "memory".into(),
                    entity_id: memory.id.to_string(),
                    change: "created".into(),
                    revision: memory.revision,
                },
            );
            Ok(serde_json::json!({ "kind": "memory", "id": memory.id.to_string() }))
        }
    }
}

#[tauri::command]
pub fn memory_create(
    app: AppHandle,
    state: State<'_, AppState>,
    input: CreateMemoryInput,
) -> Result<Memory, AppError> {
    let memory = state.memories.create(input)?;
    let _ = app.emit(
        "domain://changed",
        DomainChangeEvent {
            entity_type: "memory".into(),
            entity_id: memory.id.to_string(),
            change: "created".into(),
            revision: memory.revision,
        },
    );
    Ok(memory)
}

#[tauri::command]
pub fn memory_update(
    app: AppHandle,
    state: State<'_, AppState>,
    input: UpdateMemoryInput,
) -> Result<Memory, AppError> {
    let memory = state.memories.update(input)?;
    let _ = app.emit(
        "domain://changed",
        DomainChangeEvent {
            entity_type: "memory".into(),
            entity_id: memory.id.to_string(),
            change: "updated".into(),
            revision: memory.revision,
        },
    );
    Ok(memory)
}

#[tauri::command]
pub fn memory_get(state: State<'_, AppState>, id: EntityId) -> Result<Memory, AppError> {
    state.memories.get(id).map_err(Into::into)
}

#[tauri::command]
pub fn memory_query(
    state: State<'_, AppState>,
    query: MemoryQuery,
) -> Result<PagedResult<Memory>, AppError> {
    state.memories.query(query).map_err(Into::into)
}

#[tauri::command]
pub fn memory_delete(
    app: AppHandle,
    state: State<'_, AppState>,
    id: EntityId,
) -> Result<(), AppError> {
    state.memories.delete(id)?;
    let _ = state.search.remove(SearchEntityType::Memory, id);
    let _ = state.links.purge_for_source("memory", id);
    let _ = app.emit(
        "domain://changed",
        DomainChangeEvent {
            entity_type: "memory".into(),
            entity_id: id.to_string(),
            change: "deleted".into(),
            revision: 0,
        },
    );
    Ok(())
}

#[tauri::command]
pub fn memory_convert_to_task(
    app: AppHandle,
    state: State<'_, AppState>,
    id: EntityId,
) -> Result<ConvertMemoryToTaskResult, AppError> {
    let result = state.memories.convert_to_task(id)?;
    let _ = app.emit(
        "domain://changed",
        DomainChangeEvent {
            entity_type: "memory".into(),
            entity_id: id.to_string(),
            change: "updated".into(),
            revision: result.memory.revision,
        },
    );
    let _ = app.emit(
        "domain://changed",
        DomainChangeEvent {
            entity_type: "task".into(),
            entity_id: result.task_id.to_string(),
            change: "created".into(),
            revision: 0,
        },
    );
    Ok(result)
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LinkedAsset {
    pub link_id: EntityId,
    pub asset_id: EntityId,
    pub content_hash: String,
    pub byte_size: i64,
    pub width: Option<i64>,
    pub height: Option<i64>,
    pub thumb_base64: Option<String>,
    pub created_at: String,
}

#[tauri::command]
pub fn entity_link_create(
    state: State<'_, AppState>,
    input: LinkInput,
) -> Result<EntityLink, AppError> {
    state
        .links
        .link(
            &input.source_type,
            input.source_id,
            &input.target_type,
            input.target_id,
            &input.link_kind,
        )
        .map_err(Into::into)
}

#[tauri::command]
pub fn entity_link_remove(state: State<'_, AppState>, id: EntityId) -> Result<(), AppError> {
    state.links.unlink(id).map_err(Into::into)
}

#[tauri::command]
pub fn entity_link_list(
    state: State<'_, AppState>,
    entity_type: String,
    entity_id: EntityId,
) -> Result<Vec<EntityLink>, AppError> {
    state
        .links
        .list_outgoing(&entity_type, entity_id)
        .map_err(Into::into)
}

#[tauri::command]
pub fn entity_link_assets(
    state: State<'_, AppState>,
    entity_type: String,
    entity_id: EntityId,
) -> Result<Vec<LinkedAsset>, AppError> {
    let links = state.links.list_outgoing(&entity_type, entity_id)?;
    let assets = state.clipboard.assets();
    let mut out = Vec::new();
    for link in links {
        if link.target_type != "asset" {
            continue;
        }
        let Ok(asset) = assets.get(link.target_id) else {
            continue;
        };
        let thumb = assets.thumb_base64(&asset)?;
        out.push(LinkedAsset {
            link_id: link.id,
            asset_id: asset.id,
            content_hash: asset.content_hash,
            byte_size: asset.byte_size,
            width: asset.width,
            height: asset.height,
            thumb_base64: thumb,
            created_at: asset.created_at,
        });
    }
    out.sort_by(|a, b| a.created_at.cmp(&b.created_at));
    Ok(out)
}

#[tauri::command]
pub fn search_query(
    state: State<'_, AppState>,
    query: SearchQuery,
) -> Result<SearchResults, AppError> {
    state.search.query(query).map_err(Into::into)
}

fn emit_clipboard_change(app: &AppHandle, item: &ClipboardItem, change: &str) {
    let _ = app.emit(
        "domain://changed",
        DomainChangeEvent {
            entity_type: "clipboard".into(),
            entity_id: item.id.to_string(),
            change: change.into(),
            revision: item.revision,
        },
    );
}

#[tauri::command]
pub fn clipboard_query(
    state: State<'_, AppState>,
    query: ClipboardQuery,
) -> Result<PagedResult<ClipboardItem>, AppError> {
    state.clipboard.query(query).map_err(Into::into)
}

#[tauri::command]
pub fn clipboard_get(state: State<'_, AppState>, id: EntityId) -> Result<ClipboardItem, AppError> {
    state.clipboard.get(id).map_err(Into::into)
}

#[tauri::command]
pub fn clipboard_set_favorite(
    app: AppHandle,
    state: State<'_, AppState>,
    id: EntityId,
    favorite: bool,
) -> Result<ClipboardItem, AppError> {
    let item = state.clipboard.set_favorite(id, favorite)?;
    emit_clipboard_change(&app, &item, "updated");
    Ok(item)
}

#[tauri::command]
pub fn clipboard_copy(
    app: AppHandle,
    state: State<'_, AppState>,
    id: EntityId,
) -> Result<ClipboardItem, AppError> {
    let item = state.clipboard.get(id)?;
    match item.kind {
        ClipboardKind::Text => {
            state.clipboard.suppress_next_text(&item.content);
            app.clipboard()
                .write_text(item.content.clone())
                .map_err(|e| AppError::new("clipboard_error", e.to_string()))?;
        }
        ClipboardKind::Image => {
            state.clipboard.suppress_next(&item.content_hash);
            let payload = state.clipboard.image_copy_payload(id)?;
            let image = Image::new_owned(payload.rgba, payload.width, payload.height);
            app.clipboard()
                .write_image(&image)
                .map_err(|e| AppError::new("clipboard_error", e.to_string()))?;
        }
    }
    let item = state.clipboard.mark_used(id)?;
    emit_clipboard_change(&app, &item, "updated");
    Ok(item)
}

#[tauri::command]
pub fn asset_read_thumb(
    state: State<'_, AppState>,
    id: EntityId,
) -> Result<Option<String>, AppError> {
    let asset = state.clipboard.assets().get(id)?;
    state
        .clipboard
        .assets()
        .thumb_base64(&asset)
        .map_err(Into::into)
}

#[tauri::command]
pub fn clipboard_delete(
    app: AppHandle,
    state: State<'_, AppState>,
    id: EntityId,
) -> Result<(), AppError> {
    state.clipboard.delete(id)?;
    let _ = app.emit(
        "domain://changed",
        DomainChangeEvent {
            entity_type: "clipboard".into(),
            entity_id: id.to_string(),
            change: "deleted".into(),
            revision: 0,
        },
    );
    Ok(())
}

#[tauri::command]
pub fn clipboard_clear_non_favorites(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<u64, AppError> {
    let count = state.clipboard.clear_non_favorites()?;
    let _ = app.emit(
        "domain://changed",
        DomainChangeEvent {
            entity_type: "clipboard".into(),
            entity_id: "*".into(),
            change: "cleared".into(),
            revision: 0,
        },
    );
    Ok(count)
}

#[tauri::command]
pub fn clipboard_convert_to_task(
    app: AppHandle,
    state: State<'_, AppState>,
    id: EntityId,
) -> Result<EntityId, AppError> {
    let task_id = state.clipboard.convert_to_task(id)?;
    let _ = app.emit(
        "domain://changed",
        DomainChangeEvent {
            entity_type: "task".into(),
            entity_id: task_id.to_string(),
            change: "created".into(),
            revision: 0,
        },
    );
    Ok(task_id)
}

#[tauri::command]
pub fn clipboard_convert_to_memory(
    app: AppHandle,
    state: State<'_, AppState>,
    id: EntityId,
) -> Result<EntityId, AppError> {
    let memory_id = state.clipboard.convert_to_memory(id)?;
    let _ = app.emit(
        "domain://changed",
        DomainChangeEvent {
            entity_type: "memory".into(),
            entity_id: memory_id.to_string(),
            change: "created".into(),
            revision: 0,
        },
    );
    Ok(memory_id)
}

#[tauri::command]
pub fn clipboard_set_capture_enabled(
    app: AppHandle,
    state: State<'_, AppState>,
    enabled: bool,
) -> Result<AppSettings, AppError> {
    let mut settings = state.settings.get()?;
    settings.clipboard_capture_enabled = enabled;
    state.settings.save(&settings)?;
    let _ = app.emit(
        "domain://changed",
        DomainChangeEvent {
            entity_type: "settings".into(),
            entity_id: "clipboard_capture".into(),
            change: if enabled { "resumed" } else { "paused" }.into(),
            revision: 0,
        },
    );
    Ok(settings)
}

#[tauri::command]
pub fn backup_create(app: AppHandle, state: State<'_, AppState>) -> Result<BackupInfo, AppError> {
    let info = state.backups.create("manual")?;
    let settings = state.settings.get()?;
    let _ = state
        .backups
        .rotate(settings.backup_retention_count as usize);
    let _ = app.emit(
        "domain://changed",
        DomainChangeEvent {
            entity_type: "backup".into(),
            entity_id: info.file_name.clone(),
            change: "created".into(),
            revision: 0,
        },
    );
    Ok(info)
}

#[tauri::command]
pub fn backup_list(state: State<'_, AppState>) -> Result<Vec<BackupInfo>, AppError> {
    state.backups.list().map_err(Into::into)
}

#[tauri::command]
pub fn backup_status(state: State<'_, AppState>) -> Result<BackupStatus, AppError> {
    Ok(state.backups.status())
}

#[tauri::command]
pub fn backup_restore(
    app: AppHandle,
    state: State<'_, AppState>,
    file_name: String,
) -> Result<(), AppError> {
    state.backups.restore(&file_name)?;
    let _ = state.tasks.ensure_seed_data();
    if let Err(err) = state.search.rebuild_all() {
        tracing::warn!(error = %err, "search rebuild after restore failed");
    }
    let _ = app.emit(
        "domain://changed",
        DomainChangeEvent {
            entity_type: "backup".into(),
            entity_id: file_name,
            change: "restored".into(),
            revision: 0,
        },
    );
    Ok(())
}

#[tauri::command]
pub fn data_export(state: State<'_, AppState>) -> Result<String, AppError> {
    state.data_port.export_json().map_err(Into::into)
}

#[tauri::command]
pub fn data_import(
    app: AppHandle,
    state: State<'_, AppState>,
    json: String,
) -> Result<ImportResult, AppError> {
    // Safety backup before destructive import.
    let _ = state.backups.create("pre-import");
    let result = state.data_port.import_json(&json)?;
    let _ = state.tasks.ensure_seed_data();
    let _ = app.emit(
        "domain://changed",
        DomainChangeEvent {
            entity_type: "data".into(),
            entity_id: "*".into(),
            change: "imported".into(),
            revision: 0,
        },
    );
    Ok(result)
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
pub fn window_show_quick(app: tauri::AppHandle, mode: Option<String>) -> Result<(), AppError> {
    use tauri::Manager;

    // Anchor the popover to the tray (or a centered fallback) before showing,
    // so menu / global-shortcut / tray-menu entries all share the same position.
    crate::show_quick_anchored(&app).map_err(|e| AppError::new("window_error", e.to_string()))?;

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
