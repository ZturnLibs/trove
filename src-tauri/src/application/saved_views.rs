use crate::domain::{new_id, stamp, DomainError, EntityId, SystemClock};
use crate::infrastructure::db::Database;
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SavedView {
    pub id: EntityId,
    pub name: String,
    pub filter: Value,
    pub created_at: String,
    pub updated_at: String,
    pub revision: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateSavedViewInput {
    pub name: String,
    pub filter: Value,
}

pub struct SavedViewService {
    db: Database,
    clock: SystemClock,
}

impl SavedViewService {
    pub fn new(db: Database) -> Self {
        Self {
            db,
            clock: SystemClock,
        }
    }

    fn connect(&self) -> Result<Connection, DomainError> {
        self.db.connect().map_err(internal)
    }

    pub fn create(&self, input: CreateSavedViewInput) -> Result<SavedView, DomainError> {
        let name = input.name.trim().to_string();
        if name.is_empty() {
            return Err(DomainError::Validation("视图名称不能为空".into()));
        }
        let id = new_id();
        let now = stamp(&self.clock);
        let filter = serde_json::to_string(&input.filter).map_err(internal)?;
        let conn = self.connect()?;
        conn.execute(
            "INSERT INTO saved_views (id, name, filter_json, created_at, updated_at, revision, deleted_at)
             VALUES (?1, ?2, ?3, ?4, ?5, 1, NULL)",
            params![id.to_string(), name, filter, now, now],
        )
        .map_err(internal)?;
        self.get(id)
    }

    pub fn list(&self) -> Result<Vec<SavedView>, DomainError> {
        let conn = self.connect()?;
        let mut stmt = conn
            .prepare(
                "SELECT id, name, filter_json, created_at, updated_at, revision
                 FROM saved_views WHERE deleted_at IS NULL
                 ORDER BY updated_at DESC",
            )
            .map_err(internal)?;
        let rows = stmt.query_map([], map_saved_view).map_err(internal)?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row.map_err(internal)?);
        }
        Ok(out)
    }

    pub fn get(&self, id: EntityId) -> Result<SavedView, DomainError> {
        let conn = self.connect()?;
        conn.query_row(
            "SELECT id, name, filter_json, created_at, updated_at, revision
             FROM saved_views WHERE id = ?1 AND deleted_at IS NULL",
            [id.to_string()],
            map_saved_view,
        )
        .optional()
        .map_err(internal)?
        .ok_or_else(|| DomainError::NotFound("视图不存在".into()))
    }

    pub fn delete(&self, id: EntityId) -> Result<(), DomainError> {
        let _ = self.get(id)?;
        let now = stamp(&self.clock);
        let conn = self.connect()?;
        conn.execute(
            "UPDATE saved_views SET deleted_at = ?1, updated_at = ?1, revision = revision + 1 WHERE id = ?2",
            params![now, id.to_string()],
        )
        .map_err(internal)?;
        Ok(())
    }
}

fn map_saved_view(row: &rusqlite::Row<'_>) -> Result<SavedView, rusqlite::Error> {
    let filter_raw: String = row.get(2)?;
    let filter: Value =
        serde_json::from_str(&filter_raw).unwrap_or(Value::Object(Default::default()));
    Ok(SavedView {
        id: row.get::<_, String>(0)?.parse().map_err(|e| {
            rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(e))
        })?,
        name: row.get(1)?,
        filter,
        created_at: row.get(3)?,
        updated_at: row.get(4)?,
        revision: row.get(5)?,
    })
}

fn internal<E: std::fmt::Display>(err: E) -> DomainError {
    DomainError::Internal(err.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn create_list_delete_round_trip() {
        let dir = tempdir().unwrap();
        let db = Database::open(dir.path().join("t.db")).unwrap();
        let svc = SavedViewService::new(db);

        let created = svc
            .create(CreateSavedViewInput {
                name: " 重要任务 ".into(),
                filter: serde_json::json!({
                    "listId": "all",
                    "status": "active",
                    "priority": "high",
                    "tagId": null,
                    "smart": "none",
                }),
            })
            .unwrap();
        assert_eq!(created.name, "重要任务");
        assert_eq!(created.filter["priority"], "high");

        let listed = svc.list().unwrap();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].id, created.id);

        svc.delete(created.id).unwrap();
        assert!(svc.list().unwrap().is_empty());
        assert!(svc.get(created.id).is_err());
    }

    #[test]
    fn empty_name_rejected() {
        let dir = tempdir().unwrap();
        let db = Database::open(dir.path().join("t.db")).unwrap();
        let svc = SavedViewService::new(db);
        let err = svc
            .create(CreateSavedViewInput {
                name: "   ".into(),
                filter: serde_json::json!({}),
            })
            .unwrap_err();
        assert!(matches!(err, DomainError::Validation(_)));
    }
}
