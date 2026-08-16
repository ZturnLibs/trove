use serde::{Deserialize, Serialize};

use super::{EntityId, Revision};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileReference {
    pub id: EntityId,
    pub display_name: String,
    pub path_hint: String,
    pub mime_type: Option<String>,
    pub byte_size: Option<i64>,
    pub accessible: bool,
    pub created_at: String,
    pub updated_at: String,
    pub revision: Revision,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LinkedFileReference {
    pub link_id: EntityId,
    pub file: FileReference,
}
