use serde::{Deserialize, Serialize};

use super::{DomainError, EntityId, Revision};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Memory {
    pub id: EntityId,
    pub title: String,
    pub body: String,
    pub pinned: bool,
    pub archived: bool,
    pub quick_insert: bool,
    pub trigger_word: Option<String>,
    /// v2.0: excluded from any AI provider context (§9.4 red line).
    pub sensitive: bool,
    pub mention_use_count: i64,
    pub tag_ids: Vec<EntityId>,
    pub tag_names: Vec<String>,
    pub created_at: String,
    pub updated_at: String,
    pub revision: Revision,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateMemoryInput {
    pub title: String,
    pub body: Option<String>,
    pub pinned: Option<bool>,
    pub quick_insert: Option<bool>,
    pub trigger_word: Option<String>,
    pub tag_names: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateMemoryInput {
    pub id: EntityId,
    pub title: String,
    pub body: String,
    pub pinned: bool,
    pub archived: bool,
    pub quick_insert: bool,
    pub trigger_word: Option<String>,
    #[serde(default)]
    pub sensitive: bool,
    pub tag_names: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct MemoryQuery {
    pub pinned_only: Option<bool>,
    pub include_archived: Option<bool>,
    pub tag_id: Option<EntityId>,
    pub quick_insert_only: Option<bool>,
    pub search: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MemorySummary {
    pub id: EntityId,
    pub title: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum WikilinkPendingReason {
    Missing,
    Ambiguous,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WikilinkPending {
    pub title: String,
    pub reason: WikilinkPendingReason,
    pub candidates: Vec<MemorySummary>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum WikilinkResolutionAction {
    Link,
    Create,
    Skip,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WikilinkResolution {
    pub title: String,
    pub action: WikilinkResolutionAction,
    pub target_id: Option<EntityId>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WikilinkSyncResult {
    pub memory: Memory,
    pub linked_ids: Vec<EntityId>,
    pub pending: Vec<WikilinkPending>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoryBacklink {
    pub memory_id: EntityId,
    pub title: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RelatedMemoryHit {
    pub memory_id: EntityId,
    pub title: String,
    pub score: f64,
    pub reasons: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SearchEntityType {
    Task,
    Reminder,
    Memory,
    Clipboard,
}

impl SearchEntityType {
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
                "invalid search entity type: {value}"
            ))),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchHit {
    pub entity_type: SearchEntityType,
    pub entity_id: EntityId,
    pub title: String,
    pub snippet: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchQuery {
    pub query: String,
    pub types: Option<Vec<SearchEntityType>>,
    pub limit: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct SearchResults {
    pub tasks: Vec<SearchHit>,
    pub reminders: Vec<SearchHit>,
    pub memories: Vec<SearchHit>,
    pub clipboard: Vec<SearchHit>,
    /// v2.0 slice 8: semantic matches (empty when disabled/unavailable).
    #[serde(default)]
    pub semantic: Vec<crate::domain::ai_suggestion::SemanticHit>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConvertMemoryToTaskResult {
    pub memory: Memory,
    pub task_id: EntityId,
}
