# 技术设计：触发词展开/插入

## 1. 后端：trigger_word 参与搜索索引

`memories.rs` update/create 的 `upsert_conn` 调用，把可搜索文本拼接 trigger_word：

```rust
let searchable = if input.trigger_word.trim().is_empty() {
    input.body.clone()
} else {
    format!("{}\n{}", input.body, input.trigger_word.trim())
};
self.search.upsert_conn(&tx, SearchEntityType::Memory, input.id, &title, &searchable)?;
```

- create 分支同样处理（如 memories.rs create 也调用 upsert）。
- 搜索表 `search_index` 已按 title/body 分词，拼接后 trigger_word 参与匹配。
- 补单测：upsert 后按 trigger_word 可命中（search 模块测试或 memories 测试）。

## 2. 前端：QuickWindow capture 触发词展开

- 已有 `memoryQuery({ quickInsertOnly: true })`（openHit 内）：提取为 `snippets` 查询或复用。
- 计算触发词命中：

```ts
const snippetHit = useMemo(() => {
  if (mode !== "capture") return null;
  const q = searchText.trim().toLowerCase();
  if (!q) return null;
  return (snippets ?? []).find(
    (m) => m.triggerWord && m.triggerWord.trim().toLowerCase() === q,
  ) ?? null;
}, [mode, searchText, snippets]);
```

- 有命中时在 `paletteItems` 顶部插入：

```ts
{ kind: "command", id: `expand-${snippetHit.id}`, label: `↩ 展开「${snippetHit.title}」`, run: () => { setTitle(snippetHit.body || snippetHit.title); } }
```

- 展开后输入框内容变为片段正文，保持 captureType 等状态不变；用户可继续回车提交。
- 无触发词（null/空）不显示展开项，避免干扰。

## 3. 回归注意

- 展开仅替换标题输入（capture 的 `title`），不改 `dueDate`/`priority`/`daily`。
- `openHit` 的快速插入（复制到剪贴板）保留不动。
