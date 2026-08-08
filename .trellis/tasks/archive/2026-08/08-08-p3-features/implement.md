# P3 功能实施清单

## 状态：已合并，验收补全

- [x] 同步 `main` 至 `feat-update`（含 P3 实现）
- [x] 对照 PRD 逐项核对 UI 与后端行为
- [x] 补剪切板 `source_app` / `date_from` / `date_to` 单元测试
- [x] `pnpm typecheck`
- [x] `cargo test --lib`
- [x] 归档任务

## 验证命令

```bash
pnpm typecheck
cd src-tauri && cargo test --lib
```

## 关键文件

| 功能 | 文件 |
|------|------|
| 清单 CRUD + 搜索 | `src-tauri/src/application/tasks.rs` |
| 清单 UI | `src/features/tasks/TasksPage.tsx` |
| 剪切板筛选 | `src-tauri/src/application/clipboard.rs` |
| 剪切板 UI | `src/features/clipboard/ClipboardPage.tsx` |
