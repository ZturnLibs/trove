# v1.2 来源与关联体系（EntityLink）

## Goal

为 v1.2 后续切片（文件引用、浏览器捕获、搜索增强）建立通用的"来源与关联"基础设施：任务/记忆与资源（当前为图片资产，未来扩展文件引用、来源网址）之间可管理、可查询、可引用安全清理的关联关系，并把资源清理接入现有数据流。

用户价值：
- 图片等资源能被任务或记忆长期引用，不会被剪切板过期/超量清理或资源清理误删。
- 同一资源可被多个条目复用，不重复占空间。
- 删除业务条目时，用户清楚关联资源的处理方式。
- 不再产生无法回收的孤儿资产文件。

## 背景（代码勘察确认）

- `entity_links` 表已存在（`0004_memory_search.sql`）：`id / source_type / source_id / target_type / target_id / link_kind / created_at`，有 source/target 索引，**无唯一约束**。
- 既有两处内联 link 写入：
  - `memories.rs convert_to_task`：memory→task `converted_to`（`src-tauri/src/application/memories.rs:206-214`）
  - `clipboard.rs convert_to_memory`：memory→asset `attachment`（`src-tauri/src/application/clipboard.rs:553-567`）
- 引用保护局部存在：`clipboard.rs is_asset_linked` + `enforce_limits` 在过期/超量清理时跳过被引用的资产（`clipboard.rs:571-581, 583-667`）。
- `assets` 表 + `AssetStore`（image 类型、哈希去重、缩略图、`absolute_path/read_bytes/thumb_base64`）；`Asset` 结构体在 `domain/clipboard.rs:54-67`。
- `derived_texts` 表（OCR）存在；`search` 索引任务/提醒/记忆/剪切板。
- **无物理资产 GC**：剪切板条目软删后 `assets` 行与文件从不删除（孤儿资产）。
- 导出/导入已覆盖 `entity_links`、`assets`、`derived_texts`（`data_port.rs`）。
- 迁移由 `db/mod.rs` 的 `MIGRATIONS` 数组驱动；`0008` 需注册并同步测试断言（`db/mod.rs` 测试 `schema_version == 7` → 8）。
- 服务经 `AppState` 注入，命令在 `commands/mod.rs` 定义并注册于 `lib.rs generate_handler`。
- 前端：`TaskDetailPanel.tsx`、`MemoryPage.tsx MemoryDetail` 无附件区；`ClipboardPage.tsx` 已有图片缩略图与"保存为记忆/转为任务"入口；`ipc/client.ts` 为 IPC 类型与封装层。

## Requirements

- R1 通用关联服务：创建、按 id 移除、按实体列出关联；校验实体类型与 link_kind；同一 (source, target, kind) 幂等去重。
- R2 关联查询：给定业务实体列出其关联资源（含图片缩略图等可展示信息）；给定资源查询引用它的实体/引用计数。
- R3 引用安全清理：仅"无任何引用且超过保留期"的资产允许物理删除（行+文件）；被实体关联或仍被活动剪切板条目引用的资产必须保留。
- R4 删除带关联的业务条目时，先说明资源处理方式（引用被移除、资源按保留期与引用情况决定是否清理），确认后移除该实体的全部关联。
- R5 既有内联 link 写入（`converted_to`、`attachment`）收敛到新服务，行为不变。
- R6 前端任务详情与记忆详情增加附件区：查看缩略图、移除关联、从剪切板图片历史附加资源。

## Acceptance Criteria

- [ ] AC1 通过命令可在任务/记忆与图片资产间建立关联；同一关联重复创建不产生重复行。
- [ ] AC2 关联可列出、可按 id 移除；移除后引用计数/引用状态正确下降。
- [ ] AC3 被实体引用的图片资产在剪切板过期/超量清理与资产 GC 中均不被删除。
- [ ] AC4 无引用的孤儿资产超过保留期后行与文件一并删除；保留期内不删除。
- [ ] AC5 删除带关联的任务/记忆时出现确认说明；确认后该实体关联被移除，资源交回引用/保留期规则管理。
- [ ] AC6 `converted_to`、`attachment` 两条既有链路改走新服务后行为不变（含回归测试）。
- [ ] AC7 导出再导入后，关联与资产数据完整恢复。
- [ ] AC8 任务详情与记忆详情附件区可查看缩略图、移除关联、从剪切板图片历史附加资源。

## Out of Scope

- `SourceReference`（文件引用、浏览器来源网址）表与相关能力。
- 截图快速收藏、文件引用、浏览器捕获等 v1.2 其他切片。
- 搜索结果资源预览增强。
- 非 image 资产类型。
- 跨设备同步与云端。

## Key Decisions（已与用户确认）

- 删除带关联业务条目：确认后移除该实体全部关联；资源文件保留，后续按引用/保留期规则决定是否清理。
- 附加资源入口：任务/记忆详情附件区提供"附加图片"，从剪切板图片历史选择器选取。
- 资产 GC：随 `clipboard enforce_limits` 触发（启动、每次采集、轮询）；保留期复用 `clipboard_retention_days`，不新增设置项。
