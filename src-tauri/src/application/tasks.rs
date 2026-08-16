use crate::domain::{
    compute_today_sort_suggestions, local_today, new_id, stamp, validate_due_date,
    validate_due_time, CreateTaskInput, DeleteListResult, DomainError, EntityId,
    ListDeleteDisposition, ListKind, PagedResult, should_apply_active_list_filter, SmartListKind,
    SystemClock, Tag, Task, TaskList, TaskPriority, TaskQuery, TaskStatus, TaskWorkflowState,
    TodaySortSuggestions, TodayTasks, UpdateTaskInput, validate_due_vs_available,
};
use crate::domain::{page_limit, page_offset};
use crate::infrastructure::db::Database;
use rusqlite::{params, Connection, OptionalExtension};

pub struct TaskService {
    db: Database,
    clock: SystemClock,
}

impl TaskService {
    pub fn new(db: Database) -> Self {
        Self {
            db,
            clock: SystemClock,
        }
    }

    pub fn ensure_seed_data(&self) -> Result<(), DomainError> {
        let conn = self.connect()?;
        let inbox_exists: Option<i64> = conn
            .query_row(
                "SELECT 1 FROM task_lists WHERE kind = 'inbox' AND deleted_at IS NULL LIMIT 1",
                [],
                |row| row.get(0),
            )
            .optional()
            .map_err(internal)?;

        if inbox_exists.is_none() {
            let id = new_id();
            let now = stamp(&self.clock);
            conn.execute(
                "INSERT INTO task_lists (id, name, kind, sort_order, created_at, updated_at, revision)
                 VALUES (?1, '收件箱', 'inbox', 0, ?2, ?2, 1)",
                params![id.to_string(), now],
            )
            .map_err(internal)?;
        }
        Ok(())
    }

    fn connect(&self) -> Result<Connection, DomainError> {
        self.db.connect().map_err(internal)
    }

    pub fn list_lists(&self) -> Result<Vec<TaskList>, DomainError> {
        let conn = self.connect()?;
        let mut stmt = conn
            .prepare(
                "SELECT id, name, kind, sort_order, created_at, updated_at, revision
                 FROM task_lists
                 WHERE deleted_at IS NULL
                 ORDER BY CASE kind WHEN 'inbox' THEN 0 ELSE 1 END, sort_order, name",
            )
            .map_err(internal)?;
        let rows = stmt.query_map([], map_list_row).map_err(internal)?;
        collect_rows(rows)
    }

    pub fn create_list(&self, name: String) -> Result<TaskList, DomainError> {
        let name = name.trim().to_string();
        if name.is_empty() {
            return Err(DomainError::Validation("清单名称不能为空".into()));
        }
        let conn = self.connect()?;
        let id = new_id();
        let now = stamp(&self.clock);
        let sort_order: f64 = conn
            .query_row(
                "SELECT COALESCE(MAX(sort_order), 0) + 1 FROM task_lists WHERE deleted_at IS NULL",
                [],
                |row| row.get(0),
            )
            .unwrap_or(1.0);
        conn.execute(
            "INSERT INTO task_lists (id, name, kind, sort_order, created_at, updated_at, revision)
             VALUES (?1, ?2, 'custom', ?3, ?4, ?4, 1)",
            params![id.to_string(), name, sort_order, now],
        )
        .map_err(internal)?;
        self.get_list(id)
    }

    pub fn get_list(&self, id: EntityId) -> Result<TaskList, DomainError> {
        let conn = self.connect()?;
        conn.query_row(
            "SELECT id, name, kind, sort_order, created_at, updated_at, revision
             FROM task_lists WHERE id = ?1 AND deleted_at IS NULL",
            [id.to_string()],
            map_list_row,
        )
        .optional()
        .map_err(internal)?
        .ok_or_else(|| DomainError::NotFound("清单不存在".into()))
    }

    pub fn update_list(&self, id: EntityId, name: String) -> Result<TaskList, DomainError> {
        let list = self.get_list(id)?;
        if list.kind == ListKind::Inbox {
            return Err(DomainError::Validation("收件箱名称不可修改".into()));
        }
        let name = name.trim().to_string();
        if name.is_empty() {
            return Err(DomainError::Validation("清单名称不能为空".into()));
        }
        let now = stamp(&self.clock);
        let conn = self.connect()?;
        conn.execute(
            "UPDATE task_lists SET name = ?1, updated_at = ?2, revision = revision + 1
             WHERE id = ?3 AND deleted_at IS NULL",
            params![name, now, id.to_string()],
        )
        .map_err(internal)?;
        self.get_list(id)
    }

    pub fn count_list_todo_tasks(&self, list_id: EntityId) -> Result<i64, DomainError> {
        let _ = self.get_list(list_id)?;
        let conn = self.connect()?;
        conn.query_row(
            "SELECT COUNT(*) FROM tasks
             WHERE list_id = ?1 AND status = 'todo' AND deleted_at IS NULL",
            [list_id.to_string()],
            |row| row.get(0),
        )
        .map_err(internal)
    }

    pub fn delete_list(
        &self,
        id: EntityId,
        disposition: ListDeleteDisposition,
    ) -> Result<DeleteListResult, DomainError> {
        let list = self.get_list(id)?;
        if list.kind == ListKind::Inbox {
            return Err(DomainError::Validation("收件箱不可删除".into()));
        }
        let conn = self.connect()?;
        let now = stamp(&self.clock);
        let task_ids = self.task_ids_in_list(&conn, id)?;
        let mut archived_task_ids = Vec::new();

        match disposition {
            ListDeleteDisposition::MoveToInbox => {
                let inbox_id = self.inbox_list_id()?;
                conn.execute(
                    "UPDATE tasks SET list_id = ?1, updated_at = ?2, revision = revision + 1
                     WHERE list_id = ?3 AND deleted_at IS NULL",
                    params![inbox_id.to_string(), now, id.to_string()],
                )
                .map_err(internal)?;
            }
            ListDeleteDisposition::ArchiveTasks => {
                let mut stmt = conn
                    .prepare(
                        "SELECT id FROM tasks
                         WHERE list_id = ?1 AND status = 'todo' AND deleted_at IS NULL",
                    )
                    .map_err(internal)?;
                archived_task_ids = stmt
                    .query_map([id.to_string()], |row| {
                        let raw: String = row.get(0)?;
                        raw.parse().map_err(|e| {
                            rusqlite::Error::FromSqlConversionFailure(
                                0,
                                rusqlite::types::Type::Text,
                                Box::new(e),
                            )
                        })
                    })
                    .map_err(internal)?
                    .collect::<Result<Vec<_>, _>>()
                    .map_err(internal)?;
                conn.execute(
                    "UPDATE tasks SET status = 'archived', updated_at = ?1, revision = revision + 1
                     WHERE list_id = ?2 AND status = 'todo' AND deleted_at IS NULL",
                    params![now, id.to_string()],
                )
                .map_err(internal)?;
                let inbox_id = self.inbox_list_id()?;
                conn.execute(
                    "UPDATE tasks SET list_id = ?1, updated_at = ?2, revision = revision + 1
                     WHERE list_id = ?3 AND status != 'todo' AND deleted_at IS NULL",
                    params![inbox_id.to_string(), now, id.to_string()],
                )
                .map_err(internal)?;
            }
            ListDeleteDisposition::ForceDelete => {
                conn.execute(
                    "UPDATE tasks SET deleted_at = ?1, updated_at = ?1, revision = revision + 1
                     WHERE list_id = ?2 AND deleted_at IS NULL",
                    params![now, id.to_string()],
                )
                .map_err(internal)?;
            }
        }

        conn.execute(
            "UPDATE task_lists SET deleted_at = ?1, updated_at = ?1, revision = revision + 1
             WHERE id = ?2 AND deleted_at IS NULL",
            params![now, id.to_string()],
        )
        .map_err(internal)?;

        Ok(DeleteListResult {
            list_id: id,
            list_name: list.name,
            disposition,
            task_ids,
            archived_task_ids,
        })
    }

    pub fn undo_delete_list(&self, result: DeleteListResult) -> Result<TaskList, DomainError> {
        let conn = self.connect()?;
        let now = stamp(&self.clock);
        conn.execute(
            "UPDATE task_lists SET deleted_at = NULL, updated_at = ?1, revision = revision + 1
             WHERE id = ?2",
            params![now, result.list_id.to_string()],
        )
        .map_err(internal)?;

        match result.disposition {
            ListDeleteDisposition::MoveToInbox | ListDeleteDisposition::ArchiveTasks => {
                for task_id in result.task_ids {
                    conn.execute(
                        "UPDATE tasks SET list_id = ?1, updated_at = ?2, revision = revision + 1
                         WHERE id = ?3 AND deleted_at IS NULL",
                        params![result.list_id.to_string(), now, task_id.to_string()],
                    )
                    .map_err(internal)?;
                }
                for task_id in result.archived_task_ids {
                    conn.execute(
                        "UPDATE tasks SET status = 'todo', updated_at = ?1, revision = revision + 1
                         WHERE id = ?2 AND deleted_at IS NULL",
                        params![now, task_id.to_string()],
                    )
                    .map_err(internal)?;
                }
            }
            ListDeleteDisposition::ForceDelete => {
                for task_id in result.task_ids {
                    conn.execute(
                        "UPDATE tasks SET deleted_at = NULL, updated_at = ?1, revision = revision + 1
                         WHERE id = ?2",
                        params![now, task_id.to_string()],
                    )
                    .map_err(internal)?;
                }
            }
        }

        self.get_list(result.list_id)
    }

    fn task_ids_in_list(
        &self,
        conn: &Connection,
        list_id: EntityId,
    ) -> Result<Vec<EntityId>, DomainError> {
        let mut stmt = conn
            .prepare("SELECT id FROM tasks WHERE list_id = ?1 AND deleted_at IS NULL")
            .map_err(internal)?;
        let rows = stmt
            .query_map([list_id.to_string()], |row| {
                let raw: String = row.get(0)?;
                raw.parse().map_err(|e| {
                    rusqlite::Error::FromSqlConversionFailure(
                        0,
                        rusqlite::types::Type::Text,
                        Box::new(e),
                    )
                })
            })
            .map_err(internal)?;
        collect_rows(rows)
    }

    pub fn inbox_list_id(&self) -> Result<EntityId, DomainError> {
        let conn = self.connect()?;
        let id: String = conn
            .query_row(
                "SELECT id FROM task_lists WHERE kind = 'inbox' AND deleted_at IS NULL LIMIT 1",
                [],
                |row| row.get(0),
            )
            .map_err(|_| DomainError::Internal("收件箱清单缺失".into()))?;
        id.parse()
            .map_err(|_| DomainError::Internal("invalid inbox id".into()))
    }

    pub fn create_task(&self, input: CreateTaskInput) -> Result<Task, DomainError> {
        let title = input.title.trim().to_string();
        if title.is_empty() {
            return Err(DomainError::Validation("标题不能为空".into()));
        }
        if let Some(ref due) = input.due_date {
            validate_due_date(due)?;
        }
        if let Some(ref time) = input.due_time {
            validate_due_time(time)?;
        }

        let list_id = match input.list_id {
            Some(id) => {
                let _ = self.get_list(id)?;
                id
            }
            None => self.inbox_list_id()?,
        };

        let conn = self.connect()?;
        let id = new_id();
        let now = stamp(&self.clock);
        let priority = input.priority.unwrap_or(TaskPriority::None);
        let notes = input.notes.unwrap_or_default();
        let sort_order: f64 = conn
            .query_row(
                "SELECT COALESCE(MAX(sort_order), 0) + 1 FROM tasks WHERE list_id = ?1 AND deleted_at IS NULL",
                [list_id.to_string()],
                |row| row.get(0),
            )
            .unwrap_or(1.0);

        let tx = conn.unchecked_transaction().map_err(internal)?;
        tx.execute(
            "INSERT INTO tasks (
                id, title, notes, status, priority, list_id, due_date, due_time,
                completed_at, sort_order, created_at, updated_at, revision, deleted_at
             ) VALUES (?1, ?2, ?3, 'todo', ?4, ?5, ?6, ?7, NULL, ?8, ?9, ?9, 1, NULL)",
            params![
                id.to_string(),
                title,
                notes,
                priority.as_str(),
                list_id.to_string(),
                input.due_date,
                input.due_time,
                sort_order,
                now,
            ],
        )
        .map_err(internal)?;

        if let Some(tag_names) = input.tag_names {
            self.replace_tags(&tx, id, &tag_names)?;
        }
        tx.commit().map_err(internal)?;
        self.get_task(id)
    }

    pub fn update_task(&self, input: UpdateTaskInput) -> Result<Task, DomainError> {
        let _existing = self.get_task(input.id)?;
        let title = input.title.trim().to_string();
        if title.is_empty() {
            return Err(DomainError::Validation("标题不能为空".into()));
        }
        let _ = self.get_list(input.list_id)?;
        if let Some(ref due) = input.due_date {
            validate_due_date(due)?;
        }
        if let Some(ref time) = input.due_time {
            validate_due_time(time)?;
        }

        let conn = self.connect()?;
        let now = stamp(&self.clock);
        let tx = conn.unchecked_transaction().map_err(internal)?;
        tx.execute(
            "UPDATE tasks SET
                title = ?1,
                notes = ?2,
                priority = ?3,
                list_id = ?4,
                due_date = ?5,
                due_time = ?6,
                updated_at = ?7,
                revision = revision + 1
             WHERE id = ?8 AND deleted_at IS NULL",
            params![
                title,
                input.notes,
                input.priority.as_str(),
                input.list_id.to_string(),
                input.due_date,
                input.due_time,
                now,
                input.id.to_string(),
            ],
        )
        .map_err(internal)?;

        self.replace_tags(&tx, input.id, &input.tag_names)?;
        tx.commit().map_err(internal)?;
        self.get_task(input.id)
    }

    pub fn complete_task(&self, id: EntityId) -> Result<Task, DomainError> {
        let task = self.get_task(id)?;
        if task.status == TaskStatus::Completed {
            return Ok(task);
        }
        let conn = self.connect()?;
        let now = stamp(&self.clock);
        let tx = conn.unchecked_transaction().map_err(internal)?;
        tx.execute(
            "UPDATE tasks SET status = 'completed', completed_at = ?1, updated_at = ?1, revision = revision + 1
             WHERE id = ?2 AND deleted_at IS NULL",
            params![now, id.to_string()],
        )
        .map_err(internal)?;
        tx.execute(
            "DELETE FROM daily_focus WHERE task_id = ?1",
            params![id.to_string()],
        )
        .map_err(internal)?;

        if let Some(series_id) = task.series_id {
            self.spawn_next_series_instance(&tx, &task, series_id, &now)?;
        }
        tx.commit().map_err(internal)?;
        self.get_task(id)
    }

    pub fn skip_task_instance(&self, id: EntityId) -> Result<Task, DomainError> {
        let task = self.get_task(id)?;
        let series_id = task
            .series_id
            .ok_or_else(|| DomainError::Validation("不是周期任务实例".into()))?;
        if task.status != TaskStatus::Todo {
            return Err(DomainError::Validation("只能跳过待办实例".into()));
        }
        let conn = self.connect()?;
        let now = stamp(&self.clock);
        let tx = conn.unchecked_transaction().map_err(internal)?;
        tx.execute(
            "UPDATE tasks SET status = 'archived', updated_at = ?1, revision = revision + 1
             WHERE id = ?2 AND deleted_at IS NULL",
            params![now, id.to_string()],
        )
        .map_err(internal)?;
        self.spawn_next_series_instance(&tx, &task, series_id, &now)?;
        tx.commit().map_err(internal)?;
        self.get_task(id)
    }

    fn spawn_next_series_instance(
        &self,
        conn: &Connection,
        current: &Task,
        series_id: EntityId,
        now: &str,
    ) -> Result<(), DomainError> {
        let (recurrence_json, list_id, title, notes, priority, timezone, end_at): (
            String,
            String,
            String,
            String,
            String,
            String,
            Option<String>,
        ) = conn
            .query_row(
                "SELECT recurrence_json, list_id, title, notes, priority, timezone, end_at
                 FROM task_series WHERE id = ?1 AND deleted_at IS NULL AND enabled = 1",
                [series_id.to_string()],
                |row| {
                    Ok((
                        row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get(4)?,
                        row.get(5)?,
                        row.get(6)?,
                    ))
                },
            )
            .optional()
            .map_err(internal)?
            .ok_or_else(|| DomainError::NotFound("周期任务模板不存在或已停用".into()))?;

        let rule = crate::domain::RecurrenceRule::from_json(&recurrence_json)?;
        let due_date = current
            .due_date
            .clone()
            .ok_or_else(|| DomainError::Validation("周期任务实例缺少截止日期".into()))?;
        let due_time = current.due_time.clone().unwrap_or_else(|| "09:00".into());
        let current_dt = crate::domain::combine_date_time(&due_date, &due_time)?;
        let Some(next_dt) = crate::domain::next_after(&rule, current_dt)? else {
            conn.execute(
                "UPDATE task_series SET enabled = 0, updated_at = ?1, revision = revision + 1 WHERE id = ?2",
                params![now, series_id.to_string()],
            )
            .map_err(internal)?;
            return Ok(());
        };

        let next_date = next_dt.date().format("%Y-%m-%d").to_string();
        let next_time = next_dt.time().format("%H:%M").to_string();
        let new_id = new_id();
        let sort_order: f64 = conn
            .query_row(
                "SELECT COALESCE(MAX(sort_order), 0) + 1 FROM tasks WHERE list_id = ?1 AND deleted_at IS NULL",
                [&list_id],
                |row| row.get(0),
            )
            .unwrap_or(1.0);

        conn.execute(
            "INSERT INTO tasks (
                id, title, notes, status, priority, list_id, due_date, due_time,
                completed_at, sort_order, series_id, created_at, updated_at, revision, deleted_at
             ) VALUES (?1, ?2, ?3, 'todo', ?4, ?5, ?6, ?7, NULL, ?8, ?9, ?10, ?10, 1, NULL)",
            params![
                new_id.to_string(),
                title,
                notes,
                priority,
                list_id,
                next_date,
                next_time,
                sort_order,
                series_id.to_string(),
                now,
            ],
        )
        .map_err(internal)?;

        conn.execute(
            "UPDATE task_series SET next_due_date = ?1, updated_at = ?2, revision = revision + 1 WHERE id = ?3",
            params![next_date, now, series_id.to_string()],
        )
        .map_err(internal)?;

        let _ = timezone;
        let _ = end_at;
        Ok(())
    }

    pub fn create_recurring_task(
        &self,
        input: CreateTaskInput,
        recurrence: crate::domain::RecurrenceRule,
    ) -> Result<Task, DomainError> {
        recurrence.validate()?;
        let title = input.title.trim().to_string();
        if title.is_empty() {
            return Err(DomainError::Validation("标题不能为空".into()));
        }
        let due_date = input
            .due_date
            .clone()
            .ok_or_else(|| DomainError::Validation("周期任务需要截止日期".into()))?;
        crate::domain::validate_due_date(&due_date)?;
        let due_time = input.due_time.clone().unwrap_or_else(|| "09:00".into());
        crate::domain::validate_due_time(&due_time)?;

        let list_id = match input.list_id {
            Some(id) => {
                let _ = self.get_list(id)?;
                id
            }
            None => self.inbox_list_id()?,
        };
        let priority = input.priority.unwrap_or(TaskPriority::None);
        let notes = input.notes.unwrap_or_default();
        let series_id = new_id();
        let task_id = new_id();
        let now = stamp(&self.clock);
        let timezone = chrono::Local::now().offset().to_string();
        let conn = self.connect()?;
        let sort_order: f64 = conn
            .query_row(
                "SELECT COALESCE(MAX(sort_order), 0) + 1 FROM tasks WHERE list_id = ?1 AND deleted_at IS NULL",
                [list_id.to_string()],
                |row| row.get(0),
            )
            .unwrap_or(1.0);

        let tx = conn.unchecked_transaction().map_err(internal)?;
        tx.execute(
            "INSERT INTO task_series (
                id, title, notes, priority, list_id, recurrence_json, timezone,
                next_due_date, enabled, end_at, created_at, updated_at, revision, deleted_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, 1, ?9, ?10, ?10, 1, NULL)",
            params![
                series_id.to_string(),
                title,
                notes,
                priority.as_str(),
                list_id.to_string(),
                recurrence.to_json()?,
                timezone,
                due_date,
                recurrence.end_at.clone(),
                now,
            ],
        )
        .map_err(internal)?;

        tx.execute(
            "INSERT INTO tasks (
                id, title, notes, status, priority, list_id, due_date, due_time,
                completed_at, sort_order, series_id, created_at, updated_at, revision, deleted_at
             ) VALUES (?1, ?2, ?3, 'todo', ?4, ?5, ?6, ?7, NULL, ?8, ?9, ?10, ?10, 1, NULL)",
            params![
                task_id.to_string(),
                title,
                notes,
                priority.as_str(),
                list_id.to_string(),
                due_date,
                due_time,
                sort_order,
                series_id.to_string(),
                now,
            ],
        )
        .map_err(internal)?;

        if let Some(tag_names) = input.tag_names {
            self.replace_tags(&tx, task_id, &tag_names)?;
        }
        tx.commit().map_err(internal)?;
        self.get_task(task_id)
    }

    pub fn uncomplete_task(&self, id: EntityId) -> Result<Task, DomainError> {
        let task = self.get_task(id)?;
        if task.status != TaskStatus::Completed {
            return Ok(task);
        }
        let conn = self.connect()?;
        let now = stamp(&self.clock);
        conn.execute(
            "UPDATE tasks SET status = 'todo', completed_at = NULL, updated_at = ?1, revision = revision + 1
             WHERE id = ?2 AND deleted_at IS NULL",
            params![now, id.to_string()],
        )
        .map_err(internal)?;
        self.get_task(id)
    }

    pub fn unarchive_task(&self, id: EntityId) -> Result<Task, DomainError> {
        let task = self.get_task(id)?;
        if task.status != TaskStatus::Archived {
            return Ok(task);
        }
        let conn = self.connect()?;
        let now = stamp(&self.clock);
        conn.execute(
            "UPDATE tasks SET status = 'todo', updated_at = ?1, revision = revision + 1
             WHERE id = ?2 AND deleted_at IS NULL",
            params![now, id.to_string()],
        )
        .map_err(internal)?;
        self.get_task(id)
    }

    pub fn archive_task(&self, id: EntityId) -> Result<Task, DomainError> {
        let _ = self.get_task(id)?;
        let conn = self.connect()?;
        let now = stamp(&self.clock);
        conn.execute(
            "UPDATE tasks SET status = 'archived', updated_at = ?1, revision = revision + 1
             WHERE id = ?2 AND deleted_at IS NULL",
            params![now, id.to_string()],
        )
        .map_err(internal)?;
        self.get_task(id)
    }

    pub fn delete_task(&self, id: EntityId) -> Result<(), DomainError> {
        let _ = self.get_task(id)?;
        let conn = self.connect()?;
        let now = stamp(&self.clock);
        conn.execute(
            "UPDATE tasks SET deleted_at = ?1, updated_at = ?1, revision = revision + 1
             WHERE id = ?2 AND deleted_at IS NULL",
            params![now, id.to_string()],
        )
        .map_err(internal)?;
        Ok(())
    }

    /// Rewrites sort_order for the given tasks, numbered per list.
    ///
    /// sort_order is list-local: creation assigns MAX(sort_order)+1 within a
    /// list and list queries filter by list_id. Writing global 0..n-1 across
    /// lists (as the old implementation did) corrupts other lists' ordering.
    /// Group ordered_ids by list_id (preserving drag order) and number each
    /// group from 0, leaving tasks outside ordered_ids untouched.
    pub fn reorder_tasks(&self, ordered_ids: Vec<EntityId>) -> Result<(), DomainError> {
        if ordered_ids.is_empty() {
            return Ok(());
        }
        let conn = self.connect()?;
        let now = stamp(&self.clock);

        let placeholders = vec!["?"; ordered_ids.len()].join(", ");
        let mut id_to_list: std::collections::HashMap<EntityId, EntityId> =
            std::collections::HashMap::new();
        {
            let mut stmt = conn
                .prepare(&format!(
                    "SELECT id, list_id FROM tasks
                     WHERE id IN ({placeholders}) AND deleted_at IS NULL"
                ))
                .map_err(internal)?;
            let id_strings: Vec<String> = ordered_ids.iter().map(|id| id.to_string()).collect();
            let params_ref: Vec<&dyn rusqlite::types::ToSql> = id_strings
                .iter()
                .map(|s| s as &dyn rusqlite::types::ToSql)
                .collect();
            let rows = stmt
                .query_map(params_ref.as_slice(), |row| {
                    Ok((parse_id(row.get(0)?)?, parse_id(row.get(1)?)?))
                })
                .map_err(internal)?;
            for row in rows {
                let (id, list_id) = row.map_err(internal)?;
                id_to_list.insert(id, list_id);
            }
        }

        let tx = conn.unchecked_transaction().map_err(internal)?;
        let mut per_list: std::collections::HashMap<EntityId, Vec<EntityId>> =
            std::collections::HashMap::new();
        for id in &ordered_ids {
            if let Some(list_id) = id_to_list.get(id) {
                per_list.entry(*list_id).or_default().push(*id);
            }
        }
        for ids in per_list.values() {
            for (index, id) in ids.iter().enumerate() {
                tx.execute(
                    "UPDATE tasks SET sort_order = ?1, updated_at = ?2, revision = revision + 1
                     WHERE id = ?3 AND deleted_at IS NULL",
                    params![index as f64, now, id.to_string()],
                )
                .map_err(internal)?;
            }
        }
        tx.commit().map_err(internal)?;
        Ok(())
    }

    pub fn get_task(&self, id: EntityId) -> Result<Task, DomainError> {
        let conn = self.connect()?;
        let mut task = conn
            .query_row(
                &format!(
                    "SELECT {TASK_ROW_SELECT}
                 FROM tasks t
                 JOIN task_lists l ON l.id = t.list_id
                 WHERE t.id = ?1 AND t.deleted_at IS NULL"
                ),
                [id.to_string()],
                map_task_row,
            )
            .optional()
            .map_err(internal)?
            .ok_or_else(|| DomainError::NotFound("任务不存在".into()))?;
        self.attach_tags(&conn, &mut task)?;
        Ok(task)
    }

    pub fn query_tasks(&self, query: TaskQuery) -> Result<PagedResult<Task>, DomainError> {
        let conn = self.connect()?;
        let limit = page_limit(query.limit);
        let offset = page_offset(query.offset);
        let mut filters = String::new();
        let mut values: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

        if query.inbox_only.unwrap_or(false) {
            filters.push_str(" AND l.kind = 'inbox'");
        }
        if let Some(list_id) = query.list_id {
            filters.push_str(" AND t.list_id = ?");
            values.push(Box::new(list_id.to_string()));
        }
        if let Some(status) = query.status {
            filters.push_str(" AND t.status = ?");
            values.push(Box::new(status.as_str().to_string()));
        } else if !query.include_archived.unwrap_or(false) {
            filters.push_str(" AND t.status != 'archived'");
        }
        if let Some(priority) = query.priority {
            filters.push_str(" AND t.priority = ?");
            values.push(Box::new(priority.as_str().to_string()));
        }
        if let Some(tag_id) = query.tag_id {
            filters.push_str(
                " AND EXISTS (SELECT 1 FROM task_tags tt WHERE tt.task_id = t.id AND tt.tag_id = ?)",
            );
            values.push(Box::new(tag_id.to_string()));
        }
        if let Some(text) = query.search.as_ref().map(|s| s.trim().to_string()) {
            if !text.is_empty() {
                filters.push_str(" AND (t.title LIKE ? ESCAPE '\\' OR t.notes LIKE ? ESCAPE '\\')");
                let pattern = format!("%{}%", escape_like(&text));
                values.push(Box::new(pattern.clone()));
                values.push(Box::new(pattern));
            }
        }
        if query.due_null.unwrap_or(false) {
            filters.push_str(" AND t.due_date IS NULL");
        }
        if let Some(ref from) = query.due_from {
            filters.push_str(" AND t.due_date IS NOT NULL AND t.due_date >= ?");
            values.push(Box::new(from.clone()));
        }
        if let Some(ref to) = query.due_to {
            filters.push_str(" AND t.due_date IS NOT NULL AND t.due_date <= ?");
            values.push(Box::new(to.clone()));
        }
        if let Some(ref since) = query.completed_since {
            filters.push_str(
                " AND t.status = 'completed' AND t.completed_at IS NOT NULL AND date(t.completed_at, 'localtime') >= ?",
            );
            values.push(Box::new(since.clone()));
        }

        let today = local_today(&self.clock);
        if query.deferred_only.unwrap_or(false) {
            filters.push_str(
                " AND t.status = 'todo' AND t.workflow_state = 'active'
                  AND t.available_at IS NOT NULL AND t.available_at > ?",
            );
            values.push(Box::new(today.clone()));
        } else if query.waiting_follow_up_due.unwrap_or(false) {
            filters.push_str(
                " AND t.status = 'todo' AND t.workflow_state = 'waiting'
                  AND t.follow_up_date IS NOT NULL AND t.follow_up_date <= ?",
            );
            values.push(Box::new(today.clone()));
        } else if should_apply_active_list_filter(query.search.as_deref()) {
            if let Some(workflow_state) = query.workflow_state {
                filters.push_str(" AND t.workflow_state = ?");
                values.push(Box::new(workflow_state.as_str().to_string()));
                if workflow_state == TaskWorkflowState::Active {
                    filters.push_str(" AND (t.available_at IS NULL OR t.available_at <= ?)");
                    values.push(Box::new(today.clone()));
                }
            } else if query.status.unwrap_or(TaskStatus::Todo) == TaskStatus::Todo
                || query.status.is_none()
            {
                filters.push_str(
                    " AND t.workflow_state = 'active' AND (t.available_at IS NULL OR t.available_at <= ?)",
                );
                values.push(Box::new(today.clone()));
            }
        }

        let order_by = if query.completed_since.is_some() {
            " ORDER BY t.completed_at DESC, t.updated_at DESC"
        } else {
            " ORDER BY t.sort_order ASC, t.created_at DESC"
        };

        let from_clause = " FROM tasks t
             JOIN task_lists l ON l.id = t.list_id
             WHERE t.deleted_at IS NULL";

        let count_sql = format!("SELECT COUNT(*){from_clause}{filters}");
        let params_ref: Vec<&dyn rusqlite::types::ToSql> =
            values.iter().map(|v| v.as_ref()).collect();
        let total: i64 = conn
            .query_row(&count_sql, params_ref.as_slice(), |row| row.get(0))
            .map_err(internal)?;

        let sql = format!(
            "SELECT {TASK_ROW_SELECT}{from_clause}{filters}{order_by} LIMIT ? OFFSET ?"
        );
        values.push(Box::new(limit));
        values.push(Box::new(offset));
        let params_ref: Vec<&dyn rusqlite::types::ToSql> =
            values.iter().map(|v| v.as_ref()).collect();

        let mut stmt = conn.prepare(&sql).map_err(internal)?;
        let rows = stmt
            .query_map(params_ref.as_slice(), map_task_row)
            .map_err(internal)?;
        let mut tasks = collect_rows(rows)?;
        for task in &mut tasks {
            self.attach_tags(&conn, task)?;
        }
        Ok(PagedResult::new(tasks, total, offset))
    }

    pub fn smart_list(
        &self,
        kind: SmartListKind,
        limit: Option<i64>,
        offset: Option<i64>,
    ) -> Result<PagedResult<Task>, DomainError> {
        let today = local_today(&self.clock);
        let today_date = chrono::NaiveDate::parse_from_str(&today, "%Y-%m-%d")
            .map_err(|e| DomainError::Internal(e.to_string()))?;
        let mut query = match kind {
            SmartListKind::Tomorrow => {
                let tomorrow = (today_date + chrono::Duration::days(1))
                    .format("%Y-%m-%d")
                    .to_string();
                TaskQuery {
                    status: Some(TaskStatus::Todo),
                    due_from: Some(tomorrow.clone()),
                    due_to: Some(tomorrow),
                    ..Default::default()
                }
            }
            SmartListKind::Next7Days => {
                let end = (today_date + chrono::Duration::days(7))
                    .format("%Y-%m-%d")
                    .to_string();
                TaskQuery {
                    status: Some(TaskStatus::Todo),
                    due_from: Some(today.clone()),
                    due_to: Some(end),
                    ..Default::default()
                }
            }
            SmartListKind::Overdue => TaskQuery {
                status: Some(TaskStatus::Todo),
                due_to: Some(
                    (today_date - chrono::Duration::days(1))
                        .format("%Y-%m-%d")
                        .to_string(),
                ),
                ..Default::default()
            },
            SmartListKind::HighPriority => TaskQuery {
                status: Some(TaskStatus::Todo),
                priority: Some(TaskPriority::High),
                ..Default::default()
            },
            SmartListKind::NoDue => TaskQuery {
                status: Some(TaskStatus::Todo),
                due_null: Some(true),
                ..Default::default()
            },
            SmartListKind::RecentCompleted => {
                let since = (today_date - chrono::Duration::days(14))
                    .format("%Y-%m-%d")
                    .to_string();
                TaskQuery {
                    completed_since: Some(since),
                    include_archived: Some(false),
                    ..Default::default()
                }
            }
            SmartListKind::Deferred => TaskQuery {
                status: Some(TaskStatus::Todo),
                deferred_only: Some(true),
                ..Default::default()
            },
            SmartListKind::WaitingFollowUp => TaskQuery {
                status: Some(TaskStatus::Todo),
                waiting_follow_up_due: Some(true),
                ..Default::default()
            },
        };
        query.limit = limit;
        query.offset = offset;
        self.query_tasks(query)
    }

    /// Active todo tasks not updated within `stale_days` (for weekly review).
    pub fn query_stale_active(
        &self,
        stale_days: i64,
        limit: Option<i64>,
        offset: Option<i64>,
    ) -> Result<PagedResult<Task>, DomainError> {
        let today = local_today(&self.clock);
        let today_date = chrono::NaiveDate::parse_from_str(&today, "%Y-%m-%d")
            .map_err(|e| DomainError::Internal(e.to_string()))?;
        let cutoff = (today_date - chrono::Duration::days(stale_days.clamp(1, 365)))
            .format("%Y-%m-%d")
            .to_string();

        let conn = self.connect()?;
        let limit = page_limit(limit);
        let offset = page_offset(offset);
        let filters = " AND t.status = 'todo' AND t.workflow_state = 'active'
             AND (t.available_at IS NULL OR t.available_at <= ?1)
             AND date(t.updated_at, 'localtime') <= date(?2, 'localtime')";
        let from_clause = " FROM tasks t JOIN task_lists l ON l.id = t.list_id WHERE t.deleted_at IS NULL";
        let count_sql = format!("SELECT COUNT(*){from_clause}{filters}");
        let total: i64 = conn
            .query_row(&count_sql, params![today, cutoff], |row| row.get(0))
            .map_err(internal)?;

        let sql = format!(
            "SELECT {TASK_ROW_SELECT}{from_clause}{filters}
             ORDER BY t.updated_at ASC, t.sort_order ASC LIMIT ? OFFSET ?"
        );
        let mut stmt = conn.prepare(&sql).map_err(internal)?;
        let rows = stmt
            .query_map(params![today, cutoff, limit, offset], map_task_row)
            .map_err(internal)?;
        let mut items = collect_rows(rows)?;
        for task in &mut items {
            self.attach_tags(&conn, task)?;
        }
        Ok(PagedResult::new(items, total, offset))
    }

    pub fn postpone_task(&self, id: EntityId, days: i64) -> Result<Task, DomainError> {
        let days = days.clamp(1, 365);
        let task = self.get_task(id)?;
        if task.status != TaskStatus::Todo {
            return Err(DomainError::Validation("只能延期未完成任务".into()));
        }
        let base = task
            .due_date
            .as_deref()
            .and_then(|d| chrono::NaiveDate::parse_from_str(d, "%Y-%m-%d").ok())
            .unwrap_or_else(|| chrono::Local::now().date_naive());
        let new_due = (base + chrono::Duration::days(days))
            .format("%Y-%m-%d")
            .to_string();
        self.record_defer_event(id, "postpone")?;
        self.update_task(UpdateTaskInput {
            id,
            title: task.title,
            notes: task.notes,
            priority: task.priority,
            list_id: task.list_id,
            due_date: Some(new_due),
            due_time: task.due_time,
            tag_names: task.tag_names,
        })
    }

    pub fn set_task_defer(
        &self,
        id: EntityId,
        available_at: Option<String>,
    ) -> Result<Task, DomainError> {
        let task = self.get_task(id)?;
        if task.status != TaskStatus::Todo {
            return Err(DomainError::Validation("只能推迟待办任务".into()));
        }
        if task.workflow_state == TaskWorkflowState::Waiting {
            return Err(DomainError::Validation(
                "等待中的任务请先结束等待，再设置推迟显示".into(),
            ));
        }
        let normalized = available_at
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());
        if let Some(ref date) = normalized {
            validate_due_date(date)?;
        }
        validate_due_vs_available(task.due_date.as_deref(), normalized.as_deref())?;

        let conn = self.connect()?;
        let now = stamp(&self.clock);
        let rows = conn
            .execute(
                "UPDATE tasks SET available_at = ?1, updated_at = ?2, revision = revision + 1
                 WHERE id = ?3 AND deleted_at IS NULL",
                params![normalized, now, id.to_string()],
            )
            .map_err(internal)?;
        if rows == 0 {
            return Err(DomainError::NotFound("任务不存在".into()));
        }
        if let Some(ref date) = normalized {
            let today = local_today(&self.clock);
            if date > &today {
                self.record_defer_event(id, "defer")?;
                conn.execute(
                    "DELETE FROM daily_focus WHERE task_id = ?1",
                    params![id.to_string()],
                )
                .map_err(internal)?;
            }
        }
        self.get_task(id)
    }

    pub fn set_task_waiting(
        &self,
        id: EntityId,
        waiting_for: Option<String>,
        follow_up_date: Option<String>,
    ) -> Result<Task, DomainError> {
        let task = self.get_task(id)?;
        if task.status != TaskStatus::Todo {
            return Err(DomainError::Validation("只能标记待办任务为等待".into()));
        }
        let waiting_for = waiting_for
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());
        if let Some(ref text) = waiting_for {
            if text.chars().count() > 500 {
                return Err(DomainError::Validation(
                    "等待对象不能超过 500 个字符".into(),
                ));
            }
        }
        let follow_up = follow_up_date
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());
        if let Some(ref date) = follow_up {
            validate_due_date(date)?;
        }

        let conn = self.connect()?;
        let now = stamp(&self.clock);
        let rows = conn
            .execute(
                "UPDATE tasks SET workflow_state = 'waiting', waiting_for = ?1,
                 follow_up_date = ?2, updated_at = ?3, revision = revision + 1
                 WHERE id = ?4 AND deleted_at IS NULL",
                params![
                    waiting_for,
                    follow_up,
                    now,
                    id.to_string(),
                ],
            )
            .map_err(internal)?;
        if rows == 0 {
            return Err(DomainError::NotFound("任务不存在".into()));
        }
        conn.execute(
            "DELETE FROM daily_focus WHERE task_id = ?1",
            params![id.to_string()],
        )
        .map_err(internal)?;
        self.get_task(id)
    }

    pub fn clear_task_waiting(&self, id: EntityId) -> Result<Task, DomainError> {
        let task = self.get_task(id)?;
        if task.status != TaskStatus::Todo {
            return Err(DomainError::Validation("只能结束待办任务的等待".into()));
        }
        if task.workflow_state != TaskWorkflowState::Waiting {
            return Err(DomainError::Validation("任务不在等待中".into()));
        }

        let conn = self.connect()?;
        let now = stamp(&self.clock);
        let rows = conn
            .execute(
                "UPDATE tasks SET workflow_state = 'active', waiting_for = NULL,
                 follow_up_date = NULL, updated_at = ?1, revision = revision + 1
                 WHERE id = ?2 AND deleted_at IS NULL",
                params![now, id.to_string()],
            )
            .map_err(internal)?;
        if rows == 0 {
            return Err(DomainError::NotFound("任务不存在".into()));
        }
        self.get_task(id)
    }

    fn normalize_focus_date(&self, focus_date: Option<String>) -> Result<String, DomainError> {
        match focus_date {
            None => Ok(local_today(&self.clock)),
            Some(s) if s.trim().is_empty() => Ok(local_today(&self.clock)),
            Some(s) => {
                validate_due_date(s.trim())?;
                Ok(s.trim().to_string())
            }
        }
    }

    fn validate_focus_eligible(&self, task: &Task, today: &str) -> Result<(), DomainError> {
        if task.status != TaskStatus::Todo {
            return Err(DomainError::Validation("只能将待办任务加入今日重点".into()));
        }
        if task.workflow_state == TaskWorkflowState::Waiting {
            return Err(DomainError::Validation(
                "等待中的任务请先结束等待，再加入今日重点".into(),
            ));
        }
        if let Some(ref avail) = task.available_at {
            if avail.as_str() > today {
                return Err(DomainError::Validation(
                    "推迟显示中的任务暂不能加入今日重点".into(),
                ));
            }
        }
        Ok(())
    }

    pub fn daily_focus_add(
        &self,
        task_id: EntityId,
        focus_date: Option<String>,
    ) -> Result<Task, DomainError> {
        let date = self.normalize_focus_date(focus_date)?;
        let task = self.get_task(task_id)?;
        self.validate_focus_eligible(&task, &date)?;

        let conn = self.connect()?;
        let exists: bool = conn
            .query_row(
                "SELECT 1 FROM daily_focus WHERE focus_date = ?1 AND task_id = ?2",
                params![date, task_id.to_string()],
                |_| Ok(true),
            )
            .optional()
            .map_err(internal)?
            .is_some();
        if exists {
            return Ok(task);
        }

        let max_order: f64 = conn
            .query_row(
                "SELECT COALESCE(MAX(sort_order), -1) FROM daily_focus WHERE focus_date = ?1",
                [&date],
                |row| row.get(0),
            )
            .map_err(internal)?;
        let now = stamp(&self.clock);
        conn.execute(
            "INSERT INTO daily_focus (focus_date, task_id, sort_order, added_at, carried_from_date)
             VALUES (?1, ?2, ?3, ?4, NULL)",
            params![date, task_id.to_string(), max_order + 1.0, now],
        )
        .map_err(internal)?;
        self.get_task(task_id)
    }

    pub fn daily_focus_remove(
        &self,
        task_id: EntityId,
        focus_date: Option<String>,
    ) -> Result<Task, DomainError> {
        let date = self.normalize_focus_date(focus_date)?;
        let conn = self.connect()?;
        conn.execute(
            "DELETE FROM daily_focus WHERE focus_date = ?1 AND task_id = ?2",
            params![date, task_id.to_string()],
        )
        .map_err(internal)?;
        self.get_task(task_id)
    }

    pub fn daily_focus_reorder(
        &self,
        task_ids: Vec<EntityId>,
        focus_date: Option<String>,
    ) -> Result<(), DomainError> {
        if task_ids.is_empty() {
            return Ok(());
        }
        let date = self.normalize_focus_date(focus_date)?;
        let conn = self.connect()?;
        let tx = conn.unchecked_transaction().map_err(internal)?;
        for (index, task_id) in task_ids.iter().enumerate() {
            tx.execute(
                "UPDATE daily_focus SET sort_order = ?1
                 WHERE focus_date = ?2 AND task_id = ?3",
                params![index as f64, date, task_id.to_string()],
            )
            .map_err(internal)?;
        }
        tx.commit().map_err(internal)?;
        Ok(())
    }

    pub fn daily_focus_carry(
        &self,
        from_date: String,
        to_date: String,
    ) -> Result<Vec<Task>, DomainError> {
        validate_due_date(from_date.trim())?;
        validate_due_date(to_date.trim())?;
        let from_date = from_date.trim().to_string();
        let to_date = to_date.trim().to_string();
        let conn = self.connect()?;
        let now = stamp(&self.clock);

        let mut stmt = conn
            .prepare(
                "SELECT t.id FROM tasks t
                 JOIN daily_focus df ON df.task_id = t.id AND df.focus_date = ?1
                 WHERE t.deleted_at IS NULL AND t.status = 'todo'
                   AND t.workflow_state = 'active'
                   AND (t.available_at IS NULL OR t.available_at <= ?2)
                   AND t.id NOT IN (SELECT task_id FROM daily_focus WHERE focus_date = ?2)
                 ORDER BY df.sort_order ASC",
            )
            .map_err(internal)?;
        let rows = stmt
            .query_map(params![from_date, to_date], |row| parse_id(row.get(0)?))
            .map_err(internal)?;
        let task_ids: Vec<EntityId> = collect_rows(rows)?;

        let mut max_order: f64 = conn
            .query_row(
                "SELECT COALESCE(MAX(sort_order), -1) FROM daily_focus WHERE focus_date = ?1",
                [&to_date],
                |row| row.get(0),
            )
            .map_err(internal)?;

        let tx = conn.unchecked_transaction().map_err(internal)?;
        for task_id in &task_ids {
            max_order += 1.0;
            tx.execute(
                "INSERT INTO daily_focus (focus_date, task_id, sort_order, added_at, carried_from_date)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                params![to_date, task_id.to_string(), max_order, now, from_date],
            )
            .map_err(internal)?;
        }
        tx.commit().map_err(internal)?;

        let mut carried = Vec::with_capacity(task_ids.len());
        for task_id in task_ids {
            carried.push(self.get_task(task_id)?);
        }
        Ok(carried)
    }

    pub fn today_tasks(&self) -> Result<TodayTasks, DomainError> {
        let today = local_today(&self.clock);
        let conn = self.connect()?;

        let mut overdue = self.query_today_bucket(
            &conn,
            "AND t.status = 'todo' AND t.due_date IS NOT NULL AND t.due_date < ?1
             AND t.workflow_state = 'active' AND (t.available_at IS NULL OR t.available_at <= ?1)
             ORDER BY t.due_date ASC, t.sort_order ASC",
            &today,
        )?;
        let mut due_today = self.query_today_bucket(
            &conn,
            "AND t.status = 'todo' AND t.due_date = ?1
             AND t.workflow_state = 'active' AND (t.available_at IS NULL OR t.available_at <= ?1)
             ORDER BY t.sort_order ASC, t.created_at DESC",
            &today,
        )?;
        let mut completed_today = {
            let sql = format!(
                "SELECT {TASK_ROW_SELECT}
             FROM tasks t
             JOIN task_lists l ON l.id = t.list_id
             WHERE t.deleted_at IS NULL
               AND t.status = 'completed' AND t.completed_at IS NOT NULL
               AND date(t.completed_at, 'localtime') = date('now', 'localtime')
             ORDER BY t.completed_at DESC"
            );
            let mut stmt = conn.prepare(&sql).map_err(internal)?;
            let rows = stmt.query_map([], map_task_row).map_err(internal)?;
            collect_rows(rows)?
        };
        let mut waiting_follow_up = self.query_today_bucket(
            &conn,
            "AND t.status = 'todo' AND t.workflow_state = 'waiting'
             AND t.follow_up_date IS NOT NULL AND t.follow_up_date <= ?1
             AND (t.available_at IS NULL OR t.available_at <= ?1)
             ORDER BY t.follow_up_date ASC, t.sort_order ASC",
            &today,
        )?;
        let mut focus = self.query_focus_tasks(&conn, &today)?;
        let yesterday = (chrono::NaiveDate::parse_from_str(&today, "%Y-%m-%d")
            .map_err(|_| DomainError::Internal("invalid local today".into()))?
            - chrono::Duration::days(1))
        .format("%Y-%m-%d")
        .to_string();
        let mut focus_carry_suggestions =
            self.query_focus_carry_suggestions(&conn, &yesterday, &today)?;

        for task in overdue
            .iter_mut()
            .chain(due_today.iter_mut())
            .chain(completed_today.iter_mut())
            .chain(waiting_follow_up.iter_mut())
            .chain(focus.iter_mut())
            .chain(focus_carry_suggestions.iter_mut())
        {
            self.attach_tags(&conn, task)?;
        }

        Ok(TodayTasks {
            overdue,
            due_today,
            completed_today,
            focus,
            waiting_follow_up,
            focus_carry_suggestions,
            reminders_today: Vec::new(),
            today,
        })
    }

    fn record_defer_event(&self, task_id: EntityId, kind: &str) -> Result<(), DomainError> {
        let conn = self.connect()?;
        let now = stamp(&self.clock);
        conn.execute(
            "INSERT INTO task_defer_events (task_id, kind, recorded_at) VALUES (?1, ?2, ?3)",
            params![task_id.to_string(), kind, now],
        )
        .map_err(internal)?;
        Ok(())
    }

    pub fn defer_counts_for_tasks(
        &self,
        task_ids: &[EntityId],
    ) -> Result<std::collections::HashMap<EntityId, i64>, DomainError> {
        if task_ids.is_empty() {
            return Ok(std::collections::HashMap::new());
        }
        let conn = self.connect()?;
        let placeholders = vec!["?"; task_ids.len()].join(", ");
        let sql = format!(
            "SELECT task_id, COUNT(*) FROM task_defer_events
             WHERE task_id IN ({placeholders}) GROUP BY task_id"
        );
        let mut stmt = conn.prepare(&sql).map_err(internal)?;
        let id_strings: Vec<String> = task_ids.iter().map(|id| id.to_string()).collect();
        let params_ref: Vec<&dyn rusqlite::types::ToSql> = id_strings
            .iter()
            .map(|s| s as &dyn rusqlite::types::ToSql)
            .collect();
        let rows = stmt
            .query_map(params_ref.as_slice(), |row| {
                Ok((parse_id(row.get::<_, String>(0)?)?, row.get::<_, i64>(1)?))
            })
            .map_err(internal)?;
        let mut map = std::collections::HashMap::new();
        for row in rows {
            let (id, count) = row.map_err(internal)?;
            map.insert(id, count);
        }
        Ok(map)
    }

    pub fn today_sort_suggestions(
        &self,
        enabled: bool,
        reminder_times: std::collections::HashMap<EntityId, String>,
    ) -> Result<TodaySortSuggestions, DomainError> {
        if !enabled {
            return Ok(TodaySortSuggestions {
                enabled: false,
                suggestions: Vec::new(),
            });
        }
        let today_view = self.today_tasks()?;
        let due_today = today_view.due_today;
        if due_today.is_empty() {
            return Ok(TodaySortSuggestions {
                enabled: true,
                suggestions: Vec::new(),
            });
        }
        let ids: Vec<EntityId> = due_today.iter().map(|t| t.id).collect();
        let defer_counts = self.defer_counts_for_tasks(&ids)?;
        let suggestions =
            compute_today_sort_suggestions(&due_today, &defer_counts, &reminder_times);
        Ok(TodaySortSuggestions {
            enabled: true,
            suggestions,
        })
    }

    pub fn list_tags(&self) -> Result<Vec<Tag>, DomainError> {
        let conn = self.connect()?;
        let mut stmt = conn
            .prepare(
                "SELECT id, name, created_at, updated_at, revision
                 FROM tags WHERE deleted_at IS NULL ORDER BY name COLLATE NOCASE",
            )
            .map_err(internal)?;
        let rows = stmt
            .query_map([], |row| {
                Ok(Tag {
                    id: parse_id(row.get::<_, String>(0)?)?,
                    name: row.get(1)?,
                    created_at: row.get(2)?,
                    updated_at: row.get(3)?,
                    revision: row.get(4)?,
                })
            })
            .map_err(internal)?;
        collect_rows(rows)
    }

    pub fn counts(&self) -> Result<TaskCounts, DomainError> {
        let today = local_today(&self.clock);
        let conn = self.connect()?;
        let inbox: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM tasks t
                 JOIN task_lists l ON l.id = t.list_id
                 WHERE t.deleted_at IS NULL AND t.status = 'todo' AND l.kind = 'inbox'
                   AND t.workflow_state = 'active'
                   AND (t.available_at IS NULL OR t.available_at <= ?1)",
                [&today],
                |row| row.get(0),
            )
            .map_err(internal)?;
        let overdue: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM tasks
                 WHERE deleted_at IS NULL AND status = 'todo'
                   AND workflow_state = 'active'
                   AND (available_at IS NULL OR available_at <= ?1)
                   AND due_date IS NOT NULL AND due_date < ?1",
                [&today, &today],
                |row| row.get(0),
            )
            .map_err(internal)?;
        Ok(TaskCounts { inbox, overdue })
    }

    fn query_today_bucket(
        &self,
        conn: &Connection,
        extra: &str,
        today: &str,
    ) -> Result<Vec<Task>, DomainError> {
        let sql = format!(
            "SELECT {TASK_ROW_SELECT}
             FROM tasks t
             JOIN task_lists l ON l.id = t.list_id
             WHERE t.deleted_at IS NULL {extra}"
        );
        let mut stmt = conn.prepare(&sql).map_err(internal)?;
        let rows = stmt.query_map([today], map_task_row).map_err(internal)?;
        collect_rows(rows)
    }

    fn query_focus_tasks(
        &self,
        conn: &Connection,
        focus_date: &str,
    ) -> Result<Vec<Task>, DomainError> {
        let sql = format!(
            "SELECT {TASK_ROW_SELECT}
             FROM tasks t
             JOIN task_lists l ON l.id = t.list_id
             JOIN daily_focus df ON df.task_id = t.id AND df.focus_date = ?1
             WHERE t.deleted_at IS NULL AND t.status = 'todo'
             ORDER BY df.sort_order ASC, df.added_at ASC"
        );
        let mut stmt = conn.prepare(&sql).map_err(internal)?;
        let rows = stmt
            .query_map([focus_date], map_task_row)
            .map_err(internal)?;
        collect_rows(rows)
    }

    fn query_focus_carry_suggestions(
        &self,
        conn: &Connection,
        from_date: &str,
        to_date: &str,
    ) -> Result<Vec<Task>, DomainError> {
        let sql = format!(
            "SELECT {TASK_ROW_SELECT}
             FROM tasks t
             JOIN task_lists l ON l.id = t.list_id
             JOIN daily_focus df ON df.task_id = t.id AND df.focus_date = ?1
             WHERE t.deleted_at IS NULL AND t.status = 'todo'
               AND t.workflow_state = 'active'
               AND (t.available_at IS NULL OR t.available_at <= ?2)
               AND t.id NOT IN (SELECT task_id FROM daily_focus WHERE focus_date = ?2)
             ORDER BY df.sort_order ASC, df.added_at ASC"
        );
        let mut stmt = conn.prepare(&sql).map_err(internal)?;
        let rows = stmt
            .query_map(params![from_date, to_date], map_task_row)
            .map_err(internal)?;
        collect_rows(rows)
    }

    fn attach_tags(&self, conn: &Connection, task: &mut Task) -> Result<(), DomainError> {
        let mut stmt = conn
            .prepare(
                "SELECT tg.id, tg.name FROM tags tg
                 JOIN task_tags tt ON tt.tag_id = tg.id
                 WHERE tt.task_id = ?1 AND tg.deleted_at IS NULL
                 ORDER BY tg.name COLLATE NOCASE",
            )
            .map_err(internal)?;
        let rows = stmt
            .query_map([task.id.to_string()], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })
            .map_err(internal)?;
        let mut ids = Vec::new();
        let mut names = Vec::new();
        for row in rows {
            let (id, name) = row.map_err(internal)?;
            ids.push(parse_id(id).map_err(internal)?);
            names.push(name);
        }
        task.tag_ids = ids;
        task.tag_names = names;
        Ok(())
    }

    fn replace_tags(
        &self,
        conn: &Connection,
        task_id: EntityId,
        tag_names: &[String],
    ) -> Result<(), DomainError> {
        conn.execute(
            "DELETE FROM task_tags WHERE task_id = ?1",
            [task_id.to_string()],
        )
        .map_err(internal)?;

        let now = stamp(&self.clock);
        for raw in tag_names {
            let name = raw.trim();
            if name.is_empty() {
                continue;
            }
            let existing: Option<String> = conn
                .query_row(
                    "SELECT id FROM tags WHERE name = ?1 COLLATE NOCASE AND deleted_at IS NULL",
                    [name],
                    |row| row.get(0),
                )
                .optional()
                .map_err(internal)?;

            let tag_id = if let Some(id) = existing {
                id
            } else {
                let id = new_id().to_string();
                conn.execute(
                    "INSERT INTO tags (id, name, created_at, updated_at, revision)
                     VALUES (?1, ?2, ?3, ?3, 1)",
                    params![id, name, now],
                )
                .map_err(internal)?;
                id
            };

            conn.execute(
                "INSERT OR IGNORE INTO task_tags (task_id, tag_id) VALUES (?1, ?2)",
                params![task_id.to_string(), tag_id],
            )
            .map_err(internal)?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskCounts {
    pub inbox: i64,
    pub overdue: i64,
}

fn internal<E: std::fmt::Display>(err: E) -> DomainError {
    DomainError::Internal(err.to_string())
}

fn escape_like(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_")
}

fn parse_id(value: String) -> Result<EntityId, rusqlite::Error> {
    value.parse().map_err(|e| {
        rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(e))
    })
}

fn map_list_row(row: &rusqlite::Row<'_>) -> Result<TaskList, rusqlite::Error> {
    Ok(TaskList {
        id: parse_id(row.get(0)?)?,
        name: row.get(1)?,
        kind: ListKind::parse(&row.get::<_, String>(2)?).map_err(|e| {
            rusqlite::Error::ToSqlConversionFailure(Box::new(std::io::Error::other(e.to_string())))
        })?,
        sort_order: row.get(3)?,
        created_at: row.get(4)?,
        updated_at: row.get(5)?,
        revision: row.get(6)?,
    })
}

fn map_task_row(row: &rusqlite::Row<'_>) -> Result<Task, rusqlite::Error> {
    let status = TaskStatus::parse(&row.get::<_, String>(3)?).map_err(map_domain_sql)?;
    let priority = TaskPriority::parse(&row.get::<_, String>(4)?).map_err(map_domain_sql)?;
    let list_kind = ListKind::parse(&row.get::<_, String>(7)?).map_err(map_domain_sql)?;
    let workflow_state =
        TaskWorkflowState::parse(&row.get::<_, String>(13)?).map_err(map_domain_sql)?;
    Ok(Task {
        id: parse_id(row.get(0)?)?,
        title: row.get(1)?,
        notes: row.get(2)?,
        status,
        priority,
        list_id: parse_id(row.get(5)?)?,
        list_name: row.get(6)?,
        list_kind,
        due_date: row.get(8)?,
        due_time: row.get(9)?,
        completed_at: row.get(10)?,
        sort_order: row.get(11)?,
        series_id: row
            .get::<_, Option<String>>(12)?
            .map(parse_id)
            .transpose()?,
        tag_ids: Vec::new(),
        tag_names: Vec::new(),
        workflow_state,
        available_at: row.get(14)?,
        waiting_for: row.get(15)?,
        follow_up_date: row.get(16)?,
        created_at: row.get(17)?,
        updated_at: row.get(18)?,
        revision: row.get(19)?,
    })
}

const TASK_ROW_SELECT: &str = "t.id, t.title, t.notes, t.status, t.priority, t.list_id,
                    l.name, l.kind, t.due_date, t.due_time, t.completed_at, t.sort_order, t.series_id,
                    t.workflow_state, t.available_at, t.waiting_for, t.follow_up_date,
                    t.created_at, t.updated_at, t.revision";

fn map_domain_sql(err: DomainError) -> rusqlite::Error {
    rusqlite::Error::ToSqlConversionFailure(Box::new(std::io::Error::other(err.to_string())))
}

fn collect_rows<T, E>(rows: impl IntoIterator<Item = Result<T, E>>) -> Result<Vec<T>, DomainError>
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
    use crate::infrastructure::db::Database;
    use tempfile::tempdir;

    fn open_service() -> TaskService {
        let dir = tempdir().unwrap();
        let path = dir.path().join("t.db");
        let db = Database::open(&path).unwrap();
        let svc = TaskService::new(db);
        svc.ensure_seed_data().unwrap();
        std::mem::forget(dir);
        svc
    }

    #[test]
    fn create_defaults_to_inbox_and_today_groups() {
        let svc = open_service();
        let today = local_today(&SystemClock);

        let inbox = svc
            .create_task(CreateTaskInput {
                title: "inbox task".into(),
                notes: None,
                priority: None,
                list_id: None,
                due_date: None,
                due_time: None,
                tag_names: Some(vec!["work".into()]),
            })
            .unwrap();
        assert_eq!(inbox.list_kind, ListKind::Inbox);
        assert_eq!(inbox.tag_names, vec!["work".to_string()]);

        let due = svc
            .create_task(CreateTaskInput {
                title: "today task".into(),
                notes: None,
                priority: Some(TaskPriority::High),
                list_id: None,
                due_date: Some(today.clone()),
                due_time: Some("09:30".into()),
                tag_names: None,
            })
            .unwrap();

        let overdue = svc
            .create_task(CreateTaskInput {
                title: "overdue".into(),
                notes: None,
                priority: None,
                list_id: None,
                due_date: Some("2000-01-01".into()),
                due_time: None,
                tag_names: None,
            })
            .unwrap();

        let today_view = svc.today_tasks().unwrap();
        assert_eq!(today_view.today, today);
        assert!(today_view.overdue.iter().any(|t| t.id == overdue.id));
        assert!(today_view.due_today.iter().any(|t| t.id == due.id));
        assert!(
            today_view
                .overdue
                .first()
                .map(|t| t.id == overdue.id)
                .unwrap_or(false)
                || today_view.overdue.iter().any(|t| t.id == overdue.id)
        );

        svc.complete_task(due.id).unwrap();
        let today_view = svc.today_tasks().unwrap();
        assert!(today_view.completed_today.iter().any(|t| t.id == due.id));
        assert!(!today_view.due_today.iter().any(|t| t.id == due.id));
    }

    #[test]
    fn archive_and_unarchive_roundtrip() {
        let svc = open_service();
        let task = svc
            .create_task(CreateTaskInput {
                title: "roundtrip".into(),
                notes: None,
                priority: None,
                list_id: None,
                due_date: None,
                due_time: None,
                tag_names: None,
            })
            .unwrap();
        assert_eq!(task.status, TaskStatus::Todo);

        let archived = svc.archive_task(task.id).unwrap();
        assert_eq!(archived.status, TaskStatus::Archived);

        let restored = svc.unarchive_task(task.id).unwrap();
        assert_eq!(restored.status, TaskStatus::Todo);

        // Unarchiving a non-archived task is a no-op.
        let again = svc.unarchive_task(task.id).unwrap();
        assert_eq!(again.status, TaskStatus::Todo);
    }

    #[test]
    fn query_tasks_pagination() {
        let svc = open_service();
        for i in 0..5 {
            svc.create_task(CreateTaskInput {
                title: format!("task {i}"),
                notes: None,
                priority: None,
                list_id: None,
                due_date: None,
                due_time: None,
                tag_names: None,
            })
            .unwrap();
        }

        let page = svc
            .query_tasks(TaskQuery {
                limit: Some(2),
                offset: Some(0),
                ..Default::default()
            })
            .unwrap();
        assert_eq!(page.items.len(), 2);
        assert!(page.total >= 5);
        assert!(page.has_more);

        let last = svc
            .query_tasks(TaskQuery {
                limit: Some(100),
                offset: Some(page.total - 1),
                ..Default::default()
            })
            .unwrap();
        assert_eq!(last.items.len(), 1);
        assert!(!last.has_more);
    }

    #[test]
    fn update_list_rename_and_query_search() {
        let svc = open_service();
        let list = svc.create_list("Projects".into()).unwrap();
        let updated = svc.update_list(list.id, "Work".into()).unwrap();
        assert_eq!(updated.name, "Work");

        let task = svc
            .create_task(CreateTaskInput {
                title: "alpha task".into(),
                notes: Some("contains beta keyword".into()),
                priority: None,
                list_id: Some(list.id),
                due_date: None,
                due_time: None,
                tag_names: None,
            })
            .unwrap();

        let by_title = svc
            .query_tasks(TaskQuery {
                search: Some("alpha".into()),
                ..Default::default()
            })
            .unwrap();
        assert!(by_title.items.iter().any(|t| t.id == task.id));

        let by_notes = svc
            .query_tasks(TaskQuery {
                search: Some("beta".into()),
                ..Default::default()
            })
            .unwrap();
        assert!(by_notes.items.iter().any(|t| t.id == task.id));
    }

    #[test]
    fn delete_list_moves_tasks_and_undo_restores() {
        let svc = open_service();
        let list = svc.create_list("Temp".into()).unwrap();
        let task = svc
            .create_task(CreateTaskInput {
                title: "move me".into(),
                notes: None,
                priority: None,
                list_id: Some(list.id),
                due_date: None,
                due_time: None,
                tag_names: None,
            })
            .unwrap();

        let result = svc
            .delete_list(list.id, ListDeleteDisposition::MoveToInbox)
            .unwrap();
        assert_eq!(result.task_ids, vec![task.id]);
        assert!(svc.get_list(list.id).is_err());

        let moved = svc.get_task(task.id).unwrap();
        assert_eq!(moved.list_kind, ListKind::Inbox);

        let restored = svc.undo_delete_list(result).unwrap();
        assert_eq!(restored.name, "Temp");
        let back = svc.get_task(task.id).unwrap();
        assert_eq!(back.list_id, list.id);
    }

    fn list_order_ids(svc: &TaskService, list_id: EntityId) -> Vec<EntityId> {
        svc.query_tasks(TaskQuery {
            list_id: Some(list_id),
            limit: Some(100),
            ..Default::default()
        })
        .unwrap()
        .items
        .into_iter()
        .map(|t| t.id)
        .collect()
    }

    #[test]
    fn reorder_tasks_rewrites_sort_order_per_list() {
        let svc = open_service();
        let list = svc.create_list("Projects".into()).unwrap();
        let t1 = svc
            .create_task(CreateTaskInput {
                title: "one".into(),
                notes: None,
                priority: None,
                list_id: Some(list.id),
                due_date: None,
                due_time: None,
                tag_names: None,
            })
            .unwrap();
        let t2 = svc
            .create_task(CreateTaskInput {
                title: "two".into(),
                notes: None,
                priority: None,
                list_id: Some(list.id),
                due_date: None,
                due_time: None,
                tag_names: None,
            })
            .unwrap();
        let t3 = svc
            .create_task(CreateTaskInput {
                title: "three".into(),
                notes: None,
                priority: None,
                list_id: Some(list.id),
                due_date: None,
                due_time: None,
                tag_names: None,
            })
            .unwrap();
        assert_eq!(list_order_ids(&svc, list.id), vec![t1.id, t2.id, t3.id]);

        // Move t3 to the front within the single list.
        svc.reorder_tasks(vec![t3.id, t1.id, t2.id]).unwrap();
        assert_eq!(list_order_ids(&svc, list.id), vec![t3.id, t1.id, t2.id]);
        assert_eq!(svc.get_task(t3.id).unwrap().sort_order, 0.0);
        assert_eq!(svc.get_task(t1.id).unwrap().sort_order, 1.0);
        assert_eq!(svc.get_task(t2.id).unwrap().sort_order, 2.0);
    }

    #[test]
    fn reorder_tasks_cross_list_keeps_lists_independent() {
        let svc = open_service();
        let list_a = svc.create_list("A".into()).unwrap();
        let list_b = svc.create_list("B".into()).unwrap();
        let create_in = |title: &str, list: EntityId| {
            svc.create_task(CreateTaskInput {
                title: title.into(),
                notes: None,
                priority: None,
                list_id: Some(list),
                due_date: None,
                due_time: None,
                tag_names: None,
            })
            .unwrap()
        };
        let a1 = create_in("a1", list_a.id);
        let a2 = create_in("a2", list_a.id);
        let a3 = create_in("a3", list_a.id);
        let b1 = create_in("b1", list_b.id);
        let b2 = create_in("b2", list_b.id);
        assert_eq!(list_order_ids(&svc, list_a.id), vec![a1.id, a2.id, a3.id]);
        assert_eq!(list_order_ids(&svc, list_b.id), vec![b1.id, b2.id]);

        // Simulate a today-view drag: interleave both lists.
        svc.reorder_tasks(vec![a2.id, b2.id, a3.id, a1.id, b1.id])
            .unwrap();

        // Each list keeps its own sequence, numbered 0..n-1 within the list.
        assert_eq!(list_order_ids(&svc, list_a.id), vec![a2.id, a3.id, a1.id]);
        assert_eq!(list_order_ids(&svc, list_b.id), vec![b2.id, b1.id]);
        for (t, expected) in [
            (a2.id, 0.0),
            (a3.id, 1.0),
            (a1.id, 2.0),
            (b2.id, 0.0),
            (b1.id, 1.0),
        ] {
            assert_eq!(svc.get_task(t).unwrap().sort_order, expected);
        }
    }

    #[test]
    fn reorder_tasks_leaves_untouched_tasks_alone() {
        let svc = open_service();
        let list = svc.create_list("Projects".into()).unwrap();
        let t1 = svc
            .create_task(CreateTaskInput {
                title: "one".into(),
                notes: None,
                priority: None,
                list_id: Some(list.id),
                due_date: None,
                due_time: None,
                tag_names: None,
            })
            .unwrap();
        let t2 = svc
            .create_task(CreateTaskInput {
                title: "two".into(),
                notes: None,
                priority: None,
                list_id: Some(list.id),
                due_date: None,
                due_time: None,
                tag_names: None,
            })
            .unwrap();
        let before = svc.get_task(t1.id).unwrap().sort_order;

        // Reorder only t2; t1 must keep its sort_order.
        svc.reorder_tasks(vec![t2.id]).unwrap();
        assert_eq!(svc.get_task(t1.id).unwrap().sort_order, before);
        assert_eq!(svc.get_task(t2.id).unwrap().sort_order, 0.0);
    }

    #[test]
    fn set_task_defer_hides_from_active_query() {
        let svc = open_service();
        let today = local_today(&SystemClock);
        let tomorrow = (chrono::NaiveDate::parse_from_str(&today, "%Y-%m-%d").unwrap()
            + chrono::Duration::days(1))
        .format("%Y-%m-%d")
        .to_string();

        let task = svc
            .create_task(CreateTaskInput {
                title: "defer me".into(),
                notes: None,
                priority: None,
                list_id: None,
                due_date: None,
                due_time: None,
                tag_names: None,
            })
            .unwrap();

        svc.set_task_defer(task.id, Some(tomorrow.clone())).unwrap();

        let active = svc
            .query_tasks(TaskQuery {
                status: Some(TaskStatus::Todo),
                ..Default::default()
            })
            .unwrap();
        assert!(!active.items.iter().any(|t| t.id == task.id));

        let deferred = svc
            .query_tasks(TaskQuery {
                deferred_only: Some(true),
                ..Default::default()
            })
            .unwrap();
        assert!(deferred.items.iter().any(|t| t.id == task.id));

        svc.set_task_defer(task.id, None).unwrap();
        let active_again = svc
            .query_tasks(TaskQuery {
                status: Some(TaskStatus::Todo),
                ..Default::default()
            })
            .unwrap();
        assert!(active_again.items.iter().any(|t| t.id == task.id));
    }

    #[test]
    fn set_task_defer_rejects_due_before_available() {
        let svc = open_service();
        let task = svc
            .create_task(CreateTaskInput {
                title: "conflict".into(),
                notes: None,
                priority: None,
                list_id: None,
                due_date: Some("2026-08-10".into()),
                due_time: None,
                tag_names: None,
            })
            .unwrap();
        assert!(svc
            .set_task_defer(task.id, Some("2026-08-20".into()))
            .is_err());
    }

    #[test]
    fn set_task_waiting_hides_from_active_and_today_due() {
        let svc = open_service();
        let today = local_today(&SystemClock);

        let task = svc
            .create_task(CreateTaskInput {
                title: "waiting task".into(),
                notes: None,
                priority: None,
                list_id: None,
                due_date: Some(today.clone()),
                due_time: None,
                tag_names: None,
            })
            .unwrap();

        svc.set_task_waiting(
            task.id,
            Some("Alice".into()),
            Some(today.clone()),
        )
        .unwrap();

        let updated = svc.get_task(task.id).unwrap();
        assert_eq!(updated.workflow_state, TaskWorkflowState::Waiting);
        assert_eq!(updated.waiting_for.as_deref(), Some("Alice"));

        let active = svc
            .query_tasks(TaskQuery {
                status: Some(TaskStatus::Todo),
                ..Default::default()
            })
            .unwrap();
        assert!(!active.items.iter().any(|t| t.id == task.id));

        let today_view = svc.today_tasks().unwrap();
        assert!(!today_view.due_today.iter().any(|t| t.id == task.id));
        assert!(today_view
            .waiting_follow_up
            .iter()
            .any(|t| t.id == task.id));

        svc.clear_task_waiting(task.id).unwrap();
        let active_again = svc
            .query_tasks(TaskQuery {
                status: Some(TaskStatus::Todo),
                ..Default::default()
            })
            .unwrap();
        assert!(active_again.items.iter().any(|t| t.id == task.id));
    }

    #[test]
    fn waiting_without_follow_up_not_in_today() {
        let svc = open_service();
        let task = svc
            .create_task(CreateTaskInput {
                title: "no follow up".into(),
                notes: None,
                priority: None,
                list_id: None,
                due_date: None,
                due_time: None,
                tag_names: None,
            })
            .unwrap();

        svc.set_task_waiting(task.id, Some("Bob".into()), None)
            .unwrap();

        let today_view = svc.today_tasks().unwrap();
        assert!(!today_view.waiting_follow_up.iter().any(|t| t.id == task.id));
    }

    #[test]
    fn daily_focus_add_remove_and_carry() {
        let svc = open_service();
        let today = local_today(&SystemClock);
        let yesterday = (chrono::NaiveDate::parse_from_str(&today, "%Y-%m-%d").unwrap()
            - chrono::Duration::days(1))
        .format("%Y-%m-%d")
        .to_string();

        let task = svc
            .create_task(CreateTaskInput {
                title: "focus me".into(),
                notes: None,
                priority: None,
                list_id: None,
                due_date: Some(today.clone()),
                due_time: None,
                tag_names: None,
            })
            .unwrap();

        svc.daily_focus_add(task.id, Some(today.clone())).unwrap();
        let view = svc.today_tasks().unwrap();
        assert!(view.focus.iter().any(|t| t.id == task.id));

        svc.daily_focus_remove(task.id, Some(today.clone())).unwrap();
        let view2 = svc.today_tasks().unwrap();
        assert!(!view2.focus.iter().any(|t| t.id == task.id));

        svc.daily_focus_add(task.id, Some(yesterday.clone())).unwrap();
        let carried = svc
            .daily_focus_carry(yesterday.clone(), today.clone())
            .unwrap();
        assert_eq!(carried.len(), 1);
        assert_eq!(carried[0].id, task.id);
        let view3 = svc.today_tasks().unwrap();
        assert!(view3.focus.iter().any(|t| t.id == task.id));
        assert!(view3.focus_carry_suggestions.is_empty());
    }

    #[test]
    fn defer_to_future_removes_daily_focus() {
        let svc = open_service();
        let today = local_today(&SystemClock);
        let tomorrow = (chrono::NaiveDate::parse_from_str(&today, "%Y-%m-%d").unwrap()
            + chrono::Duration::days(1))
        .format("%Y-%m-%d")
        .to_string();

        let task = svc
            .create_task(CreateTaskInput {
                title: "defer focus".into(),
                notes: None,
                priority: None,
                list_id: None,
                due_date: None,
                due_time: None,
                tag_names: None,
            })
            .unwrap();

        svc.daily_focus_add(task.id, None).unwrap();
        svc.set_task_defer(task.id, Some(tomorrow)).unwrap();
        let view = svc.today_tasks().unwrap();
        assert!(!view.focus.iter().any(|t| t.id == task.id));
    }

    #[test]
    fn defer_event_count_increments_on_postpone() {
        let svc = open_service();
        let today = local_today(&SystemClock);
        let yesterday = (chrono::NaiveDate::parse_from_str(&today, "%Y-%m-%d").unwrap()
            - chrono::Duration::days(1))
        .format("%Y-%m-%d")
        .to_string();
        let task = svc
            .create_task(CreateTaskInput {
                title: "postpone".into(),
                notes: None,
                priority: None,
                list_id: None,
                due_date: Some(yesterday),
                due_time: None,
                tag_names: None,
            })
            .unwrap();
        svc.postpone_task(task.id, 1).unwrap();
        let counts = svc.defer_counts_for_tasks(&[task.id]).unwrap();
        assert_eq!(*counts.get(&task.id).unwrap_or(&0), 1);
    }

    #[test]
    fn today_sort_suggestions_for_due_today_bucket() {
        let svc = open_service();
        let today = local_today(&SystemClock);
        let high = svc
            .create_task(CreateTaskInput {
                title: "high".into(),
                notes: None,
                priority: Some(TaskPriority::High),
                list_id: None,
                due_date: Some(today.clone()),
                due_time: None,
                tag_names: None,
            })
            .unwrap();
        let timed = svc
            .create_task(CreateTaskInput {
                title: "timed".into(),
                notes: None,
                priority: None,
                list_id: None,
                due_date: Some(today),
                due_time: Some("09:00".into()),
                tag_names: None,
            })
            .unwrap();

        let suggestions = svc
            .today_sort_suggestions(true, std::collections::HashMap::new())
            .unwrap();
        assert!(suggestions.enabled);
        assert_eq!(suggestions.suggestions.len(), 2);
        assert_eq!(suggestions.suggestions[0].task_id, timed.id);
        assert_eq!(suggestions.suggestions[1].task_id, high.id);
    }
}
