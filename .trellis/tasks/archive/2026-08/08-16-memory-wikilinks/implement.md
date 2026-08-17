# 记忆轻量双链 — 实现记录

## Phase 0 — 解析与链接

- [x] Migration `0014_memory_mention.sql` — `mention_use_count`
- [x] `domain/memory_wikilink.rs` — `[[title]]` 解析
- [x] `EntityLink` kind `mention`
- [x] 保存/创建时 `sync_wikilinks` 自动建链

## Phase 1 — API

- [x] `memory_wikilink_pending` / `memory_resolve_wikilinks`
- [x] `memory_backlinks` / `memory_related` / `memory_link_mention`
- [x] 删除记忆时清理入/出链并回退 `mention_use_count`

## Phase 2 — 前端

- [x] 详情「反向链接」「相关记忆」区块
- [x] 歧义/缺失双链解析对话框
- [x] QuickWindow 排序：`quick_insert` 查询按 `mention_use_count`

## Phase 3 — 文档

- [x] `docs/ui-layout-interaction.md` §7.4

## 验证

- [x] `cargo test` 78/78
- [x] `pnpm typecheck`
- [x] `pnpm test:unit` 37/37

## 手动验收（待勾选）

- [ ] 编辑 `[[A]]` 后 A 详情出现反向链接
- [ ] 目标不存在时弹出创建/跳过
- [ ] 相关记忆「建链」后出现在出站 mention
