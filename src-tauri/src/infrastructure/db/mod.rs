use rusqlite::{Connection, OptionalExtension};
use std::path::{Path, PathBuf};
use thiserror::Error;

const MIGRATIONS: &[(i64, &str)] = &[
    (1, include_str!("../../../migrations/0001_init.sql")),
    (2, include_str!("../../../migrations/0002_tasks.sql")),
    (3, include_str!("../../../migrations/0003_reminders.sql")),
];

#[derive(Debug, Error)]
pub enum DbError {
    #[error("sqlite error: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("{0}")]
    Message(String),
}

#[derive(Debug, Clone)]
pub struct Database {
    path: PathBuf,
}

impl Database {
    pub fn open(path: impl Into<PathBuf>) -> Result<Self, DbError> {
        let path = path.into();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let db = Self { path };
        db.migrate()?;
        Ok(db)
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn connect(&self) -> Result<Connection, DbError> {
        let conn = Connection::open(&self.path)?;
        conn.busy_timeout(std::time::Duration::from_secs(5))?;
        conn.execute_batch(
            "PRAGMA foreign_keys = ON;
             PRAGMA journal_mode = WAL;
             PRAGMA synchronous = NORMAL;",
        )?;
        Ok(conn)
    }

    pub fn migrate(&self) -> Result<(), DbError> {
        let conn = self.connect()?;
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS schema_migrations (
                version INTEGER PRIMARY KEY NOT NULL,
                applied_at TEXT NOT NULL
            );",
        )?;

        let current: i64 = conn
            .query_row(
                "SELECT COALESCE(MAX(version), 0) FROM schema_migrations",
                [],
                |row| row.get(0),
            )
            .unwrap_or(0);

        for (version, sql) in MIGRATIONS {
            if *version <= current {
                continue;
            }

            let tx = conn.unchecked_transaction()?;
            tx.execute_batch(sql)?;
            tx.execute(
                "INSERT INTO schema_migrations (version, applied_at) VALUES (?1, ?2)",
                rusqlite::params![
                    version,
                    chrono::Utc::now()
                        .format("%Y-%m-%dT%H:%M:%S%.3fZ")
                        .to_string()
                ],
            )?;
            tx.commit()?;
            tracing::info!(version, "applied database migration");
        }

        Ok(())
    }

    pub fn health_check(&self) -> Result<DbHealth, DbError> {
        let conn = self.connect()?;
        let version: i64 = conn.query_row(
            "SELECT COALESCE(MAX(version), 0) FROM schema_migrations",
            [],
            |row| row.get(0),
        )?;
        let user_version: i64 = conn.query_row("PRAGMA user_version", [], |row| row.get(0))?;
        let wal_mode: String = conn
            .query_row("PRAGMA journal_mode", [], |row| row.get(0))
            .optional()?
            .unwrap_or_else(|| "unknown".into());

        let fts5_available = conn
            .execute_batch(
                "CREATE VIRTUAL TABLE IF NOT EXISTS __fts5_probe USING fts5(content);
                 DROP TABLE IF EXISTS __fts5_probe;",
            )
            .is_ok();

        Ok(DbHealth {
            path: self.path.display().to_string(),
            schema_version: version,
            user_version,
            journal_mode: wal_mode,
            fts5_available,
        })
    }
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DbHealth {
    pub path: String,
    pub schema_version: i64,
    pub user_version: i64,
    pub journal_mode: String,
    pub fts5_available: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn migrates_and_reports_health() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("workbench.db");
        let db = Database::open(&db_path).unwrap();
        let health = db.health_check().unwrap();
        assert_eq!(health.schema_version, 3);
        assert_eq!(health.journal_mode.to_lowercase(), "wal");
        assert!(health.fts5_available);
    }

    #[test]
    fn migrate_is_idempotent() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("workbench.db");
        let db = Database::open(&db_path).unwrap();
        db.migrate().unwrap();
        db.migrate().unwrap();
        assert_eq!(db.health_check().unwrap().schema_version, 3);
    }
}
