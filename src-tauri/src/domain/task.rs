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
    /// 所属分组；None = 未分组。收件箱恒为 None（不可归组）。
    pub group_id: Option<EntityId>,
}

// ---------------------------------------------------------------------------
// 任务分组（分组 → 清单两级结构）
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskListGroup {
    pub id: EntityId,
    pub name: String,
    pub sort_order: f64,
    pub created_at: String,
    pub updated_at: String,
    pub revision: Revision,
}

/// 清单行摘要（侧边栏 / 下拉分层用），带未完成任务数。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskListSummary {
    pub id: EntityId,
    pub name: String,
    pub kind: ListKind,
    pub group_id: Option<EntityId>,
    pub open_count: u64,
}

/// 分组节点：组信息 + 组内清单（按 sort_order）+ 组内未完成任务总数。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskListGroupNode {
    pub group: TaskListGroup,
    pub lists: Vec<TaskListSummary>,
    pub open_count: u64,
}

/// 侧边栏任务区全量数据：收件箱固定顶部，未分组清单平铺在后。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskListGroupOverview {
    pub groups: Vec<TaskListGroupNode>,
    pub ungrouped: Vec<TaskListSummary>,
    pub inbox: TaskListSummary,
}

/// 删除分组（解散）的撤销数据：恢复分组 + 回链组内清单。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListGroupDeleteUndo {
    pub group: TaskListGroup,
    pub moved_list_ids: Vec<EntityId>,
}

/// 今日页过滤 chips 的清单归属条件（不传 = 全部）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum ListGroupScope {
    Group { group_id: EntityId },
    Ungrouped,
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
    pub parent_id: Option<EntityId>,
    pub child_order: f64,
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
    pub parent_id: Option<EntityId>,
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
    /// 按清单所属分组过滤（含未分组清单的任务时用 inbox/ungrouped 组合，这里只做精确组匹配）。
    pub list_group_id: Option<EntityId>,
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
    /// Filter to direct children of a task (subtask list in detail panel).
    pub parent_id: Option<EntityId>,
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

// ---------------------------------------------------------------------------
// v2.0 slice 6: one-level task checklist
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ChecklistItem {
    pub id: EntityId,
    pub task_id: EntityId,
    pub content: String,
    pub checked: bool,
    pub sort_order: i64,
    pub created_at: String,
    pub updated_at: String,
    pub revision: Revision,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TaskChecklist {
    pub items: Vec<ChecklistItem>,
    pub total: i64,
    pub checked_count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ChecklistUpdateInput {
    pub id: EntityId,
    pub content: Option<String>,
    pub checked: Option<bool>,
}

pub const CHECKLIST_MAX_ITEMS: usize = 50;
pub const CHECKLIST_CONTENT_MAX_CHARS: usize = 200;

// ---------------------------------------------------------------------------
// v2.1 subtasks: multi-level nested tasks
// ---------------------------------------------------------------------------

/// Maximum nesting depth for subtasks (root = depth 1, its children depth 2,
/// ... depth 5 is the deepest allowed level).
pub const MAX_SUBTASK_DEPTH: usize = 5;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TaskDeleteDisposition {
    /// Delete the task and all its descendants.
    Cascade,
    /// Delete only the task; promote its direct children to top level.
    Promote,
}

impl TaskDeleteDisposition {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Cascade => "cascade",
            Self::Promote => "promote",
        }
    }

    pub fn parse(value: &str) -> Result<Self, DomainError> {
        match value {
            "cascade" => Ok(Self::Cascade),
            "promote" => Ok(Self::Promote),
            _ => Err(DomainError::Validation(format!(
                "invalid delete disposition: {value}"
            ))),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskTreeExpanded {
    pub task_id: EntityId,
    pub expanded: bool,
    pub updated_at: String,
}

/// Validate that a task at `ancestor_chain_len` ancestors below a root still
/// respects the max nesting depth. `chain_len` is the number of ancestors the
/// new task would have (0 = top level).
pub fn validate_parent_depth(chain_len: usize) -> Result<(), DomainError> {
    if chain_len >= MAX_SUBTASK_DEPTH {
        return Err(DomainError::Validation(format!(
            "子任务嵌套最多 {MAX_SUBTASK_DEPTH} 层"
        )));
    }
    Ok(())
}

pub fn validate_checklist_content(content: &str) -> Result<String, DomainError> {
    let trimmed = content.trim();
    if trimmed.is_empty() {
        return Err(DomainError::Validation("检查项内容不能为空".into()));
    }
    if trimmed.chars().count() > CHECKLIST_CONTENT_MAX_CHARS {
        return Err(DomainError::Validation(
            "检查项内容不能超过 200 字".into(),
        ));
    }
    Ok(trimmed.to_string())
}
