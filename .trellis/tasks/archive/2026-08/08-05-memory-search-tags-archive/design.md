# 技术设计：记忆搜索/标签/归档

## 1. 后端：MemoryQuery.search

domain/memory.rs 增加字段（serde camelCase 自动映射 `search`）：

```rust
pub struct MemoryQuery {
    pub pinned_only: Option<bool>,
    pub include_archived: Option<bool>,
    pub tag_id: Option<EntityId>,
    pub quick_insert_only: Option<bool>,
    pub search: Option<String>,  // 新增
}
```

memories.rs `query()` 追加：

```rust
if let Some(text) = query.search.as_deref() {
    if !text.trim().is_empty() {
        let pattern = format!("%{}%", escape_like(text.trim()));
        sql.push_str(" AND (title LIKE ?1 ESCAPE '\\' OR body LIKE ?1 ESCAPE '\\')");
        values.push(Box::new(pattern));
    }
}
```

- `escape_like`：将 `%`、`_`、`\` 前缀 `\` 转义，避免通配符语义。
- 无迁移；`MemoryQuery` 是查询入参。

## 2. 前端

- client.ts `MemoryQuery` 增加 `search?: string`。
- 标签列表复用全局 `taskListTags`（返回 `tags` 全表，含记忆标签）。

## 3. MemoryPage

- state：`searchText`（本地防抖 250ms）、`tagId: string | null`、`showArchived: boolean`（并入 pinnedOnly 语义：archive 视图下 pinnedOnly 不适用）。
- queryKey：`["memories", { pinnedOnly, tagId, showArchived, search }]`，queryFn 传 `ipc.memoryQuery({ pinnedOnly, tagId, includeArchived: showArchived, search })`。
- actions 区（保持 h-11 头部）：
  - 搜索 Input（`type="search"`，占位「搜索标题/正文…」）；
  - 标签 `<select>`：全部标签 / 各标签；
  - 「归档」开关按钮（默认视图⇄归档视图）。
  - 既有「仅置顶」保留；归档视图时隐藏「仅置顶」以免语义冲突。
- 列表行：归档视图中已归档项显示「归档」徽标；点击选中。
- 详情面板（MemoryDetail 底部按钮组）：新增「归档」/「恢复」ConfirmButton（危险程度低用 ghost），走 `archiveMutation`（`memoryUpdate({ ...draft, archived: !draft.archived })`）；成功后失效 `["memories"]`；归档时若在默认视图清空选中（`onDeleted` 等价处理：`setSelectedId(null)`）。
- 空状态文案按视图区分（搜索无结果 / 无记忆 / 无归档）。

## 4. 一致性/回归

- 搜索用 LIKE 而非全局 search_query（避免把任务/剪切板结果混进来，保持页面内检索语义）。
- `pinnedOnly` 与 `includeArchived` 互斥：归档开关打开时强制 `pinnedOnly=false`。
- 删除/归档后若选中项不在结果集，清空选中，避免 detail 显示幽灵项。
