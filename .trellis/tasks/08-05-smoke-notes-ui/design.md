# 技术设计：smoke_notes 接入 UI

## 1. QuickWindow：capture 新增「随手记」

- `CaptureType = "task" | "reminder" | "memory" | "note"`（QuickWindow.tsx:21）。
- captureTabs（:16 附近）新增 `{ id: "note", label: "随手记" }`。
- `submit()`（:328）增加分支：

```ts
else if (captureType === "note") {
  await ipc.smokeNoteCreate(value);
  await ipc.windowHideQuick();   // 或沿用当前 memory 提交行为
}
```

- 随手记无日期/优先级/重复字段，capture 表单在 note 类型下只显示标题输入（与现有分支一致，隐藏条件字段）。

## 2. MemoryPage：「记忆/随手记」视图切换

- state：`view: "memory" | "notes"`（默认 memory）。
- actions 区新增切换按钮（「记忆」/「随手记」，样式同「仅置顶」）。
- `view === "notes"` 时：
  - 列表区渲染 `smokeNotesQuery`（`useQuery(["smoke-notes"], () => ipc.smokeNoteList())`）；
  - 顶部输入框（占位「随手记…」，Enter 提交 `smokeNoteCreate`，成功失效 + 清空）；
  - 每条显示 body + 删除 `ConfirmButton`（`smokeNoteDelete` → 失效）；
  - 空状态「还没有随手记」。
- `view === "memory"` 时保持现有记忆逻辑不变；切换不重置 `pinnedOnly/tagId/search` 等记忆筛选状态。

## 3. 一致性

- queryKey：创建/删除后 `invalidateQueries(["smoke-notes"])`。
- 随手记列表按 `updatedAt` 倒序（后端 `list_active` 排序即可，前端直接用返回顺序）。
- 删除用 `ConfirmButton`（项目既有两步确认约定）。

## 4. 边界

- 不做随手记转任务/记忆、编辑、置顶（后端无此能力，超出 MVP）。
- 不新增后端命令。
