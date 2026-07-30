use serde::{Deserialize, Serialize};

use super::{DomainError, EntityId, Revision};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClipboardItem {
    pub id: EntityId,
    pub content: String,
    pub content_hash: String,
    pub source_app: Option<String>,
    pub favorite: bool,
    pub use_count: i64,
    pub last_used_at: Option<String>,
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
