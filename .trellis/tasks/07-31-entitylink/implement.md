# 执行计划：来源与关联体系（EntityLink）

## 前置

- 工作流：Phase 2 通过 `trellis-before-dev` 注入 `.trellis/spec/` 规范；本任务为内联平台，跳过 jsonl gate。
- 验证命令：
  - `pnpm typecheck`
  - `cd src-tauri && cargo test`
  - `pnpm test:unit`
- 提交规范：功能切分提交，先 Rust 后前端。

## 实施清单（顺序执行）

### A. 数据层

- [ ] A1 `migrations/0008_entity_links.sql`：先删重复 pair（保留 MIN(id)），再建唯一索引 `idx_entity_links_pair`。
- [ ] A2 `db/mod.rs`：`MIGRATIONS` 注册 `(8, ...)`；更新两个迁移测试断言 `schema_version == 7` → `8`。
- [ ] A3 `domain/links.rs` 新增：`LinkEntityType`（task/reminder/memory/clipboard/asset，as_str/parse）、`EntityLink`、`LinkInput`；`domain/mod.rs` 导出。

### B. 服务层

- [ ] B1 新建 `application/links.rs` `EntityLinkService`：
  - `link`（校验类型与 kind 白名单、`INSERT OR IGNORE`、按 pair 回查）
  - `unlink(id)`、`purge_for_source(source_type, source_id)`
  - `list_outgoing` / `list_incoming` / `list_for_entity`
  - `is_referenced` / `reference_count`
- [ ] B2 `application/mod.rs` 注册 `links`。
- [ ] B3 `memories.rs`：加 `links` 字段；`convert_to_task` 改用 `self.links.link("memory", id, "task", task_id, "converted_to")`。
- [ ] B4 `clipboard.rs`：加 `links` 字段；
  - `convert_to_memory` 改用 `self.links.link("memory", memory_id, "asset", asset_id, "attachment")`
  - `is_asset_linked` 改用 `self.links.is_referenced("asset", asset_id)`
  - 三处软删 UPDATE（`delete`、`clear_non_favorites`、`enforce_limits` 两个分支）同时解绑 `asset_id = NULL`
  - `enforce_limits()` 末尾调用 `self.assets.collect_garbage(retention_days)`（失败仅 warn）

### C. 资产 GC

- [ ] C1 `application/assets.rs` 新增 `collect_garbage(retention_days) -> GcSummary`（条件见 design §4），处理 `assets` 行、`derived_texts`（FK 级联）、原图与缩略图文件。
- [ ] C2 `domain` 或 `assets.rs` 定义 `GcSummary { removed, freed_bytes }`（Serialize）。

### D. 命令层与状态

- [ ] D1 `app_state.rs`：`links: Arc<EntityLinkService>`，`AppState::new` 中创建并注入。
- [ ] D2 `commands/mod.rs` 新增：
  - `entity_link_create(input: LinkInput)`
  - `entity_link_remove(id: EntityId)`
  - `entity_link_list(entity_type: String, entity_id: EntityId)`
  - `entity_link_assets(entity_type: String, entity_id: EntityId)`（聚合缩略图）
- [ ] D3 `task_delete` / `memory_delete`：软删成功后 `purge_for_source`（失败仅 warn）。
- [ ] D4 `lib.rs` `generate_handler` 注册四个新命令。

### E. 前端

- [ ] E1 `ipc/client.ts`：新增 `EntityLink`、`LinkedAsset`、`LinkInput` 类型与 `entityLinkCreate/Remove/List/Assets` 方法。
- [ ] E2 新增 `src/design-system/patterns/AttachmentsSection.tsx`：
  - 查询 `entityLinkAssets` 渲染缩略图（img + 尺寸 + 移除按钮）
  - "附加图片"打开图片历史选择器浮层（`clipboardQuery({kind:"image"})`），选中即 `entityLinkCreate`
  - 成功/移除后失效 `["links", entityType, id]`
- [ ] E3 `TaskDetailPanel.tsx`：接入 `AttachmentsSection`（task）；删除确认时若有资源追加说明文案。
- [ ] E4 `MemoryPage.tsx MemoryDetail`：接入 `AttachmentsSection`（memory）；删除确认时若有资源追加说明文案。

### F. 测试与收尾

- [ ] F1 `links.rs` 单测（幂等 / 双向列表 / unlink / purge / 引用计数）。
- [ ] F2 `assets.rs` GC 单测（四个场景：超期无引用删除、有 link 保留、有活动 clipboard 引用保留、未超期保留）。
- [ ] F3 `clipboard.rs` 回归：扩展 `image_dedupe_and_linked_survives_expire`；新增软删解绑后 GC 回收用例。
- [ ] F4 `memories.rs` 回归：`convert_to_task` 生成 `converted_to` 关联。
- [ ] F5 `data_port.rs`：`export_import_roundtrip` 增加 `entity_links` 行数断言。
- [ ] F6 运行 `cd src-tauri && cargo test`、`pnpm typecheck`、`pnpm test:unit` 全绿。
- [ ] F7 手动冒烟（`pnpm tauri:dev`）：剪切板图片→任务/记忆附加→详情查看/移除→删除业务条目确认文案→孤儿图片过期后清理。

## 检查点 / 回滚点

- B1 完成后（服务层可用）为一个独立可验证检查点。
- C1+D2 完成后（GC + IPC）为第二个检查点。
- 每个检查点运行 `cargo test`。
- 回滚：移除 `enforce_limits` 末尾的 GC 调用即可停用清理；迁移 0008 幂等可重跑。

## 风险文件

- `src-tauri/src/application/clipboard.rs`（软删解绑 + is_asset_linked 改造 + GC 触发）
- `src-tauri/src/application/assets.rs`（collect_garbage）
- `src-tauri/migrations/0008_entity_links.sql` + `db/mod.rs`（迁移注册与测试断言）
- `src/design-system/patterns/AttachmentsSection.tsx`（新组件）
