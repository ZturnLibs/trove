# Design: v2.0 复杂信息提取（切片 2）

> 事实基线（切片 1 交付）：`AISuggestionService.request(feature, srcType, srcId, ctx)` 返回 pending `AISuggestionRecord{payload.items:[SuggestedItem{title,detail,dueDate,dueTime,ambiguous,sourceExcerpt}], sources}`；`decide/list/clear` 就绪；provider 管线含开关/sanitize/校验/审计。
> 编排先例：`AutomationService` 方法接收 `&TaskService` 等服务引用（automation.rs:234）。

## 1. 数据流

```
MemoryPage「提取任务」
  → ipc.aiExtractRequest(memoryId)
      commands: state.ai_suggestions.request_existing_or_new(Extract, memory, context=[{memory, id, title\nbody}])
          ├ 幂等：SELECT pending extract WHERE source_entity_id = memoryId → 直接返回
          └ 新建：切片 1 管线（含 provider 调用）
  → 草稿面板（items + excerpt + ambiguous 标记）
  → 勾选 → ipc.aiSuggestionApply(id, [indices])
      AISuggestionService::apply_extract(id, indices, &tasks, &links, &search)
          ├ 门禁：status == pending；indices 去重、范围合法、无重复
          ├ 每项：taskCreate{title, notes: detail + 来源引用, dueDate: 合法且非 ambiguous 才填}
          │       search.upsert(task)；links.link(memory, task, "ai_extract")
          └ 全部成功 → decide(accepted)；返回 Task 列表
```

**边界遵守**：`apply_extract` 消费的是**结构化** `SuggestedItem`（经切片 1 校验），自由文本无直通路径（§9.5）。AISuggestionService 保持唯一写 `ai_suggestions` 表的模块；业务写入经注入的既有服务。

## 2. 新增类型（domain/ai_suggestion.rs）

```rust
pub struct ExtractApplyInput { pub suggestion_id: String, pub selected_indices: Vec<usize> }
pub struct ExtractApplyResult { pub tasks: Vec<Task>, pub suggestion: AISuggestionRecord }
```

校验：indices 非空、去重后升序、每个 `< items.len()`。`due_date` 合法性复用 `parse_suggestion_content` 已保证（非 ambiguous 必为 YYYY-MM-DD）；apply 时二次防御：`chrono::NaiveDate::parse_from_str` 失败或 `ambiguous` → 不写日期。

## 3. 命令（commands/mod.rs）

| 命令 | 签名 | 说明 |
| --- | --- | --- |
| `ai_extract_request` | `(memoryId) -> Option<AISuggestionRecord>` | 幂等提取；找不到记忆报 NotFound |
| `ai_suggestion_apply` | `(input: ExtractApplyInput) -> ExtractApplyResult` | 应用选中项 |

lib.rs 注册两个；前端 client.ts 对应封装。

## 4. AppState 编排

`AppState` 已有 `tasks/links/search/ai_suggestions`，命令层直接传引用，无新服务、无新迁移、无新表。

## 5. UI（MemoryPage）

```
详情操作区：[转任务] [提取任务(AI)]   ← mode!=off && features.extract
     ↓ 点击
requestMutation(isPending=提取中…)
     ↓ 成功
<ExtractSuggestionsPanel record>
  ├ 每项：[✓] 标题 / 日期·时间 | ambiguous→「日期待确认，创建时不填」 / excerpt 等宽小字
  ├ [创建选中任务]（≥1 勾选才可用）→ applyMutation → 成功态：任务标题列表（点击导航任务）
  └ [都不合适] → aiSuggestionDecide(rejected) → 面板收起
     ↓ 无结果/失败
空状态文案：「没有识别出任务草稿。」/「AI 服务不可用，请检查设置。」（tone 遵循 empty-states 文档）
```

- 面板数据从 `aiSuggestionList(feature="extract", status="pending")` 恢复（切页回来不丢）——memoryId 过滤在前端按 `sourceEntityId` 匹配。
- 日期展示：合法日期显示原文；ambiguous 显示标记。
- 组件放 `src/features/memory/ExtractSuggestionsPanel.tsx`，MemoryPage 引用；文案常量内聚。

## 6. 风险与对策

| 风险 | 对策 |
| --- | --- |
| 超长记忆 | `CompletionRequest` 12k 字符截断（切片 1 已有） |
| provider 慢阻塞 UI | 命令在线程池，前端 isPending 状态；20s 上限 |
| 重复点提取 | requestMutation 禁用；后端幂等双保险 |
| 应用中途失败 | 逐项独立创建，失败即返回已建列表+错误；建议保持 pending 可重试剩余（indices 幂等去重） |
| 撤销任务后建议仍 accepted | 属预期：建议是台账审计，不跟踪任务生命周期 |

## 7. 回滚

命令/服务方法/UI/docs 各自独立 commit；无 schema 变更，revert 即回。
