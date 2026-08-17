use serde::{Deserialize, Serialize};

use super::{
    CreateMemoryInput, CreateReminderInput, CreateTaskInput, DomainError, EntityId, Memory,
    Reminder, Task, TaskPriority, TaskStatus, UrlCreateKind, UrlSchemeAction,
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
        )
    }

    pub fn is_confirmed(&self) -> bool {
        match self {
            WorkbenchAction::CreateTask { confirmed, .. }
            | WorkbenchAction::CreateReminder { confirmed, .. }
            | WorkbenchAction::CreateMemory { confirmed, .. }
            | WorkbenchAction::CompleteTask { confirmed, .. } => *confirmed,
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
}
