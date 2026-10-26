//! 任务分组服务：分组 CRUD、清单归组/移出、侧边栏聚合、今日页分组过滤。
//!
//! 以 `impl TaskService` 扩展块实现（同 crate 允许跨文件 inherent impl），
//! 不新建服务结构体、不改 AppState 装配。删除分组 = 解散：组内清单回到
//! 未分组、任务一律不动，返回撤销数据（复用清单删除的撤销注册机制）。

use std::collections::HashMap;

use crate::application::tasks::{collect_rows, internal, map_list_row, parse_id, TaskService};
use crate::domain::{
    new_id, stamp, DomainError, EntityId, ListGroupDeleteUndo, ListGroupScope, ListKind,
    TaskList, TaskListGroup, TaskListGroupNode, TaskListGroupOverview, TaskListSummary,
};
use rusqlite::{params, Connection, OptionalExtension};

/// 纯函数：任务所属清单的分组是否命中今日页过滤条件。
/// `group_by_list` 缺失某清单（软删等）时，任何具体 scope 都不命中。
pub fn list_group_scope_matches(
    list_id: EntityId,
    group_by_list: &HashMap<EntityId, Option<EntityId>>,
    scope: Option<ListGroupScope>,
) -> bool {
    let list_group = match group_by_list.get(&list_id) {
        Some(group) => *group,
        // 清单不在活跃集合中（软删等）：仅「全部」视图可见
        None => return scope.is_none(),
    };
    match scope {
        None => true,
        Some(ListGroupScope::Group { group_id }) => list_group == Some(group_id),
        Some(ListGroupScope::Ungrouped) => list_group.is_none(),
    }
}

fn map_group_row(row: &rusqlite::Row<'_>) -> Result<TaskListGroup, rusqlite::Error> {
    Ok(TaskListGroup {
        id: parse_id(row.get(0)?)?,
        name: row.get(1)?,
        sort_order: row.get(2)?,
        created_at: row.get(3)?,
        updated_at: row.get(4)?,
        revision: row.get(5)?,
    })
}

const GROUP_ROW_SELECT: &str = "id, name, sort_order, created_at, updated_at, revision";

impl TaskService {
    pub fn list_groups(&self) -> Result<Vec<TaskListGroup>, DomainError> {
        let conn = self.connect()?;
        let mut stmt = conn
            .prepare(&format!(
                "SELECT {GROUP_ROW_SELECT}
                 FROM task_list_groups
                 WHERE deleted_at IS NULL
                 ORDER BY sort_order, name COLLATE NOCASE"
            ))
            .map_err(internal)?;
        let rows = stmt.query_map([], map_group_row).map_err(internal)?;
        collect_rows(rows)
    }

    pub fn get_list_group(&self, id: EntityId) -> Result<TaskListGroup, DomainError> {
        let conn = self.connect()?;
        conn.query_row(
            &format!(
                "SELECT {GROUP_ROW_SELECT}
                 FROM task_list_groups WHERE id = ?1 AND deleted_at IS NULL"
            ),
            [id.to_string()],
            map_group_row,
        )
        .optional()
        .map_err(internal)?
        .ok_or_else(|| DomainError::NotFound("分组不存在".into()))
    }

    fn group_name_taken(
        conn: &Connection,
        name: &str,
        exclude_id: Option<EntityId>,
    ) -> Result<bool, DomainError> {
        let taken: Option<String> = match exclude_id {
            Some(exclude) => conn
                .query_row(
                    "SELECT id FROM task_list_groups
                     WHERE name = ?1 COLLATE NOCASE AND deleted_at IS NULL AND id != ?2
                     LIMIT 1",
                    params![name, exclude.to_string()],
                    |row| row.get(0),
                )
                .optional()
                .map_err(internal)?,
            None => conn
                .query_row(
                    "SELECT id FROM task_list_groups
                     WHERE name = ?1 COLLATE NOCASE AND deleted_at IS NULL
                     LIMIT 1",
                    [name],
                    |row| row.get(0),
                )
                .optional()
                .map_err(internal)?,
        };
        Ok(taken.is_some())
    }

    pub fn create_list_group(&self, name: String) -> Result<TaskListGroup, DomainError> {
        let name = name.trim().to_string();
        if name.is_empty() {
            return Err(DomainError::Validation("分组名称不能为空".into()));
        }
        let conn = self.connect()?;
        if Self::group_name_taken(&conn, &name, None)? {
            return Err(DomainError::Validation(format!("分组「{name}」已存在")));
        }
        let id = new_id();
        let now = stamp(self.clock_ref());
        let sort_order: f64 = conn
            .query_row(
                "SELECT COALESCE(MAX(sort_order), 0) + 1
                 FROM task_list_groups WHERE deleted_at IS NULL",
                [],
                |row| row.get(0),
            )
            .unwrap_or(1.0);
        conn.execute(
            "INSERT INTO task_list_groups (id, name, sort_order, created_at, updated_at, revision)
             VALUES (?1, ?2, ?3, ?4, ?4, 1)",
            params![id.to_string(), name, sort_order, now],
        )
        .map_err(internal)?;
        self.get_list_group(id)
    }

    pub fn rename_list_group(
        &self,
        id: EntityId,
        name: String,
    ) -> Result<TaskListGroup, DomainError> {
        let group = self.get_list_group(id)?;
        let name = name.trim().to_string();
        if name.is_empty() {
            return Err(DomainError::Validation("分组名称不能为空".into()));
        }
        let conn = self.connect()?;
        if name != group.name && Self::group_name_taken(&conn, &name, Some(id))? {
            return Err(DomainError::Validation(format!("分组「{name}」已存在")));
        }
        let now = stamp(self.clock_ref());
        conn.execute(
            "UPDATE task_list_groups
             SET name = ?1, updated_at = ?2, revision = revision + 1
             WHERE id = ?3 AND deleted_at IS NULL",
            params![name, now, id.to_string()],
        )
        .map_err(internal)?;
        self.get_list_group(id)
    }

    /// 按给定顺序全量重写 sort_order = 0..n；id 集合必须与现存活跃组一致。
    pub fn reorder_list_groups(&self, ordered_ids: Vec<EntityId>) -> Result<(), DomainError> {
        let existing = self.list_groups()?;
        if ordered_ids.len() != existing.len()
            || ordered_ids
                .iter()
                .any(|id| !existing.iter().any(|g| g.id == *id))
        {
            return Err(DomainError::Validation("分组排序数据不完整".into()));
        }
        let conn = self.connect()?;
        let now = stamp(self.clock_ref());
        let tx = conn.unchecked_transaction().map_err(internal)?;
        for (index, id) in ordered_ids.iter().enumerate() {
            tx.execute(
                "UPDATE task_list_groups
                 SET sort_order = ?1, updated_at = ?2, revision = revision + 1
                 WHERE id = ?3 AND deleted_at IS NULL",
                params![index as f64, now, id.to_string()],
            )
            .map_err(internal)?;
        }
        tx.commit().map_err(internal)?;
        Ok(())
    }

    /// 删除分组 = 解散：组内清单 group_id 置 NULL（清单与任务都不删），组软删。
    /// 返回撤销数据供「恢复分组 + 回链清单」。
    pub fn delete_list_group(&self, id: EntityId) -> Result<ListGroupDeleteUndo, DomainError> {
        let group = self.get_list_group(id)?;
        let conn = self.connect()?;
        let now = stamp(self.clock_ref());

        let moved_list_ids = {
            let mut stmt = conn
                .prepare(
                    "SELECT id FROM task_lists
                     WHERE group_id = ?1 AND deleted_at IS NULL
                     ORDER BY sort_order",
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
            collect_rows(rows)?
        };

        let tx = conn.unchecked_transaction().map_err(internal)?;
        tx.execute(
            "UPDATE task_lists
             SET group_id = NULL, updated_at = ?1, revision = revision + 1
             WHERE group_id = ?2 AND deleted_at IS NULL",
            params![now, id.to_string()],
        )
        .map_err(internal)?;
        tx.execute(
            "UPDATE task_list_groups
             SET deleted_at = ?1, updated_at = ?1, revision = revision + 1
             WHERE id = ?2 AND deleted_at IS NULL",
            params![now, id.to_string()],
        )
        .map_err(internal)?;
        tx.commit().map_err(internal)?;

        Ok(ListGroupDeleteUndo {
            group,
            moved_list_ids,
        })
    }

    pub fn undo_delete_list_group(
        &self,
        undo: ListGroupDeleteUndo,
    ) -> Result<TaskListGroup, DomainError> {
        let conn = self.connect()?;
        let now = stamp(self.clock_ref());
        // 与解散删除对称：恢复组 + 回链清单在同一事务内完成。
        let tx = conn.unchecked_transaction().map_err(internal)?;
        tx.execute(
            "UPDATE task_list_groups
             SET deleted_at = NULL, updated_at = ?1, revision = revision + 1
             WHERE id = ?2",
            params![now, undo.group.id.to_string()],
        )
        .map_err(internal)?;
        for list_id in &undo.moved_list_ids {
            tx.execute(
                "UPDATE task_lists
                 SET group_id = ?1, updated_at = ?2, revision = revision + 1
                 WHERE id = ?3 AND deleted_at IS NULL",
                params![undo.group.id.to_string(), now, list_id.to_string()],
            )
            .map_err(internal)?;
        }
        tx.commit().map_err(internal)?;
        self.get_list_group(undo.group.id)
    }

    /// 清单归组 / 移出分组（`group_id = None`）。收件箱拒绝归组（恒为未分组）。
    pub fn set_list_group(
        &self,
        list_id: EntityId,
        group_id: Option<EntityId>,
    ) -> Result<TaskList, DomainError> {
        let list = self.get_list(list_id)?;
        if list.kind == ListKind::Inbox {
            if group_id.is_some() {
                return Err(DomainError::Validation("收件箱不可归组".into()));
            }
            return Ok(list);
        }
        if list.group_id == group_id {
            return Ok(list);
        }
        if let Some(gid) = group_id {
            let _ = self.get_list_group(gid)?;
        }
        let conn = self.connect()?;
        let now = stamp(self.clock_ref());
        conn.execute(
            "UPDATE task_lists
             SET group_id = ?1, updated_at = ?2, revision = revision + 1
             WHERE id = ?3 AND deleted_at IS NULL",
            params![group_id.map(|g| g.to_string()), now, list_id.to_string()],
        )
        .map_err(internal)?;
        self.get_list(list_id)
    }

    /// 侧边栏全量聚合：两条查询（组 + 清单）+ 一条未完成计数聚合，内存拼装。
    pub fn task_list_overview(&self) -> Result<TaskListGroupOverview, DomainError> {
        let conn = self.connect()?;

        let groups: Vec<TaskListGroup> = {
            let mut stmt = conn
                .prepare(&format!(
                    "SELECT {GROUP_ROW_SELECT}
                     FROM task_list_groups
                     WHERE deleted_at IS NULL
                     ORDER BY sort_order, created_at"
                ))
                .map_err(internal)?;
            let rows = stmt.query_map([], map_group_row).map_err(internal)?;
            collect_rows(rows)?
        };

        let lists: Vec<TaskList> = {
            let mut stmt = conn
                .prepare(
                    "SELECT id, name, kind, sort_order, created_at, updated_at, revision, group_id
                     FROM task_lists
                     WHERE deleted_at IS NULL
                     ORDER BY sort_order, name",
                )
                .map_err(internal)?;
            let rows = stmt.query_map([], map_list_row).map_err(internal)?;
            collect_rows(rows)?
        };

        let mut open_by_list: HashMap<EntityId, u64> = HashMap::new();
        {
            let mut stmt = conn
                .prepare(
                    "SELECT list_id, COUNT(*) FROM tasks
                     WHERE status = 'todo' AND deleted_at IS NULL
                     GROUP BY list_id",
                )
                .map_err(internal)?;
            let rows = stmt
                .query_map([], |row| {
                    Ok((
                        parse_id(row.get::<_, String>(0)?)?,
                        row.get::<_, i64>(1)? as u64,
                    ))
                })
                .map_err(internal)?;
            for row in rows {
                let (list_id, count) = row.map_err(internal)?;
                open_by_list.insert(list_id, count);
            }
        }

        let summary = |list: &TaskList| TaskListSummary {
            id: list.id,
            name: list.name.clone(),
            kind: list.kind,
            group_id: list.group_id,
            open_count: open_by_list.get(&list.id).copied().unwrap_or(0),
        };

        let inbox_list = lists
            .iter()
            .find(|l| l.kind == ListKind::Inbox)
            .ok_or_else(|| DomainError::Internal("收件箱清单缺失".into()))?;
        let inbox = summary(inbox_list);

        let mut nodes: Vec<TaskListGroupNode> = groups
            .iter()
            .map(|group| TaskListGroupNode {
                group: group.clone(),
                lists: Vec::new(),
                open_count: 0,
            })
            .collect();
        let mut ungrouped: Vec<TaskListSummary> = Vec::new();
        for list in &lists {
            if list.kind != ListKind::Custom {
                continue;
            }
            match list
                .group_id
                .as_ref()
                .and_then(|gid| nodes.iter_mut().find(|n| n.group.id == *gid))
            {
                Some(node) => {
                    node.open_count += open_by_list.get(&list.id).copied().unwrap_or(0);
                    node.lists.push(summary(list));
                }
                None => ungrouped.push(summary(list)),
            }
        }

        Ok(TaskListGroupOverview {
            groups: nodes,
            ungrouped,
            inbox,
        })
    }

    /// 清单 id → 所属分组 id（None = 未分组）。今日页过滤用。
    pub(crate) fn list_group_map(
        &self,
        conn: &Connection,
    ) -> Result<HashMap<EntityId, Option<EntityId>>, DomainError> {
        let mut stmt = conn
            .prepare("SELECT id, group_id FROM task_lists WHERE deleted_at IS NULL")
            .map_err(internal)?;
        let rows = stmt
            .query_map([], |row| {
                Ok((
                    parse_id(row.get::<_, String>(0)?)?,
                    row.get::<_, Option<String>>(1)?.map(parse_id).transpose()?,
                ))
            })
            .map_err(internal)?;
        let mut map = HashMap::new();
        for row in rows {
            let (id, group) = row.map_err(internal)?;
            map.insert(id, group);
        }
        Ok(map)
    }

    /// 任务 id → 所属清单的分组 id（None = 未分组）。命令层过滤今日提醒用。
    pub fn list_group_by_task(&self) -> Result<HashMap<EntityId, Option<EntityId>>, DomainError> {
        let conn = self.connect()?;
        let mut stmt = conn
            .prepare(
                "SELECT t.id, l.group_id FROM tasks t
                 JOIN task_lists l ON l.id = t.list_id
                 WHERE t.deleted_at IS NULL",
            )
            .map_err(internal)?;
        let rows = stmt
            .query_map([], |row| {
                Ok((
                    parse_id(row.get::<_, String>(0)?)?,
                    row.get::<_, Option<String>>(1)?.map(parse_id).transpose()?,
                ))
            })
            .map_err(internal)?;
        let mut map = HashMap::new();
        for row in rows {
            let (task_id, group) = row.map_err(internal)?;
            map.insert(task_id, group);
        }
        Ok(map)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{CreateTaskInput, Task, TaskQuery, UpdateTaskInput};
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

    fn make_task(svc: &TaskService, title: &str, list_id: Option<EntityId>) -> Task {
        svc.create_task(CreateTaskInput {
            title: title.into(),
            notes: None,
            priority: None,
            list_id,
            due_date: None,
            due_time: None,
            tag_names: None,
            parent_id: None,
        })
        .unwrap()
    }

    fn set_due(svc: &TaskService, id: EntityId, due: &str) {
        let task = svc.get_task(id).unwrap();
        svc.update_task(UpdateTaskInput {
            id,
            title: task.title,
            notes: task.notes,
            priority: task.priority,
            list_id: task.list_id,
            due_date: Some(due.to_string()),
            due_time: task.due_time,
            tag_names: task.tag_names,
        })
        .unwrap();
    }

    // 1. 迁移后旧库打开正常：空组表、overview 可用、清单全部未分组（升级零感知）。
    #[test]
    fn migrated_db_has_empty_groups_and_null_group_ids() {
        let svc = open_service();
        assert!(svc.list_groups().unwrap().is_empty());
        let overview = svc.task_list_overview().unwrap();
        assert_eq!(overview.inbox.kind, ListKind::Inbox);
        assert!(overview.groups.is_empty());
        assert!(overview.ungrouped.is_empty());
        assert_eq!(overview.inbox.name, "收件箱");
    }

    // 2. CRUD：创建（sort_order 追加、重名校验）、重命名（revision+1）、排序（全量重写）。
    #[test]
    fn group_crud_create_rename_reorder() {
        let svc = open_service();
        let g1 = svc.create_list_group("工作".into()).unwrap();
        assert_eq!(g1.sort_order, 1.0);
        let g2 = svc.create_list_group("个人".into()).unwrap();
        assert_eq!(g2.sort_order, 2.0);

        // 重名校验（大小写不敏感，同 tags 约定）
        assert!(svc.create_list_group("工作".into()).is_err());
        assert!(svc.create_list_group(" 工作 ".into()).is_err());

        // 重命名 + revision
        let renamed = svc.rename_list_group(g1.id, "公司".into()).unwrap();
        assert_eq!(renamed.name, "公司");
        assert_eq!(renamed.revision, g1.revision + 1);

        // 拖拽排序：全量重写为 0..n
        svc.reorder_list_groups(vec![g2.id, g1.id]).unwrap();
        let groups = svc.list_groups().unwrap();
        assert_eq!(groups[0].id, g2.id);
        assert_eq!(groups[0].sort_order, 0.0);
        assert_eq!(groups[1].id, g1.id);
        assert_eq!(groups[1].sort_order, 1.0);

        // 排序集合不完整 → 校验失败
        assert!(svc.reorder_list_groups(vec![g2.id]).is_err());
    }

    // 3. 解散删除：组软删、组内清单 group_id=NULL、任务不动、返回 moved_list_ids；undo 恢复。
    #[test]
    fn group_delete_disbands_lists_and_undo_restores() {
        let svc = open_service();
        let group = svc.create_list_group("工作".into()).unwrap();
        let list = svc.create_list("项目A".into()).unwrap();
        svc.set_list_group(list.id, Some(group.id)).unwrap();
        let task = make_task(&svc, "任务1", Some(list.id));

        let undo = svc.delete_list_group(group.id).unwrap();
        assert_eq!(undo.moved_list_ids, vec![list.id]);

        // 组已软删
        assert!(svc.get_list_group(group.id).is_err());
        // 清单还在，且回到未分组
        let list_after = svc.get_list(list.id).unwrap();
        assert_eq!(list_after.group_id, None);
        // 任务不动
        let task_after = svc.get_task(task.id).unwrap();
        assert_eq!(task_after.list_id, list.id);

        // undo：恢复组 + 回链清单
        svc.undo_delete_list_group(undo).unwrap();
        let restored = svc.get_list_group(group.id).unwrap();
        assert_eq!(restored.name, "工作");
        let list_restored = svc.get_list(list.id).unwrap();
        assert_eq!(list_restored.group_id, Some(group.id));
    }

    // 4. 归组校验：inbox 拒绝归组；不存在/已删组拒绝；正常归组/移出。
    #[test]
    fn set_list_group_validations() {
        let svc = open_service();
        let inbox_id = svc.inbox_list_id().unwrap();
        let group = svc.create_list_group("工作".into()).unwrap();
        let list = svc.create_list("项目A".into()).unwrap();

        // inbox 拒绝归组
        assert!(svc.set_list_group(inbox_id, Some(group.id)).is_err());
        // inbox 无需「移出」也合法
        assert!(svc.set_list_group(inbox_id, None).is_ok());

        // 不存在的组拒绝
        assert!(svc.set_list_group(list.id, Some(EntityId::new_v4())).is_err());

        // 已删组拒绝
        svc.delete_list_group(group.id).unwrap();
        assert!(svc.set_list_group(list.id, Some(group.id)).is_err());

        // 正常归组 + 移出
        let group2 = svc.create_list_group("个人".into()).unwrap();
        let moved = svc.set_list_group(list.id, Some(group2.id)).unwrap();
        assert_eq!(moved.group_id, Some(group2.id));
        let moved_out = svc.set_list_group(list.id, None).unwrap();
        assert_eq!(moved_out.group_id, None);
    }

    // 5. task_list_overview：嵌套正确、open_count 只数 todo 未删、未分组/收件箱归类正确。
    #[test]
    fn overview_assembles_groups_lists_and_counts() {
        let svc = open_service();
        let group = svc.create_list_group("工作".into()).unwrap();
        let l1 = svc.create_list("项目A".into()).unwrap();
        let l2 = svc.create_list("项目B".into()).unwrap();
        svc.set_list_group(l1.id, Some(group.id)).unwrap();
        svc.set_list_group(l2.id, Some(group.id)).unwrap();
        // l1: 2 todo（1 个稍后完成）+ 收件箱: 1 todo
        let _t1 = make_task(&svc, "a1", Some(l1.id));
        let t2 = make_task(&svc, "a2", Some(l1.id));
        svc.complete_task(t2.id).unwrap();
        let _i1 = make_task(&svc, "inbox1", None);

        let overview = svc.task_list_overview().unwrap();
        assert_eq!(overview.inbox.open_count, 1);
        assert!(overview.ungrouped.is_empty());
        assert_eq!(overview.groups.len(), 1);
        let node = &overview.groups[0];
        assert_eq!(node.group.id, group.id);
        assert_eq!(node.lists.len(), 2);
        assert_eq!(node.open_count, 1); // 仅 l1 未完成的 a1（a2 已完成不计）
        let la = node.lists.iter().find(|s| s.id == l1.id).unwrap();
        assert_eq!(la.open_count, 1);
        let lb = node.lists.iter().find(|s| s.id == l2.id).unwrap();
        assert_eq!(lb.open_count, 0);

        // 未分组清单平铺
        let _l3 = svc.create_list("散装".into()).unwrap();
        let overview = svc.task_list_overview().unwrap();
        assert_eq!(overview.ungrouped.len(), 1);
        assert_eq!(overview.ungrouped[0].name, "散装");
    }

    // 6. TaskQuery.list_group_id 过滤：query_tasks 与 query_tree 结果一致、跨清单聚合正确。
    #[test]
    fn task_query_filters_by_list_group() {
        let svc = open_service();
        let group = svc.create_list_group("工作".into()).unwrap();
        let other = svc.create_list_group("个人".into()).unwrap();
        let l1 = svc.create_list("项目A".into()).unwrap();
        let l2 = svc.create_list("项目B".into()).unwrap();
        let l3 = svc.create_list("私人".into()).unwrap();
        svc.set_list_group(l1.id, Some(group.id)).unwrap();
        svc.set_list_group(l2.id, Some(group.id)).unwrap();
        svc.set_list_group(l3.id, Some(other.id)).unwrap();

        let in_group_1 = make_task(&svc, "g1-a", Some(l1.id));
        let in_group_2 = make_task(&svc, "g1-b", Some(l2.id));
        let _outside = make_task(&svc, "other", Some(l3.id));
        let _inbox_task = make_task(&svc, "inbox-t", None);

        let page = svc
            .query_tasks(TaskQuery {
                list_group_id: Some(group.id),
                ..Default::default()
            })
            .unwrap();
        let ids: Vec<_> = page.items.iter().map(|t| t.id).collect();
        assert!(ids.contains(&in_group_1.id));
        assert!(ids.contains(&in_group_2.id));
        assert_eq!(page.items.len(), 2);

        let tree = svc
            .query_tree(TaskQuery {
                list_group_id: Some(group.id),
                ..Default::default()
            })
            .unwrap();
        let tree_ids: std::collections::HashSet<_> = tree.iter().map(|t| t.id).collect();
        assert!(tree_ids.contains(&in_group_1.id));
        assert!(tree_ids.contains(&in_group_2.id));

        // 移出后再查，数量减一
        svc.set_list_group(l2.id, None).unwrap();
        let page = svc
            .query_tasks(TaskQuery {
                list_group_id: Some(group.id),
                ..Default::default()
            })
            .unwrap();
        assert_eq!(page.items.len(), 1);
    }

    // 7. today_tasks 带/不带 scope（全部 / 指定分组 / 未分组）。
    #[test]
    fn today_tasks_respects_group_scope() {
        let svc = open_service();
        let group = svc.create_list_group("工作".into()).unwrap();
        let list = svc.create_list("项目A".into()).unwrap();
        svc.set_list_group(list.id, Some(group.id)).unwrap();
        let today = crate::domain::local_today(&crate::domain::SystemClock);

        let grouped = make_task(&svc, "grouped-due", Some(list.id));
        set_due(&svc, grouped.id, &today);
        let ungrouped_task = make_task(&svc, "ungrouped-due", None);
        set_due(&svc, ungrouped_task.id, &today);

        let all = svc.today_tasks(None).unwrap();
        assert!(all.due_today.iter().any(|t| t.id == grouped.id));
        assert!(all.due_today.iter().any(|t| t.id == ungrouped_task.id));

        let scoped = svc
            .today_tasks(Some(ListGroupScope::Group { group_id: group.id }))
            .unwrap();
        assert!(scoped.due_today.iter().any(|t| t.id == grouped.id));
        assert!(!scoped.due_today.iter().any(|t| t.id == ungrouped_task.id));

        let ungrouped_view = svc
            .today_tasks(Some(ListGroupScope::Ungrouped))
            .unwrap();
        assert!(ungrouped_view
            .due_today
            .iter()
            .any(|t| t.id == ungrouped_task.id));
        assert!(!ungrouped_view.due_today.iter().any(|t| t.id == grouped.id));
    }

    // 8. SQL 分组过滤排除软删清单/软删分组（防御性状态：应用路径不会产生，
    //    但任务查询唯一 WHERE 构造器必须与今日页内存过滤语义一致）。
    #[test]
    fn group_filter_excludes_soft_deleted_list_and_group() {
        let svc = open_service();
        let group = svc.create_list_group("工作".into()).unwrap();
        let list = svc.create_list("项目A".into()).unwrap();
        svc.set_list_group(list.id, Some(group.id)).unwrap();
        let t = make_task(&svc, "task-1", Some(list.id));

        // 直接软删清单（模拟外部/历史数据）：任务行仍指向软删清单、group_id 保留。
        {
            let conn = svc.connect().unwrap();
            conn.execute(
                "UPDATE task_lists SET deleted_at = '2026-01-01T00:00:00+00:00' WHERE id = ?1",
                [list.id.to_string()],
            )
            .unwrap();
        }

        let query_group = |gid: EntityId| {
            svc.query_tasks(TaskQuery {
                list_group_id: Some(gid),
                ..Default::default()
            })
            .unwrap()
        };

        // 清单已软删 → 分组过滤不命中
        assert_eq!(query_group(group.id).items.len(), 0);

        // 解散分组（软删清单的 group_id 不会被清理）→ 仍不命中
        svc.delete_list_group(group.id).unwrap();
        assert_eq!(query_group(group.id).items.len(), 0);

        // 对照：任务本身未删，全量视图仍可见
        assert!(svc
            .query_tasks(TaskQuery::default())
            .unwrap()
            .items
            .iter()
            .any(|x| x.id == t.id));
    }

    #[test]
    fn scope_matches_pure_function() {
        let mut map = HashMap::new();
        let list_a: EntityId = uuid::Uuid::new_v4();
        let list_b: EntityId = uuid::Uuid::new_v4();
        let group: EntityId = uuid::Uuid::new_v4();
        map.insert(list_a, Some(group));
        map.insert(list_b, None);

        assert!(list_group_scope_matches(list_a, &map, None));
        assert!(list_group_scope_matches(
            list_a,
            &map,
            Some(ListGroupScope::Group { group_id: group })
        ));
        assert!(!list_group_scope_matches(
            list_b,
            &map,
            Some(ListGroupScope::Group { group_id: group })
        ));
        assert!(list_group_scope_matches(
            list_b,
            &map,
            Some(ListGroupScope::Ungrouped)
        ));
        assert!(!list_group_scope_matches(
            list_a,
            &map,
            Some(ListGroupScope::Ungrouped)
        ));
        // 软删清单不在 map 中：任何具体 scope 都不命中
        let ghost: EntityId = uuid::Uuid::new_v4();
        assert!(!list_group_scope_matches(
            ghost,
            &map,
            Some(ListGroupScope::Ungrouped)
        ));
    }
}
