use chrono::{Datelike, Local, NaiveTime};
use serde::{Deserialize, Serialize};

use super::task::TaskPriority;
use super::{DomainError, EntityId};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AutomationEntityType {
    Task,
    Reminder,
    Memory,
    Clipboard,
}

impl AutomationEntityType {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Task => "task",
            Self::Reminder => "reminder",
            Self::Memory => "memory",
            Self::Clipboard => "clipboard",
        }
    }

    pub fn parse(value: &str) -> Result<Self, DomainError> {
        match value {
            "task" => Ok(Self::Task),
            "reminder" => Ok(Self::Reminder),
            "memory" => Ok(Self::Memory),
            "clipboard" => Ok(Self::Clipboard),
            _ => Err(DomainError::Validation(format!(
                "invalid automation entity type: {value}"
            ))),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AutomationEventKind {
    TaskCreated,
    ReminderCreated,
    MemoryCreated,
    ClipboardFavorited,
    ReminderFired,
    TaskMovedToList,
    TaskTagAdded,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AutomationEvent {
    pub kind: AutomationEventKind,
    pub entity_type: AutomationEntityType,
    pub entity_id: EntityId,
    pub title: String,
    pub body: String,
    pub list_id: Option<EntityId>,
    pub tag_names: Vec<String>,
    pub priority: Option<TaskPriority>,
    pub source_app: Option<String>,
    /// Set when kind is TaskTagAdded.
    pub added_tag: Option<String>,
    /// Set when kind is TaskMovedToList.
    pub target_list_id: Option<EntityId>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum AutomationTrigger {
    TaskCreated,
    ReminderCreated,
    MemoryCreated,
    ClipboardFavorited,
    ReminderFired,
    TaskMovedToList {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        list_id: Option<EntityId>,
    },
    TaskTagAdded {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        tag_name: Option<String>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum AutomationCondition {
    TitleContains {
        text: String,
        #[serde(default = "default_true")]
        case_insensitive: bool,
    },
    BodyContains {
        text: String,
        #[serde(default = "default_true")]
        case_insensitive: bool,
    },
    EntityType {
        entity_type: AutomationEntityType,
    },
    ListId {
        list_id: EntityId,
    },
    HasTag {
        tag_name: String,
    },
    Priority {
        priority: TaskPriority,
    },
    SourceApp {
        app: String,
    },
    Weekday {
        /// 0 = Monday … 6 = Sunday (chrono weekday)
        days: Vec<u8>,
    },
    TimeRange {
        /// Local HH:MM
        start: String,
        end: String,
    },
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum AutomationAction {
    SetPriority {
        priority: TaskPriority,
    },
    MoveToList {
        list_id: EntityId,
    },
    AddTag {
        tag_name: String,
    },
    PinMemory,
    Notify {
        title: String,
        body: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AutomationRuleDefinition {
    pub trigger: AutomationTrigger,
    pub conditions: Vec<AutomationCondition>,
    pub actions: Vec<AutomationAction>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AutomationRule {
    pub id: EntityId,
    pub name: String,
    pub enabled: bool,
    pub definition: AutomationRuleDefinition,
    pub created_at: String,
    pub updated_at: String,
    pub revision: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateAutomationRuleInput {
    pub name: String,
    pub definition: AutomationRuleDefinition,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateAutomationRuleInput {
    pub id: EntityId,
    pub name: String,
    pub enabled: bool,
    pub definition: AutomationRuleDefinition,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AutomationRunStatus {
    Success,
    Skipped,
    Failed,
    DryRun,
}

impl AutomationRunStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Success => "success",
            Self::Skipped => "skipped",
            Self::Failed => "failed",
            Self::DryRun => "dry_run",
        }
    }

    pub fn parse(value: &str) -> Result<Self, DomainError> {
        match value {
            "success" => Ok(Self::Success),
            "skipped" => Ok(Self::Skipped),
            "failed" => Ok(Self::Failed),
            "dry_run" => Ok(Self::DryRun),
            _ => Err(DomainError::Validation(format!(
                "invalid automation run status: {value}"
            ))),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AutomationRun {
    pub id: EntityId,
    pub rule_id: EntityId,
    pub rule_name: String,
    pub entity_type: AutomationEntityType,
    pub entity_id: EntityId,
    pub status: AutomationRunStatus,
    pub actions_applied: Vec<AutomationAction>,
    pub error_summary: Option<String>,
    pub dry_run: bool,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AutomationDryRunResult {
    pub rule_id: EntityId,
    pub rule_name: String,
    pub matched: bool,
    pub actions: Vec<AutomationAction>,
    pub skip_reason: Option<String>,
}

pub fn trigger_matches(trigger: &AutomationTrigger, event: &AutomationEvent) -> bool {
    match (trigger, event.kind) {
        (AutomationTrigger::TaskCreated, AutomationEventKind::TaskCreated) => true,
        (AutomationTrigger::ReminderCreated, AutomationEventKind::ReminderCreated) => true,
        (AutomationTrigger::MemoryCreated, AutomationEventKind::MemoryCreated) => true,
        (AutomationTrigger::ClipboardFavorited, AutomationEventKind::ClipboardFavorited) => true,
        (AutomationTrigger::ReminderFired, AutomationEventKind::ReminderFired) => true,
        (
            AutomationTrigger::TaskMovedToList { list_id },
            AutomationEventKind::TaskMovedToList,
        ) => list_id
            .map(|id| event.target_list_id == Some(id))
            .unwrap_or(true),
        (AutomationTrigger::TaskTagAdded { tag_name }, AutomationEventKind::TaskTagAdded) => {
            match tag_name {
                Some(expected) => event
                    .added_tag
                    .as_ref()
                    .is_some_and(|t| t.eq_ignore_ascii_case(expected)),
                None => true,
            }
        }
        _ => false,
    }
}

pub fn evaluate_conditions(conditions: &[AutomationCondition], event: &AutomationEvent) -> bool {
    conditions
        .iter()
        .all(|condition| evaluate_condition(condition, event))
}

fn evaluate_condition(condition: &AutomationCondition, event: &AutomationEvent) -> bool {
    match condition {
        AutomationCondition::TitleContains {
            text,
            case_insensitive,
        } => text_matches(&event.title, text, *case_insensitive),
        AutomationCondition::BodyContains {
            text,
            case_insensitive,
        } => text_matches(&event.body, text, *case_insensitive),
        AutomationCondition::EntityType { entity_type } => event.entity_type == *entity_type,
        AutomationCondition::ListId { list_id } => event.list_id == Some(*list_id),
        AutomationCondition::HasTag { tag_name } => event
            .tag_names
            .iter()
            .any(|t| t.eq_ignore_ascii_case(tag_name)),
        AutomationCondition::Priority { priority } => event.priority == Some(*priority),
        AutomationCondition::SourceApp { app } => event
            .source_app
            .as_ref()
            .is_some_and(|s| s.eq_ignore_ascii_case(app)),
        AutomationCondition::Weekday { days } => {
            let weekday = Local::now().weekday().num_days_from_monday() as u8;
            days.contains(&weekday)
        }
        AutomationCondition::TimeRange { start, end } => time_in_range(start, end),
    }
}

fn text_matches(haystack: &str, needle: &str, case_insensitive: bool) -> bool {
    let needle = needle.trim();
    if needle.is_empty() {
        return true;
    }
    if case_insensitive {
        haystack
            .to_ascii_lowercase()
            .contains(&needle.to_ascii_lowercase())
    } else {
        haystack.contains(needle)
    }
}

fn time_in_range(start: &str, end: &str) -> bool {
    let now = Local::now().time();
    let Ok(start) = NaiveTime::parse_from_str(start.trim(), "%H:%M") else {
        return false;
    };
    let Ok(end) = NaiveTime::parse_from_str(end.trim(), "%H:%M") else {
        return false;
    };
    if start <= end {
        now >= start && now <= end
    } else {
        now >= start || now <= end
    }
}

pub fn validate_rule_definition(def: &AutomationRuleDefinition) -> Result<(), DomainError> {
    if def.actions.is_empty() {
        return Err(DomainError::Validation("规则至少需要一个动作".into()));
    }
    for action in &def.actions {
        validate_action(action)?;
    }
    Ok(())
}

fn validate_action(action: &AutomationAction) -> Result<(), DomainError> {
    match action {
        AutomationAction::SetPriority { .. } => Ok(()),
        AutomationAction::MoveToList { list_id } => {
            if list_id.is_nil() {
                return Err(DomainError::Validation("清单 ID 无效".into()));
            }
            Ok(())
        }
        AutomationAction::AddTag { tag_name } => {
            if tag_name.trim().is_empty() {
                return Err(DomainError::Validation("标签名不能为空".into()));
            }
            Ok(())
        }
        AutomationAction::PinMemory => Ok(()),
        AutomationAction::Notify { title, body: _ } => {
            if title.trim().is_empty() {
                return Err(DomainError::Validation("通知标题不能为空".into()));
            }
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::new_entity_id;

    fn sample_event(kind: AutomationEventKind) -> AutomationEvent {
        AutomationEvent {
            kind,
            entity_type: AutomationEntityType::Task,
            entity_id: new_entity_id(),
            title: "Buy Milk".into(),
            body: "From the store".into(),
            list_id: None,
            tag_names: vec!["errands".into()],
            priority: Some(TaskPriority::High),
            source_app: None,
            added_tag: None,
            target_list_id: None,
        }
    }

    #[test]
    fn trigger_matches_created_events() {
        let event = sample_event(AutomationEventKind::TaskCreated);
        assert!(trigger_matches(&AutomationTrigger::TaskCreated, &event));
        assert!(!trigger_matches(&AutomationTrigger::MemoryCreated, &event));
    }

    #[test]
    fn title_contains_condition() {
        let event = sample_event(AutomationEventKind::TaskCreated);
        let cond = AutomationCondition::TitleContains {
            text: "milk".into(),
            case_insensitive: true,
        };
        assert!(evaluate_condition(&cond, &event));
    }

    #[test]
    fn empty_keyword_matches_anything() {
        let event = sample_event(AutomationEventKind::TaskCreated);
        let cond = AutomationCondition::TitleContains {
            text: "   ".into(),
            case_insensitive: true,
        };
        assert!(evaluate_condition(&cond, &event));
    }

    #[test]
    fn validate_requires_action() {
        let def = AutomationRuleDefinition {
            trigger: AutomationTrigger::TaskCreated,
            conditions: vec![],
            actions: vec![],
        };
        assert!(validate_rule_definition(&def).is_err());
    }
}
