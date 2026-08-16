# 剪贴板智能行动 — 实现记录

## Phase 0 — 分类与存储

- [x] Migration `0013_clipboard_kind_hint.sql`
- [x] `domain/clipboard_classify.rs` — URL/邮箱/电话/日期/代码/报错规则
- [x] 捕获时写入 `kind_hint`（文本 + OCR 图片）

## Phase 1 — 后端 API

- [x] `ClipboardSmartContext` — NL 草稿、相似任务、已关联 ID
- [x] `clipboard_smart_context` / `clipboard_link_to_task`
- [x] `convert_to_task` 支持 draft + EntityLink 幂等
- [x] `clipboard_set_smart_actions_enabled` 设置项
- [x] `ClipboardQuery.kind_hint` 代码筛选

## Phase 2 — 前端

- [x] 列表行动气泡 + 详情 NL 预览 + 相似任务关联
- [x] 「代码片段」筛选
- [x] 设置「启用智能行动」开关

## Phase 3 — 文档

- [x] `docs/ui-layout-interaction.md` §7.5

## 验证

- [x] `cargo test` 75/75
- [x] `pnpm typecheck`
- [x] `pnpm test:unit` 37/37

## 手动验收（待勾选）

- [ ] 复制 URL/报错文本后列表出现对应气泡
- [ ] 关闭智能行动后气泡隐藏、条目仍正常保存
- [ ] 重复「转为任务」不创建第二条
