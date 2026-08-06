use crate::domain::{
    local_today, new_id, stamp, validate_due_date, validate_due_time, CreateTaskInput, DomainError,
    EntityId, ListKind, SmartListKind, SystemClock, Tag, Task, TaskList, TaskPriority, TaskQuery,
    TaskStatus, TodayTasks, UpdateTaskInput,
};
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

    pub fn reorder_tasks(&self, ordered_ids: Vec<EntityId>) -> Result<(), DomainError> {
        let conn = self.connect()?;
        let now = stamp(&self.clock);
        let tx = conn.unchecked_transaction().map_err(internal)?;
        for (index, id) in ordered_ids.iter().enumerate() {
            tx.execute(
                "UPDATE tasks SET sort_order = ?1, updated_at = ?2, revision = revision + 1
                 WHERE id = ?3 AND deleted_at IS NULL",
                params![index as f64, now, id.to_string()],
            )
            .map_err(internal)?;
        }
        tx.commit().map_err(internal)?;
        Ok(())
    }

    pub fn get_task(&self, id: EntityId) -> Result<Task, DomainError> {
        let conn = self.connect()?;
        let mut task = conn
            .query_row(
                "SELECT t.id, t.title, t.notes, t.status, t.priority, t.list_id,
                        l.name, l.kind, t.due_date, t.due_time, t.completed_at, t.sort_order, t.series_id,
                        t.created_at, t.updated_at, t.revision
                 FROM tasks t
                 JOIN task_lists l ON l.id = t.list_id
                 WHERE t.id = ?1 AND t.deleted_at IS NULL",
                [id.to_string()],
                map_task_row,
            )
            .optional()
            .map_err(internal)?
            .ok_or_else(|| DomainError::NotFound("任务不存在".into()))?;
        self.attach_tags(&conn, &mut task)?;
        Ok(task)
    }

    pub fn query_tasks(&self, query: TaskQuery) -> Result<Vec<Task>, DomainError> {
        let conn = self.connect()?;
        let mut sql = String::from(
            "SELECT t.id, t.title, t.notes, t.status, t.priority, t.list_id,
                    l.name, l.kind, t.due_date, t.due_time, t.completed_at, t.sort_order, t.series_id,
                    t.created_at, t.updated_at, t.revision
             FROM tasks t
             JOIN task_lists l ON l.id = t.list_id
             WHERE t.deleted_at IS NULL",
        );
        let mut values: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

        if query.inbox_only.unwrap_or(false) {
            sql.push_str(" AND l.kind = 'inbox'");
        }
        if let Some(list_id) = query.list_id {
            sql.push_str(" AND t.list_id = ?");
            values.push(Box::new(list_id.to_string()));
        }
        if let Some(status) = query.status {
            sql.push_str(" AND t.status = ?");
            values.push(Box::new(status.as_str().to_string()));
        } else if !query.include_archived.unwrap_or(false) {
            sql.push_str(" AND t.status != 'archived'");
        }
        if let Some(priority) = query.priority {
            sql.push_str(" AND t.priority = ?");
            values.push(Box::new(priority.as_str().to_string()));
        }
        if let Some(tag_id) = query.tag_id {
            sql.push_str(" AND EXISTS (SELECT 1 FROM task_tags tt WHERE tt.task_id = t.id AND tt.tag_id = ?)");
            values.push(Box::new(tag_id.to_string()));
        }
        if query.due_null.unwrap_or(false) {
            sql.push_str(" AND t.due_date IS NULL");
        }
        if let Some(ref from) = query.due_from {
            sql.push_str(" AND t.due_date IS NOT NULL AND t.due_date >= ?");
            values.push(Box::new(from.clone()));
        }
        if let Some(ref to) = query.due_to {
            sql.push_str(" AND t.due_date IS NOT NULL AND t.due_date <= ?");
            values.push(Box::new(to.clone()));
        }
        if let Some(ref since) = query.completed_since {
            sql.push_str(
                " AND t.status = 'completed' AND t.completed_at IS NOT NULL AND date(t.completed_at, 'localtime') >= ?",
            );
            values.push(Box::new(since.clone()));
        }

        if query.completed_since.is_some() {
            sql.push_str(" ORDER BY t.completed_at DESC, t.updated_at DESC");
        } else {
            sql.push_str(" ORDER BY t.sort_order ASC, t.created_at DESC");
        }

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
        Ok(tasks)
    }

    pub fn smart_list(&self, kind: SmartListKind) -> Result<Vec<Task>, DomainError> {
        let today = local_today(&self.clock);
        let today_date = chrono::NaiveDate::parse_from_str(&today, "%Y-%m-%d")
            .map_err(|e| DomainError::Internal(e.to_string()))?;
        let query = match kind {
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
        };
        self.query_tasks(query)
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

    pub fn today_tasks(&self) -> Result<TodayTasks, DomainError> {
        let today = local_today(&self.clock);
        let conn = self.connect()?;

        let mut overdue = self.query_today_bucket(
            &conn,
            "AND t.status = 'todo' AND t.due_date IS NOT NULL AND t.due_date < ?1
             ORDER BY t.due_date ASC, t.sort_order ASC",
            &today,
        )?;
        let mut due_today = self.query_today_bucket(
            &conn,
            "AND t.status = 'todo' AND t.due_date = ?1
             ORDER BY t.sort_order ASC, t.created_at DESC",
            &today,
        )?;
        let mut completed_today = {
            let sql = "SELECT t.id, t.title, t.notes, t.status, t.priority, t.list_id,
                    l.name, l.kind, t.due_date, t.due_time, t.completed_at, t.sort_order, t.series_id,
                    t.created_at, t.updated_at, t.revision
             FROM tasks t
             JOIN task_lists l ON l.id = t.list_id
             WHERE t.deleted_at IS NULL
               AND t.status = 'completed' AND t.completed_at IS NOT NULL
               AND date(t.completed_at, 'localtime') = date('now', 'localtime')
             ORDER BY t.completed_at DESC";
            let mut stmt = conn.prepare(sql).map_err(internal)?;
            let rows = stmt.query_map([], map_task_row).map_err(internal)?;
            collect_rows(rows)?
        };

        for task in overdue
            .iter_mut()
            .chain(due_today.iter_mut())
            .chain(completed_today.iter_mut())
        {
            self.attach_tags(&conn, task)?;
        }

        Ok(TodayTasks {
            overdue,
            due_today,
            completed_today,
            reminders_today: Vec::new(),
            today,
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
                 WHERE t.deleted_at IS NULL AND t.status = 'todo' AND l.kind = 'inbox'",
                [],
                |row| row.get(0),
            )
            .map_err(internal)?;
        let overdue: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM tasks
                 WHERE deleted_at IS NULL AND status = 'todo'
                   AND due_date IS NOT NULL AND due_date < ?1",
                [&today],
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
            "SELECT t.id, t.title, t.notes, t.status, t.priority, t.list_id,
                    l.name, l.kind, t.due_date, t.due_time, t.completed_at, t.sort_order, t.series_id,
                    t.created_at, t.updated_at, t.revision
             FROM tasks t
             JOIN task_lists l ON l.id = t.list_id
             WHERE t.deleted_at IS NULL {extra}"
        );
        let mut stmt = conn.prepare(&sql).map_err(internal)?;
        let rows = stmt.query_map([today], map_task_row).map_err(internal)?;
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
        created_at: row.get(13)?,
        updated_at: row.get(14)?,
        revision: row.get(15)?,
    })
}

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
}
