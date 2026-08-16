use chrono::NaiveDate;
use rusqlite::{params, Connection, OptionalExtension};

use crate::application::clipboard::ClipboardService;
use crate::application::reminders::ReminderService;
use crate::application::tasks::TaskService;
use crate::domain::{
    local_today, new_id, stamp, ReviewCompleteInput, ReviewSession, ReviewType,
    SmartListKind, SystemClock, TaskQuery, TaskStatus, WeeklyReviewSnapshot,
    DomainError, EntityId, ListKind,
};
use crate::infrastructure::db::Database;

const PREVIEW_LIMIT: i64 = 10;
const STALE_DAYS: i64 = 14;
const COMPLETED_LOOKBACK_DAYS: i64 = 7;
const RECURRING_LOOKAHEAD_DAYS: i64 = 7;
const LARGE_CLIPBOARD_MIN_BYTES: i64 = 512_000;

pub struct WeeklyReviewService {
    db: Database,
    clock: SystemClock,
}

impl WeeklyReviewService {
    pub fn new(db: Database) -> Self {
        Self {
            db,
            clock: SystemClock,
        }
    }

    fn connect(&self) -> Result<Connection, DomainError> {
        self.db.connect().map_err(internal)
    }

    pub fn snapshot(
        &self,
        tasks: &TaskService,
        reminders: &ReminderService,
        clipboard: &ClipboardService,
    ) -> Result<WeeklyReviewSnapshot, DomainError> {
        let today = local_today(&self.clock);
        let today_date = NaiveDate::parse_from_str(&today, "%Y-%m-%d")
            .map_err(|_| DomainError::Internal("invalid local today".into()))?;

        let inbox_page = tasks.query_tasks(TaskQuery {
            status: Some(TaskStatus::Todo),
            inbox_only: Some(true),
            limit: Some(PREVIEW_LIMIT),
            ..Default::default()
        })?;
        let inbox_count = inbox_page.total;

        let overdue_page = tasks.smart_list(SmartListKind::Overdue, Some(PREVIEW_LIMIT), None)?;
        let overdue_count = overdue_page.total;

        let waiting_page =
            tasks.smart_list(SmartListKind::WaitingFollowUp, Some(PREVIEW_LIMIT), None)?;
        let waiting_follow_up_count = waiting_page.total;

        let stale_page = tasks.query_stale_active(STALE_DAYS, Some(PREVIEW_LIMIT), None)?;
        let stale_active_count = stale_page.total;

        let completed_since = (today_date - chrono::Duration::days(COMPLETED_LOOKBACK_DAYS))
            .format("%Y-%m-%d")
            .to_string();
        let completed_page = tasks.query_tasks(TaskQuery {
            completed_since: Some(completed_since),
            limit: Some(PREVIEW_LIMIT),
            ..Default::default()
        })?;
        let completed_last_7_days_count = completed_page.total;

        let upcoming = reminders.upcoming_recurring(RECURRING_LOOKAHEAD_DAYS)?;
        let upcoming_recurring_count = upcoming.len() as i64;
        let upcoming_recurring_reminders = upcoming
            .into_iter()
            .take(PREVIEW_LIMIT as usize)
            .collect();

        let large_page =
            clipboard.query_large_unfavorited(LARGE_CLIPBOARD_MIN_BYTES, PREVIEW_LIMIT)?;
        let large_clipboard_count = large_page.total;

        Ok(WeeklyReviewSnapshot {
            inbox_unprocessed: inbox_page
                .items
                .into_iter()
                .filter(|t| t.list_kind == ListKind::Inbox)
                .collect(),
            inbox_count,
            overdue: overdue_page.items,
            overdue_count,
            waiting_follow_up: waiting_page.items,
            waiting_follow_up_count,
            stale_active: stale_page.items,
            stale_active_count,
            completed_last_7_days: completed_page.items,
            completed_last_7_days_count,
            upcoming_recurring_reminders,
            upcoming_recurring_count,
            large_clipboard_items: large_page.items,
            large_clipboard_count,
        })
    }

    pub fn start(&self, review_type: ReviewType) -> Result<ReviewSession, DomainError> {
        let conn = self.connect()?;
        let now = stamp(&self.clock);
        let id = new_id();
        conn.execute(
            "INSERT INTO review_sessions (id, review_type, started_at, completed_at, summary_json, created_at)
             VALUES (?1, ?2, ?3, NULL, NULL, ?3)",
            params![id.to_string(), review_type.as_str(), now],
        )
        .map_err(internal)?;
        self.get(id)
    }

    pub fn complete(
        &self,
        session_id: EntityId,
        input: ReviewCompleteInput,
    ) -> Result<ReviewSession, DomainError> {
        let conn = self.connect()?;
        let now = stamp(&self.clock);
        let summary = input
            .summary
            .map(|v| serde_json::to_string(&v).map_err(internal))
            .transpose()?;
        let rows = conn
            .execute(
                "UPDATE review_sessions SET completed_at = ?1, summary_json = ?2
                 WHERE id = ?3 AND completed_at IS NULL",
                params![now, summary, session_id.to_string()],
            )
            .map_err(internal)?;
        if rows == 0 {
            return Err(DomainError::NotFound("回顾会话不存在或已完成".into()));
        }
        self.get(session_id)
    }

    pub fn last_completed(&self, review_type: ReviewType) -> Result<Option<ReviewSession>, DomainError> {
        let conn = self.connect()?;
        conn.query_row(
            "SELECT id, review_type, started_at, completed_at, summary_json, created_at
             FROM review_sessions
             WHERE review_type = ?1 AND completed_at IS NOT NULL
             ORDER BY completed_at DESC LIMIT 1",
            [review_type.as_str()],
            map_review_session,
        )
        .optional()
        .map_err(internal)
    }

    pub fn get(&self, id: EntityId) -> Result<ReviewSession, DomainError> {
        let conn = self.connect()?;
        conn.query_row(
            "SELECT id, review_type, started_at, completed_at, summary_json, created_at
             FROM review_sessions WHERE id = ?1",
            [id.to_string()],
            map_review_session,
        )
        .optional()
        .map_err(internal)?
        .ok_or_else(|| DomainError::NotFound("回顾会话不存在".into()))
    }
}

fn map_review_session(row: &rusqlite::Row<'_>) -> Result<ReviewSession, rusqlite::Error> {
    let review_type_str: String = row.get(1)?;
    let review_type = ReviewType::parse(&review_type_str).map_err(|_| {
        rusqlite::Error::InvalidColumnType(1, review_type_str, rusqlite::types::Type::Text)
    })?;
    let summary_raw: Option<String> = row.get(4)?;
    let summary = summary_raw
        .as_deref()
        .map(serde_json::from_str)
        .transpose()
        .map_err(|e| {
            rusqlite::Error::FromSqlConversionFailure(4, rusqlite::types::Type::Text, Box::new(e))
        })?;
    Ok(ReviewSession {
        id: parse_id(row.get(0)?)?,
        review_type,
        started_at: row.get(2)?,
        completed_at: row.get(3)?,
        summary,
        created_at: row.get(5)?,
    })
}

fn parse_id(value: String) -> Result<EntityId, rusqlite::Error> {
    value.parse().map_err(|e| {
        rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(e))
    })
}

fn internal<E: std::fmt::Display>(err: E) -> DomainError {
    DomainError::Internal(err.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::clipboard::ClipboardService;
    use crate::application::reminders::ReminderService;
    use crate::application::tasks::TaskService;
    use crate::domain::{CreateTaskInput, ReviewCompleteInput};
    use crate::infrastructure::db::Database;
    use tempfile::tempdir;

    fn open_services() -> (TaskService, ReminderService, ClipboardService, WeeklyReviewService) {
        let dir = tempdir().unwrap();
        let assets_root = dir.path().join("assets");
        std::fs::create_dir_all(&assets_root).unwrap();
        let db = Database::open(dir.path().join("workbench.db")).unwrap();
        let tasks = TaskService::new(db.clone());
        tasks.ensure_seed_data().unwrap();
        std::mem::forget(dir);
        (
            tasks,
            ReminderService::new(db.clone()),
            ClipboardService::new(db.clone(), assets_root),
            WeeklyReviewService::new(db),
        )
    }

    #[test]
    fn weekly_review_snapshot_and_complete() {
        let (tasks, reminders, clipboard, review) = open_services();
        let today = local_today(&SystemClock);
        let _task = tasks
            .create_task(CreateTaskInput {
                title: "inbox item".into(),
                notes: None,
                priority: None,
                list_id: None,
                due_date: None,
                due_time: None,
                tag_names: None,
            })
            .unwrap();

        let snap = review
            .snapshot(&tasks, &reminders, &clipboard)
            .unwrap();
        assert!(snap.inbox_count >= 1);

        let session = review.start(ReviewType::Weekly).unwrap();
        let done = review
            .complete(
                session.id,
                ReviewCompleteInput {
                    summary: Some(serde_json::json!({ "inboxCount": snap.inbox_count })),
                },
            )
            .unwrap();
        assert!(done.completed_at.is_some());
        assert!(review
            .last_completed(ReviewType::Weekly)
            .unwrap()
            .is_some());
        let _ = today;
    }
}
