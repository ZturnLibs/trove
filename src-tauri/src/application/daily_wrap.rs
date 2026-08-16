use chrono::NaiveDate;
use rusqlite::{params, Connection, OptionalExtension};

use crate::application::tasks::TaskService;
use crate::domain::{
    local_today, new_id, stamp, DailyWrapCompleteInput, DailyWrapRun, DailyWrapSnapshot,
    DomainError, EntityId, ListKind, SystemClock, Task, TaskQuery, TaskStatus,
};
use crate::infrastructure::db::Database;

pub struct DailyWrapService {
    db: Database,
    clock: SystemClock,
}

impl DailyWrapService {
    pub fn new(db: Database) -> Self {
        Self {
            db,
            clock: SystemClock,
        }
    }

    fn connect(&self) -> Result<Connection, DomainError> {
        self.db.connect().map_err(internal)
    }

    fn normalize_wrap_date(&self, wrap_date: Option<String>) -> Result<String, DomainError> {
        match wrap_date {
            None => Ok(local_today(&self.clock)),
            Some(s) if s.trim().is_empty() => Ok(local_today(&self.clock)),
            Some(s) => {
                NaiveDate::parse_from_str(s.trim(), "%Y-%m-%d")
                    .map_err(|_| DomainError::Validation("wrapDate must be YYYY-MM-DD".into()))?;
                Ok(s.trim().to_string())
            }
        }
    }

    pub fn snapshot(
        &self,
        tasks: &TaskService,
        wrap_date: Option<String>,
    ) -> Result<DailyWrapSnapshot, DomainError> {
        let wrap_date = self.normalize_wrap_date(wrap_date)?;
        let today = local_today(&self.clock);
        let tomorrow = (NaiveDate::parse_from_str(&today, "%Y-%m-%d")
            .map_err(|_| DomainError::Internal("invalid local today".into()))?
            + chrono::Duration::days(1))
        .format("%Y-%m-%d")
        .to_string();

        let today_view = tasks.today_tasks()?;
        let unfinished_focus: Vec<Task> = today_view
            .focus
            .into_iter()
            .filter(|t| t.status == TaskStatus::Todo)
            .collect();

        let tomorrow_due = tasks
            .query_tasks(TaskQuery {
                status: Some(TaskStatus::Todo),
                due_from: Some(tomorrow.clone()),
                due_to: Some(tomorrow),
                limit: Some(100),
                ..Default::default()
            })?
            .items;

        let inbox_unprocessed = tasks
            .query_tasks(TaskQuery {
                status: Some(TaskStatus::Todo),
                inbox_only: Some(true),
                limit: Some(50),
                ..Default::default()
            })?
            .items
            .into_iter()
            .filter(|t| t.list_kind == ListKind::Inbox)
            .collect();

        Ok(DailyWrapSnapshot {
            wrap_date,
            unfinished_focus,
            tomorrow_due,
            inbox_unprocessed,
            completed_today_count: today_view.completed_today.len() as i64,
            reminders_today_count: today_view.reminders_today.len() as i64,
        })
    }

    pub fn start(&self, wrap_date: Option<String>) -> Result<DailyWrapRun, DomainError> {
        let wrap_date = self.normalize_wrap_date(wrap_date)?;
        let conn = self.connect()?;
        let now = stamp(&self.clock);
        let id = new_id();
        conn.execute(
            "INSERT INTO daily_wrap_runs (id, wrap_date, started_at, completed_at, steps_completed, summary_json, created_at)
             VALUES (?1, ?2, ?3, NULL, 0, NULL, ?3)",
            params![id.to_string(), wrap_date, now],
        )
        .map_err(internal)?;
        self.get(id)
    }

    pub fn complete(
        &self,
        run_id: EntityId,
        input: DailyWrapCompleteInput,
    ) -> Result<DailyWrapRun, DomainError> {
        let conn = self.connect()?;
        let now = stamp(&self.clock);
        let summary = input
            .summary
            .map(|v| serde_json::to_string(&v).map_err(internal))
            .transpose()?;
        let rows = conn
            .execute(
                "UPDATE daily_wrap_runs SET completed_at = ?1, steps_completed = ?2, summary_json = ?3
                 WHERE id = ?4 AND completed_at IS NULL",
                params![
                    now,
                    input.steps_completed,
                    summary,
                    run_id.to_string(),
                ],
            )
            .map_err(internal)?;
        if rows == 0 {
            return Err(DomainError::NotFound("收尾记录不存在或已完成".into()));
        }
        self.get(run_id)
    }

    pub fn completed_for_date(&self, wrap_date: Option<String>) -> Result<Option<DailyWrapRun>, DomainError> {
        let wrap_date = self.normalize_wrap_date(wrap_date)?;
        let conn = self.connect()?;
        conn.query_row(
            "SELECT id, wrap_date, started_at, completed_at, steps_completed, summary_json, created_at
             FROM daily_wrap_runs WHERE wrap_date = ?1 AND completed_at IS NOT NULL
             ORDER BY completed_at DESC LIMIT 1",
            [wrap_date],
            map_daily_wrap_run,
        )
        .optional()
        .map_err(internal)
    }

    pub fn get(&self, id: EntityId) -> Result<DailyWrapRun, DomainError> {
        let conn = self.connect()?;
        conn.query_row(
            "SELECT id, wrap_date, started_at, completed_at, steps_completed, summary_json, created_at
             FROM daily_wrap_runs WHERE id = ?1",
            [id.to_string()],
            map_daily_wrap_run,
        )
        .optional()
        .map_err(internal)?
        .ok_or_else(|| DomainError::NotFound("收尾记录不存在".into()))
    }
}

fn map_daily_wrap_run(row: &rusqlite::Row<'_>) -> Result<DailyWrapRun, rusqlite::Error> {
    let summary_raw: Option<String> = row.get(5)?;
    let summary = summary_raw
        .as_deref()
        .map(serde_json::from_str)
        .transpose()
        .map_err(|e| {
            rusqlite::Error::FromSqlConversionFailure(5, rusqlite::types::Type::Text, Box::new(e))
        })?;
    Ok(DailyWrapRun {
        id: parse_id(row.get(0)?)?,
        wrap_date: row.get(1)?,
        started_at: row.get(2)?,
        completed_at: row.get(3)?,
        steps_completed: row.get(4)?,
        summary,
        created_at: row.get(6)?,
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
    use crate::application::tasks::TaskService;
    use crate::domain::{CreateTaskInput, DailyWrapCompleteInput};
    use crate::infrastructure::db::Database;
    use tempfile::tempdir;

    fn open_services() -> (TaskService, DailyWrapService) {
        let dir = tempdir().unwrap();
        let db = Database::open(dir.path().join("workbench.db")).unwrap();
        let tasks = TaskService::new(db.clone());
        tasks.ensure_seed_data().unwrap();
        std::mem::forget(dir);
        (tasks, DailyWrapService::new(db))
    }

    #[test]
    fn daily_wrap_snapshot_and_complete() {
        let (tasks, wrap) = open_services();
        let today = local_today(&SystemClock);
        let task = tasks
            .create_task(CreateTaskInput {
                title: "focus item".into(),
                notes: None,
                priority: None,
                list_id: None,
                due_date: Some(today.clone()),
                due_time: None,
                tag_names: None,
            })
            .unwrap();
        tasks.daily_focus_add(task.id, Some(today.clone())).unwrap();

        let snap = wrap.snapshot(&tasks, Some(today.clone())).unwrap();
        assert_eq!(snap.unfinished_focus.len(), 1);

        let run = wrap.start(Some(today.clone())).unwrap();
        let done = wrap
            .complete(
                run.id,
                DailyWrapCompleteInput {
                    steps_completed: 5,
                    summary: Some(serde_json::json!({ "completedFocus": 0 })),
                },
            )
            .unwrap();
        assert!(done.completed_at.is_some());
        assert!(wrap.completed_for_date(Some(today)).unwrap().is_some());
    }
}
