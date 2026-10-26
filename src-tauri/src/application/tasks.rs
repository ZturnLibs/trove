use crate::domain::{
    compute_today_sort_suggestions, local_today, new_id, stamp, validate_due_date,
    validate_due_time, CreateTaskInput, DeleteListResult, DomainError, EntityId,
    ListDeleteDisposition, ListGroupScope, ListKind, PagedResult, should_apply_active_list_filter,
    SmartListKind, SystemClock, Tag, Task, TaskList, TaskPriority, TaskQuery, TaskStatus,
    TaskWorkflowState, ChecklistItem, ChecklistUpdateInput, TaskChecklist, CHECKLIST_MAX_ITEMS,
    TodaySortSuggestions, TodayTasks, UpdateTaskInput, validate_checklist_content,
    validate_due_vs_available, TaskDeleteDisposition, TaskTreeExpanded, validate_parent_depth,
};
use crate::application::list_groups::list_group_scope_matches;
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

    pub(crate) fn connect(&self) -> Result<Connection, DomainError> {
        self.db.connect().map_err(internal)
    }

    pub(crate) fn clock_ref(&self) -> &SystemClock {
        &self.clock
    }

    pub fn list_lists(&self) -> Result<Vec<TaskList>, DomainError> {
        let conn = self.connect()?;
        let mut stmt = conn
            .prepare(
                "SELECT id, name, kind, sort_order, created_at, updated_at, revision, group_id
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
            "SELECT id, name, kind, sort_order, created_at, updated_at, revision, group_id
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

        // Subtask context: validate parent and inherit its list when unset.
        let parent_chain_len = match input.parent_id {
            Some(parent_id) => {
                let parent = self.get_task(parent_id)?;
                if parent.status == TaskStatus::Archived {
                    return Err(DomainError::Validation("已归档任务不能再添加子任务".into()));
                }
                // The new task's ancestor chain = parent's chain + 1.
                let chain = self.ancestor_chain_len(&parent_id)?;
                validate_parent_depth(chain + 1)?;
                Some(parent.list_id)
            }
            None => None,
        };

        let list_id = match input.list_id {
            Some(id) => {
                let _ = self.get_list(id)?;
                id
            }
            // Inherit the parent's list when creating a subtask without an
            // explicit list; top-level tasks default to the inbox.
            None => match parent_chain_len {
                Some(parent_list_id) => parent_list_id,
                None => self.inbox_list_id()?,
            },
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
        let child_order: f64 = match input.parent_id {
            Some(parent_id) => conn
                .query_row(
                    "SELECT COALESCE(MAX(child_order), 0) + 1 FROM tasks WHERE parent_id = ?1 AND deleted_at IS NULL",
                    [parent_id.to_string()],
                    |row| row.get(0),
                )
                .unwrap_or(1.0),
            None => 0.0,
        };

        let tx = conn.unchecked_transaction().map_err(internal)?;
        tx.execute(
            "INSERT INTO tasks (
                id, title, notes, status, priority, list_id, due_date, due_time,
                completed_at, sort_order, parent_id, child_order,
                created_at, updated_at, revision, deleted_at
             ) VALUES (?1, ?2, ?3, 'todo', ?4, ?5, ?6, ?7, NULL, ?8, ?9, ?10, ?11, ?11, 1, NULL)",
            params![
                id.to_string(),
                title,
                notes,
                priority.as_str(),
                list_id.to_string(),
                input.due_date,
                input.due_time,
                sort_order,
                input.parent_id.map(|p| p.to_string()),
                child_order,
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
        let affected = self.complete_with_aggregation(&tx, id, &now)?;

        // Series spawn only for the explicitly completed target.
        if let Some(series_id) = task.series_id {
            self.spawn_next_series_instance(&tx, &task, series_id, &now)?;
        }
        tx.commit().map_err(internal)?;
        // Refresh the returned task (may have changed parent status too).
        let _ = affected;
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
        let tx = conn.unchecked_transaction().map_err(internal)?;
        self.uncomplete_with_aggregation(&tx, id, &now)?;
        tx.commit().map_err(internal)?;
        self.get_task(id)
    }

    pub fn unarchive_task(&self, id: EntityId) -> Result<Task, DomainError> {
        let task = self.get_task(id)?;
        if task.status != TaskStatus::Archived {
            return Ok(task);
        }
        let conn = self.connect()?;
        let now = stamp(&self.clock);
        let tx = conn.unchecked_transaction().map_err(internal)?;
        tx.execute(
            "UPDATE tasks SET status = 'todo', updated_at = ?1, revision = revision + 1
             WHERE id = ?2 AND deleted_at IS NULL",
            params![now, id.to_string()],
        )
        .map_err(internal)?;
        // Unarchive cascades to descendants so the subtree stays consistent.
        for d in self.descendant_ids(&tx, id)? {
            tx.execute(
                "UPDATE tasks SET status = 'todo', updated_at = ?1, revision = revision + 1
                 WHERE id = ?2 AND deleted_at IS NULL",
                params![now, d.to_string()],
            )
            .map_err(internal)?;
        }
        tx.commit().map_err(internal)?;
        self.get_task(id)
    }

    pub fn archive_task(&self, id: EntityId) -> Result<Task, DomainError> {
        let _ = self.get_task(id)?;
        let conn = self.connect()?;
        let now = stamp(&self.clock);
        let tx = conn.unchecked_transaction().map_err(internal)?;
        tx.execute(
            "UPDATE tasks SET status = 'archived', updated_at = ?1, revision = revision + 1
             WHERE id = ?2 AND deleted_at IS NULL",
            params![now, id.to_string()],
        )
        .map_err(internal)?;
        // Archive cascades to descendants so the subtree stays consistent.
        for d in self.descendant_ids(&tx, id)? {
            tx.execute(
                "UPDATE tasks SET status = 'archived', updated_at = ?1, revision = revision + 1
                 WHERE id = ?2 AND deleted_at IS NULL",
                params![now, d.to_string()],
            )
            .map_err(internal)?;
        }
        tx.commit().map_err(internal)?;
        self.get_task(id)
    }

    pub fn delete_task(&self, id: EntityId) -> Result<(), DomainError> {
        let _ = self.get_task(id)?;
        let conn = self.connect()?;
        let now = stamp(&self.clock);
        // Refuse to silently orphan children; callers with subtasks must use
        // delete_task_tree with an explicit disposition.
        let child_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM tasks WHERE parent_id = ?1 AND deleted_at IS NULL",
                [id.to_string()],
                |row| row.get(0),
            )
            .map_err(internal)?;
        if child_count > 0 {
            return Err(DomainError::Validation(
                "该任务有子任务，请选择级联删除或仅删除父任务".into(),
            ));
        }
        conn.execute(
            "UPDATE tasks SET deleted_at = ?1, updated_at = ?1, revision = revision + 1
             WHERE id = ?2 AND deleted_at IS NULL",
            params![now, id.to_string()],
        )
        .map_err(internal)?;
        // Checklist items follow their task into the tombstone (no orphans).
        conn.execute(
            "UPDATE task_checklist_items SET deleted_at = ?1, updated_at = ?1
             WHERE task_id = ?2 AND deleted_at IS NULL",
            params![now, id.to_string()],
        )
        .map_err(internal)?;
        // Tree expand state follows its task.
        conn.execute(
            "DELETE FROM task_tree_expanded WHERE task_id = ?1",
            [id.to_string()],
        )
        .map_err(internal)?;
        Ok(())
    }

    /// Delete a task with an explicit disposition for its subtree.
    /// Returns the ids of every deleted task (cascade) or the deleted parent
    /// plus promoted children (promote).
    pub fn delete_task_tree(
        &self,
        id: EntityId,
        disposition: TaskDeleteDisposition,
    ) -> Result<Vec<EntityId>, DomainError> {
        let _ = self.get_task(id)?;
        let conn = self.connect()?;
        let now = stamp(&self.clock);
        let tx = conn.unchecked_transaction().map_err(internal)?;

        let mut affected = vec![id];
        let children = self.descendant_ids(&tx, id)?;

        match disposition {
            TaskDeleteDisposition::Cascade => {
                affected.extend(children.iter().copied());
                for tid in &affected {
                    tx.execute(
                        "UPDATE tasks SET deleted_at = ?1, updated_at = ?1, revision = revision + 1
                         WHERE id = ?2 AND deleted_at IS NULL",
                        params![now, tid.to_string()],
                    )
                    .map_err(internal)?;
                    tx.execute(
                        "UPDATE task_checklist_items SET deleted_at = ?1, updated_at = ?1
                         WHERE task_id = ?2 AND deleted_at IS NULL",
                        params![now, tid.to_string()],
                    )
                    .map_err(internal)?;
                    tx.execute(
                        "DELETE FROM task_tree_expanded WHERE task_id = ?1",
                        [tid.to_string()],
                    )
                    .map_err(internal)?;
                }
            }
            TaskDeleteDisposition::Promote => {
                // Capture direct children BEFORE detaching them so we can
                // renumber their sort_order in their own list.
                let mut direct_children: Vec<EntityId> = Vec::new();
                for child in &children {
                    let is_direct: i64 = tx
                        .query_row(
                            "SELECT COUNT(*) FROM tasks WHERE id = ?1 AND parent_id = ?2 AND deleted_at IS NULL",
                            params![child.to_string(), id.to_string()],
                            |row| row.get(0),
                        )
                        .map_err(internal)?;
                    if is_direct == 1 {
                        direct_children.push(*child);
                    }
                }
                // Promote direct children to top level (parent_id = NULL).
                tx.execute(
                    "UPDATE tasks SET parent_id = NULL, child_order = 0, updated_at = ?1, revision = revision + 1
                     WHERE parent_id = ?2 AND deleted_at IS NULL",
                    params![now, id.to_string()],
                )
                .map_err(internal)?;
                for child in &direct_children {
                    let list_id: String = tx
                        .query_row(
                            "SELECT list_id FROM tasks WHERE id = ?1 AND deleted_at IS NULL",
                            [child.to_string()],
                            |row| row.get(0),
                        )
                        .map_err(internal)?;
                    let sort_order: f64 = tx
                        .query_row(
                            "SELECT COALESCE(MAX(sort_order), 0) + 1 FROM tasks WHERE list_id = ?1 AND deleted_at IS NULL",
                            [&list_id],
                            |row| row.get(0),
                        )
                        .unwrap_or(1.0);
                    tx.execute(
                        "UPDATE tasks SET sort_order = ?1, updated_at = ?2, revision = revision + 1
                         WHERE id = ?3 AND deleted_at IS NULL",
                        params![sort_order, now, child.to_string()],
                    )
                    .map_err(internal)?;
                    affected.push(*child);
                }
                tx.execute(
                    "UPDATE tasks SET deleted_at = ?1, updated_at = ?1, revision = revision + 1
                     WHERE id = ?2 AND deleted_at IS NULL",
                    params![now, id.to_string()],
                )
                .map_err(internal)?;
                tx.execute(
                    "UPDATE task_checklist_items SET deleted_at = ?1, updated_at = ?1
                     WHERE task_id = ?2 AND deleted_at IS NULL",
                    params![now, id.to_string()],
                )
                .map_err(internal)?;
                tx.execute(
                    "DELETE FROM task_tree_expanded WHERE task_id = ?1",
                    [id.to_string()],
                )
                .map_err(internal)?;
            }
        }

        tx.commit().map_err(internal)?;
        Ok(affected)
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

    /// Nest a task under a parent (or promote to top level when None).
    /// Validates cycles, max depth, and active state on both sides.
    pub fn set_task_parent(
        &self,
        id: EntityId,
        parent_id: Option<EntityId>,
    ) -> Result<Task, DomainError> {
        let task = self.get_task(id)?;
        let conn = self.connect()?;
        let now = stamp(&self.clock);
        let tx = conn.unchecked_transaction().map_err(internal)?;

        match parent_id {
            Some(parent) => {
                if parent == id {
                    return Err(DomainError::Validation("不能把任务设为自己的子任务".into()));
                }
                // Cycle check: parent must not be one of our descendants.
                let descendants = self.descendant_ids(&tx, id)?;
                if descendants.contains(&parent) {
                    return Err(DomainError::Validation(
                        "不能把任务设为后代的子任务（会形成循环）".into(),
                    ));
                }
                // Depth check: the task's ancestor chain = parent's chain + 1.
                let chain_len = self.ancestor_chain_len(&parent)?;
                validate_parent_depth(chain_len + 1)?;
                // Parent must be active.
                let parent_status: String = tx
                    .query_row(
                        "SELECT status FROM tasks WHERE id = ?1 AND deleted_at IS NULL",
                        [parent.to_string()],
                        |row| row.get(0),
                    )
                    .optional()
                    .map_err(internal)?
                    .ok_or_else(|| DomainError::NotFound("父任务不存在".into()))?;
                if parent_status == "archived" {
                    return Err(DomainError::Validation("已归档任务不能再添加子任务".into()));
                }
                let child_order: f64 = tx
                    .query_row(
                        "SELECT COALESCE(MAX(child_order), 0) + 1 FROM tasks WHERE parent_id = ?1 AND deleted_at IS NULL",
                        [parent.to_string()],
                        |row| row.get(0),
                    )
                    .unwrap_or(1.0);
                tx.execute(
                    "UPDATE tasks SET parent_id = ?1, child_order = ?2, updated_at = ?3, revision = revision + 1
                     WHERE id = ?4 AND deleted_at IS NULL",
                    params![parent.to_string(), child_order, now, id.to_string()],
                )
                .map_err(internal)?;
            }
            None => {
                // Promote to top level: keep list_id, append at its end.
                let sort_order: f64 = tx
                    .query_row(
                        "SELECT COALESCE(MAX(sort_order), 0) + 1 FROM tasks WHERE list_id = ?1 AND deleted_at IS NULL",
                        [task.list_id.to_string()],
                        |row| row.get(0),
                    )
                    .unwrap_or(1.0);
                tx.execute(
                    "UPDATE tasks SET parent_id = NULL, child_order = 0, sort_order = ?1, updated_at = ?2, revision = revision + 1
                     WHERE id = ?3 AND deleted_at IS NULL",
                    params![sort_order, now, id.to_string()],
                )
                .map_err(internal)?;
            }
        }

        tx.commit().map_err(internal)?;
        self.get_task(id)
    }

    /// Rewrite sibling ordering within one parent (or among top-level tasks
    /// when `parent_id` is None). Mirrors `reorder_tasks` but scoped by parent.
    pub fn reorder_subtasks(
        &self,
        parent_id: Option<EntityId>,
        ordered_ids: Vec<EntityId>,
    ) -> Result<(), DomainError> {
        if ordered_ids.is_empty() {
            return Ok(());
        }
        // Top-level reorder keeps list-scoped sort_order semantics (the same
        // as the pre-tree drag reorder).
        if parent_id.is_none() {
            return self.reorder_tasks(ordered_ids);
        }
        let conn = self.connect()?;
        let now = stamp(&self.clock);

        let placeholders = vec!["?"; ordered_ids.len()].join(", ");
        let id_strings: Vec<String> = ordered_ids.iter().map(|id| id.to_string()).collect();
        let params_ref: Vec<&dyn rusqlite::types::ToSql> = id_strings
            .iter()
            .map(|s| s as &dyn rusqlite::types::ToSql)
            .collect();
        let expected_parent = parent_id.map(|p| p.to_string());
        {
            let mut stmt = conn
                .prepare(&format!(
                    "SELECT id, parent_id FROM tasks
                     WHERE id IN ({placeholders}) AND deleted_at IS NULL"
                ))
                .map_err(internal)?;
            let rows = stmt
                .query_map(params_ref.as_slice(), |row| {
                    Ok((parse_id(row.get(0)?)?, row.get::<_, Option<String>>(1)?))
                })
                .map_err(internal)?;
            for row in rows {
                let (_id, parent) = row.map_err(internal)?;
                if parent != expected_parent {
                    return Err(DomainError::Validation(
                        "排序列表包含不属于该父任务的任务".into(),
                    ));
                }
            }
        }

        let tx = conn.unchecked_transaction().map_err(internal)?;
        for (index, id) in ordered_ids.iter().enumerate() {
            tx.execute(
                "UPDATE tasks SET child_order = ?1, updated_at = ?2, revision = revision + 1
                 WHERE id = ?3 AND deleted_at IS NULL",
                params![index as f64, now, id.to_string()],
            )
            .map_err(internal)?;
        }
        tx.commit().map_err(internal)?;
        Ok(())
    }

    /// Tree query: matching tasks plus their ancestors and descendants (all
    /// depths), deduplicated. Business filters apply to the matched set only;
    /// the closure is only constrained by active (non-deleted) status.
    pub fn query_tree(&self, query: TaskQuery) -> Result<Vec<Task>, DomainError> {
        let conn = self.connect()?;
        // Build the same filters as query_tasks (no pagination).
        let (filters, values) = self.build_task_filters(&query)?;
        let from_clause = " FROM tasks t
             JOIN task_lists l ON l.id = t.list_id
             WHERE t.deleted_at IS NULL";

        let matched_sql = format!(
            "SELECT t.id{from_clause}{filters}"
        );
        let params_ref: Vec<&dyn rusqlite::types::ToSql> =
            values.iter().map(|v| v.as_ref()).collect();
        let mut stmt = conn.prepare(&matched_sql).map_err(internal)?;
        let matched_rows = stmt
            .query_map(params_ref.as_slice(), |row| {
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
        let matched = collect_rows(matched_rows)?;

        // Closure: matched + ancestors + descendants.
        let mut ids: std::collections::HashSet<EntityId> =
            matched.iter().copied().collect();
        for id in &matched {
            let mut current = id.to_string();
            loop {
                let parent: Option<String> = conn
                    .query_row(
                        "SELECT parent_id FROM tasks WHERE id = ?1 AND deleted_at IS NULL",
                        [&current],
                        |row| row.get::<_, Option<String>>(0),
                    )
                    .map_err(internal)?;
                match parent {
                    Some(p) => {
                        let parsed = p
                            .parse()
                            .map_err(|e| DomainError::Internal(format!("invalid parent id: {e}")))?;
                        ids.insert(parsed);
                        current = p;
                    }
                    None => break,
                }
            }
            for d in self.descendant_ids(&conn, *id)? {
                ids.insert(d);
            }
        }

        if ids.is_empty() {
            return Ok(Vec::new());
        }
        let placeholders = vec!["?"; ids.len()].join(", ");
        let id_strings: Vec<String> = ids.iter().map(|id| id.to_string()).collect();
        let params_ref: Vec<&dyn rusqlite::types::ToSql> = id_strings
            .iter()
            .map(|s| s as &dyn rusqlite::types::ToSql)
            .collect();
        let sql = format!(
            "SELECT {TASK_ROW_SELECT}{from_clause} AND t.id IN ({placeholders})
             ORDER BY t.parent_id IS NOT NULL, t.parent_id, t.child_order ASC, t.sort_order ASC, t.created_at DESC"
        );
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

    pub fn tree_expanded_list(&self) -> Result<Vec<TaskTreeExpanded>, DomainError> {
        let conn = self.connect()?;
        let mut stmt = conn
            .prepare(
                "SELECT task_id, expanded, updated_at FROM task_tree_expanded",
            )
            .map_err(internal)?;
        let rows = stmt
            .query_map([], |row| {
                Ok(TaskTreeExpanded {
                    task_id: parse_id(row.get(0)?)?,
                    expanded: row.get::<_, i64>(1)? != 0,
                    updated_at: row.get(2)?,
                })
            })
            .map_err(internal)?;
        collect_rows(rows)
    }

    pub fn set_tree_expanded(&self, task_id: EntityId, expanded: bool) -> Result<(), DomainError> {
        let conn = self.connect()?;
        let now = stamp(&self.clock);
        if expanded {
            // Default is expanded; removing the row restores the default.
            conn.execute(
                "DELETE FROM task_tree_expanded WHERE task_id = ?1",
                [task_id.to_string()],
            )
            .map_err(internal)?;
        } else {
            conn.execute(
                "INSERT INTO task_tree_expanded (task_id, expanded, updated_at)
                 VALUES (?1, 0, ?2)
                 ON CONFLICT(task_id) DO UPDATE SET expanded = 0, updated_at = excluded.updated_at",
                params![task_id.to_string(), now],
            )
            .map_err(internal)?;
        }
        Ok(())
    }

    /// Build the WHERE filters used by query_tasks (and query_tree).
    fn build_task_filters(
        &self,
        query: &TaskQuery,
    ) -> Result<(String, Vec<Box<dyn rusqlite::types::ToSql>>), DomainError> {
        let mut filters = String::new();
        let mut values: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

        if query.inbox_only.unwrap_or(false) {
            filters.push_str(" AND l.kind = 'inbox'");
        }
        if let Some(list_id) = query.list_id {
            filters.push_str(" AND t.list_id = ?");
            values.push(Box::new(list_id.to_string()));
        }
        if let Some(group_id) = query.list_group_id {
            // 任务页按分组过滤：任务所属清单挂在目标分组下。
            // 软删清单/软删分组一律不命中（归档任务可能仍挂在软删清单上，
            // 解散分组也只清理活跃清单的 group_id），与今日页内存过滤语义一致。
            filters.push_str(
                " AND EXISTS (SELECT 1 FROM task_lists lg
                     JOIN task_list_groups lgg ON lgg.id = lg.group_id
                     WHERE lg.id = t.list_id AND lg.deleted_at IS NULL
                       AND lgg.deleted_at IS NULL AND lgg.id = ?)",
            );
            values.push(Box::new(group_id.to_string()));
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
        if let Some(parent_id) = query.parent_id {
            filters.push_str(" AND t.parent_id = ?");
            values.push(Box::new(parent_id.to_string()));
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

        Ok((filters, values))
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
        let (filters, mut values) = self.build_task_filters(&query)?;

        let order_by = if query.completed_since.is_some() {
            " ORDER BY t.completed_at DESC, t.updated_at DESC"
        } else if query.parent_id.is_some() {
            // Subtask list: sibling order within the parent.
            " ORDER BY t.child_order ASC, t.created_at DESC"
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

    /// 今日页聚合。`scope` 来自今日页过滤 chips：None = 全部（现状语义）；
    /// Group/Ungrouped 时五个任务区块（重点/等待/逾期/今日任务/已完成）统一
    /// 按任务所属清单的分组过滤；提醒区块在命令层按关联任务的分组过滤。
    pub fn today_tasks(&self, scope: Option<ListGroupScope>) -> Result<TodayTasks, DomainError> {
        let today = local_today(&self.clock);
        let conn = self.connect()?;
        let group_by_list = self.list_group_map(&conn)?;

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

        if scope.is_some() {
            let keep = |t: &Task| {
                list_group_scope_matches(t.list_id, &group_by_list, scope)
            };
            overdue.retain(keep);
            due_today.retain(keep);
            completed_today.retain(keep);
            waiting_follow_up.retain(keep);
            focus.retain(keep);
            focus_carry_suggestions.retain(keep);
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
        let today_view = self.today_tasks(None)?;
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

    // -------------------------------------------------------------------
    // v2.1 subtasks: helpers
    // -------------------------------------------------------------------

    /// Read the two subtask aggregation flags from the settings table.
    /// Missing/legacy settings default to true (current behavior for both).
    fn subtask_aggregation_settings(&self) -> Result<(bool, bool), DomainError> {
        #[derive(serde::Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct SubtaskFlags {
            #[serde(default = "default_true")]
            subtask_auto_complete_parent: bool,
            #[serde(default = "default_true")]
            subtask_cascade_children: bool,
        }
        fn default_true() -> bool {
            true
        }

        let conn = self.connect()?;
        let raw: Option<String> = conn
            .query_row(
                "SELECT value_json FROM settings WHERE key = 'app.settings'",
                [],
                |row| row.get(0),
            )
            .optional()
            .map_err(internal)?;
        match raw {
            Some(json) => {
                let flags: SubtaskFlags = serde_json::from_str(&json)
                    .map_err(|e| DomainError::Internal(format!("invalid settings json: {e}")))?;
                Ok((
                    flags.subtask_auto_complete_parent,
                    flags.subtask_cascade_children,
                ))
            }
            None => Ok((true, true)),
        }
    }

    /// Number of ancestors the task currently has (0 = top level).
    fn ancestor_chain_len(&self, id: &EntityId) -> Result<usize, DomainError> {
        let conn = self.connect()?;
        let mut current = id.to_string();
        let mut len = 0usize;
        loop {
            let parent: Option<String> = conn
                .query_row(
                    "SELECT parent_id FROM tasks WHERE id = ?1 AND deleted_at IS NULL",
                    [&current],
                    |row| row.get::<_, Option<String>>(0),
                )
                .map_err(internal)?;
            match parent {
                Some(p) => {
                    current = p;
                    len += 1;
                }
                None => return Ok(len),
            }
        }
    }

    /// Recursively collect all active descendants of a task (excluding itself).
    fn descendant_ids(&self, conn: &Connection, id: EntityId) -> Result<Vec<EntityId>, DomainError> {
        let mut stmt = conn
            .prepare(
                "WITH RECURSIVE descendants(id) AS (
                    SELECT id FROM tasks WHERE parent_id = ?1 AND deleted_at IS NULL
                    UNION ALL
                    SELECT t.id FROM tasks t
                      JOIN descendants d ON t.parent_id = d.id
                    WHERE t.deleted_at IS NULL
                 )
                 SELECT id FROM descendants",
            )
            .map_err(internal)?;
        let rows = stmt
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
            .map_err(internal)?;
        collect_rows(rows)
    }

    /// Are all active direct children of `parent_id` completed?
    /// A parent with no active children returns true (trivially complete).
    fn all_children_completed(&self, conn: &Connection, parent_id: EntityId) -> Result<bool, DomainError> {
        let total: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM tasks WHERE parent_id = ?1 AND deleted_at IS NULL",
                [parent_id.to_string()],
                |row| row.get(0),
            )
            .map_err(internal)?;
        if total == 0 {
            return Ok(true);
        }
        let done: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM tasks WHERE parent_id = ?1 AND deleted_at IS NULL
                 AND status = 'completed'",
                [parent_id.to_string()],
                |row| row.get(0),
            )
            .map_err(internal)?;
        Ok(done == total)
    }

    /// Complete a task plus (per settings) cascade descendants / auto-complete
    /// ancestors. Runs inside the caller's transaction.
    fn complete_with_aggregation(
        &self,
        tx: &Connection,
        id: EntityId,
        now: &str,
    ) -> Result<Vec<EntityId>, DomainError> {
        let (auto_complete_parent, cascade_children) = self.subtask_aggregation_settings()?;
        let mut affected = vec![id];

        tx.execute(
            "UPDATE tasks SET status = 'completed', completed_at = ?1, updated_at = ?1, revision = revision + 1
             WHERE id = ?2 AND deleted_at IS NULL",
            params![now, id.to_string()],
        )
        .map_err(internal)?;
        tx.execute("DELETE FROM daily_focus WHERE task_id = ?1", params![id.to_string()])
            .map_err(internal)?;

        // Downward cascade: complete all active descendants (no per-task
        // series spawn — only the explicit target spawns its next instance).
        if cascade_children {
            for d in self.descendant_ids(tx, id)? {
                tx.execute(
                    "UPDATE tasks SET status = 'completed', completed_at = ?1, updated_at = ?1, revision = revision + 1
                     WHERE id = ?2 AND deleted_at IS NULL",
                    params![now, d.to_string()],
                )
                .map_err(internal)?;
                tx.execute("DELETE FROM daily_focus WHERE task_id = ?1", params![d.to_string()])
                    .map_err(internal)?;
                affected.push(d);
            }
        }

        // Upward aggregation: auto-complete ancestors whose direct children
        // are all completed.
        if auto_complete_parent {
            let mut current = id;
            loop {
                let parent: Option<String> = tx
                    .query_row(
                        "SELECT parent_id FROM tasks WHERE id = ?1 AND deleted_at IS NULL",
                        [current.to_string()],
                        |row| row.get::<_, Option<String>>(0),
                    )
                    .map_err(internal)?;
                let Some(parent_raw) = parent else { break };
                let parent_id: EntityId = parent_raw
                    .parse()
                    .map_err(|e| DomainError::Internal(format!("invalid parent id: {e}")))?;
                if !self.all_children_completed(tx, parent_id)? {
                    break;
                }
                tx.execute(
                    "UPDATE tasks SET status = 'completed', completed_at = ?1, updated_at = ?1, revision = revision + 1
                     WHERE id = ?2 AND deleted_at IS NULL",
                    params![now, parent_id.to_string()],
                )
                .map_err(internal)?;
                tx.execute("DELETE FROM daily_focus WHERE task_id = ?1", params![parent_id.to_string()])
                    .map_err(internal)?;
                affected.push(parent_id);
                current = parent_id;
            }
        }

        Ok(affected)
    }

    /// Restore a completed task plus (per settings) cascade descendants /
    /// auto-restore ancestors. Runs inside the caller's transaction.
    fn uncomplete_with_aggregation(
        &self,
        tx: &Connection,
        id: EntityId,
        now: &str,
    ) -> Result<Vec<EntityId>, DomainError> {
        let (auto_complete_parent, cascade_children) = self.subtask_aggregation_settings()?;
        let mut affected = vec![id];

        tx.execute(
            "UPDATE tasks SET status = 'todo', completed_at = NULL, updated_at = ?1, revision = revision + 1
             WHERE id = ?2 AND deleted_at IS NULL",
            params![now, id.to_string()],
        )
        .map_err(internal)?;

        if cascade_children {
            for d in self.descendant_ids(tx, id)? {
                tx.execute(
                    "UPDATE tasks SET status = 'todo', completed_at = NULL, updated_at = ?1, revision = revision + 1
                     WHERE id = ?2 AND deleted_at IS NULL",
                    params![now, d.to_string()],
                )
                .map_err(internal)?;
                affected.push(d);
            }
        }

        // Upward aggregation (bidirectional): restore completed ancestors.
        if auto_complete_parent {
            let mut current = id;
            loop {
                let parent: Option<String> = tx
                    .query_row(
                        "SELECT parent_id FROM tasks WHERE id = ?1 AND deleted_at IS NULL",
                        [current.to_string()],
                        |row| row.get::<_, Option<String>>(0),
                    )
                    .map_err(internal)?;
                let Some(parent_raw) = parent else { break };
                let parent_id: EntityId = parent_raw
                    .parse()
                    .map_err(|e| DomainError::Internal(format!("invalid parent id: {e}")))?;
                // Stop at the first ancestor that is not completed.
                let parent_status: String = tx
                    .query_row(
                        "SELECT status FROM tasks WHERE id = ?1 AND deleted_at IS NULL",
                        [parent_id.to_string()],
                        |row| row.get(0),
                    )
                    .optional()
                    .map_err(internal)?
                    .unwrap_or_default();
                if parent_status != "completed" {
                    break;
                }
                tx.execute(
                    "UPDATE tasks SET status = 'todo', completed_at = NULL, updated_at = ?1, revision = revision + 1
                     WHERE id = ?2 AND deleted_at IS NULL",
                    params![now, parent_id.to_string()],
                )
                .map_err(internal)?;
                affected.push(parent_id);
                current = parent_id;
            }
        }

        Ok(affected)
    }
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskCounts {
    pub inbox: i64,
    pub overdue: i64,
}

pub(crate) fn internal<E: std::fmt::Display>(err: E) -> DomainError {
    DomainError::Internal(err.to_string())
}

fn escape_like(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_")
}

pub(crate) fn parse_id(value: String) -> Result<EntityId, rusqlite::Error> {
    value.parse().map_err(|e| {
        rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(e))
    })
}

pub(crate) fn map_list_row(row: &rusqlite::Row<'_>) -> Result<TaskList, rusqlite::Error> {
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
        // Nullable column: must be read as Option (spec pitfall).
        group_id: row
            .get::<_, Option<String>>(7)?
            .map(parse_id)
            .transpose()?,
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
        parent_id: row
            .get::<_, Option<String>>(20)?
            .map(parse_id)
            .transpose()?,
        child_order: row.get(21)?,
        created_at: row.get(17)?,
        updated_at: row.get(18)?,
        revision: row.get(19)?,
    })
}

const TASK_ROW_SELECT: &str = "t.id, t.title, t.notes, t.status, t.priority, t.list_id,
                    l.name, l.kind, t.due_date, t.due_time, t.completed_at, t.sort_order, t.series_id,
                    t.workflow_state, t.available_at, t.waiting_for, t.follow_up_date,
                    t.created_at, t.updated_at, t.revision, t.parent_id, t.child_order";

fn map_domain_sql(err: DomainError) -> rusqlite::Error {
    rusqlite::Error::ToSqlConversionFailure(Box::new(std::io::Error::other(err.to_string())))
}

pub(crate) fn collect_rows<T, E>(rows: impl IntoIterator<Item = Result<T, E>>) -> Result<Vec<T>, DomainError>
where
    E: std::fmt::Display,
{
    let mut out = Vec::new();
    for row in rows {
        out.push(row.map_err(internal)?);
    }
    Ok(out)
}

//
// v2.0 slice 6: checklist (impl methods were appended below; move the brace)
//
impl TaskService {
    // -----------------------------------------------------------------
    // v2.0 slice 6: checklist
    // -----------------------------------------------------------------

    fn checklist_freeze_check(&self, task_id: EntityId) -> Result<Task, DomainError> {
        let task = self.get_task(task_id)?;
        if task.status == TaskStatus::Completed {
            return Err(DomainError::Validation("任务已完成，检查项不可修改".into()));
        }
        Ok(task)
    }

    /// Rewrite the task's search-index row with checklist text appended, so
    /// sub-items stay findable via task search.
    fn reindex_task_with_checklist(
        &self,
        conn: &Connection,
        task_id: EntityId,
    ) -> Result<(), DomainError> {
        let task = self.get_task(task_id)?;
        let items = active_checklist_rows(conn, task_id)?;
        let checklist_text = items
            .iter()
            .map(|i| i.content.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        let body = if checklist_text.is_empty() {
            task.notes.clone()
        } else {
            format!("{}\n{}", task.notes, checklist_text)
        };
        crate::application::search::SearchService::reindex_one(
            conn,
            crate::domain::SearchEntityType::Task,
            task_id,
            &task.title,
            &body,
        )
    }

    pub fn checklist_list(&self, task_id: EntityId) -> Result<TaskChecklist, DomainError> {
        let conn = self.connect()?;
        let items = active_checklist_rows(&conn, task_id)?;
        let checked_count = items.iter().filter(|i| i.checked).count() as i64;
        Ok(TaskChecklist {
            total: items.len() as i64,
            checked_count,
            items,
        })
    }

    pub fn checklist_add(
        &self,
        task_id: EntityId,
        content: &str,
    ) -> Result<ChecklistItem, DomainError> {
        self.checklist_freeze_check(task_id)?;
        let content = validate_checklist_content(content)?;
        let conn = self.connect()?;
        let now = stamp(&self.clock);
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM task_checklist_items
                 WHERE task_id = ?1 AND deleted_at IS NULL",
                [task_id.to_string()],
                |row| row.get(0),
            )
            .map_err(internal)?;
        if count as usize >= CHECKLIST_MAX_ITEMS {
            return Err(DomainError::Validation(
                "检查项最多 50 条，考虑把任务拆分得更聚焦".into(),
            ));
        }
        let next_order: i64 = conn
            .query_row(
                "SELECT COALESCE(MAX(sort_order), -1) + 1 FROM task_checklist_items
                 WHERE task_id = ?1 AND deleted_at IS NULL",
                [task_id.to_string()],
                |row| row.get(0),
            )
            .map_err(internal)?;
        let id = new_id();
        conn.execute(
            "INSERT INTO task_checklist_items
                (id, task_id, content, checked, sort_order, created_at, updated_at, revision)
             VALUES (?1, ?2, ?3, 0, ?4, ?5, ?5, 1)",
            params![id.to_string(), task_id.to_string(), content, next_order, now],
        )
        .map_err(internal)?;
        conn.execute(
            "UPDATE tasks SET updated_at = ?1 WHERE id = ?2 AND deleted_at IS NULL",
            params![now, task_id.to_string()],
        )
        .map_err(internal)?;
        self.reindex_task_with_checklist(&conn, task_id)?;
        Ok(ChecklistItem {
            id,
            task_id,
            content,
            checked: false,
            sort_order: next_order,
            created_at: now.clone(),
            updated_at: now,
            revision: 1,
        })
    }

    pub fn checklist_update(
        &self,
        input: ChecklistUpdateInput,
    ) -> Result<ChecklistItem, DomainError> {
        let conn = self.connect()?;
        let existing: Option<(String, String)> = conn
            .query_row(
                "SELECT task_id, content FROM task_checklist_items
                 WHERE id = ?1 AND deleted_at IS NULL",
                [input.id.to_string()],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()
            .map_err(internal)?;
        let (task_id_str, _) =
            existing.ok_or_else(|| DomainError::Validation("检查项不存在".into()))?;
        let task_id: EntityId = task_id_str
            .parse()
            .map_err(|_| DomainError::Validation("任务 id 非法".into()))?;
        self.checklist_freeze_check(task_id)?;

        let content = match input.content.as_deref() {
            None | Some("") => None,
            Some(raw) => Some(validate_checklist_content(raw)?),
        };
        let now = stamp(&self.clock);
        conn.execute(
            "UPDATE task_checklist_items SET
                content = COALESCE(?2, content),
                checked = COALESCE(?3, checked),
                updated_at = ?4,
                revision = revision + 1
             WHERE id = ?1 AND deleted_at IS NULL",
            params![
                input.id.to_string(),
                content,
                input.checked.map(|c| if c { 1 } else { 0 }),
                now,
            ],
        )
        .map_err(internal)?;
        conn.execute(
            "UPDATE tasks SET updated_at = ?1 WHERE id = ?2 AND deleted_at IS NULL",
            params![now, task_id_str],
        )
        .map_err(internal)?;
        self.reindex_task_with_checklist(&conn, task_id)?;

        active_checklist_rows(&conn, task_id)?
            .into_iter()
            .find(|i| i.id == input.id)
            .ok_or_else(|| DomainError::Internal("checklist row vanished".into()))
    }

    pub fn checklist_delete(&self, id: EntityId) -> Result<(), DomainError> {
        let conn = self.connect()?;
        let task_id_str: Option<String> = conn
            .query_row(
                "SELECT task_id FROM task_checklist_items WHERE id = ?1 AND deleted_at IS NULL",
                [id.to_string()],
                |row| row.get(0),
            )
            .optional()
            .map_err(internal)?;
        let task_id_str =
            task_id_str.ok_or_else(|| DomainError::Validation("检查项不存在".into()))?;
        let task_id: EntityId = task_id_str
            .parse()
            .map_err(|_| DomainError::Validation("任务 id 非法".into()))?;
        self.checklist_freeze_check(task_id)?;

        let now = stamp(&self.clock);
        conn.execute(
            "UPDATE task_checklist_items SET deleted_at = ?1, updated_at = ?1
             WHERE id = ?2 AND deleted_at IS NULL",
            params![now, id.to_string()],
        )
        .map_err(internal)?;
        self.normalize_checklist_order(&conn, task_id)?;
        conn.execute(
            "UPDATE tasks SET updated_at = ?1 WHERE id = ?2 AND deleted_at IS NULL",
            params![now, task_id_str],
        )
        .map_err(internal)?;
        self.reindex_task_with_checklist(&conn, task_id)?;
        Ok(())
    }

    pub fn checklist_reorder(
        &self,
        task_id: EntityId,
        ordered_ids: Vec<EntityId>,
    ) -> Result<(), DomainError> {
        self.checklist_freeze_check(task_id)?;
        let conn = self.connect()?;
        let active = active_checklist_rows(&conn, task_id)?;
        if ordered_ids.len() != active.len() {
            return Err(DomainError::Validation(
                "排序列表必须包含全部检查项".into(),
            ));
        }
        let active_ids: std::collections::HashSet<EntityId> =
            active.iter().map(|i| i.id).collect();
        if !ordered_ids.iter().all(|id| active_ids.contains(id)) {
            return Err(DomainError::Validation("排序列表包含未知检查项".into()));
        }
        let now = stamp(&self.clock);
        for (index, id) in ordered_ids.iter().enumerate() {
            conn.execute(
                "UPDATE task_checklist_items SET sort_order = ?1, updated_at = ?2
                 WHERE id = ?3 AND deleted_at IS NULL",
                params![index as i64, now, id.to_string()],
            )
            .map_err(internal)?;
        }
        conn.execute(
            "UPDATE tasks SET updated_at = ?1 WHERE id = ?2 AND deleted_at IS NULL",
            params![now, task_id.to_string()],
        )
        .map_err(internal)?;
        Ok(())
    }

    fn normalize_checklist_order(
        &self,
        conn: &Connection,
        task_id: EntityId,
    ) -> Result<(), DomainError> {
        let now = stamp(&self.clock);
        let rows = active_checklist_rows(conn, task_id)?;
        for (index, item) in rows.iter().enumerate() {
            if item.sort_order != index as i64 {
                conn.execute(
                    "UPDATE task_checklist_items SET sort_order = ?1, updated_at = ?2
                     WHERE id = ?3 AND deleted_at IS NULL",
                    params![index as i64, now, item.id.to_string()],
                )
                .map_err(internal)?;
            }
        }
        Ok(())
    }
}

fn active_checklist_rows(
    conn: &Connection,
    task_id: EntityId,
) -> Result<Vec<ChecklistItem>, DomainError> {
    let mut stmt = conn
        .prepare(
            "SELECT id, task_id, content, checked, sort_order, created_at, updated_at, revision
             FROM task_checklist_items
             WHERE task_id = ?1 AND deleted_at IS NULL
             ORDER BY sort_order ASC",
        )
        .map_err(internal)?;
    let rows = stmt
        .query_map([task_id.to_string()], |row| {
            Ok(ChecklistItem {
                id: row
                    .get::<_, String>(0)?
                    .parse()
                    .map_err(|e| {
                        rusqlite::Error::FromSqlConversionFailure(
                            0,
                            rusqlite::types::Type::Text,
                            Box::new(e),
                        )
                    })?,
                task_id: row
                    .get::<_, String>(1)?
                    .parse()
                    .map_err(|e| {
                        rusqlite::Error::FromSqlConversionFailure(
                            1,
                            rusqlite::types::Type::Text,
                            Box::new(e),
                        )
                    })?,
                content: row.get(2)?,
                checked: row.get::<_, i64>(3)? == 1,
                sort_order: row.get(4)?,
                created_at: row.get(5)?,
                updated_at: row.get(6)?,
                revision: row.get(7)?,
            })
        })
        .map_err(internal)?;
    let mut items = Vec::new();
    for row in rows {
        items.push(row.map_err(internal)?);
    }
    Ok(items)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::MAX_SUBTASK_DEPTH;
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

    /// Shared temp db for tests that need two services over one database.
    fn test_db() -> Database {
        let dir = tempdir().unwrap();
        let db = Database::open(dir.path().join("shared.db")).unwrap();
        std::mem::forget(dir);
        db
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
                parent_id: None,
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
                parent_id: None,
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
                parent_id: None,
            })
            .unwrap();

        let today_view = svc.today_tasks(None).unwrap();
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
        let today_view = svc.today_tasks(None).unwrap();
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
                parent_id: None,
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
                parent_id: None,
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
                parent_id: None,
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
                parent_id: None,
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
                parent_id: None,
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
                parent_id: None,
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
                parent_id: None,
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
                parent_id: None,
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
                parent_id: None,
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
                parent_id: None,
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
                parent_id: None,
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
                parent_id: None,
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
                parent_id: None,
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

        let today_view = svc.today_tasks(None).unwrap();
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
                parent_id: None,
            })
            .unwrap();

        svc.set_task_waiting(task.id, Some("Bob".into()), None)
            .unwrap();

        let today_view = svc.today_tasks(None).unwrap();
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
                parent_id: None,
            })
            .unwrap();

        svc.daily_focus_add(task.id, Some(today.clone())).unwrap();
        let view = svc.today_tasks(None).unwrap();
        assert!(view.focus.iter().any(|t| t.id == task.id));

        svc.daily_focus_remove(task.id, Some(today.clone())).unwrap();
        let view2 = svc.today_tasks(None).unwrap();
        assert!(!view2.focus.iter().any(|t| t.id == task.id));

        svc.daily_focus_add(task.id, Some(yesterday.clone())).unwrap();
        let carried = svc
            .daily_focus_carry(yesterday.clone(), today.clone())
            .unwrap();
        assert_eq!(carried.len(), 1);
        assert_eq!(carried[0].id, task.id);
        let view3 = svc.today_tasks(None).unwrap();
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
                parent_id: None,
            })
            .unwrap();

        svc.daily_focus_add(task.id, None).unwrap();
        svc.set_task_defer(task.id, Some(tomorrow)).unwrap();
        let view = svc.today_tasks(None).unwrap();
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
                parent_id: None,
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
                parent_id: None,
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
                parent_id: None,
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

    // -----------------------------------------------------------------
    // v2.0 slice 6: checklist
    // -----------------------------------------------------------------

    #[test]
    fn checklist_crud_and_counts() {
        let service = open_service();
        let task = service
            .create_task(CreateTaskInput {
                title: "上线发布".into(),
                notes: None,
                priority: None,
                list_id: None,
                due_date: None,
                due_time: None,
                tag_names: None,
                parent_id: None,
            })
            .unwrap();

        let a = service.checklist_add(task.id, " 更新版本号 ").unwrap();
        assert_eq!(a.content, "更新版本号", "content is trimmed");
        let b = service.checklist_add(task.id, "写 release note").unwrap();
        let c = service.checklist_add(task.id, "检查签名").unwrap();

        let list = service.checklist_list(task.id).unwrap();
        assert_eq!(list.total, 3);
        assert_eq!(list.checked_count, 0);
        assert_eq!(
            list.items.iter().map(|i| i.sort_order).collect::<Vec<_>>(),
            vec![0, 1, 2]
        );

        let updated = service
            .checklist_update(ChecklistUpdateInput {
                id: b.id,
                content: Some("写 release note 并通知群".into()),
                checked: Some(true),
            })
            .unwrap();
        assert!(updated.checked);
        assert_eq!(updated.content, "写 release note 并通知群");
        assert_eq!(service.checklist_list(task.id).unwrap().checked_count, 1);

        // Delete → order normalized.
        service.checklist_delete(a.id).unwrap();
        let list = service.checklist_list(task.id).unwrap();
        assert_eq!(list.total, 2);
        assert_eq!(
            list.items.iter().map(|i| i.sort_order).collect::<Vec<_>>(),
            vec![0, 1]
        );

        // Reorder full list.
        service
            .checklist_reorder(task.id, vec![c.id, b.id])
            .unwrap();
        let list = service.checklist_list(task.id).unwrap();
        assert_eq!(list.items[0].id, c.id);

        // Checking everything does NOT complete the task.
        service
            .checklist_update(ChecklistUpdateInput { id: c.id, content: None, checked: Some(true) })
            .unwrap();
        let reloaded = service.get_task(task.id).unwrap();
        assert_eq!(reloaded.status, TaskStatus::Todo);
    }

    #[test]
    fn checklist_freezes_on_completed_task_and_cascades_on_delete() {
        let service = open_service();
        let task = service
            .create_task(CreateTaskInput {
                title: "冻结测试".into(),
                notes: None,
                priority: None,
                list_id: None,
                due_date: None,
                due_time: None,
                tag_names: None,
                parent_id: None,
            })
            .unwrap();
        let item = service.checklist_add(task.id, "子项").unwrap();

        service.complete_task(task.id).unwrap();
        assert!(service.checklist_add(task.id, "新项").is_err());
        assert!(service
            .checklist_update(ChecklistUpdateInput { id: item.id, content: None, checked: Some(true) })
            .is_err());
        // Reads stay available (read-only display).
        assert_eq!(service.checklist_list(task.id).unwrap().total, 1);

        // Cascade: delete the task → no orphan checklist rows anywhere.
        let conn = service.connect().unwrap();
        conn.execute("UPDATE tasks SET deleted_at = NULL, status = 'todo' WHERE id = ?1", [task.id.to_string()])
            .unwrap();
        drop(conn);
        service.delete_task(task.id).unwrap();
        let conn = service.connect().unwrap();
        let orphaned: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM task_checklist_items WHERE deleted_at IS NULL",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(orphaned, 0, "no active orphans after task deletion");
    }

    #[test]
    fn checklist_caps_at_50_and_search_indexes_items() {
        let db = test_db();
        let service = TaskService::new(db.clone());
        service.ensure_seed_data().unwrap();
        let task = service
            .create_task(CreateTaskInput {
                title: "大批量".into(),
                notes: None,
                priority: None,
                list_id: None,
                due_date: None,
                due_time: None,
                tag_names: None,
                parent_id: None,
            })
            .unwrap();
        for i in 0..50 {
            service.checklist_add(task.id, &format!("第{i}步")).unwrap();
        }
        assert!(service.checklist_add(task.id, "第51步").is_err());

        // Sub-item text is searchable via the task index.
        let search = crate::application::search::SearchService::new(db);
        let hits = search
            .query(crate::domain::SearchQuery {
                query: "第42步".into(),
                types: Some(vec![crate::domain::SearchEntityType::Task]),
                limit: Some(10),
            })
            .unwrap();
        assert!(hits.tasks.iter().any(|h| h.entity_id == task.id));
    }

    // -------------------------------------------------------------------
    // v2.1 subtasks
    // -------------------------------------------------------------------

    fn make_input(title: &str) -> CreateTaskInput {
        CreateTaskInput {
            title: title.into(),
            notes: None,
            priority: None,
            list_id: None,
            due_date: None,
            due_time: None,
            tag_names: None,
            parent_id: None,
        }
    }

    #[test]
    fn subtask_create_inherits_list_and_orders_siblings() {
        let svc = open_service();
        let parent = svc.create_task(make_input("父任务")).unwrap();
        let child1 = svc
            .create_task(CreateTaskInput {
                title: "子1".into(),
                parent_id: Some(parent.id),
                ..make_input("")
            })
            .unwrap();
        let child2 = svc
            .create_task(CreateTaskInput {
                title: "子2".into(),
                parent_id: Some(parent.id),
                ..make_input("")
            })
            .unwrap();

        assert_eq!(child1.parent_id, Some(parent.id));
        assert_eq!(child2.parent_id, Some(parent.id));
        // Inherits parent's list when list_id is unset.
        assert_eq!(child1.list_id, parent.list_id);
        assert_eq!(child2.list_id, parent.list_id);
        // Sibling order increments.
        assert!(child1.child_order < child2.child_order);

        // query with parent_id filter returns direct children ordered.
        let list = svc
            .query_tasks(TaskQuery {
                parent_id: Some(parent.id),
                ..Default::default()
            })
            .unwrap();
        assert_eq!(list.total, 2);
        assert_eq!(list.items[0].id, child1.id);
        assert_eq!(list.items[1].id, child2.id);
    }

    #[test]
    fn subtask_depth_limit_enforced() {
        let svc = open_service();
        let mut prev = svc.create_task(make_input("L1")).unwrap();
        for _ in 2..=MAX_SUBTASK_DEPTH {
            prev = svc
                .create_task(CreateTaskInput {
                    title: "child".into(),
                    parent_id: Some(prev.id),
                    ..make_input("")
                })
                .unwrap();
        }
        // prev is at depth MAX; one more must fail on both create and reparent.
        assert!(svc
            .create_task(CreateTaskInput {
                title: "too deep".into(),
                parent_id: Some(prev.id),
                ..make_input("")
            })
            .is_err());
        let root = svc.create_task(make_input("root")).unwrap();
        assert!(svc.set_task_parent(root.id, Some(prev.id)).is_err());
    }

    #[test]
    fn subtask_cycle_prevention() {
        let svc = open_service();
        let a = svc.create_task(make_input("A")).unwrap();
        let b = svc
            .create_task(CreateTaskInput {
                title: "B".into(),
                parent_id: Some(a.id),
                ..make_input("")
            })
            .unwrap();
        let c = svc
            .create_task(CreateTaskInput {
                title: "C".into(),
                parent_id: Some(b.id),
                ..make_input("")
            })
            .unwrap();

        // Self-parent and descendant-parent are rejected.
        assert!(svc.set_task_parent(a.id, Some(a.id)).is_err());
        assert!(svc.set_task_parent(a.id, Some(c.id)).is_err());
        assert!(svc.set_task_parent(b.id, Some(c.id)).is_err());
        // Valid move is accepted.
        let moved = svc.set_task_parent(b.id, Some(a.id)).unwrap();
        assert_eq!(moved.parent_id, Some(a.id));
    }

    #[test]
    fn subtask_aggregation_defaults_on() {
        let svc = open_service();
        let parent = svc.create_task(make_input("P")).unwrap();
        let c1 = svc
            .create_task(CreateTaskInput {
                title: "C1".into(),
                parent_id: Some(parent.id),
                ..make_input("")
            })
            .unwrap();
        let c2 = svc
            .create_task(CreateTaskInput {
                title: "C2".into(),
                parent_id: Some(parent.id),
                ..make_input("")
            })
            .unwrap();

        // Completing one child does not complete the parent.
        svc.complete_task(c1.id).unwrap();
        assert_eq!(svc.get_task(parent.id).unwrap().status, TaskStatus::Todo);

        // Completing the last child auto-completes the parent.
        svc.complete_task(c2.id).unwrap();
        assert_eq!(svc.get_task(parent.id).unwrap().status, TaskStatus::Completed);

        // Uncompleting one child restores the parent (bidirectional up).
        svc.uncomplete_task(c1.id).unwrap();
        assert_eq!(svc.get_task(parent.id).unwrap().status, TaskStatus::Todo);
    }

    #[test]
    fn subtask_cascade_children_on_complete_and_uncomplete() {
        let svc = open_service();
        let parent = svc.create_task(make_input("P")).unwrap();
        let c1 = svc
            .create_task(CreateTaskInput {
                title: "C1".into(),
                parent_id: Some(parent.id),
                ..make_input("")
            })
            .unwrap();
        let c2 = svc
            .create_task(CreateTaskInput {
                title: "C2".into(),
                parent_id: Some(c1.id),
                ..make_input("")
            })
            .unwrap();

        // Completing the parent cascades to all descendants.
        svc.complete_task(parent.id).unwrap();
        assert_eq!(svc.get_task(c1.id).unwrap().status, TaskStatus::Completed);
        assert_eq!(svc.get_task(c2.id).unwrap().status, TaskStatus::Completed);

        // Uncompleting the parent cascades back (bidirectional down).
        svc.uncomplete_task(parent.id).unwrap();
        assert_eq!(svc.get_task(c1.id).unwrap().status, TaskStatus::Todo);
        assert_eq!(svc.get_task(c2.id).unwrap().status, TaskStatus::Todo);
    }

    #[test]
    fn subtask_aggregation_off_when_toggles_disabled() {
        let db = test_db();
        let svc = TaskService::new(db.clone());
        svc.ensure_seed_data().unwrap();
        // Disable both toggles.
        let conn = db.connect().unwrap();
        conn.execute(
            "INSERT INTO settings (key, value_json, updated_at) VALUES ('app.settings', ?1, '2026-01-01T00:00:00Z')
             ON CONFLICT(key) DO UPDATE SET value_json = excluded.value_json",
            rusqlite::params![r#"{"subtaskAutoCompleteParent":false,"subtaskCascadeChildren":false}"#.to_string()],
        )
        .unwrap();
        drop(conn);

        let parent = svc.create_task(make_input("P")).unwrap();
        let c1 = svc
            .create_task(CreateTaskInput {
                title: "C1".into(),
                parent_id: Some(parent.id),
                ..make_input("")
            })
            .unwrap();
        let c2 = svc
            .create_task(CreateTaskInput {
                title: "C2".into(),
                parent_id: Some(parent.id),
                ..make_input("")
            })
            .unwrap();

        svc.complete_task(c1.id).unwrap();
        svc.complete_task(c2.id).unwrap();
        // No auto-complete of the parent.
        assert_eq!(svc.get_task(parent.id).unwrap().status, TaskStatus::Todo);

        svc.complete_task(parent.id).unwrap();
        // No cascade to children.
        assert_eq!(svc.get_task(c1.id).unwrap().status, TaskStatus::Completed);
    }

    #[test]
    fn subtask_archive_cascades() {
        let svc = open_service();
        let parent = svc.create_task(make_input("P")).unwrap();
        let c1 = svc
            .create_task(CreateTaskInput {
                title: "C1".into(),
                parent_id: Some(parent.id),
                ..make_input("")
            })
            .unwrap();
        let c2 = svc
            .create_task(CreateTaskInput {
                title: "C2".into(),
                parent_id: Some(c1.id),
                ..make_input("")
            })
            .unwrap();

        svc.archive_task(parent.id).unwrap();
        assert_eq!(svc.get_task(c1.id).unwrap().status, TaskStatus::Archived);
        assert_eq!(svc.get_task(c2.id).unwrap().status, TaskStatus::Archived);

        svc.unarchive_task(parent.id).unwrap();
        assert_eq!(svc.get_task(c1.id).unwrap().status, TaskStatus::Todo);
        assert_eq!(svc.get_task(c2.id).unwrap().status, TaskStatus::Todo);
    }

    #[test]
    fn subtask_delete_tree_cascade_and_promote() {
        let svc = open_service();
        // Cascade.
        let parent = svc.create_task(make_input("P")).unwrap();
        let c1 = svc
            .create_task(CreateTaskInput {
                title: "C1".into(),
                parent_id: Some(parent.id),
                ..make_input("")
            })
            .unwrap();
        let c2 = svc
            .create_task(CreateTaskInput {
                title: "C2".into(),
                parent_id: Some(c1.id),
                ..make_input("")
            })
            .unwrap();
        let affected = svc
            .delete_task_tree(parent.id, TaskDeleteDisposition::Cascade)
            .unwrap();
        assert!(affected.contains(&parent.id));
        assert!(affected.contains(&c1.id));
        assert!(affected.contains(&c2.id));
        assert!(svc.get_task(parent.id).is_err());
        assert!(svc.get_task(c1.id).is_err());
        assert!(svc.get_task(c2.id).is_err());

        // Promote: children rise to top level, grandchildren stay attached.
        let parent2 = svc.create_task(make_input("P2")).unwrap();
        let g1 = svc
            .create_task(CreateTaskInput {
                title: "G1".into(),
                parent_id: Some(parent2.id),
                ..make_input("")
            })
            .unwrap();
        let g2 = svc
            .create_task(CreateTaskInput {
                title: "G2".into(),
                parent_id: Some(g1.id),
                ..make_input("")
            })
            .unwrap();
        let affected = svc
            .delete_task_tree(parent2.id, TaskDeleteDisposition::Promote)
            .unwrap();
        assert_eq!(affected.len(), 2); // parent + direct child
        let promoted = svc.get_task(g1.id).unwrap();
        assert_eq!(promoted.parent_id, None);
        assert_eq!(promoted.list_id, parent2.list_id, "list_id unchanged");
        assert_eq!(svc.get_task(g2.id).unwrap().parent_id, Some(g1.id));
    }

    #[test]
    fn subtask_delete_refuses_when_children_exist() {
        let svc = open_service();
        let parent = svc.create_task(make_input("P")).unwrap();
        svc.create_task(CreateTaskInput {
            title: "C".into(),
            parent_id: Some(parent.id),
            ..make_input("")
        })
        .unwrap();
        assert!(svc.delete_task(parent.id).is_err());
        // Leaf delete still works.
        let leaf = svc.create_task(make_input("leaf")).unwrap();
        svc.delete_task(leaf.id).unwrap();
    }

    #[test]
    fn subtask_query_tree_closure() {
        let svc = open_service();
        let root_a = svc.create_task(make_input("A")).unwrap();
        let child_a1 = svc
            .create_task(CreateTaskInput {
                title: "A1".into(),
                parent_id: Some(root_a.id),
                ..make_input("")
            })
            .unwrap();
        svc.create_task(CreateTaskInput {
            title: "A1a".into(),
            parent_id: Some(child_a1.id),
            ..make_input("")
        })
        .unwrap();
        let root_b = svc.create_task(make_input("B")).unwrap();

        // Query matching only A (by search) still returns the full subtree +
        // root B is excluded when it does not match.
        let tree = svc
            .query_tree(TaskQuery {
                search: Some("A".into()),
                ..Default::default()
            })
            .unwrap();
        let ids: Vec<EntityId> = tree.iter().map(|t| t.id).collect();
        assert!(ids.contains(&root_a.id));
        assert!(ids.contains(&child_a1.id));
        assert!(!ids.contains(&root_b.id));

        // Search matching a leaf pulls its ancestors too.
        let tree2 = svc
            .query_tree(TaskQuery {
                search: Some("A1a".into()),
                ..Default::default()
            })
            .unwrap();
        let ids2: Vec<EntityId> = tree2.iter().map(|t| t.id).collect();
        assert!(ids2.contains(&root_a.id));
        assert!(ids2.contains(&child_a1.id));
    }

    #[test]
    fn subtask_reorder_subtasks() {
        let svc = open_service();
        let parent = svc.create_task(make_input("P")).unwrap();
        let c1 = svc
            .create_task(CreateTaskInput {
                title: "C1".into(),
                parent_id: Some(parent.id),
                ..make_input("")
            })
            .unwrap();
        let c2 = svc
            .create_task(CreateTaskInput {
                title: "C2".into(),
                parent_id: Some(parent.id),
                ..make_input("")
            })
            .unwrap();

        svc.reorder_subtasks(Some(parent.id), vec![c2.id, c1.id])
            .unwrap();
        let list = svc
            .query_tasks(TaskQuery {
                parent_id: Some(parent.id),
                ..Default::default()
            })
            .unwrap();
        assert_eq!(list.items[0].id, c2.id);
        assert_eq!(list.items[1].id, c1.id);

        // Mixed-parent ids are rejected.
        let other = svc.create_task(make_input("other")).unwrap();
        assert!(svc
            .reorder_subtasks(Some(parent.id), vec![c1.id, other.id])
            .is_err());
    }

    #[test]
    fn subtask_tree_expanded_state() {
        let svc = open_service();
        let parent = svc.create_task(make_input("P")).unwrap();
        svc.set_tree_expanded(parent.id, false).unwrap();
        let list = svc.tree_expanded_list().unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].task_id, parent.id);
        assert!(!list[0].expanded);

        // Restore default (expanded) removes the row.
        svc.set_tree_expanded(parent.id, true).unwrap();
        assert!(svc.tree_expanded_list().unwrap().is_empty());

        // Delete removes expand state rows.
        svc.set_tree_expanded(parent.id, false).unwrap();
        svc.delete_task(parent.id).unwrap();
        assert!(svc.tree_expanded_list().unwrap().is_empty());
    }
}
