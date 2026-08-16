use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::{format_utc, new_entity_id, Clock, DomainError, EntityId, Revision, SystemClock};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TaskStatus {
    Todo,
    Completed,
    Archived,
}

impl TaskStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Todo => "todo",
            Self::Completed => "completed",
            Self::Archived => "archived",
        }
    }

    pub fn parse(value: &str) -> Result<Self, DomainError> {
        match value {
            "todo" => Ok(Self::Todo),
            "completed" => Ok(Self::Completed),
            "archived" => Ok(Self::Archived),
            _ => Err(DomainError::Validation(format!("invalid status: {value}"))),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TaskPriority {
    None,
    Low,
    Medium,
    High,
}

impl TaskPriority {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Low => "low",
            Self::Medium => "medium",
            Self::High => "high",
        }
    }

    pub fn parse(value: &str) -> Result<Self, DomainError> {
        match value {
            "none" => Ok(Self::None),
            "low" => Ok(Self::Low),
            "medium" => Ok(Self::Medium),
            "high" => Ok(Self::High),
            _ => Err(DomainError::Validation(format!(
                "invalid priority: {value}"
            ))),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ListKind {
    Inbox,
    Custom,
}

impl ListKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Inbox => "inbox",
            Self::Custom => "custom",
        }
    }

    pub fn parse(value: &str) -> Result<Self, DomainError> {
        match value {
            "inbox" => Ok(Self::Inbox),
            "custom" => Ok(Self::Custom),
            _ => Err(DomainError::Validation(format!(
                "invalid list kind: {value}"
            ))),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskList {
    pub id: EntityId,
    pub name: String,
    pub kind: ListKind,
    pub sort_order: f64,
    pub created_at: String,
    pub updated_at: String,
    pub revision: Revision,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ListDeleteDisposition {
    MoveToInbox,
    ArchiveTasks,
    ForceDelete,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteListResult {
    pub list_id: EntityId,
    pub list_name: String,
    pub disposition: ListDeleteDisposition,
    pub task_ids: Vec<EntityId>,
    pub archived_task_ids: Vec<EntityId>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Tag {
    pub id: EntityId,
    pub name: String,
    pub created_at: String,
    pub updated_at: String,
    pub revision: Revision,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TaskWorkflowState {
    Active,
    Waiting,
}

impl TaskWorkflowState {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Waiting => "waiting",
        }
    }

    pub fn parse(value: &str) -> Result<Self, DomainError> {
        match value {
            "active" => Ok(Self::Active),
            "waiting" => Ok(Self::Waiting),
            _ => Err(DomainError::Validation(format!(
                "invalid workflow state: {value}"
            ))),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Task {
    pub id: EntityId,
    pub title: String,
    pub notes: String,
    pub status: TaskStatus,
    pub priority: TaskPriority,
    pub list_id: EntityId,
    pub list_name: String,
    pub list_kind: ListKind,
    pub due_date: Option<String>,
    pub due_time: Option<String>,
    pub completed_at: Option<String>,
    pub sort_order: f64,
    pub series_id: Option<EntityId>,
    pub tag_ids: Vec<EntityId>,
    pub tag_names: Vec<String>,
    pub workflow_state: TaskWorkflowState,
    pub available_at: Option<String>,
    pub waiting_for: Option<String>,
    pub follow_up_date: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub revision: Revision,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateTaskInput {
    pub title: String,
    pub notes: Option<String>,
    pub priority: Option<TaskPriority>,
    pub list_id: Option<EntityId>,
    pub due_date: Option<String>,
    pub due_time: Option<String>,
    pub tag_names: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateTaskInput {
    pub id: EntityId,
    pub title: String,
    pub notes: String,
    pub priority: TaskPriority,
    pub list_id: EntityId,
    pub due_date: Option<String>,
    pub due_time: Option<String>,
    pub tag_names: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct TaskQuery {
    pub list_id: Option<EntityId>,
    pub inbox_only: Option<bool>,
    pub status: Option<TaskStatus>,
    pub priority: Option<TaskPriority>,
    pub tag_id: Option<EntityId>,
    pub include_archived: Option<bool>,
    /// Inclusive YYYY-MM-DD
    pub due_from: Option<String>,
    /// Inclusive YYYY-MM-DD
    pub due_to: Option<String>,
    pub due_null: Option<bool>,
    /// completed_at date >= YYYY-MM-DD (local)
    pub completed_since: Option<String>,
    pub search: Option<String>,
    pub workflow_state: Option<TaskWorkflowState>,
    pub deferred_only: Option<bool>,
    pub waiting_follow_up_due: Option<bool>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SmartListKind {
    Tomorrow,
    Next7Days,
    Overdue,
    HighPriority,
    NoDue,
    RecentCompleted,
    Deferred,
    WaitingFollowUp,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TodayTasks {
    pub overdue: Vec<Task>,
    pub due_today: Vec<Task>,
    pub completed_today: Vec<Task>,
    pub focus: Vec<Task>,
    pub waiting_follow_up: Vec<Task>,
    pub focus_carry_suggestions: Vec<Task>,
    pub reminders_today: Vec<super::TodayReminderItem>,
    pub today: String,
}

pub fn validate_due_date(value: &str) -> Result<(), DomainError> {
    NaiveDate::parse_from_str(value, "%Y-%m-%d")
        .map(|_| ())
        .map_err(|_| DomainError::Validation("dueDate must be YYYY-MM-DD".into()))
}

pub fn validate_due_time(value: &str) -> Result<(), DomainError> {
    if value.len() == 5
        && value.as_bytes()[2] == b':'
        && value[..2].parse::<u8>().ok().filter(|h| *h < 24).is_some()
        && value[3..].parse::<u8>().ok().filter(|m| *m < 60).is_some()
    {
        Ok(())
    } else {
        Err(DomainError::Validation("dueTime must be HH:MM".into()))
    }
}

pub fn validate_due_vs_available(
    due_date: Option<&str>,
    available_at: Option<&str>,
) -> Result<(), DomainError> {
    super::task_activity::validate_due_vs_available(due_date, available_at)
        .map_err(DomainError::Validation)
}

pub fn local_today(_clock: &impl Clock) -> String {
    chrono::Local::now()
        .date_naive()
        .format("%Y-%m-%d")
        .to_string()
}

pub fn new_id() -> EntityId {
    new_entity_id()
}

pub fn now_utc(clock: &SystemClock) -> DateTime<Utc> {
    clock.now()
}

pub fn stamp(clock: &SystemClock) -> String {
    format_utc(clock.now())
}

pub fn parse_uuid(value: &str) -> Result<Uuid, DomainError> {
    value
        .parse()
        .map_err(|_| DomainError::Validation(format!("invalid id: {value}")))
}
