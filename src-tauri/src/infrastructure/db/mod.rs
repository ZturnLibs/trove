use rusqlite::{backup::Backup, Connection, OptionalExtension};
use std::path::{Path, PathBuf};
use std::time::Duration;
use thiserror::Error;

const MIGRATIONS: &[(i64, &str)] = &[
    (1, include_str!("../../../migrations/0001_init.sql")),
    (2, include_str!("../../../migrations/0002_tasks.sql")),
    (3, include_str!("../../../migrations/0003_reminders.sql")),
    (
        4,
        include_str!("../../../migrations/0004_memory_search.sql"),
    ),
    (5, include_str!("../../../migrations/0005_clipboard.sql")),
    (
        6,
        include_str!("../../../migrations/0006_v11_efficiency.sql"),
    ),
    (7, include_str!("../../../migrations/0007_assets_ocr.sql")),
    (8, include_str!("../../../migrations/0008_entity_links.sql")),
    (9, include_str!("../../../migrations/0009_saved_views.sql")),
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
        Self::open_with_backup_dir(path, None)
    }

    pub fn open_with_backup_dir(
        path: impl Into<PathBuf>,
        backup_dir: Option<PathBuf>,
    ) -> Result<Self, DbError> {
        let path = path.into();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let db = Self { path };
        db.migrate(backup_dir.as_deref())?;
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

    pub fn migrate(&self, backup_dir: Option<&Path>) -> Result<(), DbError> {
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

        let pending: Vec<&(i64, &str)> = MIGRATIONS
            .iter()
            .filter(|(version, _)| *version > current)
            .collect();

        if !pending.is_empty() {
            if let Some(dir) = backup_dir {
                if current > 0 && self.path.exists() {
                    if let Err(err) = self.snapshot_before_migrate(dir, current) {
                        tracing::error!(error = %err, "pre-migration backup failed");
                        return Err(DbError::Message(format!(
                            "数据库升级前备份失败，已中止迁移: {err}"
                        )));
                    }
                }
            }

            for (version, sql) in pending {
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
        }

        Ok(())
    }

    fn snapshot_before_migrate(&self, backup_dir: &Path, current: i64) -> Result<(), DbError> {
        std::fs::create_dir_all(backup_dir)?;
        let stamp = chrono::Local::now().format("%Y%m%d-%H%M%S");
        let dest = backup_dir.join(format!("workbench-pre-migrate-v{current}-{stamp}.db"));
        let src = self.connect()?;
        let mut dst = Connection::open(&dest)?;
        {
            let backup = Backup::new(&src, &mut dst)?;
            backup.run_to_completion(100, Duration::from_millis(25), None)?;
        }
        tracing::info!(path = %dest.display(), "created pre-migration backup");
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
        assert_eq!(health.schema_version, 9);
        assert_eq!(health.journal_mode.to_lowercase(), "wal");
        assert!(health.fts5_available);
    }

    #[test]
    fn migrate_is_idempotent() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("workbench.db");
        let db = Database::open(&db_path).unwrap();
        db.migrate(None).unwrap();
        db.migrate(None).unwrap();
        assert_eq!(db.health_check().unwrap().schema_version, 9);
    }

    #[test]
    fn migrate_creates_backup_before_pending_migration() {
        use rusqlite::Connection;

        let dir = tempdir().unwrap();
        let db_path = dir.path().join("workbench.db");
        let backup_dir = dir.path().join("backups");

        let conn = Connection::open(&db_path).unwrap();
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS schema_migrations (
                version INTEGER PRIMARY KEY NOT NULL,
                applied_at TEXT NOT NULL
            );",
        )
        .unwrap();
        for (version, sql) in MIGRATIONS.iter().take(8) {
            conn.execute_batch(sql).unwrap();
            conn.execute(
                "INSERT INTO schema_migrations (version, applied_at) VALUES (?1, 'test')",
                [version],
            )
            .unwrap();
        }
        drop(conn);

        let db = Database { path: db_path.clone() };
        db.migrate(Some(&backup_dir)).unwrap();
        assert_eq!(db.health_check().unwrap().schema_version, 9);

        let backups: Vec<_> = std::fs::read_dir(&backup_dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| {
                e.path()
                    .file_name()
                    .and_then(|n| n.to_str())
                    .is_some_and(|n| n.starts_with("workbench-pre-migrate-v8-"))
            })
            .collect();
        assert_eq!(backups.len(), 1);
    }
}
