use serde::{Deserialize, Serialize};

use super::{ClipboardItem, EntityId, Reminder, Task};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ReviewType {
    Weekly,
}

impl ReviewType {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Weekly => "weekly",
        }
    }

    pub fn parse(value: &str) -> Result<Self, super::DomainError> {
        match value {
            "weekly" => Ok(Self::Weekly),
            _ => Err(super::DomainError::Validation(format!(
                "invalid review type: {value}"
            ))),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReviewSession {
    pub id: EntityId,
    pub review_type: ReviewType,
    pub started_at: String,
    pub completed_at: Option<String>,
    pub summary: Option<serde_json::Value>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WeeklyReviewSnapshot {
    pub inbox_unprocessed: Vec<Task>,
    pub inbox_count: i64,
    pub overdue: Vec<Task>,
    pub overdue_count: i64,
    pub waiting_follow_up: Vec<Task>,
    pub waiting_follow_up_count: i64,
    pub stale_active: Vec<Task>,
    pub stale_active_count: i64,
    pub completed_last_7_days: Vec<Task>,
    pub completed_last_7_days_count: i64,
    pub upcoming_recurring_reminders: Vec<Reminder>,
    pub upcoming_recurring_count: i64,
    pub large_clipboard_items: Vec<ClipboardItem>,
    pub large_clipboard_count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReviewCompleteInput {
    pub summary: Option<serde_json::Value>,
}
