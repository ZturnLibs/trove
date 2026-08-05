use crate::domain::DomainError;
use crate::infrastructure::db::Database;
use chrono::Local;
use rusqlite::{backup::Backup, Connection, OpenFlags};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupInfo {
    pub file_name: String,
    pub path: String,
    pub size_bytes: u64,
    pub created_at: String,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupStatus {
    pub directory: String,
    pub count: usize,
    pub latest: Option<BackupInfo>,
    pub last_error: Option<String>,
}

pub struct BackupService {
    db: Database,
    backup_dir: PathBuf,
    last_error: std::sync::Mutex<Option<String>>,
}

impl BackupService {
    pub fn new(db: Database, backup_dir: PathBuf) -> Self {
        Self {
            db,
            backup_dir,
            last_error: std::sync::Mutex::new(None),
        }
    }

    pub fn backup_dir(&self) -> &Path {
        &self.backup_dir
    }

    fn set_error(&self, message: Option<String>) {
        if let Ok(mut guard) = self.last_error.lock() {
            *guard = message;
        }
    }

    pub fn status(&self) -> BackupStatus {
        let list = self.list().unwrap_or_default();
        let last_error = self.last_error.lock().ok().and_then(|g| g.clone());
        BackupStatus {
            directory: self.backup_dir.display().to_string(),
            count: list.len(),
            latest: list.first().cloned(),
            last_error,
        }
    }

    pub fn create(&self, reason: &str) -> Result<BackupInfo, DomainError> {
        match self.create_inner(reason) {
            Ok(info) => {
                self.set_error(None);
                Ok(info)
            }
            Err(err) => {
                self.set_error(Some(err.to_string()));
                Err(err)
            }
        }
    }

    fn create_inner(&self, reason: &str) -> Result<BackupInfo, DomainError> {
        std::fs::create_dir_all(&self.backup_dir).map_err(io_err)?;
        let stamp = Local::now().format("%Y%m%d-%H%M%S").to_string();
        let safe_reason: String = reason
            .chars()
            .map(|c| {
                if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                    c
                } else {
                    '-'
                }
            })
            .collect();
        let file_name = format!("workbench-{safe_reason}-{stamp}.db");
        let dest = self.backup_dir.join(&file_name);

        let src = self.db.connect().map_err(internal)?;
        let mut dst = Connection::open(&dest).map_err(internal)?;
        {
            let backup = Backup::new(&src, &mut dst).map_err(internal)?;
            backup
                .run_to_completion(100, Duration::from_millis(25), None)
                .map_err(internal)?;
        }

        let meta = std::fs::metadata(&dest).map_err(io_err)?;
        Ok(BackupInfo {
            file_name,
            path: dest.display().to_string(),
            size_bytes: meta.len(),
            created_at: Local::now().format("%Y-%m-%dT%H:%M:%S").to_string(),
            reason: reason.into(),
        })
    }

    pub fn list(&self) -> Result<Vec<BackupInfo>, DomainError> {
        if !self.backup_dir.exists() {
            return Ok(Vec::new());
        }
        let mut items = Vec::new();
        for entry in std::fs::read_dir(&self.backup_dir).map_err(io_err)? {
            let entry = entry.map_err(io_err)?;
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("db") {
                continue;
            }
            let meta = entry.metadata().map_err(io_err)?;
            if !meta.is_file() {
                continue;
            }
            let file_name = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or_default()
                .to_string();
            let created_at = meta
                .modified()
                .ok()
                .and_then(|t| {
                    chrono::DateTime::<chrono::Local>::from(t)
                        .format("%Y-%m-%dT%H:%M:%S")
                        .to_string()
                        .into()
                })
                .unwrap_or_else(|| "unknown".into());
            let reason = file_name
                .strip_prefix("workbench-")
                .and_then(|rest| rest.rsplit_once('-').map(|(r, _)| r.to_string()))
                .unwrap_or_else(|| "backup".into());
            items.push(BackupInfo {
                file_name,
                path: path.display().to_string(),
                size_bytes: meta.len(),
                created_at,
                reason,
            });
        }
        items.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        Ok(items)
    }

    pub fn rotate(&self, keep: usize) -> Result<usize, DomainError> {
        let keep = keep.max(1);
        let list = self.list()?;
        let mut removed = 0;
        for item in list.into_iter().skip(keep) {
            let path = self.resolve_backup_path(&item.file_name)?;
            std::fs::remove_file(path).map_err(io_err)?;
            removed += 1;
        }
        Ok(removed)
    }

    pub fn restore(&self, file_name: &str) -> Result<(), DomainError> {
        let path = self.resolve_backup_path(file_name)?;
        // Validate the backup is a readable, uncorrupted SQLite DB *before*
        // touching the target database.
        {
            let check = Connection::open_with_flags(&path, OpenFlags::SQLITE_OPEN_READ_ONLY)
                .map_err(|err| {
                    DomainError::Validation(format!(
                        "备份文件无法打开或已损坏（{err}），已取消恢复，数据未改动"
                    ))
                })?;
            let quick_check: String = check
                .query_row("PRAGMA quick_check", [], |row| row.get(0))
                .map_err(|err| {
                    DomainError::Validation(format!(
                        "备份文件校验失败（{err}），已取消恢复，数据未改动"
                    ))
                })?;
            if quick_check != "ok" {
                return Err(DomainError::Validation(format!(
                    "备份文件校验失败（{quick_check}），已取消恢复，数据未改动"
                )));
            }
        }
        // Safety snapshot of current DB before overwrite.
        let _ = self.create_inner("pre-restore");

        let src = Connection::open(path).map_err(internal)?;
        let mut dst = self.db.connect().map_err(internal)?;
        {
            let backup = Backup::new(&src, &mut dst).map_err(internal)?;
            backup
                .run_to_completion(100, Duration::from_millis(25), None)
                .map_err(internal)?;
        }
        // Bring the restored DB up to the current schema (idempotent).
        self.db.migrate(None).map_err(internal)?;
        self.set_error(None);
        Ok(())
    }

    fn resolve_backup_path(&self, file_name: &str) -> Result<PathBuf, DomainError> {
        if file_name.contains("..")
            || file_name.contains('/')
            || file_name.contains('\\')
            || !file_name.ends_with(".db")
        {
            return Err(DomainError::Validation("非法备份文件名".into()));
        }
        let path = self.backup_dir.join(file_name);
        if !path.exists() {
            return Err(DomainError::NotFound("备份不存在".into()));
        }
        Ok(path)
    }
}

fn io_err(err: std::io::Error) -> DomainError {
    DomainError::Internal(err.to_string())
}

fn internal<E: std::fmt::Display>(err: E) -> DomainError {
    DomainError::Internal(err.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::tasks::TaskService;
    use tempfile::tempdir;

    #[test]
    fn create_list_restore_roundtrip() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("workbench.db");
        let backup_dir = dir.path().join("backups");
        let db = Database::open(&db_path).unwrap();
        TaskService::new(db.clone()).ensure_seed_data().unwrap();
        {
            let conn = db.connect().unwrap();
            conn.execute(
                "INSERT INTO smoke_notes (id, body, created_at, updated_at, revision, deleted_at)
                 VALUES ('n1', 'hello-backup', 't', 't', 1, NULL)",
                [],
            )
            .unwrap();
        }

        let svc = BackupService::new(db.clone(), backup_dir);
        let info = svc.create("manual").unwrap();
        assert!(Path::new(&info.path).exists());
        assert_eq!(svc.list().unwrap().len(), 1);

        {
            let conn = db.connect().unwrap();
            conn.execute("DELETE FROM smoke_notes", []).unwrap();
        }
        svc.restore(&info.file_name).unwrap();
        let body: String = db
            .connect()
            .unwrap()
            .query_row("SELECT body FROM smoke_notes WHERE id = 'n1'", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(body, "hello-backup");
    }

    #[test]
    fn restore_rejects_corrupt_backup() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("workbench.db");
        let backup_dir = dir.path().join("backups");
        std::fs::create_dir_all(&backup_dir).unwrap();
        let db = Database::open(&db_path).unwrap();
        TaskService::new(db.clone()).ensure_seed_data().unwrap();
        {
            let conn = db.connect().unwrap();
            conn.execute(
                "INSERT INTO smoke_notes (id, body, created_at, updated_at, revision, deleted_at)
                 VALUES ('keep-me', 'intact', 't', 't', 1, NULL)",
                [],
            )
            .unwrap();
        }

        let corrupt = backup_dir.join("workbench-corrupt-20260101-000000.db");
        std::fs::write(&corrupt, [0xDE, 0xAD, 0xBE, 0xEF, 0x00, 0x01, 0x02, 0x03]).unwrap();

        let svc = BackupService::new(db.clone(), backup_dir);
        let err = svc
            .restore("workbench-corrupt-20260101-000000.db")
            .unwrap_err();
        assert!(
            matches!(err, DomainError::Validation(_)),
            "expected Validation error, got {err:?}"
        );

        // Target DB is untouched by the rejected restore.
        let count: i64 = db
            .connect()
            .unwrap()
            .query_row("SELECT COUNT(*) FROM smoke_notes", [], |row| row.get(0))
            .unwrap();
        assert_eq!(count, 1);
        let body: String = db
            .connect()
            .unwrap()
            .query_row(
                "SELECT body FROM smoke_notes WHERE id = 'keep-me'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(body, "intact");
    }

    #[test]
    fn restore_old_schema_backup_then_migrates() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("workbench.db");
        let backup_dir = dir.path().join("backups");
        std::fs::create_dir_all(&backup_dir).unwrap();

        // Build a genuine v2-schema backup (migrations 1..=2 only).
        let old_backup = backup_dir.join("workbench-old-v2-20260101-000000.db");
        {
            let conn = Connection::open(&old_backup).unwrap();
            conn.execute_batch(include_str!("../../migrations/0001_init.sql"))
                .unwrap();
            conn.execute_batch(include_str!("../../migrations/0002_tasks.sql"))
                .unwrap();
            conn.execute(
                "INSERT INTO schema_migrations (version, applied_at) VALUES (1, 't'), (2, 't')",
                [],
            )
            .unwrap();
            conn.execute(
                "INSERT INTO task_lists (id, name, kind, sort_order, created_at, updated_at, revision)
                 VALUES ('old-list', '旧清单', 'inbox', 0, 't', 't', 1)",
                [],
            )
            .unwrap();
        }

        let db = Database::open(&db_path).unwrap();
        assert_eq!(db.health_check().unwrap().schema_version, 8);

        let svc = BackupService::new(db.clone(), backup_dir);
        svc.restore("workbench-old-v2-20260101-000000.db").unwrap();

        // Restored DB is migrated up to the current schema.
        assert_eq!(db.health_check().unwrap().schema_version, 8);
        let name: String = db
            .connect()
            .unwrap()
            .query_row(
                "SELECT name FROM task_lists WHERE id = 'old-list'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(name, "旧清单");
    }
}
