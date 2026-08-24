use chrono::{Duration, NaiveDate};
use rusqlite::{params, Connection, OptionalExtension};
use std::path::{Path, PathBuf};

use crate::application::assets::AssetStore;
use crate::application::backup::BackupService;
use crate::application::tasks::TaskService;
use crate::domain::{
    local_today, stamp, ClipboardHealthStats, DailyCompletionCount, DomainError,
    HealthBackupSummary, HealthDashboardSnapshot, ReminderOutcomeStats, StorageBreakdown,
    StorageGcPreview, SystemClock, TaskHealthStats,
};
use crate::infrastructure::db::Database;
use crate::infrastructure::settings::SettingsService;

const STALE_DAYS: i64 = 14;
const COMPLETION_TREND_DAYS: i64 = 7;

pub struct HealthDashboardService {
    db: Database,
    assets_root: PathBuf,
    clock: SystemClock,
}

impl HealthDashboardService {
    pub fn new(db: Database, assets_root: PathBuf) -> Self {
        Self {
            db,
            assets_root,
            clock: SystemClock,
        }
    }

    pub fn snapshot(
        &self,
        backups: &BackupService,
        tasks: &TaskService,
        settings: &SettingsService,
    ) -> Result<HealthDashboardSnapshot, DomainError> {
        let conn = self.db.connect().map_err(internal)?;
        let today = local_today(&self.clock);
        let today_date = NaiveDate::parse_from_str(&today, "%Y-%m-%d")
            .map_err(|_| DomainError::Internal("invalid local today".into()))?;
        let now_s = crate::domain::local_now_naive()
            .format("%Y-%m-%dT%H:%M:%S")
            .to_string();

        let since_7 = (today_date - Duration::days(6))
            .format("%Y-%m-%d")
            .to_string();
        let since_30 = (today_date - Duration::days(29))
            .format("%Y-%m-%d")
            .to_string();

        let backup_status = backups.status();
        let backup = HealthBackupSummary {
            directory: backup_status.directory,
            count: backup_status.count,
            latest_created_at: backup_status.latest.map(|b| b.created_at),
            last_error: backup_status.last_error,
        };
        let backup_total_bytes = backups
            .list()
            .map(|items| items.iter().map(|b| b.size_bytes).sum())
            .unwrap_or(0);

        let storage = self.storage_breakdown(&conn)?;
        let reminders_7d = reminder_outcome_stats(&conn, &since_7, &now_s)?;
        let reminders_30d = reminder_outcome_stats(&conn, &since_30, &now_s)?;
        let task_stats = self.task_stats(&conn, tasks, &today_date)?;
        let app_settings = settings.get().map_err(internal)?;
        let clipboard = clipboard_stats(
            &conn,
            app_settings.clipboard_max_items,
            app_settings.clipboard_retention_days,
        )?;
        let asset_store = AssetStore::new(self.db.clone(), self.assets_root.clone());
        let gc = asset_store.gc_preview(app_settings.clipboard_retention_days)?;
        let storage_gc = StorageGcPreview {
            candidate_count: gc.candidate_count,
            candidate_bytes: gc.candidate_bytes,
            retention_days: gc.retention_days,
            note: "仅清理无剪切板/实体链接引用且超过保留期的孤儿图片资源；收藏与被引用的图片不受影响。"
                .into(),
        };

        Ok(HealthDashboardSnapshot {
            backup,
            backup_total_bytes,
            storage,
            storage_gc,
            reminders_7d,
            reminders_30d,
            tasks: task_stats,
            clipboard,
            generated_at: stamp(&self.clock),
        })
    }

    fn storage_breakdown(&self, conn: &Connection) -> Result<StorageBreakdown, DomainError> {
        let db_path = self.db.path();
        let database_bytes = file_size(db_path);
        let wal_bytes = file_size(&db_path.with_extension("db-wal"));
        let assets_bytes: i64 = conn
            .query_row(
                "SELECT COALESCE(SUM(byte_size), 0) FROM assets WHERE deleted_at IS NULL",
                [],
                |row| row.get(0),
            )
            .map_err(internal)?;
        let thumb_dir = self.assets_root.join("clipboard/thumbs");
        let thumb_bytes = dir_size(&thumb_dir);

        Ok(StorageBreakdown {
            database_bytes,
            wal_bytes,
            assets_bytes,
            thumb_bytes,
            assets_root: self.assets_root.display().to_string(),
            note: "数据库为主文件与 WAL 字节；资源为 assets 表 byte_size 合计；缩略图为磁盘 thumbs 目录文件合计；备份占用为备份目录内 .db 文件合计。".into(),
        })
    }

    fn task_stats(
        &self,
        conn: &Connection,
        tasks: &TaskService,
        today_date: &NaiveDate,
    ) -> Result<TaskHealthStats, DomainError> {
        let inbox_count: i64 = conn
            .query_row(
                "SELECT COUNT(*)
                 FROM tasks t
                 JOIN task_lists l ON l.id = t.list_id AND l.deleted_at IS NULL
                 WHERE t.deleted_at IS NULL AND t.status = 'todo' AND l.kind = 'inbox'",
                [],
                |row| row.get(0),
            )
            .map_err(internal)?;

        let inbox_oldest_days = conn
            .query_row(
                "SELECT MIN(t.created_at)
                 FROM tasks t
                 JOIN task_lists l ON l.id = t.list_id AND l.deleted_at IS NULL
                 WHERE t.deleted_at IS NULL AND t.status = 'todo' AND l.kind = 'inbox'",
                [],
                |row| row.get::<_, Option<String>>(0),
            )
            .optional()
            .map_err(internal)?
            .flatten()
            .map(|iso| days_since_iso(&iso, today_date));

        let stale_page = tasks.query_stale_active(STALE_DAYS, Some(1), None)?;
        let stale_active_count = stale_page.total;

        let trend_start = (*today_date - Duration::days(COMPLETION_TREND_DAYS - 1))
            .format("%Y-%m-%d")
            .to_string();
        let mut stmt = conn
            .prepare(
                "SELECT date(completed_at, 'localtime') AS d, COUNT(*)
                 FROM tasks
                 WHERE deleted_at IS NULL
                   AND status = 'completed'
                   AND completed_at IS NOT NULL
                   AND date(completed_at, 'localtime') >= date(?1, 'localtime')
                 GROUP BY d
                 ORDER BY d",
            )
            .map_err(internal)?;
        let rows: Vec<(String, i64)> = stmt
            .query_map([&trend_start], |row| Ok((row.get(0)?, row.get(1)?)))
            .map_err(internal)?
            .collect::<Result<_, _>>()
            .map_err(internal)?;
        drop(stmt);

        let mut counts = std::collections::HashMap::new();
        for (date, count) in rows {
            counts.insert(date, count);
        }
        let mut completion_trend = Vec::with_capacity(COMPLETION_TREND_DAYS as usize);
        for offset in 0..COMPLETION_TREND_DAYS {
            let day = (*today_date - Duration::days(COMPLETION_TREND_DAYS - 1 - offset))
                .format("%Y-%m-%d")
                .to_string();
            completion_trend.push(DailyCompletionCount {
                date: day.clone(),
                count: *counts.get(&day).unwrap_or(&0),
            });
        }

        Ok(TaskHealthStats {
            inbox_count,
            inbox_oldest_days,
            stale_active_count,
            completion_trend,
        })
    }
}

fn reminder_outcome_stats(
    conn: &Connection,
    since_date: &str,
    now_s: &str,
) -> Result<ReminderOutcomeStats, DomainError> {
    conn.query_row(
        "SELECT
            COALESCE(SUM(CASE
                WHEN o.status = 'actioned' AND o.snooze_until IS NULL THEN 1
                ELSE 0 END), 0),
            COALESCE(SUM(CASE
                WHEN o.status = 'snoozed'
                  OR (o.status = 'actioned' AND o.snooze_until IS NOT NULL) THEN 1
                ELSE 0 END), 0),
            COALESCE(SUM(CASE
                WHEN o.status = 'inferred_missed' THEN 1
                ELSE 0 END), 0),
            COALESCE(SUM(CASE
                WHEN o.status IN ('pending', 'scheduled') AND o.scheduled_at < ?2 THEN 1
                ELSE 0 END), 0)
         FROM reminder_occurrences o
         JOIN reminders r ON r.id = o.reminder_id AND r.deleted_at IS NULL
         WHERE date(o.scheduled_at, 'localtime') >= date(?1, 'localtime')
           AND o.status != 'cancelled'",
        params![since_date, now_s],
        |row| {
            Ok(ReminderOutcomeStats {
                on_time: row.get(0)?,
                snoozed: row.get(1)?,
                missed: row.get(2)?,
                pending_overdue: row.get(3)?,
            })
        },
    )
    .map_err(internal)
}

fn clipboard_stats(
    conn: &Connection,
    max_items: u32,
    retention_days: u32,
) -> Result<ClipboardHealthStats, DomainError> {
    let total_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM clipboard_items WHERE deleted_at IS NULL",
            [],
            |row| row.get(0),
        )
        .map_err(internal)?;
    let favorite_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM clipboard_items WHERE deleted_at IS NULL AND favorite = 1",
            [],
            |row| row.get(0),
        )
        .map_err(internal)?;
    let remaining_slots = (max_items as i64 - total_count).max(0);

    Ok(ClipboardHealthStats {
        total_count,
        favorite_count,
        max_items,
        retention_days,
        remaining_slots,
    })
}

fn file_size(path: &Path) -> u64 {
    std::fs::metadata(path).map(|m| m.len()).unwrap_or(0)
}

fn dir_size(path: &Path) -> u64 {
    let mut total = 0u64;
    let Ok(read_dir) = std::fs::read_dir(path) else {
        return 0;
    };
    for entry in read_dir.flatten() {
        let p = entry.path();
        if p.is_file() {
            total += file_size(&p);
        }
    }
    total
}

fn days_since_iso(iso: &str, today: &NaiveDate) -> i64 {
    let prefix = iso.get(..10).unwrap_or(iso);
    NaiveDate::parse_from_str(prefix, "%Y-%m-%d")
        .ok()
        .map(|d| (*today - d).num_days().max(0))
        .unwrap_or(0)
}

fn internal<E: std::fmt::Display>(err: E) -> DomainError {
    DomainError::Internal(err.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::backup::BackupService;
    use crate::application::tasks::TaskService;
    use crate::domain::{CreateTaskInput, EntityId, OccurrenceStatus};
    use crate::infrastructure::settings::SettingsService;
    use tempfile::tempdir;

    fn open_services() -> (
        Database,
        PathBuf,
        BackupService,
        TaskService,
        SettingsService,
        HealthDashboardService,
    ) {
        let dir = tempdir().unwrap();
        let assets_root = dir.path().join("assets");
        std::fs::create_dir_all(&assets_root).unwrap();
        let backup_dir = dir.path().join("backups");
        let db = Database::open(dir.path().join("workbench.db")).unwrap();
        let tasks = TaskService::new(db.clone());
        tasks.ensure_seed_data().unwrap();
        std::mem::forget(dir);
        let backups = BackupService::new(db.clone(), backup_dir);
        let settings = SettingsService::new(db.clone());
        let health = HealthDashboardService::new(db.clone(), assets_root.clone());
        (db, assets_root, backups, tasks, settings, health)
    }

    fn insert_occurrence(
        conn: &Connection,
        reminder_id: EntityId,
        scheduled_at: &str,
        status: OccurrenceStatus,
        snooze_until: Option<&str>,
    ) {
        let id = crate::domain::new_id();
        let now = "2026-08-16T10:00:00";
        conn.execute(
            "INSERT INTO reminder_occurrences (
                id, reminder_id, scheduled_at, status, needs_schedule,
                system_notification_id, actioned_at, snooze_until,
                created_at, updated_at, revision
             ) VALUES (?1, ?2, ?3, ?4, 0, NULL, NULL, ?5, ?6, ?6, 1)",
            params![
                id.to_string(),
                reminder_id.to_string(),
                scheduled_at,
                status.as_str(),
                snooze_until,
                now
            ],
        )
        .unwrap();
    }

    #[test]
    fn health_dashboard_snapshot_basic() {
        let (_db, _assets, backups, tasks, settings, health) = open_services();
        let _task = tasks
            .create_task(CreateTaskInput {
                title: "inbox backlog".into(),
                notes: None,
                priority: None,
                list_id: None,
                due_date: None,
                due_time: None,
                tag_names: None,
            })
            .unwrap();

        let snap = health.snapshot(&backups, &tasks, &settings).unwrap();
        assert!(snap.tasks.inbox_count >= 1);
        assert_eq!(snap.reminders_7d.on_time, 0);
        assert!(snap.storage.note.contains("WAL"));
    }

    #[test]
    fn reminder_stats_respect_local_date_window() {
        let (db, _, backups, tasks, settings, health) = open_services();
        let conn = db.connect().unwrap();
        let reminder_id = crate::domain::new_id();
        conn.execute(
            "INSERT INTO reminders (
                id, title, notes, task_id, recurrence_json, timezone, next_fire_at, end_at,
                enabled, created_at, updated_at, revision, deleted_at
             ) VALUES (?1, 't', '', NULL, NULL, 'Asia/Shanghai', '2026-08-10T09:00:00', NULL,
                       1, '2026-08-10T09:00:00', '2026-08-10T09:00:00', 1, NULL)",
            [reminder_id.to_string()],
        )
        .unwrap();

        // Dates are relative to "today" so the 7/30-day windows hold on any
        // run date (this test previously hardcoded 2026-08 dates and flaked).
        let fmt_day = |offset: i64| {
            use chrono::{Datelike, Duration, Local};
            let day = (Local::now().date_naive() - Duration::days(offset)).format("%Y-%m-%d");
            format!("{day}T09:00:00")
        };
        insert_occurrence(&conn, reminder_id, &fmt_day(1), OccurrenceStatus::Actioned, None);
        insert_occurrence(&conn, reminder_id, &fmt_day(3), OccurrenceStatus::InferredMissed, None);
        insert_occurrence(&conn, reminder_id, &fmt_day(12), OccurrenceStatus::Actioned, None);

        let snap = health.snapshot(&backups, &tasks, &settings).unwrap();
        assert_eq!(snap.reminders_7d.on_time, 1);
        assert_eq!(snap.reminders_7d.missed, 1);
        assert_eq!(snap.reminders_30d.on_time, 2);
    }
}
