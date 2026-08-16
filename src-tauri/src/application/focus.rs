use rusqlite::{params, Connection, OptionalExtension};

use crate::application::tasks::TaskService;
use crate::domain::{
    new_id, stamp, DomainError, EntityId, FocusOutcome, FocusSession, SystemClock,
};
use crate::infrastructure::db::Database;

pub struct FocusService {
    db: Database,
    clock: SystemClock,
}

impl FocusService {
    pub fn new(db: Database) -> Self {
        Self {
            db,
            clock: SystemClock,
        }
    }

    fn connect(&self) -> Result<Connection, DomainError> {
        self.db.connect().map_err(internal)
    }

    pub fn start(
        &self,
        tasks: &TaskService,
        task_id: EntityId,
        planned_minutes: Option<i64>,
    ) -> Result<FocusSession, DomainError> {
        let task = tasks.get_task(task_id)?;
        if task.status != crate::domain::TaskStatus::Todo {
            return Err(DomainError::Validation("只能专注待办任务".into()));
        }
        if let Some(minutes) = planned_minutes {
            if !(1..=480).contains(&minutes) {
                return Err(DomainError::Validation(
                    "专注时长需在 1–480 分钟之间".into(),
                ));
            }
        }

        let conn = self.connect()?;
        let now = stamp(&self.clock);
        let tx = conn.unchecked_transaction().map_err(internal)?;
        tx.execute(
            "UPDATE focus_sessions SET outcome = 'abandoned', ended_at = ?1, updated_at = ?1
             WHERE outcome = 'in_progress'",
            params![now],
        )
        .map_err(internal)?;

        let id = new_id();
        tx.execute(
            "INSERT INTO focus_sessions
             (id, task_id, started_at, ended_at, planned_minutes, outcome, progress_note, created_at, updated_at)
             VALUES (?1, ?2, ?3, NULL, ?4, 'in_progress', NULL, ?3, ?3)",
            params![
                id.to_string(),
                task_id.to_string(),
                now,
                planned_minutes,
            ],
        )
        .map_err(internal)?;
        tx.commit().map_err(internal)?;
        self.get(id)
    }

    pub fn end(
        &self,
        tasks: &TaskService,
        session_id: EntityId,
        outcome: FocusOutcome,
        progress_note: Option<String>,
    ) -> Result<FocusSession, DomainError> {
        if outcome == FocusOutcome::InProgress {
            return Err(DomainError::Validation(
                "结束专注时必须指定结果（completed / kept_todo / abandoned）".into(),
            ));
        }
        let session = self.get(session_id)?;
        if session.outcome != FocusOutcome::InProgress {
            return Err(DomainError::Validation("专注会话已结束".into()));
        }

        let note = progress_note
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());

        let conn = self.connect()?;
        let now = stamp(&self.clock);
        conn.execute(
            "UPDATE focus_sessions SET outcome = ?1, ended_at = ?2, progress_note = ?3, updated_at = ?2
             WHERE id = ?4 AND outcome = 'in_progress'",
            params![
                outcome.as_str(),
                now,
                note,
                session_id.to_string(),
            ],
        )
        .map_err(internal)?;

        if outcome == FocusOutcome::Completed {
            tasks.complete_task(session.task_id)?;
        }

        self.get(session_id)
    }

    pub fn abandon_active(&self) -> Result<Option<FocusSession>, DomainError> {
        let Some(active) = self.active()? else {
            return Ok(None);
        };
        self.end(
            &TaskService::new(self.db.clone()),
            active.id,
            FocusOutcome::Abandoned,
            None,
        )?;
        Ok(Some(self.get(active.id)?))
    }

    pub fn active(&self) -> Result<Option<FocusSession>, DomainError> {
        let conn = self.connect()?;
        conn.query_row(
            "SELECT id, task_id, started_at, ended_at, planned_minutes, outcome, progress_note, created_at, updated_at
             FROM focus_sessions WHERE outcome = 'in_progress'
             ORDER BY started_at DESC LIMIT 1",
            [],
            map_focus_session,
        )
        .optional()
        .map_err(internal)
    }

    pub fn get(&self, id: EntityId) -> Result<FocusSession, DomainError> {
        let conn = self.connect()?;
        conn.query_row(
            "SELECT id, task_id, started_at, ended_at, planned_minutes, outcome, progress_note, created_at, updated_at
             FROM focus_sessions WHERE id = ?1",
            [id.to_string()],
            map_focus_session,
        )
        .optional()
        .map_err(internal)?
        .ok_or_else(|| DomainError::NotFound("专注会话不存在".into()))
    }

    pub fn list(
        &self,
        task_id: Option<EntityId>,
        limit: Option<i64>,
    ) -> Result<Vec<FocusSession>, DomainError> {
        let limit = limit.unwrap_or(50).clamp(1, 200);
        let conn = self.connect()?;
        let mut out = Vec::new();
        if let Some(task_id) = task_id {
            let mut stmt = conn
                .prepare(
                    "SELECT id, task_id, started_at, ended_at, planned_minutes, outcome, progress_note, created_at, updated_at
                     FROM focus_sessions WHERE task_id = ?1
                     ORDER BY started_at DESC LIMIT ?2",
                )
                .map_err(internal)?;
            let rows = stmt
                .query_map(params![task_id.to_string(), limit], map_focus_session)
                .map_err(internal)?;
            for row in rows {
                out.push(row.map_err(internal)?);
            }
        } else {
            let mut stmt = conn
                .prepare(
                    "SELECT id, task_id, started_at, ended_at, planned_minutes, outcome, progress_note, created_at, updated_at
                     FROM focus_sessions ORDER BY started_at DESC LIMIT ?1",
                )
                .map_err(internal)?;
            let rows = stmt.query_map([limit], map_focus_session).map_err(internal)?;
            for row in rows {
                out.push(row.map_err(internal)?);
            }
        }
        Ok(out)
    }
}

fn map_focus_session(row: &rusqlite::Row<'_>) -> Result<FocusSession, rusqlite::Error> {
    let outcome_str: String = row.get(5)?;
    let outcome = FocusOutcome::parse(&outcome_str).map_err(|_| {
        rusqlite::Error::InvalidColumnType(5, outcome_str, rusqlite::types::Type::Text)
    })?;
    Ok(FocusSession {
        id: parse_id(row.get(0)?)?,
        task_id: parse_id(row.get(1)?)?,
        started_at: row.get(2)?,
        ended_at: row.get(3)?,
        planned_minutes: row.get(4)?,
        outcome,
        progress_note: row.get(6)?,
        created_at: row.get(7)?,
        updated_at: row.get(8)?,
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
    use crate::domain::{CreateTaskInput, FocusOutcome, TaskStatus};
    use crate::infrastructure::db::Database;
    use tempfile::tempdir;

    fn open_services() -> (TaskService, FocusService) {
        let dir = tempdir().unwrap();
        let db = Database::open(dir.path().join("workbench.db")).unwrap();
        let tasks = TaskService::new(db.clone());
        tasks.ensure_seed_data().unwrap();
        std::mem::forget(dir);
        (tasks, FocusService::new(db))
    }

    #[test]
    fn focus_start_end_completed_marks_task_done() {
        let (tasks, focus) = open_services();
        let task = tasks
            .create_task(CreateTaskInput {
                title: "focus target".into(),
                notes: None,
                priority: None,
                list_id: None,
                due_date: None,
                due_time: None,
                tag_names: None,
            })
            .unwrap();

        let session = focus.start(&tasks, task.id, Some(25)).unwrap();
        assert_eq!(session.outcome, FocusOutcome::InProgress);

        focus
            .end(&tasks, session.id, FocusOutcome::Completed, None)
            .unwrap();
        assert_eq!(
            tasks.get_task(task.id).unwrap().status,
            TaskStatus::Completed
        );
    }

    #[test]
    fn focus_start_abandons_previous_in_progress() {
        let (tasks, focus) = open_services();
        let t1 = tasks
            .create_task(CreateTaskInput {
                title: "one".into(),
                notes: None,
                priority: None,
                list_id: None,
                due_date: None,
                due_time: None,
                tag_names: None,
            })
            .unwrap();
        let t2 = tasks
            .create_task(CreateTaskInput {
                title: "two".into(),
                notes: None,
                priority: None,
                list_id: None,
                due_date: None,
                due_time: None,
                tag_names: None,
            })
            .unwrap();

        let s1 = focus.start(&tasks, t1.id, None).unwrap();
        let _s2 = focus.start(&tasks, t2.id, None).unwrap();
        let ended = focus.get(s1.id).unwrap();
        assert_eq!(ended.outcome, FocusOutcome::Abandoned);
        assert!(focus.active().unwrap().is_some());
    }

    #[test]
    fn focus_end_kept_todo_leaves_task_open() {
        let (tasks, focus) = open_services();
        let task = tasks
            .create_task(CreateTaskInput {
                title: "keep".into(),
                notes: None,
                priority: None,
                list_id: None,
                due_date: None,
                due_time: None,
                tag_names: None,
            })
            .unwrap();
        let session = focus.start(&tasks, task.id, None).unwrap();
        focus
            .end(
                &tasks,
                session.id,
                FocusOutcome::KeptTodo,
                Some("made progress".into()),
            )
            .unwrap();
        assert_eq!(tasks.get_task(task.id).unwrap().status, TaskStatus::Todo);
        let ended = focus.get(session.id).unwrap();
        assert_eq!(ended.progress_note.as_deref(), Some("made progress"));
    }
}
