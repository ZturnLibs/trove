# 技术设计：拖拽排序跨清单语义修复 + @dnd-kit

## 1. 问题分析

### 1.1 sort_order 语义

数据库 `tasks.sort_order` 为 `f64`，**按清单独立编号**：
- 创建任务：`SELECT COALESCE(MAX(sort_order),0)+1 FROM tasks WHERE list_id=?1 AND deleted_at IS NULL`（`tasks.rs:324`）
- 任务列表查询：`ORDER BY t.sort_order ASC, t.created_at DESC`（`tasks.rs:784`）——在单清单过滤（`list_id=?`）下语义正确
- 今日任务分组：`ORDER BY t.sort_order ASC, t.created_at DESC`（`tasks.rs:925`）——**无 list 过滤，跨清单混排**

### 1.2 reorder_tasks 的缺陷

```rust
// tasks.rs:689 现状
for (index, id) in ordered_ids.iter().enumerate() {
    tx.execute("UPDATE tasks SET sort_order = ?1 ... WHERE id = ?3 ...", params![index as f64, ...]);
}
```

传入 id 被写成全局连续 `0..n-1`。单清单视图没问题；今日视图的 dueToday 可能包含 list A 的 2 个任务 + list B 的 1 个任务，全部被写成 0,1,2，而 list A 原有其他任务 sort_order 也在 0..2 区间 → 排序冲突、错乱，且 list A 内这些任务的相对顺序被整体打乱。

### 1.3 前端痛点

原生 HTML5 DnD：跨浏览器 drop 事件不一致、落点反馈弱（仅 2px 指示线）、无拖拽中的悬浮预览。

## 2. 修复方案

### 2.1 后端：按清单分组重写 sort_order

`reorder_tasks` 改造：对 `ordered_ids` 先按 `list_id` 分组，组内按传入顺序分配连续序号。

```
输入: [tA(list1), tB(list2), tC(list1), tD(list2)]  (拖拽后的目标顺序)
分组: list1 -> [tA, tC] -> sort_order 0,1
      list2 -> [tB, tD] -> sort_order 0,1
```

事务内执行。未在 ordered_ids 中的任务不动。

- 单清单视图：所有 id 同 list，行为与现一致。
- 今日视图：各清单内相对顺序按拖拽目标顺序更新，跨清单顺序通过 `created_at DESC` 次级排序兜底——因 dueToday 跨清单并无统一业务序，这是合理语义。
- **不污染**：每个清单只有出现在拖拽结果中的任务被重写，其他任务序号不变。

实现要点（`tasks.rs`）：
1. 一次查询拿到所有 id 的 `(id, list_id)` 映射。
2. 按 list_id 分组保留拖拽顺序。
3. 事务内对每组按序写 `sort_order = 组内索引`。
4. 保持 `revision + 1` 与 `updated_at` 更新。

### 2.2 前端：@dnd-kit

依赖：`@dnd-kit/core`、`@dnd-kit/sortable`、`@dnd-kit/utilities`（仅这三个，不引 bridge 库）。

组件结构（任务列表页）：

```
<DndContext sensors={PointerSensor + KeyboardSensor} collisionDetection={closestCenter}
            onDragStart onDragOver onDragEnd>
  <SortableContext items={tasks.map(t=>t.id)} strategy={verticalListSortingStrategy}>
    <SortableTaskRow task onReorderResult=... />
  </SortableContext>
</DndContext>
```

- `TaskRow` 包一层 `useSortable({id})`（新增 `SortableTaskRow` 或在 TaskRow 内条件启用）。
- `onDragEnd` 计算目标位置 → 复用现有 `applyDragReorder` 逻辑（取 drop 到的 target id + 前后关系）→ `ipc.taskReorder(orderedIds)`。
- DragOverlay：拖拽时渲染被拖行跟随鼠标，列表其余行用 `transform` 平滑让位（sortable 内置）。
- 插入位置指示：sortable 自带的 `transform` 让位即为可视反馈；如需更明确指示可在 overlay 下加下划线样式。

今日页同样改造，仅 dueToday 分组包 `SortableContext`，逾期/已完成/提醒分组不包。

**键盘可访问性**：@dnd-kit 提供 `KeyboardSensor` + `SortableContext` 的拖拽手柄/空格键支持，比 HTML5 DnD 更优。保留 Enter/Space 选中与双击重命名（`onSelect`/`onDoubleClick` 不变，通过 PointerSensor 的 `activationConstraint: { distance: 8 }` 避免点击误触拖拽）。

### 2.3 TaskRow 兼容

- 保留现有原生 DnD props 或移除均可；因两个调用页都换 @dnd-kit，倾向移除原生 DnD props，仅保留 `useSortable` 包装层。若影响 InboxPage（InboxPage 是否需拖拽？——不在本次范围，保持不可拖拽）。
- 双击重命名在拖拽手柄区分开：拖拽只由行主体触发，标题双击编辑不受影响。

## 3. 数据流

```
前端拖拽结束
  → onDragEnd(active.id, over.id, position)
  → 由当前已加载列表计算新 orderedIds
  → ipc.taskReorder(orderedIds)
  → 后端按 list 分组写 sort_order
  → 返回后 invalidate ["tasks"] / ["tasks","today"]
```

## 4. 兼容性与回滚

- 后端：`reorder_tasks` 语义变更，Rust 测试覆盖单清单与跨清单两种场景。回滚即还原此函数。
- 前端：@dnd-kit 是纯增量依赖，未改动数据契约。若 @dnd-kit 有问题，可回退到 HTML5 DnD 实现（git revert）。
- 版本：`@dnd-kit/*` 当前稳定版（core v6、sortable v8、utilities v3），与 React 19 兼容（用 `^` 范围，实施时锁定实测可用版本）。

## 5. 风险

- @dnd-kit 与 React 19 的 peer 兼容性——实施第一步先 `pnpm add` 并跑 typecheck 验证。
- DragOverlay 需要渲染在 portal；Tauri WebView（WKWebView/WebView2）下 DnD 事件正常（库基于 pointer 事件，不依赖 HTML5 DnD）。
- 今日视图跨清单排序语义：跨清单顺序由 `created_at` 兜底，非用户完全可控——文档化这一限制。
