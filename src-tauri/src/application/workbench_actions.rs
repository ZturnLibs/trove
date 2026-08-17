use chrono::Days;
use tauri::{AppHandle, Emitter, Manager};

use crate::app_state::AppState;
use crate::commands;
use crate::domain::{
    format_local_datetime, local_now_naive, memory_hit, query_limit, reminder_hit, snippet_hit,
    task_hit, ActionDispatchOptions, ActionOutcome, ActionSource, ActionTodayHit, ClipboardKind,
    ClipboardQuery, DomainError, MemoryQuery, SmartListKind, TaskQuery, TaskStatus,
    UrlCreateKind, UrlSchemeAction, WorkbenchAction,
};

pub fn dispatch(
    app: &AppHandle,
    state: Option<&AppState>,
    action: WorkbenchAction,
    options: ActionDispatchOptions,
) -> Result<ActionOutcome, DomainError> {
    if options.dry_run {
        return Ok(ActionOutcome::DryRun {
            description: crate::domain::workbench_action_description(&action),
        });
    }

    match action {
        WorkbenchAction::Navigate { path } => {
            let _ = commands::window_show_main(app.clone());
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.emit("main://navigate", &path);
            }
            Ok(ActionOutcome::Navigated { path })
        }
        WorkbenchAction::OpenSearch { query } => {
            let _ = commands::window_show_quick(app.clone(), Some("search".into()));
            if let Some(window) = app.get_webview_window("quick") {
                let _ = window.emit("quick://set-search-query", &query);
            }
            Ok(ActionOutcome::SearchOpened { query })
        }
        WorkbenchAction::CreatePreview {
            kind,
            title,
            notes,
            due_date,
            fire_at,
        } => dispatch_create_preview(app, kind, title, notes, due_date, fire_at),
        WorkbenchAction::CreateTask { confirmed: false, .. }
        | WorkbenchAction::CreateReminder { confirmed: false, .. }
        | WorkbenchAction::CreateMemory { confirmed: false, .. }
        | WorkbenchAction::CompleteTask { confirmed: false, .. } => Err(DomainError::Validation(
            "该动作需要 confirmed=true 才能执行".into(),
        )),
        WorkbenchAction::CreateTask { .. }
        | WorkbenchAction::CreateReminder { .. }
        | WorkbenchAction::CreateMemory { .. }
        | WorkbenchAction::CompleteTask { .. }
            if matches!(options.source, ActionSource::UrlScheme) =>
        {
            Err(DomainError::Validation(
                "URL Scheme 不支持直接写入，请使用 create 预览流程".into(),
            ))
        }
        WorkbenchAction::CreateTask { input, .. } => {
            let state = state.ok_or_else(|| DomainError::Validation("应用未就绪".into()))?;
            let task = state.tasks.create_task(input)?;
            index_task(state, &task);
            emit_task_change(app, &task, "created");
            Ok(ActionOutcome::TaskCreated { task })
        }
        WorkbenchAction::CreateReminder { input, .. } => {
            let state = state.ok_or_else(|| DomainError::Validation("应用未就绪".into()))?;
            let reminder = state.reminders.create(input)?;
            index_reminder(state, &reminder);
            emit_reminder_change(app, &reminder, "created");
            Ok(ActionOutcome::ReminderCreated { reminder })
        }
        WorkbenchAction::CreateMemory { input, .. } => {
            let state = state.ok_or_else(|| DomainError::Validation("应用未就绪".into()))?;
            let memory = state.memories.create(input)?;
            index_memory(state, &memory);
            emit_memory_change(app, &memory, "created");
            Ok(ActionOutcome::MemoryCreated { memory })
        }
        WorkbenchAction::CompleteTask { task_id, .. } => {
            let state = state.ok_or_else(|| DomainError::Validation("应用未就绪".into()))?;
            let task = state.tasks.complete_task(task_id)?;
            index_task(state, &task);
            emit_task_change(app, &task, "completed");
            Ok(ActionOutcome::TaskCompleted { task })
        }
        WorkbenchAction::QueryToday => {
            let state = state.ok_or_else(|| DomainError::Validation("应用未就绪".into()))?;
            let today = state.tasks.today_tasks()?;
            Ok(ActionOutcome::TodayQueried {
                data: ActionTodayHit {
                    today: today.today,
                    overdue: today.overdue.iter().map(task_hit).collect(),
                    due_today: today.due_today.iter().map(task_hit).collect(),
                    focus: today.focus.iter().map(task_hit).collect(),
                    reminders_today: today
                        .reminders_today
                        .iter()
                        .map(|item| {
                            reminder_hit(item.reminder.id, &item.reminder.title, &item.reminder.next_fire_at)
                        })
                        .collect(),
                },
            })
        }
        WorkbenchAction::QueryOverdue { limit } => {
            let state = state.ok_or_else(|| DomainError::Validation("应用未就绪".into()))?;
            query_smart_list(state, SmartListKind::Overdue, limit)
        }
        WorkbenchAction::QueryInbox { limit } => {
            let state = state.ok_or_else(|| DomainError::Validation("应用未就绪".into()))?;
            let page = state.tasks.query_tasks(TaskQuery {
                inbox_only: Some(true),
                status: Some(TaskStatus::Todo),
                limit: Some(query_limit(limit)),
                ..Default::default()
            })?;
            Ok(ActionOutcome::TasksQueried {
                items: page.items.iter().map(task_hit).collect(),
                total: page.total,
            })
        }
        WorkbenchAction::QueryList { list_id, limit } => {
            let state = state.ok_or_else(|| DomainError::Validation("应用未就绪".into()))?;
            let page = state.tasks.query_tasks(TaskQuery {
                list_id: Some(list_id),
                status: Some(TaskStatus::Todo),
                limit: Some(query_limit(limit)),
                ..Default::default()
            })?;
            Ok(ActionOutcome::TasksQueried {
                items: page.items.iter().map(task_hit).collect(),
                total: page.total,
            })
        }
        WorkbenchAction::SearchMemories { query, limit } => {
            let state = state.ok_or_else(|| DomainError::Validation("应用未就绪".into()))?;
            let page = state.memories.query(MemoryQuery {
                search: Some(query),
                limit: Some(query_limit(limit)),
                ..Default::default()
            })?;
            Ok(ActionOutcome::MemoriesQueried {
                items: page.items.iter().map(memory_hit).collect(),
                total: page.total,
            })
        }
        WorkbenchAction::GetSnippets { query, limit } => {
            let state = state.ok_or_else(|| DomainError::Validation("应用未就绪".into()))?;
            let page = state.clipboard.query(ClipboardQuery {
                favorites_only: Some(true),
                search: query.filter(|q| !q.trim().is_empty()),
                kind: Some(ClipboardKind::Text),
                limit: Some(query_limit(limit)),
                ..Default::default()
            })?;
            Ok(ActionOutcome::SnippetsQueried {
                items: page
                    .items
                    .iter()
                    .map(|item| {
                        snippet_hit(item.id, &item.content, item.source_app.clone(), item.favorite)
                    })
                    .collect(),
                total: page.total,
            })
        }
    }
}

fn query_smart_list(
    state: &AppState,
    kind: SmartListKind,
    limit: Option<i64>,
) -> Result<ActionOutcome, DomainError> {
    let page = state
        .tasks
        .smart_list(kind, Some(query_limit(limit)), Some(0))?;
    Ok(ActionOutcome::TasksQueried {
        items: page.items.iter().map(task_hit).collect(),
        total: page.total,
    })
}

fn dispatch_create_preview(
    app: &AppHandle,
    kind: UrlCreateKind,
    title: String,
    notes: Option<String>,
    due_date: Option<String>,
    fire_at: Option<String>,
) -> Result<ActionOutcome, DomainError> {
    let fire_at = if matches!(kind, UrlCreateKind::Reminder) {
        Some(fire_at.unwrap_or_else(default_reminder_fire_at))
    } else {
        fire_at
    };
    let _ = commands::window_show_main(app.clone());
    if let Some(window) = app.get_webview_window("main") {
        let payload = UrlSchemeAction::CreatePreview {
            kind: kind.clone(),
            title: title.clone(),
            notes: notes.clone(),
            due_date: due_date.clone(),
            fire_at: fire_at.clone(),
        };
        let _ = window.emit("url-scheme://pending-create", payload);
    }
    Ok(ActionOutcome::CreatePreviewPending {
        kind,
        title,
        notes,
        due_date,
        fire_at,
    })
}

fn default_reminder_fire_at() -> String {
    let tomorrow = local_now_naive().date() + Days::new(1);
    let dt = tomorrow.and_hms_opt(9, 0, 0).expect("valid time");
    format_local_datetime(dt)
}

fn emit_task_change(app: &AppHandle, task: &crate::domain::Task, change: &str) {
    let _ = app.emit(
        "domain://changed",
        commands::DomainChangeEvent {
            entity_type: "task".into(),
            entity_id: task.id.to_string(),
            change: change.into(),
            revision: task.revision,
        },
    );
}

fn emit_reminder_change(app: &AppHandle, reminder: &crate::domain::Reminder, change: &str) {
    let _ = app.emit(
        "domain://changed",
        commands::DomainChangeEvent {
            entity_type: "reminder".into(),
            entity_id: reminder.id.to_string(),
            change: change.into(),
            revision: reminder.revision,
        },
    );
}

fn emit_memory_change(app: &AppHandle, memory: &crate::domain::Memory, change: &str) {
    let _ = app.emit(
        "domain://changed",
        commands::DomainChangeEvent {
            entity_type: "memory".into(),
            entity_id: memory.id.to_string(),
            change: change.into(),
            revision: memory.revision,
        },
    );
}

fn index_task(state: &AppState, task: &crate::domain::Task) {
    use crate::domain::SearchEntityType;
    let _ = state
        .search
        .upsert(SearchEntityType::Task, task.id, &task.title, &task.notes);
}

fn index_reminder(state: &AppState, reminder: &crate::domain::Reminder) {
    use crate::domain::SearchEntityType;
    let _ = state.search.upsert(
        SearchEntityType::Reminder,
        reminder.id,
        &reminder.title,
        &reminder.notes,
    );
}

fn index_memory(state: &AppState, memory: &crate::domain::Memory) {
    use crate::domain::SearchEntityType;
    let _ = state.search.upsert(
        SearchEntityType::Memory,
        memory.id,
        &memory.title,
        &memory.body,
    );
}