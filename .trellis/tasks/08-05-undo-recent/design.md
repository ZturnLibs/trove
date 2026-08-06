# 技术设计：撤销/最近操作

## 1. 后端：task_unarchive

tasks.rs 新增（参照 uncomplete_task:446-460）：

```rust
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
    ).map_err(internal)?;
    self.get_task(id)
}
```

commands `task_unarchive` + lib.rs 注册（参照 task_archive）。

## 2. 前端：useRecentActions store

`src/stores/recent-actions.ts`（zustand）：

```ts
export type RecentAction = {
  id: number;
  label: string;
  undo: () => Promise<void>;
};
export const useRecentActions = create<{
  actions: RecentAction[];
  push: (a: Omit<RecentAction, "id">) => void;
  pop: (id: number) => void;
  clear: () => void;
}>((set) => ({
  actions: [],
  push: (a) => set((s) => ({ actions: [...s.actions, { ...a, id: Date.now() }].slice(-5) })),
  pop: (id) => set((s) => ({ actions: s.actions.filter((x) => x.id !== id) })),
  clear: () => set({ actions: [] }),
}));
```

## 3. RecentActionToast（MainShell 底部）

- 取 `actions[actions.length - 1]` 展示「已{label} · 撤销」。
- 撤销：`await undo()` → 失效 `["tasks"]` → `pop(id)` → 显示下一条；失败 `pop` 并提示。
- 5 秒自动 `pop`（新动作重置计时）。

## 4. 接线（mutation onSuccess）

统一帮助函数：

```ts
function record(store, label: string, undo: () => Promise<void>) { store.push({ label, undo }); }
```

- TaskDetailPanel complete/uncomplete：undo 反向调 `taskUncomplete`/`taskComplete`。
- TaskDetailPanel archive：undo → `ipc.taskUnarchive(taskId)`。
- TaskDetailPanel delete：undo → `ipc.taskCreate({ title, notes?, listId, dueDate, dueTime, priority, tagNames })`（重建）。
- TodayPage / TasksPage 勾选完成：undo 反向（`taskUncomplete`/`taskComplete`）。

## 5. 边界

- 不覆盖延期/移动/提醒；删除撤销为重建（新 id，关联不迁移）——在 toast label 注明「已删除任务（重建）」。
- undo 中 `taskCreate` 需要任务字段快照，接线时从当时的 task 对象捕获闭包。
