use chrono::Days;
use tauri::{AppHandle, Emitter, Manager};

use crate::app_state::AppState;
use crate::commands;
use crate::domain::{
    format_local_datetime, local_now_naive, memory_hit, query_limit, reminder_hit, snippet_hit,
    task_hit, ActionDispatchOptions, ActionListHit, ActionOutcome, ActionSource, ActionTodayHit,
    ActionUpdateTaskInput, ClipboardKind, ClipboardQuery, DomainError, MemoryQuery,
    SearchEntityType, SmartListKind, Task, TaskQuery, TaskStatus, UpdateTaskInput,
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
        | WorkbenchAction::CompleteTask { confirmed: false, .. }
        | WorkbenchAction::UpdateTask { confirmed: false, .. }
        | WorkbenchAction::ReopenTask { confirmed: false, .. }
        | WorkbenchAction::ArchiveTask { confirmed: false, .. }
        | WorkbenchAction::DeleteTask { confirmed: false, .. }
        | WorkbenchAction::PostponeTask { confirmed: false, .. }
        | WorkbenchAction::SetTaskDefer { confirmed: false, .. } => Err(DomainError::Validation(
            "该动作需要 confirmed=true 才能执行".into(),
        )),
        WorkbenchAction::CreateTask { .. }
        | WorkbenchAction::CreateReminder { .. }
        | WorkbenchAction::CreateMemory { .. }
        | WorkbenchAction::CompleteTask { .. }
        | WorkbenchAction::UpdateTask { .. }
        | WorkbenchAction::ReopenTask { .. }
        | WorkbenchAction::ArchiveTask { .. }
        | WorkbenchAction::DeleteTask { .. }
        | WorkbenchAction::PostponeTask { .. }
        | WorkbenchAction::SetTaskDefer { .. }
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
            let today = state.tasks.today_tasks(None)?;
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
        WorkbenchAction::ExportTasksCsv => {
            let state = state.ok_or_else(|| DomainError::Validation("应用未就绪".into()))?;
            let csv = state.csv_tasks.export_csv()?;
            Ok(ActionOutcome::TasksCsvExported { csv })
        }
        WorkbenchAction::QueryTasks { query } => {
            let state = state.ok_or_else(|| DomainError::Validation("应用未就绪".into()))?;
            let page = state.tasks.query_tasks(query)?;
            Ok(ActionOutcome::TasksQueried {
                items: page.items.iter().map(task_hit).collect(),
                total: page.total,
            })
        }
        WorkbenchAction::GetTask { task_id } => {
            let state = state.ok_or_else(|| DomainError::Validation("应用未就绪".into()))?;
            let task = state.tasks.get_task(task_id)?;
            Ok(ActionOutcome::TaskFetched { task })
        }
        WorkbenchAction::QueryLists => {
            let state = state.ok_or_else(|| DomainError::Validation("应用未就绪".into()))?;
            let lists = state.tasks.list_lists()?;
            let hits = lists
                .iter()
                .map(|list| {
                    let open_count = state
                        .tasks
                        .count_list_todo_tasks(list.id)
                        .unwrap_or_default();
                    ActionListHit {
                        id: list.id,
                        name: list.name.clone(),
                        kind: list.kind,
                        open_count: open_count.unsigned_abs(),
                    }
                })
                .collect();
            Ok(ActionOutcome::ListsQueried { lists: hits })
        }
        WorkbenchAction::UpdateTask { input, .. } => {
            let state = state.ok_or_else(|| DomainError::Validation("应用未就绪".into()))?;
            let existing = state.tasks.get_task(input.id)?;
            let merged = merge_update_input(&existing, &input);
            let task = state.tasks.update_task(merged)?;
            index_task(state, &task);
            emit_task_change(app, &task, "updated");
            Ok(ActionOutcome::TaskUpdated { task })
        }
        WorkbenchAction::ReopenTask { task_id, .. } => {
            let state = state.ok_or_else(|| DomainError::Validation("应用未就绪".into()))?;
            let task = state.tasks.uncomplete_task(task_id)?;
            index_task(state, &task);
            emit_task_change(app, &task, "updated");
            Ok(ActionOutcome::TaskReopened { task })
        }
        WorkbenchAction::ArchiveTask { task_id, .. } => {
            let state = state.ok_or_else(|| DomainError::Validation("应用未就绪".into()))?;
            let task = state.tasks.archive_task(task_id)?;
            index_task(state, &task);
            emit_task_change(app, &task, "updated");
            Ok(ActionOutcome::TaskArchived { task })
        }
        WorkbenchAction::DeleteTask {
            task_id, disposition, ..
        } => {
            let state = state.ok_or_else(|| DomainError::Validation("应用未就绪".into()))?;
            state.tasks.delete_task_tree(task_id, disposition)?;
            let _ = state.search.remove(SearchEntityType::Task, task_id);
            let _ = app.emit(
                "domain://changed",
                commands::DomainChangeEvent {
                    entity_type: "task".into(),
                    entity_id: task_id.to_string(),
                    change: "deleted".into(),
                    revision: 0,
                },
            );
            Ok(ActionOutcome::TaskDeleted { task_id })
        }
        WorkbenchAction::PostponeTask { task_id, days, .. } => {
            let state = state.ok_or_else(|| DomainError::Validation("应用未就绪".into()))?;
            let task = state.tasks.postpone_task(task_id, days)?;
            index_task(state, &task);
            emit_task_change(app, &task, "updated");
            Ok(ActionOutcome::TaskPostponed { task })
        }
        WorkbenchAction::SetTaskDefer {
            task_id,
            available_at,
            ..
        } => {
            let state = state.ok_or_else(|| DomainError::Validation("应用未就绪".into()))?;
            let task = state.tasks.set_task_defer(task_id, available_at)?;
            index_task(state, &task);
            emit_task_change(app, &task, "updated");
            Ok(ActionOutcome::TaskDeferred { task })
        }
    }
}

/// Merge a partial CLI update onto the existing task. Application-level
/// `UpdateTaskInput` requires complete fields, so `None` fields fall back to
/// the current values; `clear_due` wins over due fields.
fn merge_update_input(existing: &Task, input: &ActionUpdateTaskInput) -> UpdateTaskInput {
    UpdateTaskInput {
        id: existing.id,
        title: input.title.clone().unwrap_or_else(|| existing.title.clone()),
        notes: input.notes.clone().unwrap_or_else(|| existing.notes.clone()),
        priority: input.priority.unwrap_or(existing.priority),
        list_id: input.list_id.unwrap_or(existing.list_id),
        due_date: if input.clear_due {
            None
        } else {
            input.due_date.clone().or_else(|| existing.due_date.clone())
        },
        due_time: if input.clear_due {
            None
        } else {
            input.due_time.clone().or_else(|| existing.due_time.clone())
        },
        tag_names: input
            .tag_names
            .clone()
            .unwrap_or_else(|| existing.tag_names.clone()),
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::tasks::TaskService;
    use crate::domain::{CreateTaskInput, TaskDeleteDisposition};
    use crate::infrastructure::db::Database;
    use tempfile::tempdir;

    fn open_service() -> TaskService {
        let dir = tempdir().unwrap();
        let path = dir.path().join("wb.db");
        let db = Database::open(&path).unwrap();
        let svc = TaskService::new(db);
        svc.ensure_seed_data().unwrap();
        std::mem::forget(dir);
        svc
    }

    fn seeded_task() -> Task {
        let svc = open_service();
        svc.create_task(CreateTaskInput {
            title: "原始标题".into(),
            notes: Some("原始备注".into()),
            priority: Some(crate::domain::TaskPriority::Medium),
            list_id: None,
            due_date: Some("2026-11-20".into()),
            due_time: Some("09:00".into()),
            tag_names: Some(vec!["work".into()]),
            parent_id: None,
        })
        .unwrap()
    }

    /// AC3: partial update keeps untouched fields (read-modify-write merge).
    #[test]
    fn merge_update_keeps_unspecified_fields() {
        let existing = seeded_task();
        let merged = merge_update_input(
            &existing,
            &ActionUpdateTaskInput {
                id: existing.id,
                title: Some("新标题".into()),
                notes: None,
                priority: None,
                list_id: None,
                due_date: None,
                due_time: None,
                clear_due: false,
                tag_names: None,
            },
        );
        assert_eq!(merged.title, "新标题");
        assert_eq!(merged.notes, "原始备注");
        assert_eq!(merged.priority, existing.priority);
        assert_eq!(merged.list_id, existing.list_id);
        assert_eq!(merged.due_date.as_deref(), Some("2026-11-20"));
        assert_eq!(merged.due_time.as_deref(), Some("09:00"));
        assert_eq!(merged.tag_names, vec!["work".to_string()]);
    }

    #[test]
    fn merge_update_clear_due_wins_and_empty_notes_clears() {
        let existing = seeded_task();
        let merged = merge_update_input(
            &existing,
            &ActionUpdateTaskInput {
                id: existing.id,
                title: None,
                notes: Some(String::new()),
                priority: None,
                list_id: None,
                due_date: Some("2026-12-01".into()),
                due_time: Some("10:00".into()),
                clear_due: true,
                tag_names: Some(Vec::new()),
            },
        );
        assert_eq!(merged.due_date, None);
        assert_eq!(merged.due_time, None);
        assert_eq!(merged.notes, "");
        assert!(merged.tag_names.is_empty());
    }

    /// AC5: reopen is idempotent at the service layer (completed → todo).
    #[test]
    fn reopen_completed_then_reopen_again_is_idempotent() {
        let svc = open_service();
        let task = svc
            .create_task(CreateTaskInput {
                title: "重开测试".into(),
                notes: None,
                priority: None,
                list_id: None,
                due_date: None,
                due_time: None,
                tag_names: None,
                parent_id: None,
            })
            .unwrap();
        let completed = svc.complete_task(task.id).unwrap();
        assert_eq!(completed.status, crate::domain::TaskStatus::Completed);
        let reopened = svc.uncomplete_task(task.id).unwrap();
        assert_eq!(reopened.status, crate::domain::TaskStatus::Todo);
        let again = svc.uncomplete_task(task.id).unwrap();
        assert_eq!(again.status, crate::domain::TaskStatus::Todo);
    }

    /// AC5: postpone only applies to todo tasks.
    #[test]
    fn postpone_rejects_completed_task() {
        let svc = open_service();
        let task = svc
            .create_task(CreateTaskInput {
                title: "延期测试".into(),
                notes: None,
                priority: None,
                list_id: None,
                due_date: Some("2026-11-20".into()),
                due_time: None,
                tag_names: None,
                parent_id: None,
            })
            .unwrap();
        svc.complete_task(task.id).unwrap();
        assert!(svc.postpone_task(task.id, 3).is_err());
    }

    /// AC6 helper: delete_tree honors the explicit disposition.
    #[test]
    fn delete_task_tree_cascades_children() {
        let svc = open_service();
        let parent = svc
            .create_task(CreateTaskInput {
                title: "父任务".into(),
                notes: None,
                priority: None,
                list_id: None,
                due_date: None,
                due_time: None,
                tag_names: None,
                parent_id: None,
            })
            .unwrap();
        let child = svc
            .create_task(CreateTaskInput {
                title: "子任务".into(),
                notes: None,
                priority: None,
                list_id: None,
                due_date: None,
                due_time: None,
                tag_names: None,
                parent_id: Some(parent.id),
            })
            .unwrap();
        let affected = svc
            .delete_task_tree(parent.id, TaskDeleteDisposition::Cascade)
            .unwrap();
        assert!(affected.contains(&parent.id) && affected.contains(&child.id));
        assert!(svc.get_task(child.id).is_err());
    }
}