use serde::{Deserialize, Serialize};

use super::recurrence::RecurrenceRule;
use super::{DomainError, EntityId, Revision};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum OccurrenceStatus {
    Pending,
    Scheduled,
    Actioned,
    Snoozed,
    Cancelled,
    InferredMissed,
}

impl OccurrenceStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Scheduled => "scheduled",
            Self::Actioned => "actioned",
            Self::Snoozed => "snoozed",
            Self::Cancelled => "cancelled",
            Self::InferredMissed => "inferred_missed",
        }
    }

    pub fn parse(value: &str) -> Result<Self, DomainError> {
        match value {
            "pending" => Ok(Self::Pending),
            "scheduled" => Ok(Self::Scheduled),
            "actioned" => Ok(Self::Actioned),
            "snoozed" => Ok(Self::Snoozed),
            "cancelled" => Ok(Self::Cancelled),
            "inferred_missed" => Ok(Self::InferredMissed),
            _ => Err(DomainError::Validation(format!(
                "invalid occurrence status: {value}"
            ))),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Reminder {
    pub id: EntityId,
    pub title: String,
    pub notes: String,
    pub task_id: Option<EntityId>,
    pub recurrence: Option<RecurrenceRule>,
    pub timezone: String,
    pub next_fire_at: String,
    pub end_at: Option<String>,
    pub enabled: bool,
    pub created_at: String,
    pub updated_at: String,
    pub revision: Revision,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReminderOccurrence {
    pub id: EntityId,
    pub reminder_id: EntityId,
    pub scheduled_at: String,
    pub status: OccurrenceStatus,
    pub needs_schedule: bool,
    pub system_notification_id: Option<i32>,
    pub actioned_at: Option<String>,
    pub snooze_until: Option<String>,
    pub title: String,
    pub task_id: Option<EntityId>,
    pub created_at: String,
    pub updated_at: String,
    pub revision: Revision,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateReminderInput {
    pub title: String,
    pub notes: Option<String>,
    pub task_id: Option<EntityId>,
    pub fire_at: String,
    pub recurrence: Option<RecurrenceRule>,
    pub timezone: Option<String>,
    pub end_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateReminderInput {
    pub id: EntityId,
    pub title: String,
    pub notes: String,
    pub fire_at: String,
    pub recurrence: Option<RecurrenceRule>,
    pub enabled: bool,
    pub end_at: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SnoozePreset {
    Minutes10,
    Hour1,
    Tomorrow,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TodayReminderItem {
    pub occurrence: ReminderOccurrence,
    pub reminder: Reminder,
}
