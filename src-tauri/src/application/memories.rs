use crate::application::links::EntityLinkService;
use crate::application::search::SearchService;
use crate::application::tasks::TaskService;
use crate::domain::{
    new_id, parse_wikilink_titles, stamp, ConvertMemoryToTaskResult, CreateMemoryInput,
    CreateTaskInput, DomainError, EntityId, Memory, MemoryBacklink, MemoryQuery, MemorySummary,
    PagedResult, RelatedMemoryHit, SearchEntityType, SystemClock, UpdateMemoryInput,
    WikilinkPending, WikilinkPendingReason, WikilinkResolution, WikilinkResolutionAction,
    WikilinkSyncResult, LINK_KIND_MENTION,
};
use crate::domain::{page_limit, page_offset};
use std::collections::{HashMap, HashSet};
use crate::infrastructure::db::Database;
use rusqlite::{params, Connection, OptionalExtension};

pub struct MemoryService {
    db: Database,
    clock: SystemClock,
    search: SearchService,
    tasks: TaskService,
    links: EntityLinkService,
}

impl MemoryService {
    pub fn new(db: Database) -> Self {
        Self {
            search: SearchService::new(db.clone()),
            tasks: TaskService::new(db.clone()),
            links: EntityLinkService::new(db.clone()),
            db,
            clock: SystemClock,
        }
    }

    fn connect(&self) -> Result<Connection, DomainError> {
        self.db.connect().map_err(internal)
    }

    pub fn create(&self, input: CreateMemoryInput) -> Result<Memory, DomainError> {
        let title = input.title.trim().to_string();
        if title.is_empty() {
            return Err(DomainError::Validation("标题不能为空".into()));
        }
        let body = input.body.unwrap_or_default();
        let id = new_id();
        let now = stamp(&self.clock);
        let pinned = input.pinned.unwrap_or(false);
        let quick_insert = input.quick_insert.unwrap_or(false);
        let trigger_word = input
            .trigger_word
            .as_ref()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());
        let conn = self.connect()?;
        let tx = conn.unchecked_transaction().map_err(internal)?;
        tx.execute(
            "INSERT INTO memories (
                id, title, body, pinned, archived, quick_insert, trigger_word,
                created_at, updated_at, revision, deleted_at
             ) VALUES (?1, ?2, ?3, ?4, 0, ?5, ?6, ?7, ?7, 1, NULL)",
            params![
                id.to_string(),
                title,
                body,
                if pinned { 1 } else { 0 },
                if quick_insert { 1 } else { 0 },
                trigger_word,
                now
            ],
        )
        .map_err(internal)?;
        if let Some(tag_names) = input.tag_names {
            self.replace_tags(&tx, id, &tag_names)?;
        }
        let searchable = match &trigger_word {
            Some(word) => format!("{}\n{}", body, word),
            None => body.clone(),
        };
        self.search
            .upsert_conn(&tx, SearchEntityType::Memory, id, &title, &searchable)?;
        tx.commit().map_err(internal)?;
        let _ = self.sync_wikilinks(id, None)?;
        self.get(id)
    }

    pub fn update(&self, input: UpdateMemoryInput) -> Result<Memory, DomainError> {
        let _ = self.get(input.id)?;
        let title = input.title.trim().to_string();
        if title.is_empty() {
            return Err(DomainError::Validation("标题不能为空".into()));
        }
        let now = stamp(&self.clock);
        let conn = self.connect()?;
        let tx = conn.unchecked_transaction().map_err(internal)?;
        tx.execute(
            "UPDATE memories SET title = ?1, body = ?2, pinned = ?3, archived = ?4,
                quick_insert = ?5, trigger_word = ?6, sensitive = ?7,
                updated_at = ?8, revision = revision + 1
             WHERE id = ?9 AND deleted_at IS NULL",
            params![
                title,
                input.body,
                if input.pinned { 1 } else { 0 },
                if input.archived { 1 } else { 0 },
                if input.quick_insert { 1 } else { 0 },
                input
                    .trigger_word
                    .as_ref()
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty()),
                if input.sensitive { 1 } else { 0 },
                now,
                input.id.to_string()
            ],
        )
        .map_err(internal)?;
        self.replace_tags(&tx, input.id, &input.tag_names)?;
        if !input.archived {
            let searchable = match input.trigger_word.as_deref() {
                Some(word) if !word.trim().is_empty() => {
                    format!("{}\n{}", input.body, word.trim())
                }
                _ => input.body.clone(),
            };
            self.search.upsert_conn(
                &tx,
                SearchEntityType::Memory,
                input.id,
                &title,
                &searchable,
            )?;
        }
        tx.commit().map_err(internal)?;
        if input.archived {
            self.search.remove(SearchEntityType::Memory, input.id)?;
        }
        let _ = self.sync_wikilinks(input.id, None)?;
        self.get(input.id)
    }

    pub fn sync_wikilinks(
        &self,
        memory_id: EntityId,
        resolutions: Option<Vec<WikilinkResolution>>,
    ) -> Result<WikilinkSyncResult, DomainError> {
        let memory = self.get(memory_id)?;
        let titles = parse_wikilink_titles(&memory.body);
        let resolution_map = resolutions
            .unwrap_or_default()
            .into_iter()
            .map(|r| (r.title.to_ascii_lowercase(), r))
            .collect::<HashMap<_, _>>();

        let old_targets: HashSet<EntityId> = self
            .links
            .list_outgoing("memory", memory_id)?
            .into_iter()
            .filter(|l| l.link_kind == LINK_KIND_MENTION && l.target_type == "memory")
            .map(|l| l.target_id)
            .collect();

        let conn = self.connect()?;
        let mut new_targets = HashSet::new();
        let mut linked_ids = Vec::new();
        let mut pending = Vec::new();

        for title in titles {
            if let Some(resolution) = resolution_map.get(&title.to_ascii_lowercase()) {
                match resolution.action {
                    WikilinkResolutionAction::Skip => continue,
                    WikilinkResolutionAction::Create => {
                        let created = self.create(CreateMemoryInput {
                            title: title.clone(),
                            body: Some(String::new()),
                            pinned: None,
                            quick_insert: None,
                            trigger_word: None,
                            tag_names: None,
                        })?;
                        if created.id != memory_id {
                            new_targets.insert(created.id);
                            linked_ids.push(created.id);
                        }
                        continue;
                    }
                    WikilinkResolutionAction::Link => {
                        if let Some(id) = resolution.target_id {
                            if id != memory_id {
                                new_targets.insert(id);
                                linked_ids.push(id);
                            }
                        }
                        continue;
                    }
                }
            }

            let candidates = self.find_by_title(&conn, &title, Some(memory_id))?;
            match candidates.len() {
                0 => pending.push(WikilinkPending {
                    title: title.clone(),
                    reason: WikilinkPendingReason::Missing,
                    candidates: Vec::new(),
                }),
                1 => {
                    new_targets.insert(candidates[0].id);
                    linked_ids.push(candidates[0].id);
                }
                _ => pending.push(WikilinkPending {
                    title: title.clone(),
                    reason: WikilinkPendingReason::Ambiguous,
                    candidates,
                }),
            }
        }

        for removed in old_targets.difference(&new_targets) {
            self.bump_mention_use_count(&conn, *removed, -1)?;
        }
        for added in new_targets.difference(&old_targets) {
            self.bump_mention_use_count(&conn, *added, 1)?;
        }

        conn.execute(
            "DELETE FROM entity_links
             WHERE source_type = 'memory' AND source_id = ?1 AND link_kind = ?2",
            params![memory_id.to_string(), LINK_KIND_MENTION],
        )
        .map_err(internal)?;

        for target_id in &new_targets {
            self.links.link(
                "memory",
                memory_id,
                "memory",
                *target_id,
                LINK_KIND_MENTION,
            )?;
        }

        Ok(WikilinkSyncResult {
            memory: self.get(memory_id)?,
            linked_ids,
            pending,
        })
    }

    pub fn wikilink_pending(&self, memory_id: EntityId) -> Result<Vec<WikilinkPending>, DomainError> {
        let memory = self.get(memory_id)?;
        let titles = parse_wikilink_titles(&memory.body);
        let conn = self.connect()?;
        let mut pending = Vec::new();
        for title in titles {
            let candidates = self.find_by_title(&conn, &title, Some(memory_id))?;
            match candidates.len() {
                0 => pending.push(WikilinkPending {
                    title: title.clone(),
                    reason: WikilinkPendingReason::Missing,
                    candidates: Vec::new(),
                }),
                1 => {}
                _ => pending.push(WikilinkPending {
                    title,
                    reason: WikilinkPendingReason::Ambiguous,
                    candidates,
                }),
            }
        }
        Ok(pending)
    }

    pub fn resolve_wikilinks(
        &self,
        memory_id: EntityId,
        resolutions: Vec<WikilinkResolution>,
    ) -> Result<WikilinkSyncResult, DomainError> {
        self.sync_wikilinks(memory_id, Some(resolutions))
    }

    pub fn backlinks(&self, memory_id: EntityId) -> Result<Vec<MemoryBacklink>, DomainError> {
        let _ = self.get(memory_id)?;
        let links = self.links.list_incoming("memory", memory_id)?;
        let conn = self.connect()?;
        let mut out = Vec::new();
        for link in links {
            if link.link_kind != LINK_KIND_MENTION || link.source_type != "memory" {
                continue;
            }
            let title: Option<String> = conn
                .query_row(
                    "SELECT title FROM memories WHERE id = ?1 AND deleted_at IS NULL",
                    [link.source_id.to_string()],
                    |row| row.get(0),
                )
                .optional()
                .map_err(internal)?;
            if let Some(title) = title {
                out.push(MemoryBacklink {
                    memory_id: link.source_id,
                    title,
                });
            }
        }
        out.sort_by(|a, b| a.title.cmp(&b.title));
        Ok(out)
    }

    pub fn related_memories(&self, memory_id: EntityId) -> Result<Vec<RelatedMemoryHit>, DomainError> {
        let memory = self.get(memory_id)?;
        let conn = self.connect()?;
        let existing_mentions: HashSet<EntityId> = self
            .links
            .list_outgoing("memory", memory_id)?
            .into_iter()
            .filter(|l| l.link_kind == LINK_KIND_MENTION && l.target_type == "memory")
            .map(|l| l.target_id)
            .collect();

        let mut stmt = conn
            .prepare(
                "SELECT m.id, m.title, m.body, GROUP_CONCAT(tg.name) AS tags
                 FROM memories m
                 LEFT JOIN memory_tags mt ON mt.memory_id = m.id
                 LEFT JOIN tags tg ON tg.id = mt.tag_id AND tg.deleted_at IS NULL
                 WHERE m.deleted_at IS NULL AND m.archived = 0 AND m.id != ?1
                 GROUP BY m.id
                 ORDER BY m.updated_at DESC
                 LIMIT 120",
            )
            .map_err(internal)?;
        let rows = stmt
            .query_map([memory_id.to_string()], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, Option<String>>(3)?,
                ))
            })
            .map_err(internal)?;

        let source_tags: HashSet<String> = memory.tag_names.iter().cloned().collect();
        let source_tokens = tokenize(&format!("{} {}", memory.title, memory.body));
        let mut hits = Vec::new();

        for row in rows {
            let (id, title, body, tags_csv) = row.map_err(internal)?;
            let target_id: EntityId = id.parse().map_err(internal)?;
            if existing_mentions.contains(&target_id) {
                continue;
            }
            let mut score = 0.0f64;
            let mut reasons = Vec::new();
            let target_tags: HashSet<String> = tags_csv
                .unwrap_or_default()
                .split(',')
                .filter(|s| !s.is_empty())
                .map(str::to_string)
                .collect();
            let shared: Vec<String> = source_tags.intersection(&target_tags).cloned().collect();
            if !shared.is_empty() {
                score += 0.35 * shared.len() as f64;
                reasons.push(format!("共同标签：{}", shared.join("、")));
            }
            let linked = self
                .links
                .list_for_entity("memory", memory_id)?
                .iter()
                .any(|l| {
                    (l.source_id == target_id || l.target_id == target_id)
                        && l.source_type == "memory"
                        && l.target_type == "memory"
                });
            if linked {
                score += 0.25;
                reasons.push("已有其他关联".into());
            }
            let kw = jaccard_similarity(&source_tokens, &tokenize(&format!("{title} {body}")));
            if kw >= 0.2 {
                score += kw * 0.5;
                reasons.push("正文关键词重合".into());
            }
            if score >= 0.25 {
                hits.push(RelatedMemoryHit {
                    memory_id: target_id,
                    title,
                    score,
                    reasons,
                });
            }
        }
        hits.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        hits.truncate(5);
        Ok(hits)
    }

    pub fn link_mention(&self, source_id: EntityId, target_id: EntityId) -> Result<(), DomainError> {
        if source_id == target_id {
            return Err(DomainError::Validation("不能关联自身".into()));
        }
        let _ = self.get(source_id)?;
        let _ = self.get(target_id)?;
        let existing: HashSet<EntityId> = self
            .links
            .list_outgoing("memory", source_id)?
            .into_iter()
            .filter(|l| l.link_kind == LINK_KIND_MENTION)
            .map(|l| l.target_id)
            .collect();
        if !existing.contains(&target_id) {
            let conn = self.connect()?;
            self.bump_mention_use_count(&conn, target_id, 1)?;
            self.links.link(
                "memory",
                source_id,
                "memory",
                target_id,
                LINK_KIND_MENTION,
            )?;
        }
        Ok(())
    }

    fn find_by_title(
        &self,
        conn: &Connection,
        title: &str,
        exclude: Option<EntityId>,
    ) -> Result<Vec<MemorySummary>, DomainError> {
        let mut sql = String::from(
            "SELECT id, title FROM memories
             WHERE deleted_at IS NULL AND archived = 0 AND title = ?1 COLLATE NOCASE",
        );
        let mut params: Vec<Box<dyn rusqlite::types::ToSql>> = vec![Box::new(title.to_string())];
        if let Some(id) = exclude {
            sql.push_str(" AND id != ?2");
            params.push(Box::new(id.to_string()));
        }
        sql.push_str(" ORDER BY updated_at DESC LIMIT 5");
        let params_ref: Vec<&dyn rusqlite::types::ToSql> =
            params.iter().map(|p| p.as_ref()).collect();
        let mut stmt = conn.prepare(&sql).map_err(internal)?;
        let rows = stmt
            .query_map(params_ref.as_slice(), |row| {
                Ok(MemorySummary {
                    id: row.get::<_, String>(0)?.parse().map_err(|e| {
                        rusqlite::Error::FromSqlConversionFailure(
                            0,
                            rusqlite::types::Type::Text,
                            Box::new(e),
                        )
                    })?,
                    title: row.get(1)?,
                })
            })
            .map_err(internal)?;
        collect(rows)
    }

    fn bump_mention_use_count(
        &self,
        conn: &Connection,
        memory_id: EntityId,
        delta: i64,
    ) -> Result<(), DomainError> {
        conn.execute(
            "UPDATE memories SET mention_use_count = MAX(0, mention_use_count + ?1)
             WHERE id = ?2 AND deleted_at IS NULL",
            params![delta, memory_id.to_string()],
        )
        .map_err(internal)?;
        Ok(())
    }

    pub fn get(&self, id: EntityId) -> Result<Memory, DomainError> {
        let conn = self.connect()?;
        let mut memory = conn
            .query_row(
                "SELECT id, title, body, pinned, archived, quick_insert, trigger_word,
                        mention_use_count, sensitive, created_at, updated_at, revision
                 FROM memories WHERE id = ?1 AND deleted_at IS NULL",
                [id.to_string()],
                map_memory_row,
            )
            .optional()
            .map_err(internal)?
            .ok_or_else(|| DomainError::NotFound("记忆不存在".into()))?;
        self.attach_tags(&conn, &mut memory)?;
        Ok(memory)
    }

    pub fn query(&self, query: MemoryQuery) -> Result<PagedResult<Memory>, DomainError> {
        let conn = self.connect()?;
        let limit = page_limit(query.limit);
        let offset = page_offset(query.offset);
        let mut filters = String::new();
        let mut values: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();
        if !query.include_archived.unwrap_or(false) {
            filters.push_str(" AND archived = 0");
        }
        if query.pinned_only.unwrap_or(false) {
            filters.push_str(" AND pinned = 1");
        }
        if query.quick_insert_only.unwrap_or(false) {
            filters.push_str(" AND quick_insert = 1");
        }
        if let Some(tag_id) = query.tag_id {
            filters.push_str(
                " AND EXISTS (SELECT 1 FROM memory_tags mt WHERE mt.memory_id = memories.id AND mt.tag_id = ?)",
            );
            values.push(Box::new(tag_id.to_string()));
        }
        if let Some(text) = query.search.as_ref().map(|s| s.trim().to_string()) {
            if !text.is_empty() {
                filters.push_str(" AND (title LIKE ? ESCAPE '\\' OR body LIKE ? ESCAPE '\\')");
                let pattern = format!("%{}%", escape_like(&text));
                values.push(Box::new(pattern.clone()));
                values.push(Box::new(pattern));
            }
        }

        let from_clause = " FROM memories WHERE deleted_at IS NULL";
        let count_sql = format!("SELECT COUNT(*){from_clause}{filters}");
        let params_ref: Vec<&dyn rusqlite::types::ToSql> =
            values.iter().map(|v| v.as_ref()).collect();
        let total: i64 = conn
            .query_row(&count_sql, params_ref.as_slice(), |row| row.get(0))
            .map_err(internal)?;

        let order = if query.quick_insert_only.unwrap_or(false) {
            "pinned DESC, mention_use_count DESC, updated_at DESC"
        } else {
            "pinned DESC, updated_at DESC"
        };
        let sql = format!(
            "SELECT id, title, body, pinned, archived, quick_insert, trigger_word,
                    mention_use_count, sensitive, created_at, updated_at, revision{from_clause}{filters}
             ORDER BY {order} LIMIT ? OFFSET ?"
        );
        values.push(Box::new(limit));
        values.push(Box::new(offset));
        let params_ref: Vec<&dyn rusqlite::types::ToSql> =
            values.iter().map(|v| v.as_ref()).collect();
        let mut stmt = conn.prepare(&sql).map_err(internal)?;
        let rows = stmt
            .query_map(params_ref.as_slice(), map_memory_row)
            .map_err(internal)?;
        let mut memories = collect(rows)?;
        for memory in &mut memories {
            self.attach_tags(&conn, memory)?;
        }
        Ok(PagedResult::new(memories, total, offset))
    }

    pub fn delete(&self, id: EntityId) -> Result<(), DomainError> {
        let _ = self.get(id)?;
        let conn = self.connect()?;
        for link in self.links.list_outgoing("memory", id)? {
            if link.link_kind == LINK_KIND_MENTION && link.target_type == "memory" {
                self.bump_mention_use_count(&conn, link.target_id, -1)?;
            }
        }
        let now = stamp(&self.clock);
        conn.execute(
            "UPDATE memories SET deleted_at = ?1, updated_at = ?1, revision = revision + 1 WHERE id = ?2",
            params![now, id.to_string()],
        )
        .map_err(internal)?;
        self.search.remove(SearchEntityType::Memory, id)?;
        let _ = self.links.purge_for_source("memory", id);
        let _ = self.links.purge_incoming_for_target("memory", id);
        Ok(())
    }

    pub fn convert_to_task(&self, id: EntityId) -> Result<ConvertMemoryToTaskResult, DomainError> {
        let memory = self.get(id)?;
        let task = self.tasks.create_task(CreateTaskInput {
            title: memory.title.clone(),
            notes: Some(memory.body.clone()),
            priority: None,
            list_id: None,
            due_date: None,
            due_time: None,
            tag_names: Some(memory.tag_names.clone()),
        })?;
        // Index task
        self.search
            .upsert(SearchEntityType::Task, task.id, &task.title, &task.notes)?;

        // Record the conversion relationship.
        self.links
            .link("memory", id, "task", task.id, "converted_to")?;

        Ok(ConvertMemoryToTaskResult {
            memory,
            task_id: task.id,
        })
    }

    fn attach_tags(&self, conn: &Connection, memory: &mut Memory) -> Result<(), DomainError> {
        let mut stmt = conn
            .prepare(
                "SELECT tg.id, tg.name FROM tags tg
                 JOIN memory_tags mt ON mt.tag_id = tg.id
                 WHERE mt.memory_id = ?1 AND tg.deleted_at IS NULL
                 ORDER BY tg.name COLLATE NOCASE",
            )
            .map_err(internal)?;
        let rows = stmt
            .query_map([memory.id.to_string()], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })
            .map_err(internal)?;
        let mut ids = Vec::new();
        let mut names = Vec::new();
        for row in rows {
            let (id, name) = row.map_err(internal)?;
            ids.push(
                id.parse()
                    .map_err(|e| DomainError::Internal(format!("{e}")))?,
            );
            names.push(name);
        }
        memory.tag_ids = ids;
        memory.tag_names = names;
        Ok(())
    }

    fn replace_tags(
        &self,
        conn: &Connection,
        memory_id: EntityId,
        tag_names: &[String],
    ) -> Result<(), DomainError> {
        conn.execute(
            "DELETE FROM memory_tags WHERE memory_id = ?1",
            [memory_id.to_string()],
        )
        .map_err(internal)?;
        let now = stamp(&self.clock);
        for raw in tag_names {
            let name = raw.trim();
            if name.is_empty() {
                continue;
            }
            let existing: Option<String> = conn
                .query_row(
                    "SELECT id FROM tags WHERE name = ?1 COLLATE NOCASE AND deleted_at IS NULL",
                    [name],
                    |row| row.get(0),
                )
                .optional()
                .map_err(internal)?;
            let tag_id = if let Some(id) = existing {
                id
            } else {
                let id = new_id().to_string();
                conn.execute(
                    "INSERT INTO tags (id, name, created_at, updated_at, revision)
                     VALUES (?1, ?2, ?3, ?3, 1)",
                    params![id, name, now],
                )
                .map_err(internal)?;
                id
            };
            conn.execute(
                "INSERT OR IGNORE INTO memory_tags (memory_id, tag_id) VALUES (?1, ?2)",
                params![memory_id.to_string(), tag_id],
            )
            .map_err(internal)?;
        }
        Ok(())
    }
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

fn map_memory_row(row: &rusqlite::Row<'_>) -> Result<Memory, rusqlite::Error> {
    Ok(Memory {
        id: row.get::<_, String>(0)?.parse().map_err(|e| {
            rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(e))
        })?,
        title: row.get(1)?,
        body: row.get(2)?,
        pinned: row.get::<_, i64>(3)? == 1,
        archived: row.get::<_, i64>(4)? == 1,
        quick_insert: row.get::<_, i64>(5)? == 1,
        trigger_word: row.get(6)?,
        mention_use_count: row.get(7)?,
        sensitive: row.get::<_, i64>(8)? == 1,
        tag_ids: Vec::new(),
        tag_names: Vec::new(),
        created_at: row.get(9)?,
        updated_at: row.get(10)?,
        revision: row.get(11)?,
    })
}

fn tokenize(text: &str) -> HashSet<String> {
    text.to_lowercase()
        .split(|c: char| !c.is_alphanumeric() && c != '_')
        .filter(|w| w.len() >= 3)
        .map(str::to_string)
        .collect()
}

fn jaccard_similarity(a: &HashSet<String>, b: &HashSet<String>) -> f64 {
    if a.is_empty() || b.is_empty() {
        return 0.0;
    }
    let inter = a.intersection(b).count();
    let union = a.union(b).count();
    inter as f64 / union as f64
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
    use crate::domain::SearchQuery;
    use tempfile::tempdir;

    #[test]
    fn create_and_convert_keeps_memory() {
        let dir = tempdir().unwrap();
        let db = Database::open(dir.path().join("m.db")).unwrap();
        let tasks = TaskService::new(db.clone());
        tasks.ensure_seed_data().unwrap();
        let svc = MemoryService::new(db);
        let memory = svc
            .create(CreateMemoryInput {
                title: "API Key 备忘".into(),
                body: Some("https://example.com/docs".into()),
                pinned: Some(true),
                quick_insert: None,
                trigger_word: None,
                tag_names: Some(vec!["work".into()]),
            })
            .unwrap();
        assert!(memory.pinned);
        let converted = svc.convert_to_task(memory.id).unwrap();
        assert_eq!(converted.memory.id, memory.id);
        let still = svc.get(memory.id).unwrap();
        assert_eq!(still.title, "API Key 备忘");
        let task = tasks.get_task(converted.task_id).unwrap();
        assert_eq!(task.title, "API Key 备忘");

        let links = svc.links.list_outgoing("memory", memory.id).unwrap();
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].link_kind, "converted_to");
        assert_eq!(links[0].target_type, "task");
        assert_eq!(links[0].target_id, converted.task_id);
    }

    #[test]
    fn query_search_matches_title_and_body() {
        let dir = tempdir().unwrap();
        let db = Database::open(dir.path().join("m.db")).unwrap();
        let svc = MemoryService::new(db);
        let alpha = svc
            .create(CreateMemoryInput {
                title: "Alpha 备忘".into(),
                body: Some("https://example.com/docs".into()),
                pinned: None,
                quick_insert: None,
                trigger_word: None,
                tag_names: None,
            })
            .unwrap();
        let beta = svc
            .create(CreateMemoryInput {
                title: "Beta 备忘".into(),
                body: Some("包含 keyword 的正文".into()),
                pinned: None,
                quick_insert: None,
                trigger_word: None,
                tag_names: None,
            })
            .unwrap();
        let _gamma = svc
            .create(CreateMemoryInput {
                title: "Gamma".into(),
                body: Some("无关内容".into()),
                pinned: None,
                quick_insert: None,
                trigger_word: None,
                tag_names: None,
            })
            .unwrap();

        // 标题命中
        let by_title = svc
            .query(MemoryQuery {
                search: Some("Alpha".into()),
                ..Default::default()
            })
            .unwrap();
        assert_eq!(
            by_title.items.iter().map(|m| m.id).collect::<Vec<_>>(),
            vec![alpha.id]
        );

        // 正文命中
        let by_body = svc
            .query(MemoryQuery {
                search: Some("keyword".into()),
                ..Default::default()
            })
            .unwrap();
        assert_eq!(
            by_body.items.iter().map(|m| m.id).collect::<Vec<_>>(),
            vec![beta.id]
        );

        // 空白搜索退化为全量
        let blank = svc
            .query(MemoryQuery {
                search: Some("   ".into()),
                ..Default::default()
            })
            .unwrap();
        assert_eq!(blank.items.len(), 3);
        assert_eq!(blank.total, 3);
        assert!(!blank.has_more);

        // 未命中
        let none = svc
            .query(MemoryQuery {
                search: Some("不存在".into()),
                ..Default::default()
            })
            .unwrap();
        assert!(none.items.is_empty());
    }

    #[test]
    fn search_index_includes_trigger_word() {
        let dir = tempdir().unwrap();
        let db = Database::open(dir.path().join("m.db")).unwrap();
        let svc = MemoryService::new(db);
        svc.create(CreateMemoryInput {
            title: "会议模板".into(),
            body: Some("会议纪要模板正文".into()),
            pinned: None,
            quick_insert: Some(true),
            trigger_word: Some("meeting".into()),
            tag_names: None,
        })
        .unwrap();

        // 触发词参与搜索索引：按 trigger_word 检索可命中该记忆
        let results = svc
            .search
            .query(SearchQuery {
                query: "meeting".into(),
                types: Some(vec![SearchEntityType::Memory]),
                limit: Some(10),
            })
            .unwrap();
        assert!(
            results.memories.iter().any(|h| h.title == "会议模板"),
            "trigger_word 应参与搜索索引，按触发词可命中"
        );

        // 直接断言 upsert 写入的搜索文档包含 trigger_word
        let conn = svc.connect().unwrap();
        let stored: String = conn
            .query_row(
                "SELECT body FROM search_documents WHERE entity_type = 'memory'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert!(
            stored.contains("meeting"),
            "search_documents.body 应包含 trigger_word，实际: {stored}"
        );
    }

    #[test]
    fn query_search_escapes_wildcards() {
        let dir = tempdir().unwrap();
        let db = Database::open(dir.path().join("m.db")).unwrap();
        let svc = MemoryService::new(db);
        svc.create(CreateMemoryInput {
            title: "折扣 50% off".into(),
            body: Some("下划线 100_200".into()),
            pinned: None,
            quick_insert: None,
            trigger_word: None,
            tag_names: None,
        })
        .unwrap();
        svc.create(CreateMemoryInput {
            title: "普通标题".into(),
            body: Some("不相关内容".into()),
            pinned: None,
            quick_insert: None,
            trigger_word: None,
            tag_names: None,
        })
        .unwrap();

        // `%` 作为字面量：不应命中所有记忆
        let percent = svc
            .query(MemoryQuery {
                search: Some("50%".into()),
                ..Default::default()
            })
            .unwrap();
        assert_eq!(percent.items.len(), 1);
        assert_eq!(percent.items[0].title, "折扣 50% off");

        // `_` 作为字面量：不应把 100200 当通配符命中
        let underscore = svc
            .query(MemoryQuery {
                search: Some("100_200".into()),
                ..Default::default()
            })
            .unwrap();
        assert_eq!(underscore.items.len(), 1);
        assert_eq!(underscore.items[0].body, "下划线 100_200");
    }

    #[test]
    fn query_pagination_returns_total_and_has_more() {
        let dir = tempdir().unwrap();
        let db = Database::open(dir.path().join("m.db")).unwrap();
        let svc = MemoryService::new(db);
        for i in 0..5 {
            svc.create(CreateMemoryInput {
                title: format!("Item {i}"),
                body: None,
                pinned: None,
                quick_insert: None,
                trigger_word: None,
                tag_names: None,
            })
            .unwrap();
        }

        let page1 = svc
            .query(MemoryQuery {
                limit: Some(2),
                offset: Some(0),
                ..Default::default()
            })
            .unwrap();
        assert_eq!(page1.items.len(), 2);
        assert_eq!(page1.total, 5);
        assert!(page1.has_more);

        let page3 = svc
            .query(MemoryQuery {
                limit: Some(2),
                offset: Some(4),
                ..Default::default()
            })
            .unwrap();
        assert_eq!(page3.items.len(), 1);
        assert!(!page3.has_more);
    }

    #[test]
    fn wikilink_creates_mention_and_backlink() {
        let dir = tempdir().unwrap();
        let db = Database::open(dir.path().join("m.db")).unwrap();
        let svc = MemoryService::new(db);
        let target = svc
            .create(CreateMemoryInput {
                title: "Alpha".into(),
                body: Some("target body".into()),
                pinned: None,
                quick_insert: Some(true),
                trigger_word: Some("alpha".into()),
                tag_names: None,
            })
            .unwrap();
        let source = svc
            .create(CreateMemoryInput {
                title: "Source".into(),
                body: Some("see [[Alpha]] here".into()),
                pinned: None,
                quick_insert: None,
                trigger_word: None,
                tag_names: None,
            })
            .unwrap();

        let links = svc.links.list_outgoing("memory", source.id).unwrap();
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].link_kind, LINK_KIND_MENTION);
        assert_eq!(links[0].target_id, target.id);

        let updated = svc.get(target.id).unwrap();
        assert_eq!(updated.mention_use_count, 1);

        let backlinks = svc.backlinks(target.id).unwrap();
        assert_eq!(backlinks.len(), 1);
        assert_eq!(backlinks[0].memory_id, source.id);
    }
}
