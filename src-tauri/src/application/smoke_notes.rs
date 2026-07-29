use crate::domain::{format_utc, new_entity_id, Clock, DomainError, EntityId, Revision, SystemClock};
use crate::infrastructure::db::Database;
use rusqlite::OptionalExtension;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SmokeNote {
    pub id: EntityId,
    pub body: String,
    pub created_at: String,
    pub updated_at: String,
    pub revision: Revision,
}

pub struct SmokeNoteService {
    db: Database,
    clock: SystemClock,
}

impl SmokeNoteService {
    pub fn new(db: Database) -> Self {
        Self {
            db,
            clock: SystemClock,
        }
    }

    pub fn create(&self, body: String) -> Result<SmokeNote, DomainError> {
        let body = body.trim().to_string();
        if body.is_empty() {
            return Err(DomainError::Validation("body is required".into()));
        }

        let id = new_entity_id();
        let now = format_utc(self.clock.now());
        let conn = self
            .db
            .connect()
            .map_err(|e| DomainError::Internal(e.to_string()))?;

        conn.execute(
            "INSERT INTO smoke_notes (id, body, created_at, updated_at, revision, deleted_at)
             VALUES (?1, ?2, ?3, ?3, 1, NULL)",
            rusqlite::params![id.to_string(), body, now],
        )
        .map_err(|e| DomainError::Internal(e.to_string()))?;

        Ok(SmokeNote {
            id,
            body,
            created_at: now.clone(),
            updated_at: now,
            revision: 1,
        })
    }

    pub fn list_active(&self) -> Result<Vec<SmokeNote>, DomainError> {
        let conn = self
            .db
            .connect()
            .map_err(|e| DomainError::Internal(e.to_string()))?;
        let mut stmt = conn
            .prepare(
                "SELECT id, body, created_at, updated_at, revision
                 FROM smoke_notes
                 WHERE deleted_at IS NULL
                 ORDER BY updated_at DESC",
            )
            .map_err(|e| DomainError::Internal(e.to_string()))?;

        let rows = stmt
            .query_map([], |row| {
                let id: String = row.get(0)?;
                Ok(SmokeNote {
                    id: id.parse().map_err(|e| {
                        rusqlite::Error::FromSqlConversionFailure(
                            0,
                            rusqlite::types::Type::Text,
                            Box::new(e),
                        )
                    })?,
                    body: row.get(1)?,
                    created_at: row.get(2)?,
                    updated_at: row.get(3)?,
                    revision: row.get(4)?,
                })
            })
            .map_err(|e| DomainError::Internal(e.to_string()))?;

        let mut notes = Vec::new();
        for row in rows {
            notes.push(row.map_err(|e| DomainError::Internal(e.to_string()))?);
        }
        Ok(notes)
    }

    pub fn soft_delete(&self, id: EntityId) -> Result<(), DomainError> {
        let conn = self
            .db
            .connect()
            .map_err(|e| DomainError::Internal(e.to_string()))?;
        let now = format_utc(self.clock.now());
        let updated = conn
            .execute(
                "UPDATE smoke_notes
                 SET deleted_at = ?1,
                     updated_at = ?1,
                     revision = revision + 1
                 WHERE id = ?2 AND deleted_at IS NULL",
                rusqlite::params![now, id.to_string()],
            )
            .map_err(|e| DomainError::Internal(e.to_string()))?;

        if updated == 0 {
            let exists: Option<i64> = conn
                .query_row(
                    "SELECT 1 FROM smoke_notes WHERE id = ?1",
                    [id.to_string()],
                    |row| row.get(0),
                )
                .optional()
                .map_err(|e| DomainError::Internal(e.to_string()))?;
            if exists.is_none() {
                return Err(DomainError::NotFound("smoke note not found".into()));
            }
        }
        Ok(())
    }
}
