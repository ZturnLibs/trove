# 记忆搜索/标签/归档（memory-search-tags-archive）

## Goal

记忆页（MemoryPage）补齐「搜索 + 标签筛选 + 归档视图 + 归档/恢复」能力，激活后端已支持但 UI 不可达的 `MemoryQuery.tagId` / `includeArchived`，让记忆在多条时可按内容/标签/归档状态检索，`archived` 字段不再形同虚设。

## 背景（勘察确认）

- `MemoryQuery`（domain/memory.rs:48）：`pinnedOnly / includeArchived / tagId / quickInsertOnly`，**无文本搜索字段**。
- `MemoryService.query`（memories.rs:140-174）：支持 pinned/quick/tag 过滤与 `ORDER BY pinned DESC, updated_at DESC`，**无标题/正文 LIKE**。
- 标签为全局 `tags` 表，任务/记忆共用（`task_tags`/`memory_tags`）；已有 `task_list_tags` 命令（commands/mod.rs:439）返回**全部**标签，可复用于记忆页标签列表。
- `UpdateMemoryInput`（client.ts:286）已含 `archived`，详情面板暂无归档/恢复入口。
- MemoryPage（MemoryPage.tsx）：仅 `pinnedOnly` 开关 + 列表，无搜索框、无标签筛选、无归档视图。

## Requirements

- R1 后端：`MemoryQuery` 增加 `search: Option<String>`，`query()` 对标题/正文做 `LIKE` 过滤（转义 `%`/`_`）。
- R2 前端 IPC：`MemoryQuery` 类型增加 `search`；记忆页复用 `taskListTags` 获取标签列表。
- R3 记忆页 actions 区：增加搜索框（防抖，标题/正文模糊匹配）、标签筛选（下拉，含「全部标签」）、「归档」视图开关（`includeArchived`）。
- R4 记忆详情面板：新增「归档」/「恢复」按钮（走 `memoryUpdate` 切 `archived`），归档后从列表消失（默认视图）并清空选中态。
- R5 归档视图下列出已归档记忆，可「恢复」；搜索结果与筛选叠加生效。
- R6 交互：搜索/筛选/归档切换后刷新 `["memories"]` 查询；选中项在筛选变化后若不可见则清空选中。

## Acceptance Criteria

- [ ] AC1 `MemoryQuery.search` 后端生效，按标题/正文模糊匹配；含 `%`/`_` 字面量可正常搜索。
- [ ] AC2 记忆页出现搜索框、标签筛选、归档开关，三者可叠加生效。
- [ ] AC3 详情面板可归档/恢复记忆；归档后默认列表移除该项，归档视图可见并支持恢复。
- [ ] AC4 `cargo test`、`pnpm typecheck`、`pnpm build` 通过。

## Notes

- 轻量-中复杂度任务：PRD + 简要 design。改动含 Rust（MemoryQuery.search）与前端（MemoryPage）。
- 标签列表复用全局 `taskListTags`，不新增命令，避免重复后端。
- 不改变既有 `pinnedOnly` 行为，与其叠加。
