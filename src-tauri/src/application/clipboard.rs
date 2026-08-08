use crate::application::assets::AssetStore;
use crate::application::links::EntityLinkService;
use crate::application::memories::MemoryService;
use crate::application::search::SearchService;
use crate::application::tasks::TaskService;
use crate::domain::{
    new_id, stamp, ClipboardItem, ClipboardKind, ClipboardQuery, CreateMemoryInput,
    CreateTaskInput, DomainError, EntityId, PagedResult, SearchEntityType, SystemClock,
};
use crate::domain::{page_limit, page_offset};
use crate::infrastructure::db::Database;
use crate::infrastructure::settings::SettingsService;
use crate::platform::ocr;
use rusqlite::{params, Connection, OptionalExtension};
use sha2::{Digest, Sha256};
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::{Duration, Instant};

pub struct ClipboardService {
    db: Database,
    clock: SystemClock,
    search: SearchService,
    tasks: TaskService,
    memories: MemoryService,
    settings: SettingsService,
    assets: AssetStore,
    links: EntityLinkService,
    suppress_until: Mutex<Option<(String, Instant)>>,
}

pub struct ImageCopyPayload {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

impl ClipboardService {
    pub fn new(db: Database, assets_root: PathBuf) -> Self {
        Self {
            search: SearchService::new(db.clone()),
            tasks: TaskService::new(db.clone()),
            memories: MemoryService::new(db.clone()),
            settings: SettingsService::new(db.clone()),
            assets: AssetStore::new(db.clone(), assets_root),
            links: EntityLinkService::new(db.clone()),
            db,
            clock: SystemClock,
            suppress_until: Mutex::new(None),
        }
    }

    pub fn assets(&self) -> &AssetStore {
        &self.assets
    }

    fn connect(&self) -> Result<Connection, DomainError> {
        self.db.connect().map_err(internal)
    }

    pub fn hash_content(content: &str) -> String {
        let normalized = content.replace("\r\n", "\n");
        let mut hasher = Sha256::new();
        hasher.update(normalized.as_bytes());
        hex::encode(hasher.finalize())
    }

    pub fn suppress_next(&self, hash: &str) {
        if let Ok(mut guard) = self.suppress_until.lock() {
            *guard = Some((hash.to_string(), Instant::now() + Duration::from_secs(2)));
        }
    }

    pub fn suppress_next_text(&self, content: &str) {
        self.suppress_next(&Self::hash_content(content));
    }

    fn is_suppressed(&self, hash: &str) -> bool {
        let Ok(mut guard) = self.suppress_until.lock() else {
            return false;
        };
        match guard.as_ref() {
            Some((h, until)) if h == hash && Instant::now() < *until => true,
            Some((_, until)) if Instant::now() >= *until => {
                *guard = None;
                false
            }
            _ => false,
        }
    }

    fn excluded(
        source_app: &Option<String>,
        settings: &crate::infrastructure::settings::AppSettings,
    ) -> bool {
        let Some(ref app) = source_app else {
            return false;
        };
        settings
            .clipboard_excluded_apps
            .iter()
            .any(|excluded| app.eq_ignore_ascii_case(excluded) || app.contains(excluded))
    }

    pub fn capture_text(
        &self,
        content: String,
        source_app: Option<String>,
    ) -> Result<Option<ClipboardItem>, DomainError> {
        let settings = self.settings.get()?;
        if !settings.clipboard_capture_enabled {
            return Ok(None);
        }
        let content = content.replace("\r\n", "\n");
        if content.trim().is_empty() {
            return Ok(None);
        }
        if content.len() > 200_000 {
            return Ok(None);
        }
        if Self::excluded(&source_app, &settings) {
            return Ok(None);
        }

        let hash = Self::hash_content(&content);
        if self.is_suppressed(&hash) {
            return Ok(None);
        }

        let now = stamp(&self.clock);
        let conn = self.connect()?;

        let latest: Option<(String, String)> = conn
            .query_row(
                "SELECT id, content_hash FROM clipboard_items
                 WHERE deleted_at IS NULL
                 ORDER BY created_at DESC LIMIT 1",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()
            .map_err(internal)?;

        if let Some((id, latest_hash)) = latest {
            if latest_hash == hash {
                conn.execute(
                    "UPDATE clipboard_items SET updated_at = ?1, revision = revision + 1 WHERE id = ?2",
                    params![now, id],
                )
                .map_err(internal)?;
                let item_id: EntityId = id
                    .parse()
                    .map_err(|e| DomainError::Internal(format!("{e}")))?;
                return self.get(item_id).map(Some);
            }
        }

        let id = new_id();
        let title = content
            .lines()
            .next()
            .unwrap_or("剪切板")
            .chars()
            .take(80)
            .collect::<String>();
        conn.execute(
            "INSERT INTO clipboard_items (
                id, content, content_hash, source_app, favorite, use_count, last_used_at,
                created_at, updated_at, revision, deleted_at, kind, asset_id
             ) VALUES (?1, ?2, ?3, ?4, 0, 0, NULL, ?5, ?5, 1, NULL, 'text', NULL)",
            params![id.to_string(), content, hash, source_app, now],
        )
        .map_err(internal)?;
        self.search
            .upsert(SearchEntityType::Clipboard, id, &title, &content)?;
        self.enforce_limits()?;
        self.get(id).map(Some)
    }

    pub fn capture_image(
        &self,
        width: u32,
        height: u32,
        rgba: &[u8],
        source_app: Option<String>,
    ) -> Result<Option<ClipboardItem>, DomainError> {
        let settings = self.settings.get()?;
        if !settings.clipboard_capture_enabled {
            return Ok(None);
        }
        if Self::excluded(&source_app, &settings) {
            return Ok(None);
        }
        if width * height > 16_000_000 {
            return Ok(None);
        }

        let stored = self.assets.store_rgba_image(width, height, rgba)?;
        let hash = stored.asset.content_hash.clone();
        if self.is_suppressed(&hash) {
            return Ok(None);
        }

        let now = stamp(&self.clock);
        let conn = self.connect()?;
        let latest: Option<(String, String)> = conn
            .query_row(
                "SELECT id, content_hash FROM clipboard_items
                 WHERE deleted_at IS NULL
                 ORDER BY created_at DESC LIMIT 1",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()
            .map_err(internal)?;

        if let Some((id, latest_hash)) = latest {
            if latest_hash == hash {
                conn.execute(
                    "UPDATE clipboard_items SET updated_at = ?1, revision = revision + 1 WHERE id = ?2",
                    params![now, id],
                )
                .map_err(internal)?;
                let item_id: EntityId = id
                    .parse()
                    .map_err(|e| DomainError::Internal(format!("{e}")))?;
                return self.get(item_id).map(Some);
            }
        }

        // OCR (local). Empty on failure.
        let ocr = ocr::recognize_png(&stored.png_bytes);
        self.upsert_derived_text(stored.asset.id, &ocr.text, &ocr.engine_version)?;

        let id = new_id();
        let summary = if ocr.text.trim().is_empty() {
            format!("图片 {width}×{height}")
        } else {
            ocr.text
                .lines()
                .next()
                .unwrap_or("图片")
                .chars()
                .take(80)
                .collect()
        };
        conn.execute(
            "INSERT INTO clipboard_items (
                id, content, content_hash, source_app, favorite, use_count, last_used_at,
                created_at, updated_at, revision, deleted_at, kind, asset_id
             ) VALUES (?1, ?2, ?3, ?4, 0, 0, NULL, ?5, ?5, 1, NULL, 'image', ?6)",
            params![
                id.to_string(),
                summary,
                hash,
                source_app,
                now,
                stored.asset.id.to_string()
            ],
        )
        .map_err(internal)?;

        let body = if ocr.text.is_empty() {
            summary.clone()
        } else {
            format!("[来自图片识别]\n{}", ocr.text)
        };
        self.search
            .upsert(SearchEntityType::Clipboard, id, &summary, &body)?;
        self.enforce_limits()?;
        self.get(id).map(Some)
    }

    fn upsert_derived_text(
        &self,
        asset_id: EntityId,
        text: &str,
        engine_version: &str,
    ) -> Result<(), DomainError> {
        let now = stamp(&self.clock);
        let conn = self.connect()?;
        let existing: Option<String> = conn
            .query_row(
                "SELECT id FROM derived_texts WHERE asset_id = ?1 AND kind = 'ocr'",
                [asset_id.to_string()],
                |row| row.get(0),
            )
            .optional()
            .map_err(internal)?;
        if let Some(id) = existing {
            conn.execute(
                "UPDATE derived_texts SET text = ?1, engine_version = ?2, updated_at = ?3 WHERE id = ?4",
                params![text, engine_version, now, id],
            )
            .map_err(internal)?;
        } else {
            conn.execute(
                "INSERT INTO derived_texts (id, asset_id, kind, text, engine_version, created_at, updated_at)
                 VALUES (?1, ?2, 'ocr', ?3, ?4, ?5, ?5)",
                params![
                    new_id().to_string(),
                    asset_id.to_string(),
                    text,
                    engine_version,
                    now
                ],
            )
            .map_err(internal)?;
        }
        Ok(())
    }

    pub fn ocr_text_for_asset(&self, asset_id: EntityId) -> Result<String, DomainError> {
        let conn = self.connect()?;
        Ok(conn
            .query_row(
                "SELECT text FROM derived_texts WHERE asset_id = ?1 AND kind = 'ocr'",
                [asset_id.to_string()],
                |row| row.get(0),
            )
            .optional()
            .map_err(internal)?
            .unwrap_or_default())
    }

    pub fn get(&self, id: EntityId) -> Result<ClipboardItem, DomainError> {
        let conn = self.connect()?;
        let mut item = conn
            .query_row(
                "SELECT c.id, c.content, c.content_hash, c.source_app, c.favorite, c.use_count, c.last_used_at,
                        c.created_at, c.updated_at, c.revision, c.kind, c.asset_id,
                        a.width, a.height
                 FROM clipboard_items c
                 LEFT JOIN assets a ON a.id = c.asset_id AND a.deleted_at IS NULL
                 WHERE c.id = ?1 AND c.deleted_at IS NULL",
                [id.to_string()],
                map_item,
            )
            .optional()
            .map_err(internal)?
            .ok_or_else(|| DomainError::NotFound("剪切板条目不存在".into()))?;
        self.attach_thumb(&mut item)?;
        Ok(item)
    }

    fn attach_thumb(&self, item: &mut ClipboardItem) -> Result<(), DomainError> {
        if let Some(asset_id) = item.asset_id {
            if let Ok(asset) = self.assets.get(asset_id) {
                item.thumb_base64 = self.assets.thumb_base64(&asset)?;
                item.width = asset.width;
                item.height = asset.height;
            }
            let ocr = self.ocr_text_for_asset(asset_id)?;
            if !ocr.is_empty() {
                item.ocr_text = Some(ocr);
            }
        }
        Ok(())
    }

    pub fn query(&self, query: ClipboardQuery) -> Result<PagedResult<ClipboardItem>, DomainError> {
        let conn = self.connect()?;
        let limit = page_limit(query.limit);
        let offset = page_offset(query.offset);
        let mut filters = String::new();
        let mut values: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();
        if query.favorites_only.unwrap_or(false) {
            filters.push_str(" AND c.favorite = 1");
        }
        if let Some(kind) = query.kind {
            filters.push_str(" AND c.kind = ?");
            values.push(Box::new(kind.as_str().to_string()));
        }
        if let Some(search) = query.search.as_ref().map(|s| s.trim().to_string()) {
            if !search.is_empty() {
                filters.push_str(
                    " AND (c.content LIKE ? ESCAPE '\\'
                      OR EXISTS (
                        SELECT 1 FROM derived_texts d
                        WHERE d.asset_id = c.asset_id AND d.kind = 'ocr' AND d.text LIKE ? ESCAPE '\\'
                      ))",
                );
                let pattern = format!("%{}%", escape_like(&search));
                values.push(Box::new(pattern.clone()));
                values.push(Box::new(pattern));
            }
        }
        if let Some(ref app) = query
            .source_app
            .as_ref()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
        {
            filters.push_str(" AND c.source_app = ?");
            values.push(Box::new(app.clone()));
        }
        if let Some(ref from) = query.date_from {
            filters.push_str(" AND date(c.created_at) >= date(?)");
            values.push(Box::new(from.clone()));
        }
        if let Some(ref to) = query.date_to {
            filters.push_str(" AND date(c.created_at) <= date(?)");
            values.push(Box::new(to.clone()));
        }

        let from_clause = " FROM clipboard_items c
             LEFT JOIN assets a ON a.id = c.asset_id AND a.deleted_at IS NULL
             WHERE c.deleted_at IS NULL";
        let count_sql = format!("SELECT COUNT(*){from_clause}{filters}");
        let params_ref: Vec<&dyn rusqlite::types::ToSql> =
            values.iter().map(|v| v.as_ref()).collect();
        let total: i64 = conn
            .query_row(&count_sql, params_ref.as_slice(), |row| row.get(0))
            .map_err(internal)?;

        let sql = format!(
            "SELECT c.id, c.content, c.content_hash, c.source_app, c.favorite, c.use_count, c.last_used_at,
                    c.created_at, c.updated_at, c.revision, c.kind, c.asset_id,
                    a.width, a.height{from_clause}{filters}
             ORDER BY c.favorite DESC, c.created_at DESC LIMIT ? OFFSET ?"
        );
        values.push(Box::new(limit));
        values.push(Box::new(offset));
        let params_ref: Vec<&dyn rusqlite::types::ToSql> =
            values.iter().map(|v| v.as_ref()).collect();
        let mut stmt = conn.prepare(&sql).map_err(internal)?;
        let rows = stmt
            .query_map(params_ref.as_slice(), map_item)
            .map_err(internal)?;
        let mut items = collect(rows)?;
        for item in &mut items {
            let _ = self.attach_thumb(item);
        }
        Ok(PagedResult::new(items, total, offset))
    }

    pub fn list_source_apps(&self) -> Result<Vec<String>, DomainError> {
        let conn = self.connect()?;
        let mut stmt = conn
            .prepare(
                "SELECT DISTINCT source_app FROM clipboard_items
                 WHERE deleted_at IS NULL AND source_app IS NOT NULL AND source_app != ''
                 ORDER BY source_app COLLATE NOCASE",
            )
            .map_err(internal)?;
        let rows = stmt
            .query_map([], |row| row.get(0))
            .map_err(internal)?;
        collect(rows)
    }

    pub fn set_favorite(&self, id: EntityId, favorite: bool) -> Result<ClipboardItem, DomainError> {
        let _ = self.get(id)?;
        let now = stamp(&self.clock);
        let conn = self.connect()?;
        conn.execute(
            "UPDATE clipboard_items SET favorite = ?1, updated_at = ?2, revision = revision + 1 WHERE id = ?3",
            params![if favorite { 1 } else { 0 }, now, id.to_string()],
        )
        .map_err(internal)?;
        self.get(id)
    }

    pub fn mark_used(&self, id: EntityId) -> Result<ClipboardItem, DomainError> {
        let _ = self.get(id)?;
        let now = stamp(&self.clock);
        let conn = self.connect()?;
        conn.execute(
            "UPDATE clipboard_items SET use_count = use_count + 1, last_used_at = ?1, updated_at = ?1, revision = revision + 1
             WHERE id = ?2",
            params![now, id.to_string()],
        )
        .map_err(internal)?;
        self.get(id)
    }

    pub fn image_copy_payload(&self, id: EntityId) -> Result<ImageCopyPayload, DomainError> {
        let item = self.get(id)?;
        let asset_id = item
            .asset_id
            .ok_or_else(|| DomainError::Validation("不是图片条目".into()))?;
        let asset = self.assets.get(asset_id)?;
        let png = self.assets.read_bytes(&asset)?;
        let dyn_img = image::load_from_memory(&png)
            .map_err(|e| DomainError::Internal(format!("decode image: {e}")))?;
        let rgba = dyn_img.to_rgba8();
        Ok(ImageCopyPayload {
            width: rgba.width(),
            height: rgba.height(),
            rgba: rgba.into_raw(),
        })
    }

    pub fn delete(&self, id: EntityId) -> Result<(), DomainError> {
        let _ = self.get(id)?;
        let now = stamp(&self.clock);
        let conn = self.connect()?;
        conn.execute(
            "UPDATE clipboard_items SET deleted_at = ?1, updated_at = ?1, revision = revision + 1,
                    asset_id = NULL WHERE id = ?2",
            params![now, id.to_string()],
        )
        .map_err(internal)?;
        self.search.remove(SearchEntityType::Clipboard, id)?;
        Ok(())
    }

    pub fn clear_non_favorites(&self) -> Result<u64, DomainError> {
        let now = stamp(&self.clock);
        let conn = self.connect()?;
        let ids: Vec<String> = {
            let mut stmt = conn
                .prepare("SELECT id FROM clipboard_items WHERE deleted_at IS NULL AND favorite = 0")
                .map_err(internal)?;
            let rows = stmt.query_map([], |row| row.get(0)).map_err(internal)?;
            collect(rows)?
        };
        let count = ids.len() as u64;
        conn.execute(
            "UPDATE clipboard_items SET deleted_at = ?1, updated_at = ?1, revision = revision + 1,
                    asset_id = NULL
             WHERE deleted_at IS NULL AND favorite = 0",
            params![now],
        )
        .map_err(internal)?;
        for id in ids {
            if let Ok(entity_id) = id.parse::<EntityId>() {
                let _ = self.search.remove(SearchEntityType::Clipboard, entity_id);
            }
        }
        Ok(count)
    }

    pub fn convert_to_task(&self, id: EntityId) -> Result<EntityId, DomainError> {
        let item = self.get(id)?;
        let notes = if item.kind == ClipboardKind::Image {
            if let Some(asset_id) = item.asset_id {
                let ocr = self.ocr_text_for_asset(asset_id)?;
                if ocr.is_empty() {
                    item.content.clone()
                } else {
                    ocr
                }
            } else {
                item.content.clone()
            }
        } else {
            item.content.clone()
        };
        let title = notes
            .lines()
            .next()
            .unwrap_or("剪切板任务")
            .chars()
            .take(80)
            .collect::<String>();
        let task = self.tasks.create_task(CreateTaskInput {
            title,
            notes: Some(notes),
            priority: None,
            list_id: None,
            due_date: None,
            due_time: None,
            tag_names: None,
        })?;
        self.search
            .upsert(SearchEntityType::Task, task.id, &task.title, &task.notes)?;
        Ok(task.id)
    }

    pub fn convert_to_memory(&self, id: EntityId) -> Result<EntityId, DomainError> {
        let item = self.get(id)?;
        let body = if item.kind == ClipboardKind::Image {
            if let Some(asset_id) = item.asset_id {
                let ocr = self.ocr_text_for_asset(asset_id)?;
                if ocr.is_empty() {
                    item.content.clone()
                } else {
                    format!("[来自图片识别]\n{ocr}")
                }
            } else {
                item.content.clone()
            }
        } else {
            item.content.clone()
        };
        let title = body
            .lines()
            .find(|l| !l.starts_with('['))
            .unwrap_or("剪切板记忆")
            .chars()
            .take(80)
            .collect::<String>();
        let memory = self.memories.create(CreateMemoryInput {
            title,
            body: Some(body),
            pinned: None,
            quick_insert: None,
            trigger_word: None,
            tag_names: None,
        })?;

        if let Some(asset_id) = item.asset_id {
            self.links
                .link("memory", memory.id, "asset", asset_id, "attachment")?;
        }
        Ok(memory.id)
    }

    fn is_asset_linked(&self, asset_id: &str) -> Result<bool, DomainError> {
        asset_id
            .parse()
            .map_err(|e| DomainError::Internal(format!("{e}")))
            .and_then(|id: EntityId| self.links.is_referenced("asset", id))
    }

    pub fn enforce_limits(&self) -> Result<(), DomainError> {
        let settings = self.settings.get()?;
        let now = stamp(&self.clock);
        let conn = self.connect()?;

        let cutoff = (chrono::Local::now()
            - chrono::Duration::days(settings.clipboard_retention_days as i64))
        .format("%Y-%m-%dT%H:%M:%S")
        .to_string();

        let candidates: Vec<(String, Option<String>)> = {
            let mut stmt = conn
                .prepare(
                    "SELECT id, asset_id FROM clipboard_items
                     WHERE deleted_at IS NULL AND favorite = 0 AND created_at < ?1",
                )
                .map_err(internal)?;
            let rows = stmt
                .query_map([&cutoff], |row| Ok((row.get(0)?, row.get(1)?)))
                .map_err(internal)?;
            collect(rows)?
        };
        for (id, asset_id) in candidates {
            if let Some(ref aid) = asset_id {
                if self.is_asset_linked(aid)? {
                    continue;
                }
            }
            conn.execute(
                "UPDATE clipboard_items SET deleted_at = ?1, updated_at = ?1, revision = revision + 1,
                        asset_id = NULL WHERE id = ?2",
                params![now, id],
            )
            .map_err(internal)?;
            if let Ok(entity_id) = id.parse::<EntityId>() {
                let _ = self.search.remove(SearchEntityType::Clipboard, entity_id);
            }
        }

        let total: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM clipboard_items WHERE deleted_at IS NULL",
                [],
                |row| row.get(0),
            )
            .map_err(internal)?;
        let max = settings.clipboard_max_items as i64;
        if total > max {
            let overflow = total - max;
            let old_rows: Vec<(String, Option<String>)> = {
                let mut stmt = conn
                    .prepare(
                        "SELECT id, asset_id FROM clipboard_items
                         WHERE deleted_at IS NULL AND favorite = 0
                         ORDER BY created_at ASC
                         LIMIT ?1",
                    )
                    .map_err(internal)?;
                let rows = stmt
                    .query_map([overflow * 2], |row| Ok((row.get(0)?, row.get(1)?)))
                    .map_err(internal)?;
                collect(rows)?
            };
            let mut removed = 0i64;
            for (id, asset_id) in old_rows {
                if removed >= overflow {
                    break;
                }
                if let Some(ref aid) = asset_id {
                    if self.is_asset_linked(aid)? {
                        continue;
                    }
                }
                conn.execute(
                    "UPDATE clipboard_items SET deleted_at = ?1, updated_at = ?1, revision = revision + 1,
                            asset_id = NULL WHERE id = ?2",
                    params![now, id],
                )
                .map_err(internal)?;
                if let Ok(entity_id) = id.parse::<EntityId>() {
                    let _ = self.search.remove(SearchEntityType::Clipboard, entity_id);
                }
                removed += 1;
            }
        }
        let _ = self
            .assets
            .collect_garbage(settings.clipboard_retention_days);
        Ok(())
    }
}

fn map_item(row: &rusqlite::Row<'_>) -> Result<ClipboardItem, rusqlite::Error> {
    let kind = ClipboardKind::parse(&row.get::<_, String>(10)?).map_err(|e| {
        rusqlite::Error::FromSqlConversionFailure(10, rusqlite::types::Type::Text, Box::new(e))
    })?;
    let asset_id = row
        .get::<_, Option<String>>(11)?
        .map(|s| {
            s.parse().map_err(|e| {
                rusqlite::Error::FromSqlConversionFailure(
                    11,
                    rusqlite::types::Type::Text,
                    Box::new(e),
                )
            })
        })
        .transpose()?;
    Ok(ClipboardItem {
        id: row.get::<_, String>(0)?.parse().map_err(|e| {
            rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(e))
        })?,
        content: row.get(1)?,
        content_hash: row.get(2)?,
        source_app: row.get(3)?,
        favorite: row.get::<_, i64>(4)? == 1,
        use_count: row.get(5)?,
        last_used_at: row.get(6)?,
        created_at: row.get(7)?,
        updated_at: row.get(8)?,
        revision: row.get(9)?,
        kind,
        asset_id,
        width: row.get(12)?,
        height: row.get(13)?,
        thumb_base64: None,
        ocr_text: None,
    })
}

fn internal<E: std::fmt::Display>(err: E) -> DomainError {
    DomainError::Internal(err.to_string())
}

fn escape_like(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_")
}

fn collect<T, E>(rows: impl IntoIterator<Item = Result<T, E>>) -> Result<Vec<T>, DomainError>
where
    E: std::fmt::Display,
{
    let mut out = Vec::new();
    for row in rows {
        out.push(row.map_err(internal)?);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::tasks::TaskService;
    use tempfile::tempdir;

    fn svc(dir: &std::path::Path) -> ClipboardService {
        let db = Database::open(dir.join("c.db")).unwrap();
        TaskService::new(db.clone()).ensure_seed_data().unwrap();
        ClipboardService::new(db, dir.join("assets"))
    }

    #[test]
    fn adjacent_dedupe_and_favorite_survives_clear() {
        let dir = tempdir().unwrap();
        let svc = svc(dir.path());
        let a = svc.capture_text("hello".into(), None).unwrap().unwrap();
        let b = svc.capture_text("hello".into(), None).unwrap().unwrap();
        assert_eq!(a.id, b.id);
        svc.set_favorite(a.id, true).unwrap();
        svc.capture_text("other".into(), None).unwrap();
        let cleared = svc.clear_non_favorites().unwrap();
        assert!(cleared >= 1);
        assert!(svc.get(a.id).is_ok());
    }

    #[test]
    fn image_dedupe_and_linked_survives_expire() {
        let dir = tempdir().unwrap();
        let svc = svc(dir.path());
        let rgba = vec![
            10u8, 20, 30, 255, 40, 50, 60, 255, 70, 80, 90, 255, 100, 110, 120, 255,
        ];
        let a = svc.capture_image(2, 2, &rgba, None).unwrap().unwrap();
        let b = svc.capture_image(2, 2, &rgba, None).unwrap().unwrap();
        assert_eq!(a.id, b.id);
        assert_eq!(a.kind, ClipboardKind::Image);
        assert!(a.asset_id.is_some());

        // Manually set OCR text and convert to memory (creates entity_link).
        svc.upsert_derived_text(a.asset_id.unwrap(), "发票金额 128", "test")
            .unwrap();
        let memory_id = svc.convert_to_memory(a.id).unwrap();
        assert!(!memory_id.to_string().is_empty());

        // Force expire path: set created_at old then enforce.
        {
            let conn = svc.connect().unwrap();
            conn.execute(
                "UPDATE clipboard_items SET created_at = '2000-01-01T00:00:00', favorite = 0 WHERE id = ?1",
                [a.id.to_string()],
            )
            .unwrap();
        }
        svc.enforce_limits().unwrap();
        // Linked image clipboard row should still exist.
        assert!(svc.get(a.id).is_ok());
        // The attachment link is present and protects the asset.
        assert!(svc
            .links
            .is_referenced("asset", a.asset_id.unwrap())
            .unwrap());
    }

    #[test]
    fn soft_deleted_unlinked_image_is_reclaimed_by_gc() {
        let dir = tempdir().unwrap();
        let svc = svc(dir.path());
        let rgba = vec![
            200u8, 30, 30, 255, 40, 250, 60, 255, 70, 80, 190, 255, 100, 110, 120, 255,
        ];
        let a = svc.capture_image(2, 2, &rgba, None).unwrap().unwrap();
        let asset_id = a.asset_id.unwrap();
        let asset = svc.assets().get(asset_id).unwrap();
        let file = svc.assets().absolute_path(&asset);
        assert!(file.exists());

        // Soft delete the clipboard row; asset_id is detached.
        svc.delete(a.id).unwrap();
        {
            let conn = svc.connect().unwrap();
            let asset_id_str = asset_id.to_string();
            conn.execute(
                "UPDATE assets SET created_at = '2000-01-01T00:00:00' WHERE id = ?1",
                [asset_id_str],
            )
            .unwrap();
            let retained: bool = conn
                .query_row(
                    "SELECT asset_id IS NOT NULL FROM clipboard_items WHERE id = ?1",
                    [a.id.to_string()],
                    |row| row.get(0),
                )
                .unwrap();
            assert!(!retained);
        }

        svc.enforce_limits().unwrap();

        assert!(svc.assets().get(asset_id).is_err());
        assert!(!file.exists());
    }
}
