use serde::{Deserialize, Serialize};

use super::{DomainError, EntityId, ParsedCapture, Revision};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ClipboardKindHint {
    Plain,
    Url,
    Email,
    Phone,
    Date,
    Code,
    Error,
}

impl ClipboardKindHint {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Plain => "plain",
            Self::Url => "url",
            Self::Email => "email",
            Self::Phone => "phone",
            Self::Date => "date",
            Self::Code => "code",
            Self::Error => "error",
        }
    }

    pub fn parse(value: &str) -> Result<Self, DomainError> {
        match value {
            "plain" => Ok(Self::Plain),
            "url" => Ok(Self::Url),
            "email" => Ok(Self::Email),
            "phone" => Ok(Self::Phone),
            "date" => Ok(Self::Date),
            "code" => Ok(Self::Code),
            "error" => Ok(Self::Error),
            _ => Err(DomainError::Validation(format!(
                "invalid clipboard kind hint: {value}"
            ))),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ClipboardKind {
    Text,
    Image,
}

impl ClipboardKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Text => "text",
            Self::Image => "image",
        }
    }

    pub fn parse(value: &str) -> Result<Self, DomainError> {
        match value {
            "text" => Ok(Self::Text),
            "image" => Ok(Self::Image),
            _ => Err(DomainError::Validation(format!(
                "invalid clipboard kind: {value}"
            ))),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClipboardItem {
    pub id: EntityId,
    pub kind: ClipboardKind,
    pub kind_hint: ClipboardKindHint,
    pub content: String,
    pub content_hash: String,
    pub asset_id: Option<EntityId>,
    pub source_app: Option<String>,
    pub favorite: bool,
    pub use_count: i64,
    pub last_used_at: Option<String>,
    pub width: Option<i64>,
    pub height: Option<i64>,
    pub thumb_base64: Option<String>,
    pub ocr_text: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub revision: Revision,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Asset {
    pub id: EntityId,
    pub kind: String,
    pub content_hash: String,
    pub relative_path: String,
    pub thumb_path: Option<String>,
    pub mime_type: String,
    pub byte_size: i64,
    pub width: Option<i64>,
    pub height: Option<i64>,
    pub created_at: String,
    pub updated_at: String,
    pub revision: Revision,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ClipboardQuery {
    pub favorites_only: Option<bool>,
    pub search: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
    pub kind: Option<ClipboardKind>,
    pub kind_hint: Option<ClipboardKindHint>,
    pub source_app: Option<String>,
    pub date_from: Option<String>,
    pub date_to: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClipboardCaptureSettings {
    pub enabled: bool,
    pub retention_days: u32,
    pub max_items: u32,
    pub excluded_apps: Vec<String>,
}

impl Default for ClipboardCaptureSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            retention_days: 30,
            max_items: 500,
            excluded_apps: default_excluded_apps(),
        }
    }
}

pub fn default_excluded_apps() -> Vec<String> {
    vec![
        "1Password".into(),
        "1Password for Safari".into(),
        "Bitwarden".into(),
        "LastPass".into(),
        "KeePassXC".into(),
        "Keeper Password Manager".into(),
        "Authy Desktop".into(),
        "Secrets".into(),
    ]
}

pub fn validate_clipboard_settings(settings: &ClipboardCaptureSettings) -> Result<(), DomainError> {
    if settings.retention_days == 0 || settings.retention_days > 3650 {
        return Err(DomainError::Validation("保留天数需在 1–3650".into()));
    }
    if settings.max_items < 10 || settings.max_items > 20_000 {
        return Err(DomainError::Validation("最大条数需在 10–20000".into()));
    }
    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClipboardTaskDraftInput {
    pub title: Option<String>,
    pub notes: Option<String>,
    pub due_date: Option<String>,
    pub due_time: Option<String>,
    pub priority: Option<super::TaskPriority>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SimilarTaskHit {
    pub task_id: EntityId,
    pub title: String,
    pub score: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClipboardSmartContext {
    pub kind_hint: ClipboardKindHint,
    pub task_draft: Option<ParsedCapture>,
    pub similar_tasks: Vec<SimilarTaskHit>,
    pub linked_task_id: Option<EntityId>,
    pub linked_memory_id: Option<EntityId>,
}
