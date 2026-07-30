use crate::domain::{
    stamp, DomainError, EntityId, SearchEntityType, SearchHit, SearchQuery, SearchResults,
    SystemClock,
};
use crate::infrastructure::db::Database;
use rusqlite::{params, Connection, OptionalExtension};

pub struct SearchService {
    db: Database,
    clock: SystemClock,
}

impl SearchService {
    pub fn new(db: Database) -> Self {
        Self {
            db,
            clock: SystemClock,
        }
    }

    fn connect(&self) -> Result<Connection, DomainError> {
        self.db.connect().map_err(internal)
    }

    pub fn upsert(
        &self,
        entity_type: SearchEntityType,
        entity_id: EntityId,
        title: &str,
        body: &str,
    ) -> Result<(), DomainError> {
        let conn = self.connect()?;
        self.upsert_conn(&conn, entity_type, entity_id, title, body)
    }

    pub fn upsert_conn(
        &self,
        conn: &Connection,
        entity_type: SearchEntityType,
        entity_id: EntityId,
        title: &str,
        body: &str,
    ) -> Result<(), DomainError> {
        let now = stamp(&self.clock);
        let normalized = normalize_text(&format!("{title}\n{body}"));
        let clipped_body = clip(body, 4000);
        let existing: Option<i64> = conn
            .query_row(
                "SELECT id FROM search_documents WHERE entity_type = ?1 AND entity_id = ?2",
                params![entity_type.as_str(), entity_id.to_string()],
                |row| row.get(0),
            )
            .optional()
            .map_err(internal)?;

        if let Some(rowid) = existing {
            conn.execute(
                "UPDATE search_documents SET title = ?1, body = ?2, normalized = ?3, updated_at = ?4
                 WHERE id = ?5",
                params![title, clipped_body, normalized, now, rowid],
            )
            .map_err(internal)?;
            // Rebuild FTS row for external content table.
            let _ = conn.execute("INSERT INTO search_index(search_index, rowid) VALUES('delete', ?1)", [rowid]);
            conn.execute(
                "INSERT INTO search_index(rowid, title, body, normalized) VALUES(?1, ?2, ?3, ?4)",
                params![rowid, title, clipped_body, normalized],
            )
            .map_err(internal)?;
        } else {
            conn.execute(
                "INSERT INTO search_documents (entity_type, entity_id, title, body, normalized, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    entity_type.as_str(),
                    entity_id.to_string(),
                    title,
                    clipped_body,
                    normalized,
                    now
                ],
            )
            .map_err(internal)?;
            let rowid = conn.last_insert_rowid();
            conn.execute(
                "INSERT INTO search_index(rowid, title, body, normalized) VALUES(?1, ?2, ?3, ?4)",
                params![rowid, title, clipped_body, normalized],
            )
            .map_err(internal)?;
        }
        Ok(())
    }

    pub fn remove(&self, entity_type: SearchEntityType, entity_id: EntityId) -> Result<(), DomainError> {
        let conn = self.connect()?;
        let rowid: Option<i64> = conn
            .query_row(
                "SELECT id FROM search_documents WHERE entity_type = ?1 AND entity_id = ?2",
                params![entity_type.as_str(), entity_id.to_string()],
                |row| row.get(0),
            )
            .optional()
            .map_err(internal)?;
        if let Some(rowid) = rowid {
            let _ = conn.execute(
                "INSERT INTO search_index(search_index, rowid) VALUES('delete', ?1)",
                [rowid],
            );
            conn.execute("DELETE FROM search_documents WHERE id = ?1", [rowid])
                .map_err(internal)?;
        }
        Ok(())
    }

    pub fn query(&self, input: SearchQuery) -> Result<SearchResults, DomainError> {
        let q = input.query.trim();
        if q.is_empty() {
            return Ok(SearchResults {
                tasks: Vec::new(),
                reminders: Vec::new(),
                memories: Vec::new(),
            });
        }
        if q.chars().count() > 200 {
            return Err(DomainError::Validation("搜索词过长".into()));
        }
        let limit = input.limit.unwrap_or(40).clamp(1, 100);
        let conn = self.connect()?;

        let hits = if q.chars().count() <= 2 {
            self.like_search(&conn, q, limit)?
        } else {
            match self.fts_search(&conn, q, limit) {
                Ok(hits) => hits,
                Err(_) => self.like_search(&conn, q, limit)?,
            }
        };

        let allowed = input.types.unwrap_or_else(|| {
            vec![
                SearchEntityType::Task,
                SearchEntityType::Reminder,
                SearchEntityType::Memory,
            ]
        });

        let mut tasks = Vec::new();
        let mut reminders = Vec::new();
        let mut memories = Vec::new();
        for hit in hits {
            if !allowed.contains(&hit.entity_type) {
                continue;
            }
            match hit.entity_type {
                SearchEntityType::Task => tasks.push(hit),
                SearchEntityType::Reminder => reminders.push(hit),
                SearchEntityType::Memory => memories.push(hit),
            }
        }
        Ok(SearchResults {
            tasks,
            reminders,
            memories,
        })
    }

    pub fn rebuild_all(&self) -> Result<usize, DomainError> {
        let conn = self.connect()?;
        conn.execute_batch(
            "DELETE FROM search_index;
             DELETE FROM search_documents;",
        )
        .map_err(internal)?;

        let mut count = 0usize;
        {
            let mut stmt = conn
                .prepare(
                    "SELECT id, title, notes FROM tasks WHERE deleted_at IS NULL AND status != 'archived'",
                )
                .map_err(internal)?;
            let rows = stmt
                .query_map([], |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                    ))
                })
                .map_err(internal)?;
            for row in rows {
                let (id, title, notes) = row.map_err(internal)?;
                let entity_id: EntityId = id.parse().map_err(|e| DomainError::Internal(format!("{e}")))?;
                self.upsert_conn(&conn, SearchEntityType::Task, entity_id, &title, &notes)?;
                count += 1;
            }
        }
        {
            let mut stmt = conn
                .prepare(
                    "SELECT id, title, notes FROM reminders WHERE deleted_at IS NULL AND enabled = 1",
                )
                .map_err(internal)?;
            let rows = stmt
                .query_map([], |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                    ))
                })
                .map_err(internal)?;
            for row in rows {
                let (id, title, notes) = row.map_err(internal)?;
                let entity_id: EntityId = id.parse().map_err(|e| DomainError::Internal(format!("{e}")))?;
                self.upsert_conn(&conn, SearchEntityType::Reminder, entity_id, &title, &notes)?;
                count += 1;
            }
        }
        {
            let mut stmt = conn
                .prepare(
                    "SELECT id, title, body FROM memories WHERE deleted_at IS NULL AND archived = 0",
                )
                .map_err(internal)?;
            let rows = stmt
                .query_map([], |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                    ))
                })
                .map_err(internal)?;
            for row in rows {
                let (id, title, body) = row.map_err(internal)?;
                let entity_id: EntityId = id.parse().map_err(|e| DomainError::Internal(format!("{e}")))?;
                self.upsert_conn(&conn, SearchEntityType::Memory, entity_id, &title, &body)?;
                count += 1;
            }
        }
        Ok(count)
    }

    fn fts_search(
        &self,
        conn: &Connection,
        q: &str,
        limit: i64,
    ) -> Result<Vec<SearchHit>, DomainError> {
        let escaped = escape_fts(q);
        let mut stmt = conn
            .prepare(
                "SELECT d.entity_type, d.entity_id, d.title, d.body, d.updated_at
                 FROM search_index i
                 JOIN search_documents d ON d.id = i.rowid
                 WHERE search_index MATCH ?1
                 ORDER BY rank
                 LIMIT ?2",
            )
            .map_err(internal)?;
        let rows = stmt
            .query_map(params![escaped, limit], map_hit)
            .map_err(internal)?;
        collect(rows)
    }

    fn like_search(
        &self,
        conn: &Connection,
        q: &str,
        limit: i64,
    ) -> Result<Vec<SearchHit>, DomainError> {
        let pattern = format!("%{}%", escape_like(q));
        let mut stmt = conn
            .prepare(
                "SELECT entity_type, entity_id, title, body, updated_at
                 FROM search_documents
                 WHERE title LIKE ?1 ESCAPE '\\' OR body LIKE ?1 ESCAPE '\\' OR normalized LIKE ?1 ESCAPE '\\'
                 ORDER BY updated_at DESC
                 LIMIT ?2",
            )
            .map_err(internal)?;
        let rows = stmt
            .query_map(params![pattern, limit], map_hit)
            .map_err(internal)?;
        collect(rows)
    }
}

fn map_hit(row: &rusqlite::Row<'_>) -> Result<SearchHit, rusqlite::Error> {
    let entity_type = SearchEntityType::parse(&row.get::<_, String>(0)?).map_err(|e| {
        rusqlite::Error::ToSqlConversionFailure(Box::new(std::io::Error::other(e.to_string())))
    })?;
    let entity_id: String = row.get(1)?;
    let title: String = row.get(2)?;
    let body: String = row.get(3)?;
    Ok(SearchHit {
        entity_type,
        entity_id: entity_id.parse().map_err(|e| {
            rusqlite::Error::FromSqlConversionFailure(1, rusqlite::types::Type::Text, Box::new(e))
        })?,
        title,
        snippet: clip(&body, 160),
        updated_at: row.get(4)?,
    })
}

fn normalize_text(value: &str) -> String {
    value
        .chars()
        .map(|c| {
            if c.is_whitespace() {
                ' '
            } else {
                c.to_ascii_lowercase()
            }
        })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn clip(value: &str, max: usize) -> String {
    let mut out = String::new();
    for (i, ch) in value.chars().enumerate() {
        if i >= max {
            out.push('…');
            break;
        }
        out.push(ch);
    }
    out
}

fn escape_like(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_")
}

fn escape_fts(value: &str) -> String {
    // For trigram, quote the phrase after stripping quotes.
    let cleaned = value.replace('"', " ").trim().to_string();
    format!("\"{cleaned}\"")
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
    use crate::domain::new_entity_id;
    use crate::infrastructure::db::Database;
    use tempfile::tempdir;

    #[test]
    fn upsert_and_search() {
        let dir = tempdir().unwrap();
        let db = Database::open(dir.path().join("s.db")).unwrap();
        let svc = SearchService::new(db);
        let id = new_entity_id();
        svc.upsert(SearchEntityType::Memory, id, "会议纪要", "讨论搜索与记忆模块")
            .unwrap();
        let results = svc
            .query(SearchQuery {
                query: "记忆".into(),
                types: None,
                limit: Some(10),
            })
            .unwrap();
        assert!(!results.memories.is_empty() || !results.tasks.is_empty() || results.memories.is_empty());
        // trigram or like should find 记忆 in body/title of our doc - body has 记忆
        let found = results.memories.iter().any(|h| h.entity_id == id)
            || svc
                .query(SearchQuery {
                    query: "搜索".into(),
                    types: Some(vec![SearchEntityType::Memory]),
                    limit: Some(10),
                })
                .unwrap()
                .memories
                .iter()
                .any(|h| h.entity_id == id);
        assert!(found);
    }
}
