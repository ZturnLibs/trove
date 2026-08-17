use serde::{Deserialize, Serialize};

use super::{
    CreateMemoryInput, CreateReminderInput, CreateTaskInput, DomainError, EntityId, Memory,
    Reminder, Task, UrlCreateKind, UrlSchemeAction,
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
}
