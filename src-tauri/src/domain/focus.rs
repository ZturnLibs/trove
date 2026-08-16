use serde::{Deserialize, Serialize};

use super::{DomainError, EntityId, Task};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum FocusOutcome {
    InProgress,
    Completed,
    KeptTodo,
    Abandoned,
}

impl FocusOutcome {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::InProgress => "in_progress",
            Self::Completed => "completed",
            Self::KeptTodo => "kept_todo",
            Self::Abandoned => "abandoned",
        }
    }

    pub fn parse(value: &str) -> Result<Self, DomainError> {
        match value {
            "in_progress" => Ok(Self::InProgress),
            "completed" => Ok(Self::Completed),
            "kept_todo" => Ok(Self::KeptTodo),
            "abandoned" => Ok(Self::Abandoned),
            _ => Err(DomainError::Validation(format!(
                "invalid focus outcome: {value}"
            ))),
        }
    }

    pub fn is_terminal(self) -> bool {
        !matches!(self, Self::InProgress)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FocusSession {
    pub id: EntityId,
    pub task_id: EntityId,
    pub started_at: String,
    pub ended_at: Option<String>,
    pub planned_minutes: Option<i64>,
    pub outcome: FocusOutcome,
    pub progress_note: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DailyWrapRun {
    pub id: EntityId,
    pub wrap_date: String,
    pub started_at: String,
    pub completed_at: Option<String>,
    pub steps_completed: i64,
    pub summary: Option<serde_json::Value>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DailyWrapSnapshot {
    pub wrap_date: String,
    pub unfinished_focus: Vec<Task>,
    pub tomorrow_due: Vec<Task>,
    pub inbox_unprocessed: Vec<Task>,
    pub completed_today_count: i64,
    pub reminders_today_count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DailyWrapCompleteInput {
    pub steps_completed: i64,
    pub summary: Option<serde_json::Value>,
}
