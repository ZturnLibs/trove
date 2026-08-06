use crate::application::links::EntityLinkService;
use crate::application::search::SearchService;
use crate::application::tasks::TaskService;
use crate::domain::{
    new_id, stamp, ConvertMemoryToTaskResult, CreateMemoryInput, CreateTaskInput, DomainError,
    EntityId, Memory, MemoryQuery, SearchEntityType, SystemClock, UpdateMemoryInput,
};
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
                quick_insert = ?5, trigger_word = ?6,
                updated_at = ?7, revision = revision + 1
             WHERE id = ?8 AND deleted_at IS NULL",
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
        self.get(input.id)
    }

    pub fn get(&self, id: EntityId) -> Result<Memory, DomainError> {
        let conn = self.connect()?;
        let mut memory = conn
            .query_row(
                "SELECT id, title, body, pinned, archived, quick_insert, trigger_word,
                        created_at, updated_at, revision
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

    pub fn query(&self, query: MemoryQuery) -> Result<Vec<Memory>, DomainError> {
        let conn = self.connect()?;
        let mut sql = String::from(
            "SELECT id, title, body, pinned, archived, quick_insert, trigger_word,
                    created_at, updated_at, revision
             FROM memories WHERE deleted_at IS NULL",
        );
        let mut values: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();
        if !query.include_archived.unwrap_or(false) {
            sql.push_str(" AND archived = 0");
        }
        if query.pinned_only.unwrap_or(false) {
            sql.push_str(" AND pinned = 1");
        }
        if query.quick_insert_only.unwrap_or(false) {
            sql.push_str(" AND quick_insert = 1");
        }
        if let Some(tag_id) = query.tag_id {
            sql.push_str(
                " AND EXISTS (SELECT 1 FROM memory_tags mt WHERE mt.memory_id = memories.id AND mt.tag_id = ?)",
            );
            values.push(Box::new(tag_id.to_string()));
        }
        if let Some(text) = query.search.as_ref().map(|s| s.trim().to_string()) {
            if !text.is_empty() {
                sql.push_str(" AND (title LIKE ? ESCAPE '\\' OR body LIKE ? ESCAPE '\\')");
                let pattern = format!("%{}%", escape_like(&text));
                values.push(Box::new(pattern.clone()));
                values.push(Box::new(pattern));
            }
        }
        sql.push_str(" ORDER BY pinned DESC, updated_at DESC");
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
        Ok(memories)
    }

    pub fn delete(&self, id: EntityId) -> Result<(), DomainError> {
        let _ = self.get(id)?;
        let now = stamp(&self.clock);
        let conn = self.connect()?;
        conn.execute(
            "UPDATE memories SET deleted_at = ?1, updated_at = ?1, revision = revision + 1 WHERE id = ?2",
            params![now, id.to_string()],
        )
        .map_err(internal)?;
        self.search.remove(SearchEntityType::Memory, id)?;
        let _ = self.links.purge_for_source("memory", id);
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
        tag_ids: Vec::new(),
        tag_names: Vec::new(),
        created_at: row.get(7)?,
        updated_at: row.get(8)?,
        revision: row.get(9)?,
    })
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
                pinned_only: None,
                include_archived: None,
                tag_id: None,
                quick_insert_only: None,
                search: Some("Alpha".into()),
            })
            .unwrap();
        assert_eq!(
            by_title.iter().map(|m| m.id).collect::<Vec<_>>(),
            vec![alpha.id]
        );

        // 正文命中
        let by_body = svc
            .query(MemoryQuery {
                pinned_only: None,
                include_archived: None,
                tag_id: None,
                quick_insert_only: None,
                search: Some("keyword".into()),
            })
            .unwrap();
        assert_eq!(
            by_body.iter().map(|m| m.id).collect::<Vec<_>>(),
            vec![beta.id]
        );

        // 空白搜索退化为全量
        let blank = svc
            .query(MemoryQuery {
                pinned_only: None,
                include_archived: None,
                tag_id: None,
                quick_insert_only: None,
                search: Some("   ".into()),
            })
            .unwrap();
        assert_eq!(blank.len(), 3);

        // 未命中
        let none = svc
            .query(MemoryQuery {
                pinned_only: None,
                include_archived: None,
                tag_id: None,
                quick_insert_only: None,
                search: Some("不存在".into()),
            })
            .unwrap();
        assert!(none.is_empty());
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
                pinned_only: None,
                include_archived: None,
                tag_id: None,
                quick_insert_only: None,
                search: Some("50%".into()),
            })
            .unwrap();
        assert_eq!(percent.len(), 1);
        assert_eq!(percent[0].title, "折扣 50% off");

        // `_` 作为字面量：不应把 100200 当通配符命中
        let underscore = svc
            .query(MemoryQuery {
                pinned_only: None,
                include_archived: None,
                tag_id: None,
                quick_insert_only: None,
                search: Some("100_200".into()),
            })
            .unwrap();
        assert_eq!(underscore.len(), 1);
        assert_eq!(underscore[0].body, "下划线 100_200");
    }
}
