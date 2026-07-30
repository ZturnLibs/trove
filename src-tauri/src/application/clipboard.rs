use crate::application::memories::MemoryService;
use crate::application::search::SearchService;
use crate::application::tasks::TaskService;
use crate::domain::{
    new_id, stamp, ClipboardItem, ClipboardQuery, CreateMemoryInput, CreateTaskInput, DomainError,
    EntityId, SearchEntityType, SystemClock,
};
use crate::infrastructure::db::Database;
use crate::infrastructure::settings::SettingsService;
use rusqlite::{params, Connection, OptionalExtension};
use sha2::{Digest, Sha256};
use std::sync::Mutex;
use std::time::{Duration, Instant};

pub struct ClipboardService {
    db: Database,
    clock: SystemClock,
    search: SearchService,
    tasks: TaskService,
    memories: MemoryService,
    settings: SettingsService,
    suppress_until: Mutex<Option<(String, Instant)>>,
}

impl ClipboardService {
    pub fn new(db: Database) -> Self {
        Self {
            search: SearchService::new(db.clone()),
            tasks: TaskService::new(db.clone()),
            memories: MemoryService::new(db.clone()),
            settings: SettingsService::new(db.clone()),
            db,
            clock: SystemClock,
            suppress_until: Mutex::new(None),
        }
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

    pub fn suppress_next(&self, content: &str) {
        if let Ok(mut guard) = self.suppress_until.lock() {
            *guard = Some((Self::hash_content(content), Instant::now() + Duration::from_secs(2)));
        }
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
        // Hard cap to avoid huge pastes bloating DB.
        if content.len() > 200_000 {
            return Ok(None);
        }
        if let Some(ref app) = source_app {
            if settings
                .clipboard_excluded_apps
                .iter()
                .any(|excluded| app.eq_ignore_ascii_case(excluded) || app.contains(excluded))
            {
                return Ok(None);
            }
        }

        let hash = Self::hash_content(&content);
        if self.is_suppressed(&hash) {
            return Ok(None);
        }

        let now = stamp(&self.clock);
        let conn = self.connect()?;

        // Adjacent duplicate: update latest matching active row instead of insert.
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
        let title = content.lines().next().unwrap_or("剪切板").chars().take(80).collect::<String>();
        conn.execute(
            "INSERT INTO clipboard_items (
                id, content, content_hash, source_app, favorite, use_count, last_used_at,
                created_at, updated_at, revision, deleted_at
             ) VALUES (?1, ?2, ?3, ?4, 0, 0, NULL, ?5, ?5, 1, NULL)",
            params![
                id.to_string(),
                content,
                hash,
                source_app,
                now
            ],
        )
        .map_err(internal)?;
        self.search
            .upsert(SearchEntityType::Clipboard, id, &title, &content)?;
        self.enforce_limits()?;
        self.get(id).map(Some)
    }

    pub fn get(&self, id: EntityId) -> Result<ClipboardItem, DomainError> {
        let conn = self.connect()?;
        conn.query_row(
            "SELECT id, content, content_hash, source_app, favorite, use_count, last_used_at,
                    created_at, updated_at, revision
             FROM clipboard_items WHERE id = ?1 AND deleted_at IS NULL",
            [id.to_string()],
            map_item,
        )
        .optional()
        .map_err(internal)?
        .ok_or_else(|| DomainError::NotFound("剪切板条目不存在".into()))
    }

    pub fn query(&self, query: ClipboardQuery) -> Result<Vec<ClipboardItem>, DomainError> {
        let conn = self.connect()?;
        let limit = query.limit.unwrap_or(200).clamp(1, 1000);
        let mut sql = String::from(
            "SELECT id, content, content_hash, source_app, favorite, use_count, last_used_at,
                    created_at, updated_at, revision
             FROM clipboard_items WHERE deleted_at IS NULL",
        );
        let mut values: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();
        if query.favorites_only.unwrap_or(false) {
            sql.push_str(" AND favorite = 1");
        }
        if let Some(search) = query.search.as_ref().map(|s| s.trim().to_string()) {
            if !search.is_empty() {
                sql.push_str(" AND content LIKE ? ESCAPE '\\'");
                let pattern = format!(
                    "%{}%",
                    search
                        .replace('\\', "\\\\")
                        .replace('%', "\\%")
                        .replace('_', "\\_")
                );
                values.push(Box::new(pattern));
            }
        }
        sql.push_str(" ORDER BY favorite DESC, created_at DESC LIMIT ?");
        values.push(Box::new(limit));
        let params_ref: Vec<&dyn rusqlite::types::ToSql> =
            values.iter().map(|v| v.as_ref()).collect();
        let mut stmt = conn.prepare(&sql).map_err(internal)?;
        let rows = stmt
            .query_map(params_ref.as_slice(), map_item)
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

    pub fn delete(&self, id: EntityId) -> Result<(), DomainError> {
        let _ = self.get(id)?;
        let now = stamp(&self.clock);
        let conn = self.connect()?;
        conn.execute(
            "UPDATE clipboard_items SET deleted_at = ?1, updated_at = ?1, revision = revision + 1 WHERE id = ?2",
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
                .prepare(
                    "SELECT id FROM clipboard_items WHERE deleted_at IS NULL AND favorite = 0",
                )
                .map_err(internal)?;
            let rows = stmt
                .query_map([], |row| row.get(0))
                .map_err(internal)?;
            collect(rows)?
        };
        let count = ids.len() as u64;
        conn.execute(
            "UPDATE clipboard_items SET deleted_at = ?1, updated_at = ?1, revision = revision + 1
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
        let title = item
            .content
            .lines()
            .next()
            .unwrap_or("剪切板任务")
            .chars()
            .take(80)
            .collect::<String>();
        let task = self.tasks.create_task(CreateTaskInput {
            title,
            notes: Some(item.content.clone()),
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
        let title = item
            .content
            .lines()
            .next()
            .unwrap_or("剪切板记忆")
            .chars()
            .take(80)
            .collect::<String>();
        let memory = self.memories.create(CreateMemoryInput {
            title,
            body: Some(item.content),
            pinned: None,
            quick_insert: None,
            trigger_word: None,
            tag_names: None,
        })?;
        Ok(memory.id)
    }

    pub fn enforce_limits(&self) -> Result<(), DomainError> {
        let settings = self.settings.get()?;
        let now = stamp(&self.clock);
        let conn = self.connect()?;

        // Expire by age (non-favorites).
        let cutoff = (chrono::Local::now()
            - chrono::Duration::days(settings.clipboard_retention_days as i64))
        .format("%Y-%m-%dT%H:%M:%S")
        .to_string();
        let expired_ids: Vec<String> = {
            let mut stmt = conn
                .prepare(
                    "SELECT id FROM clipboard_items
                     WHERE deleted_at IS NULL AND favorite = 0 AND created_at < ?1",
                )
                .map_err(internal)?;
            let rows = stmt
                .query_map([&cutoff], |row| row.get(0))
                .map_err(internal)?;
            collect(rows)?
        };
        if !expired_ids.is_empty() {
            conn.execute(
                "UPDATE clipboard_items SET deleted_at = ?1, updated_at = ?1, revision = revision + 1
                 WHERE deleted_at IS NULL AND favorite = 0 AND created_at < ?2",
                params![now, cutoff],
            )
            .map_err(internal)?;
            for id in expired_ids {
                if let Ok(entity_id) = id.parse::<EntityId>() {
                    let _ = self.search.remove(SearchEntityType::Clipboard, entity_id);
                }
            }
        }

        // Cap max items (delete oldest non-favorites first).
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
            let old_ids: Vec<String> = {
                let mut stmt = conn
                    .prepare(
                        "SELECT id FROM clipboard_items
                         WHERE deleted_at IS NULL AND favorite = 0
                         ORDER BY created_at ASC
                         LIMIT ?1",
                    )
                    .map_err(internal)?;
                let rows = stmt
                    .query_map([overflow], |row| row.get(0))
                    .map_err(internal)?;
                collect(rows)?
            };
            for id in &old_ids {
                conn.execute(
                    "UPDATE clipboard_items SET deleted_at = ?1, updated_at = ?1, revision = revision + 1 WHERE id = ?2",
                    params![now, id],
                )
                .map_err(internal)?;
                if let Ok(entity_id) = id.parse::<EntityId>() {
                    let _ = self.search.remove(SearchEntityType::Clipboard, entity_id);
                }
            }
        }
        Ok(())
    }
}

fn map_item(row: &rusqlite::Row<'_>) -> Result<ClipboardItem, rusqlite::Error> {
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
    })
}

fn internal<E: std::fmt::Display>(err: E) -> DomainError {
    DomainError::Internal(err.to_string())
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

    #[test]
    fn adjacent_dedupe_and_favorite_survives_clear() {
        let dir = tempdir().unwrap();
        let db = Database::open(dir.path().join("c.db")).unwrap();
        TaskService::new(db.clone()).ensure_seed_data().unwrap();
        let svc = ClipboardService::new(db);
        let a = svc
            .capture_text("hello".into(), None)
            .unwrap()
            .unwrap();
        let b = svc
            .capture_text("hello".into(), None)
            .unwrap()
            .unwrap();
        assert_eq!(a.id, b.id);
        svc.set_favorite(a.id, true).unwrap();
        svc.capture_text("other".into(), None).unwrap();
        let cleared = svc.clear_non_favorites().unwrap();
        assert!(cleared >= 1);
        assert!(svc.get(a.id).is_ok());
    }
}
