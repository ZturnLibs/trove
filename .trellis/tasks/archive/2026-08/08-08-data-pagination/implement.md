# Implement: 数据层分页

## Checklist

### Phase A — Domain & backend

- [ ] A1 新增 `src-tauri/src/domain/page.rs`：`PagedResult<T>`、`page_limit()` / `page_offset()` helpers
- [ ] A2 `TaskQuery` / `MemoryQuery` / `ClipboardQuery` 增加 `limit`、`offset`
- [ ] A3 `TaskService.query_tasks`：COUNT + LIMIT/OFFSET + `PagedResult`
- [ ] A4 `MemoryService.query`：同上
- [ ] A5 `ClipboardService.query`：补 offset + total + hasMore
- [ ] A6 `TaskService.smart_list`：分页参数透传
- [ ] A7 `commands/mod.rs` 返回类型更新；`task_smart_list` 签名扩展
- [ ] A8 Rust 单测：`tasks.rs` / `memories.rs` 分页边界（空、末页、hasMore、筛选 total）

### Phase B — IPC & frontend types

- [ ] B1 `client.ts`：`PagedResult<T>`、`PageParams`；四个 invoke 返回类型更新
- [ ] B2 `rg taskQuery|memoryQuery|clipboardQuery|taskSmartList` 穷尽调用点，改读 `.items`

### Phase C — UI

- [ ] C1 可选 `src/features/shared/usePagedList.ts`（或各页内联，遵循 YAGNI）
- [ ] C2 `TasksPage`：加载更多 + 筛选重置
- [ ] C3 `MemoryPage`：加载更多 + 搜索/筛选重置
- [ ] C4 `ClipboardPage`：加载更多
- [ ] C5 `AttachmentsSection`：`clipboardQuery` 适配

### Phase D — Verify

- [ ] D1 `cd src-tauri && cargo test`
- [ ] D2 `pnpm typecheck && pnpm test:unit && pnpm build`
- [ ] D3 手动：Tasks 页切换筛选 → 加载更多 → 无重复

## Validation commands

```bash
cd src-tauri && cargo test
pnpm typecheck
pnpm test:unit
pnpm build
```

## Risky files

- `src-tauri/src/application/tasks.rs` — 核心查询 SQL
- `src/ipc/client.ts` — 破坏性类型变更
- `TasksPage.tsx` — 筛选 + 分页 state 交互

## Rollback point

Phase A 完成后可先 `cargo test`；Phase B 未完成前前端会 typecheck 失败，宜同 PR 合并。
