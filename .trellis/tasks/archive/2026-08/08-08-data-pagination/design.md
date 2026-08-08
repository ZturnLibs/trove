# Design: 数据层分页

## Architecture

```text
Frontend (TasksPage / MemoryPage / ClipboardPage)
    │  taskQuery({ ..., limit, offset })
    ▼
IPC client.ts  →  PagedResult<T> { items, total, hasMore }
    ▼
commands/mod.rs  →  TaskService / MemoryService / ClipboardService
    ▼
SQLite  SELECT ... LIMIT ? OFFSET ?
          SELECT COUNT(*) ... (same WHERE)
```

## Contracts

### Shared types (Rust + TypeScript)

```typescript
// client.ts
export type PageParams = { limit?: number; offset?: number };

export type PagedResult<T> = {
  items: T[];
  total: number;
  hasMore: boolean;
};
```

Rust 侧在 `domain/` 新增 `PagedResult<T>`（`Serialize`/`Deserialize`，`camelCase`），各 `*Query` struct 增加 `limit: Option<i64>`、`offset: Option<i64>`。

### Defaults & clamps

| 字段 | 默认 |  clamp |
| --- | --- | --- |
| limit | 200 | 1..=1000 |
| offset | 0 | >= 0 |

与 `clipboard.rs:362` 现有 clamp 对齐。

### Command return types (breaking)

| Command | Before | After |
| --- | --- | --- |
| `task_query` | `Vec<Task>` | `PagedResult<Task>` |
| `task_smart_list` | `Vec<Task>` | `PagedResult<Task>` |
| `memory_query` | `Vec<Memory>` | `PagedResult<Memory>` |
| `clipboard_query` | `Vec<ClipboardItem>` | `PagedResult<ClipboardItem>` |

`task_smart_list` 增加可选 `limit` / `offset` 参数（Tauri 命令签名扩展）。

## Backend implementation notes

### COUNT query pattern

对动态 WHERE 构建，推荐：

1. 复用同一 `WHERE` 子句与 bind 参数；
2. `SELECT COUNT(*) FROM tasks t JOIN ... WHERE ...`（tasks/memories）；
3. 主查询末尾 `ORDER BY ... LIMIT ? OFFSET ?`。

`attach_tags` 逻辑不变，仅对当前页 items 执行。

### Files

| Layer | Files |
| --- | --- |
| Domain | `src-tauri/src/domain/page.rs`（新）、`task.rs`、`memory.rs`、`clipboard.rs` |
| Application | `tasks.rs:540+`、`memories.rs:150+`、`clipboard.rs:360+`、`tasks.rs:610 smart_list` |
| Commands | `commands/mod.rs` |
| Frontend | `ipc/client.ts`、`*Page.tsx`、可选 `usePagedQuery` hook |

## Frontend data flow

### Recommended hook: `usePagedList`

```text
state: items[], offset, total, hasMore, isLoadingMore
fetchPage(offset):
  - offset===0 → replace items
  - offset>0   → append items
reset on filter change → offset=0, refetch
loadMore → fetchPage(offset + limit) if hasMore
```

React Query：首屏 `useQuery` with `offset: 0`；`loadMore` 可 `queryClient.setQueryData` 合并或 local state 管理 appended items（实现阶段选一种，避免 double-fetch）。

### UI

- 列表底部按钮：「加载更多（已显示 X / 共 total）」；`!hasMore` 时隐藏。
- 不在本期做 infinite scroll intersection observer（可后续增强）；按钮即可满足 AC。

## Compatibility

- 桌面应用无外部 API 消费者；IPC 变更是项目内闭环。
- `AttachmentsSection` 的 `clipboardQuery({ kind: "image", limit: 60 })` 改为读 `.items`。
- 测试夹具中直接 `query_tasks` 的 Rust 单测需改断言 `.items`。

## Rollback

-  revert domain + command 返回类型 + 前端调用即可；无数据库迁移。
