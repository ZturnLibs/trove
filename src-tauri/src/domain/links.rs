use serde::{Deserialize, Serialize};

use super::{DomainError, EntityId};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum LinkEntityType {
    Task,
    Reminder,
    Memory,
    Clipboard,
    Asset,
    FileRef,
}

impl LinkEntityType {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Task => "task",
            Self::Reminder => "reminder",
            Self::Memory => "memory",
            Self::Clipboard => "clipboard",
            Self::Asset => "asset",
            Self::FileRef => "file_ref",
        }
    }

    pub fn parse(value: &str) -> Result<Self, DomainError> {
        match value {
            "task" => Ok(Self::Task),
            "reminder" => Ok(Self::Reminder),
            "memory" => Ok(Self::Memory),
            "clipboard" => Ok(Self::Clipboard),
            "asset" => Ok(Self::Asset),
            "file_ref" => Ok(Self::FileRef),
            _ => Err(DomainError::Validation(format!(
                "invalid link entity type: {value}"
            ))),
        }
    }
}

pub const LINK_KIND_ATTACHMENT: &str = "attachment";
pub const LINK_KIND_CONVERTED_TO: &str = "converted_to";
pub const LINK_KIND_RELATED: &str = "related";
pub const LINK_KIND_MENTION: &str = "mention";
/// v2.0 slice 2: task created from an AI extract suggestion (provenance).
pub const LINK_KIND_AI_EXTRACT: &str = "ai_extract";

pub fn validate_link_kind(value: &str) -> Result<(), DomainError> {
    match value {
        LINK_KIND_ATTACHMENT
            | LINK_KIND_CONVERTED_TO
            | LINK_KIND_RELATED
            | LINK_KIND_MENTION
            | LINK_KIND_AI_EXTRACT => {
            Ok(())
        }
        _ => Err(DomainError::Validation(format!(
            "invalid link kind: {value}"
        ))),
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EntityLink {
    pub id: EntityId,
    pub source_type: String,
    pub source_id: EntityId,
    pub target_type: String,
    pub target_id: EntityId,
    pub link_kind: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LinkInput {
    pub source_type: String,
    pub source_id: EntityId,
    pub target_type: String,
    pub target_id: EntityId,
    pub link_kind: String,
}
