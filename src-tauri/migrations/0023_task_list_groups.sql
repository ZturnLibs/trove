-- 任务分组：在清单（task_lists）之上引入「分组」层级（分组 → 清单两级）。
--
-- task_list_groups 与 task_lists 同构：软删除（deleted_at）+ revision 乐观并发。
-- task_lists.group_id 为可空外键：NULL = 未分组；删除分组走应用层「解散」
-- （组内清单 group_id 置 NULL、任务不动），不做 FK ON DELETE 级联。
-- 分组折叠状态是纯客户端视图偏好（localStorage），不落库。

CREATE TABLE task_list_groups (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL,
  sort_order REAL NOT NULL DEFAULT 0,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  revision INTEGER NOT NULL DEFAULT 1,
  deleted_at TEXT
);

ALTER TABLE task_lists ADD COLUMN group_id TEXT REFERENCES task_list_groups(id);

CREATE INDEX idx_task_lists_group ON task_lists (group_id, deleted_at, sort_order);
