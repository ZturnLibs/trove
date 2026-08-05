# 技术设计：来源与关联体系（EntityLink）

## 1. 架构与边界

新增独立服务模块 `application/links.rs`（`EntityLinkService`），成为 `entity_links` 表的唯一访问入口；`assets.rs` 扩展 `collect_garbage` 资产清理；`clipboard.rs` / `memories.rs` 改用新服务；命令层暴露 IPC；前端新增附件区。

```
commands/mod.rs  ──►  EntityLinkService  ──►  entity_links 表
       │                    ▲
       │                    │ 注入（AppState 与各业务服务）
  业务服务（memories/clipboard/tasks）
       │
   AssetStore.collect_garbage()  ──►  assets + derived_texts + 文件
```

- `EntityLinkService` 无状态、持有 `Database`，每次操作 `connect()` 打开连接（与现有服务模式一致）。
- 各业务服务内自建 `EntityLinkService`（参照 `MemoryService` 内部自建 `TaskService` 的先例），`AppState` 再单独持有一个供命令层使用。

## 2. 数据模型

### 领域结构（`domain/links.rs` 新增）

```rust
pub enum LinkEntityType { Task, Reminder, Memory, Clipboard, Asset }  // as_str / parse

pub struct EntityLink {
    id: EntityId,
    source_type: String,   // 或 LinkEntityType
    source_id: EntityId,
    target_type: String,
    target_id: EntityId,
    link_kind: String,     // "attachment" | "converted_to"
    created_at: String,
}

pub struct LinkInput { source_type, source_id, target_type, target_id, link_kind }
```

link_kind 校验白名单：`attachment`、`converted_to`（未知值拒绝）。

### 迁移 `migrations/0008_entity_links.sql`

```sql
-- 迁移前先去重，再建唯一索引（幂等创建）
DELETE FROM entity_links WHERE id NOT IN (
  SELECT MIN(id) FROM entity_links
  GROUP BY source_type, source_id, target_type, target_id, link_kind
);
CREATE UNIQUE INDEX IF NOT EXISTS idx_entity_links_pair
  ON entity_links (source_type, source_id, target_type, target_id, link_kind);
```

- `db/mod.rs` 的 `MIGRATIONS` 注册 `(8, include_str!("../../../migrations/0008_entity_links.sql"))`。
- 更新 `db/mod.rs` 两个迁移测试断言 `schema_version == 7` → `8`。

## 3. EntityLinkService 接口

```rust
pub fn link(&self, source_type, source_id, target_type, target_id, link_kind) -> Result<EntityLink>
// 校验类型/kind → INSERT OR IGNORE → 按 pair 回查返回（幂等）
pub fn unlink(&self, id: EntityId) -> Result<()>                       // 按 id 删除
pub fn purge_for_source(&self, source_type, source_id) -> Result<usize> // 删除源实体的全部关联
pub fn list_for_entity(&self, entity_type, entity_id) -> Result<Vec<EntityLink>>
// 同时覆盖 source=实体（向外）与 target=实体（被引用）两条方向；提供参数区分
pub fn list_outgoing(&self, source_type, source_id) -> Result<Vec<EntityLink>>
pub fn list_incoming(&self, target_type, target_id) -> Result<Vec<EntityLink>>
pub fn is_referenced(&self, target_type, target_id) -> Result<bool>
pub fn reference_count(&self, target_type, target_id) -> Result<i64>
```

### 既有链路收敛

- `memories.rs convert_to_task`：`self.links.link("memory", id, "task", task_id, "converted_to")`（替换 `memories.rs:206-214`）。
- `clipboard.rs convert_to_memory`：`self.links.link("memory", memory_id, "asset", asset_id, "attachment")`（替换 `clipboard.rs:553-567`）。
- `clipboard.rs is_asset_linked`：改为 `self.links.is_referenced("asset", asset_id)?`（替换 `clipboard.rs:571-581`；调用处保持不变）。

服务注入：`MemoryService`、`ClipboardService` 增加 `links: EntityLinkService` 字段并在 `new()` 内自建。

## 4. 资产 GC（`application/assets.rs`）

### 依赖断开：剪切板软删时解绑 asset

`clipboard.rs` 中三处软删 UPDATE（`delete`、`clear_non_favorites`、`enforce_limits` 两个分支）同时执行 `asset_id = NULL`：

```sql
UPDATE clipboard_items SET deleted_at = ?, updated_at = ?, revision = revision + 1,
       asset_id = CASE WHEN asset_id IS NOT NULL THEN NULL ELSE asset_id END
WHERE id = ?
```

说明：实体关联（entity_links）独立保护资产，因此解绑 clipboard 引用不会破坏被实体引用的资产。

### collect_garbage

```rust
pub struct GcSummary { pub removed: usize, pub freed_bytes: i64 }

pub fn collect_garbage(&self, retention_days: u32) -> Result<GcSummary>
```

判定条件（全部满足才清理）：

```sql
SELECT id, relative_path, thumb_path, byte_size FROM assets a
WHERE a.deleted_at IS NULL
  AND a.created_at < ?cutoff
  AND NOT EXISTS (SELECT 1 FROM entity_links el
                  WHERE el.target_type = 'asset' AND el.target_id = a.id)
  AND NOT EXISTS (SELECT 1 FROM clipboard_items ci
                  WHERE ci.asset_id = a.id)
```

处理顺序（单连接内）：`DELETE FROM assets WHERE id = ?`（`derived_texts` 依赖 FK `ON DELETE CASCADE` 自动清理）→ 删除 `relative_path` 与 `thumb_path` 文件（文件不存在时忽略）。

### 触发

在 `clipboard.rs enforce_limits()` 末尾调用 `self.assets.collect_garbage(settings.clipboard_retention_days)`（忽略失败并 `tracing::warn`，不阻塞清理主流程）。`enforce_limits` 已在启动、每次采集、轮询触发（`app_state.rs:60`、`clipboard.rs:169/257`、`clipboard_poller.rs:76`）。

## 5. 删除业务条目时的关联处理

命令层（`commands/mod.rs`）在软删后清理该实体全部关联：

- `task_delete`：`state.links.purge_for_source("task", id)?`（放软删成功之后，失败仅 `tracing::warn`，避免影响删除主操作）。
- `memory_delete`：`state.links.purge_for_source("memory", id)?`（同上）。

前端删除确认：删除前查询 `entity_link_list`，若有关联则 `confirm()` 文案追加"N 个关联资源将随之移除，资源文件按保留规则保留"。

## 6. IPC 命令与前端契约

```rust
entity_link_create(input: LinkInput) -> Result<EntityLink>
entity_link_remove(id: EntityId) -> Result<()>
entity_link_list(entity_type: String, entity_id: EntityId) -> Result<Vec<EntityLink>>
entity_link_assets(entity_type: String, entity_id: EntityId) -> Result<Vec<LinkedAsset>>
// LinkedAsset = { linkId, assetId, contentHash, byteSize, width, height,
//                 thumbBase64, createdAt }
```

- `entity_link_assets` 聚合 links + assets + 缩略图，供附件区渲染（单次调用）。内部可复用 `AssetStore`。
- 注册到 `lib.rs generate_handler`；前端 `ipc/client.ts` 增加对应方法与类型。
- 前端附加选择器复用现有 `clipboard_query({ kind: "image", limit: N })` 展示图片历史，选中后调用 `entity_link_create`。

### 前端组件

- 新增 `src/design-system/patterns/AttachmentsSection.tsx`：查询 `entity_link_assets` 渲染缩略图列表；提供移除按钮与"附加图片"按钮；"附加图片"打开图片历史选择器浮层（无新依赖，固定定位 overlay）。
- 接入 `TaskDetailPanel.tsx`（source_type `task`）与 `MemoryPage.tsx MemoryDetail`（source_type `memory`）。
- 数据失效：link 创建/移除后 `invalidateQueries(["links", entityType, id])`。

## 7. 导出 / 导入兼容

- `entity_links` 已在 `EXPORT_TABLES` 与导入删除清单中，无需改动。
- 唯一索引不影响正常导出/导入；迁移前生成的、含重复 pair 的旧导出文件导入可能命中唯一约束失败，属已知低风险，不阻塞（迁移会清理库内重复，后续导出不再含重复）。

## 8. 测试

- `application/links.rs` 单测：link 幂等、list_outgoing/list_incoming、unlink、purge_for_source、is_referenced/reference_count。
- `application/assets.rs` 单测：
  - 无引用 + 超保留期 → GC 删除行与文件；
  - 有 entity_links 引用 → 保留；
  - 有活动 clipboard 引用 → 保留；
  - 未超保留期 → 保留。
- `clipboard.rs` 回归：现有 `image_dedupe_and_linked_survives_expire` 保留并扩展为经新服务断言；新增软删解绑后 GC 可回收用例。
- `memories.rs` 回归：`convert_to_task` 生成 `converted_to` 关联。
- `data_port.rs` 回归：`export_import_roundtrip` 增加对 `entity_links` 行数的断言。
- `db/mod.rs`：迁移测试断言版本号更新到 8。

## 9. 回滚

- 迁移 0008 为幂等去重 + 唯一索引，破坏性仅限库内重复 pair 的清理（预期无损）。
- 若需回退：保留 GC 不自动执行（移除 `enforce_limits` 末尾调用），关联服务与命令可独立下线；已删除的孤儿资产文件不可恢复（符合"过期可清理"的产品语义）。

## 10. 风险

- GC 误删风险：判定条件为"无 entity_links + 无任何 clipboard_items.asset_id 引用 + 超保留期"，双重引用保护；删除前可再行数二次确认（测试覆盖）。
- 性能：`is_referenced` 在 `enforce_limits` 循环内每次新开连接查询，数据量小，可接受；若变慢可改批量预查询（不做本期优化）。
