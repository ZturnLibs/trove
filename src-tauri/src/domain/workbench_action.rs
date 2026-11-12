use serde::{Deserialize, Serialize};

use super::{
    CreateMemoryInput, CreateReminderInput, CreateTaskInput, DomainError, EntityId, ListKind,
    Memory, Reminder, Task, TaskDeleteDisposition, TaskPriority, TaskQuery, TaskStatus,
    UrlCreateKind, UrlSchemeAction,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ActionSource {
    UrlScheme,
    CommandPalette,
    Cli,
    Internal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ActionDispatchOptions {
    pub source: ActionSource,
    /// When true, mutating actions describe intent without persisting.
    pub dry_run: bool,
}

impl ActionDispatchOptions {
    pub fn url_scheme() -> Self {
        Self {
            source: ActionSource::UrlScheme,
            dry_run: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "camelCase")]
pub enum WorkbenchAction {
    Navigate {
        path: String,
    },
    OpenSearch {
        query: String,
    },
    CreatePreview {
        kind: UrlCreateKind,
        title: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        notes: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        due_date: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        fire_at: Option<String>,
    },
    CreateTask {
        input: CreateTaskInput,
        confirmed: bool,
    },
    CreateReminder {
        input: CreateReminderInput,
        confirmed: bool,
    },
    CreateMemory {
        input: CreateMemoryInput,
        confirmed: bool,
    },
    CompleteTask {
        task_id: EntityId,
        confirmed: bool,
    },
    /// Read-only: today's overdue / due / focus / reminders.
    QueryToday,
    QueryOverdue {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        limit: Option<i64>,
    },
    QueryInbox {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        limit: Option<i64>,
    },
    QueryList {
        list_id: EntityId,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        limit: Option<i64>,
    },
    SearchMemories {
        query: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        limit: Option<i64>,
    },
    GetSnippets {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        query: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        limit: Option<i64>,
    },
    /// Read-only: export the task CSV (same columns as the Settings export).
    ExportTasksCsv,
    /// Read-only: generic task query (status / list / search / due range / parent).
    QueryTasks {
        #[serde(default)]
        query: TaskQuery,
    },
    /// Read-only: full task detail (complete notes, tags, parent link).
    GetTask {
        task_id: EntityId,
    },
    /// Read-only: list inventory (id / name / kind / open count) for name→uuid resolution.
    QueryLists,
    /// Partial update; the app merges with the existing task before persisting.
    UpdateTask {
        input: ActionUpdateTaskInput,
        confirmed: bool,
    },
    ReopenTask {
        task_id: EntityId,
        confirmed: bool,
    },
    ArchiveTask {
        task_id: EntityId,
        confirmed: bool,
    },
    DeleteTask {
        task_id: EntityId,
        disposition: TaskDeleteDisposition,
        confirmed: bool,
    },
    PostponeTask {
        task_id: EntityId,
        days: i64,
        confirmed: bool,
    },
    /// Some(date) = set defer date; None = clear it.
    SetTaskDefer {
        task_id: EntityId,
        available_at: Option<String>,
        confirmed: bool,
    },
}

const PREVIEW_LEN: usize = 120;
const SNIPPET_LEN: usize = 4000;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ActionTaskHit {
    pub id: EntityId,
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes_preview: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub due_date: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub due_time: Option<String>,
    pub priority: TaskPriority,
    pub list_name: String,
    pub status: TaskStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ActionReminderHit {
    pub id: EntityId,
    pub title: String,
    pub next_fire_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ActionTodayHit {
    pub today: String,
    pub overdue: Vec<ActionTaskHit>,
    pub due_today: Vec<ActionTaskHit>,
    pub focus: Vec<ActionTaskHit>,
    pub reminders_today: Vec<ActionReminderHit>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ActionMemoryHit {
    pub id: EntityId,
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body_preview: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ActionSnippetHit {
    pub id: EntityId,
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_app: Option<String>,
    pub favorite: bool,
}

pub fn task_hit(task: &Task) -> ActionTaskHit {
    ActionTaskHit {
        id: task.id,
        title: task.title.clone(),
        notes_preview: preview_text(&task.notes, PREVIEW_LEN),
        due_date: task.due_date.clone(),
        due_time: task.due_time.clone(),
        priority: task.priority,
        list_name: task.list_name.clone(),
        status: task.status,
    }
}

pub fn reminder_hit(id: EntityId, title: &str, next_fire_at: &str) -> ActionReminderHit {
    ActionReminderHit {
        id,
        title: title.to_string(),
        next_fire_at: next_fire_at.to_string(),
    }
}

pub fn memory_hit(memory: &Memory) -> ActionMemoryHit {
    ActionMemoryHit {
        id: memory.id,
        title: memory.title.clone(),
        body_preview: preview_text(&memory.body, PREVIEW_LEN),
    }
}

fn preview_text(value: &str, max: usize) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return None;
    }
    let mut iter = trimmed.chars();
    let preview: String = iter.by_ref().take(max).collect();
    if iter.next().is_some() {
        Some(format!("{preview}…"))
    } else {
        Some(preview)
    }
}

pub fn snippet_hit(
    id: EntityId,
    content: &str,
    source_app: Option<String>,
    favorite: bool,
) -> ActionSnippetHit {
    let mut iter = content.chars();
    let clipped: String = iter.by_ref().take(SNIPPET_LEN).collect();
    ActionSnippetHit {
        id,
        content: clipped,
        source_app,
        favorite,
    }
}

/// Partial update semantics: `None` = keep existing value. Clearing optional
/// fields is explicit (`clear_due`), mirroring the three-state convention in
/// `commands::task_list_update`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ActionUpdateTaskInput {
    pub id: EntityId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// Some("") clears the notes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub priority: Option<TaskPriority>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub list_id: Option<EntityId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub due_date: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub due_time: Option<String>,
    #[serde(default)]
    pub clear_due: bool,
    /// None = keep existing tags; Some(vec) = replace wholesale (empty vec clears).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tag_names: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ActionListHit {
    pub id: EntityId,
    pub name: String,
    pub kind: ListKind,
    pub open_count: u64,
}

pub fn query_limit(limit: Option<i64>) -> i64 {
    limit.unwrap_or(20).clamp(1, 100)
}

impl From<UrlSchemeAction> for WorkbenchAction {
    fn from(action: UrlSchemeAction) -> Self {
        match action {
            UrlSchemeAction::Navigate { path } => WorkbenchAction::Navigate { path },
            UrlSchemeAction::Search { query } => WorkbenchAction::OpenSearch { query },
            UrlSchemeAction::CreatePreview {
                kind,
                title,
                notes,
                due_date,
                fire_at,
            } => WorkbenchAction::CreatePreview {
                kind,
                title,
                notes,
                due_date,
                fire_at,
            },
        }
    }
}

impl WorkbenchAction {
    pub fn requires_confirmation(&self) -> bool {
        matches!(
            self,
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
        )
    }

    pub fn is_confirmed(&self) -> bool {
        match self {
            WorkbenchAction::CreateTask { confirmed, .. }
            | WorkbenchAction::CreateReminder { confirmed, .. }
            | WorkbenchAction::CreateMemory { confirmed, .. }
            | WorkbenchAction::CompleteTask { confirmed, .. }
            | WorkbenchAction::UpdateTask { confirmed, .. }
            | WorkbenchAction::ReopenTask { confirmed, .. }
            | WorkbenchAction::ArchiveTask { confirmed, .. }
            | WorkbenchAction::DeleteTask { confirmed, .. }
            | WorkbenchAction::PostponeTask { confirmed, .. }
            | WorkbenchAction::SetTaskDefer { confirmed, .. } => *confirmed,
            _ => true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "outcome", rename_all = "camelCase")]
pub enum ActionOutcome {
    Navigated {
        path: String,
    },
    SearchOpened {
        query: String,
    },
    CreatePreviewPending {
        kind: UrlCreateKind,
        title: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        notes: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        due_date: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        fire_at: Option<String>,
    },
    DryRun {
        description: String,
    },
    TaskCreated {
        task: Task,
    },
    ReminderCreated {
        reminder: Reminder,
    },
    MemoryCreated {
        memory: Memory,
    },
    TaskCompleted {
        task: Task,
    },
    TodayQueried {
        data: ActionTodayHit,
    },
    TasksQueried {
        items: Vec<ActionTaskHit>,
        total: i64,
    },
    MemoriesQueried {
        items: Vec<ActionMemoryHit>,
        total: i64,
    },
    SnippetsQueried {
        items: Vec<ActionSnippetHit>,
        total: i64,
    },
    TasksCsvExported {
        csv: String,
    },
    TaskFetched {
        task: Task,
    },
    TaskUpdated {
        task: Task,
    },
    TaskReopened {
        task: Task,
    },
    TaskArchived {
        task: Task,
    },
    TaskDeleted {
        task_id: EntityId,
    },
    TaskPostponed {
        task: Task,
    },
    TaskDeferred {
        task: Task,
    },
    ListsQueried {
        lists: Vec<ActionListHit>,
    },
    Rejected {
        reason: String,
    },
}

pub fn reject_unconfirmed(action: &WorkbenchAction) -> Result<(), DomainError> {
    if action.requires_confirmation() && !action.is_confirmed() {
        return Err(DomainError::Validation(
            "该动作需要 confirmed=true 才能执行".into(),
        ));
    }
    Ok(())
}

pub fn workbench_action_description(action: &WorkbenchAction) -> String {
    match action {
        WorkbenchAction::Navigate { path } => format!("导航到 {path}"),
        WorkbenchAction::OpenSearch { query } => format!("打开搜索：{query}"),
        WorkbenchAction::CreatePreview { title, kind, .. } => {
            format!("预览创建 {}：{title}", create_kind_label(kind))
        }
        WorkbenchAction::CreateTask { input, .. } => format!("创建任务：{}", input.title),
        WorkbenchAction::CreateReminder { input, .. } => format!("创建提醒：{}", input.title),
        WorkbenchAction::CreateMemory { input, .. } => format!("创建记忆：{}", input.title),
        WorkbenchAction::CompleteTask { task_id, .. } => format!("完成任务 {task_id}"),
        WorkbenchAction::QueryToday => "查询今日事项".into(),
        WorkbenchAction::QueryOverdue { .. } => "查询逾期任务".into(),
        WorkbenchAction::QueryInbox { .. } => "查询收件箱".into(),
        WorkbenchAction::QueryList { list_id, .. } => format!("查询清单 {list_id}"),
        WorkbenchAction::SearchMemories { query, .. } => format!("搜索记忆：{query}"),
        WorkbenchAction::GetSnippets { query, .. } => match query {
            Some(q) if !q.trim().is_empty() => format!("查询文本片段：{q}"),
            _ => "查询收藏文本片段".into(),
        },
        WorkbenchAction::ExportTasksCsv => "导出任务 CSV".into(),
        WorkbenchAction::QueryTasks { query } => {
            let parts: Vec<String> = [
                query.status.map(|s| format!("状态={s:?}")),
                query.search.clone().map(|s| format!("搜索「{s}」")),
                query
                    .due_from
                    .clone()
                    .map(|d| format!("截止≥{d}")),
                query.due_to.clone().map(|d| format!("截止≤{d}")),
            ]
            .into_iter()
            .flatten()
            .collect();
            if parts.is_empty() {
                "查询任务".into()
            } else {
                format!("查询任务（{}）", parts.join("，"))
            }
        }
        WorkbenchAction::GetTask { task_id } => format!("查询任务详情 {task_id}"),
        WorkbenchAction::QueryLists => "查询清单列表".into(),
        WorkbenchAction::UpdateTask { input, .. } => format!("更新任务 {}", input.id),
        WorkbenchAction::ReopenTask { task_id, .. } => format!("重开任务 {task_id}"),
        WorkbenchAction::ArchiveTask { task_id, .. } => format!("归档任务 {task_id}"),
        WorkbenchAction::DeleteTask {
            task_id, disposition, ..
        } => format!("删除任务 {task_id}（{}）", disposition.as_str()),
        WorkbenchAction::PostponeTask { task_id, days, .. } => {
            format!("任务 {task_id} 延后 {days} 天")
        }
        WorkbenchAction::SetTaskDefer {
            task_id,
            available_at,
            ..
        } => match available_at {
            Some(date) => format!("任务 {task_id} 延期至 {date} 显示"),
            None => format!("清除任务 {task_id} 的延期显示"),
        },
    }
}

fn create_kind_label(kind: &UrlCreateKind) -> &'static str {
    match kind {
        UrlCreateKind::Task => "task",
        UrlCreateKind::Reminder => "reminder",
        UrlCreateKind::Memory => "memory",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn url_navigate_converts() {
        let action = WorkbenchAction::from(UrlSchemeAction::Navigate {
            path: "/today".into(),
        });
        assert!(matches!(
            action,
            WorkbenchAction::Navigate { path } if path == "/today"
        ));
    }

    #[test]
    fn mutating_actions_require_confirmation() {
        let action = WorkbenchAction::CreateTask {
            input: CreateTaskInput {
                title: "x".into(),
                notes: None,
                priority: None,
                list_id: None,
                due_date: None,
                due_time: None,
                tag_names: None,
                parent_id: None,
            },
            confirmed: false,
        };
        assert!(reject_unconfirmed(&action).is_err());
    }

    #[test]
    fn query_actions_skip_confirmation() {
        assert!(!WorkbenchAction::QueryToday.requires_confirmation());
        assert!(WorkbenchAction::QueryToday.is_confirmed());
        assert!(reject_unconfirmed(&WorkbenchAction::QueryToday).is_ok());
    }

    #[test]
    fn query_limit_defaults_and_clamps() {
        assert_eq!(query_limit(None), 20);
        assert_eq!(query_limit(Some(0)), 1);
        assert_eq!(query_limit(Some(500)), 100);
    }

    #[test]
    fn new_mutating_actions_require_confirmation() {
        let id = uuid::Uuid::new_v4();
        let cases = vec![
            WorkbenchAction::UpdateTask {
                input: ActionUpdateTaskInput {
                    id,
                    title: Some("t".into()),
                    notes: None,
                    priority: None,
                    list_id: None,
                    due_date: None,
                    due_time: None,
                    clear_due: false,
                    tag_names: None,
                },
                confirmed: false,
            },
            WorkbenchAction::ReopenTask {
                task_id: id,
                confirmed: false,
            },
            WorkbenchAction::ArchiveTask {
                task_id: id,
                confirmed: false,
            },
            WorkbenchAction::DeleteTask {
                task_id: id,
                disposition: TaskDeleteDisposition::Cascade,
                confirmed: false,
            },
            WorkbenchAction::PostponeTask {
                task_id: id,
                days: 1,
                confirmed: false,
            },
            WorkbenchAction::SetTaskDefer {
                task_id: id,
                available_at: Some("2026-11-20".into()),
                confirmed: false,
            },
        ];
        for action in cases {
            assert!(reject_unconfirmed(&action).is_err(), "{action:?}");
            assert!(action.requires_confirmation());
        }
    }

    #[test]
    fn update_task_wire_round_trips() {
        let id = uuid::Uuid::new_v4();
        let wire = serde_json::json!({
            "action": "updateTask",
            "input": { "id": id.to_string(), "clearDue": true },
            "confirmed": true,
        });
        let action: WorkbenchAction = serde_json::from_value(wire.clone()).unwrap();
        assert!(matches!(
            &action,
            WorkbenchAction::UpdateTask { input, confirmed: true }
                if input.id == id && input.clear_due && input.title.is_none()
        ));
        assert_eq!(serde_json::to_value(&action).unwrap(), wire);
    }

    #[test]
    fn query_tasks_parses_with_default_query() {
        let action: WorkbenchAction =
            serde_json::from_value(serde_json::json!({ "action": "queryTasks" })).unwrap();
        assert!(matches!(
            &action,
            WorkbenchAction::QueryTasks { query }
                if query.status.is_none() && query.limit.is_none()
        ));
    }

    #[test]
    fn delete_task_wire_uses_disposition() {
        let id = uuid::Uuid::new_v4();
        let action: WorkbenchAction = serde_json::from_value(serde_json::json!({
            "action": "deleteTask",
            "task_id": id.to_string(),
            "disposition": "cascade",
            "confirmed": true,
        }))
        .unwrap();
        assert!(matches!(
            action,
            WorkbenchAction::DeleteTask { disposition: TaskDeleteDisposition::Cascade, .. }
        ));
        assert_eq!(
            workbench_action_description(&WorkbenchAction::DeleteTask {
                task_id: id,
                disposition: TaskDeleteDisposition::Promote,
                confirmed: true,
            }),
            format!("删除任务 {id}（promote）")
        );
    }

    #[test]
    fn lists_queried_outcome_round_trips() {
        let id = uuid::Uuid::new_v4();
        let wire = serde_json::json!({
            "outcome": "listsQueried",
            "lists": [
                { "id": id.to_string(), "name": "收件箱", "kind": "inbox", "openCount": 3 }
            ]
        });
        let outcome: ActionOutcome = serde_json::from_value(wire.clone()).unwrap();
        assert!(matches!(
            &outcome,
            ActionOutcome::ListsQueried { lists } if lists.len() == 1 && lists[0].open_count == 3
        ));
        assert_eq!(serde_json::to_value(&outcome).unwrap(), wire);
    }

    #[test]
    fn export_tasks_csv_round_trips() {
        let action: WorkbenchAction =
            serde_json::from_value(serde_json::json!({ "action": "exportTasksCsv" })).unwrap();
        assert!(matches!(action, WorkbenchAction::ExportTasksCsv));
        assert_eq!(
            serde_json::to_value(&action).unwrap(),
            serde_json::json!({ "action": "exportTasksCsv" })
        );
        // Read-only: no confirmation gate (same as the Query* variants).
        assert!(!action.requires_confirmation());

        let wire = serde_json::to_value(&ActionOutcome::TasksCsvExported {
            csv: "title\n".into(),
        })
        .unwrap();
        assert_eq!(
            wire,
            serde_json::json!({ "outcome": "tasksCsvExported", "csv": "title\n" })
        );
        let outcome: ActionOutcome = serde_json::from_value(wire).unwrap();
        assert!(matches!(
            outcome,
            ActionOutcome::TasksCsvExported { ref csv } if csv == "title\n"
        ));
    }
}
