# Design: v2.0 AI 任务拆分（切片 7）

## 1. 数据流

```
ChecklistSection [AI 拆分]
  → ipc.aiSplitRequest(taskId)
      request_split(task_id, tasks)
        ├ dismiss_pending(Split, task_id)
        ├ frozen? (completed → Err)
        ├ source_text = title + "\n" + notes；chars < 4 → None（无内容可拆，不打 provider）
        ├ context=[{task, id, source_text}] → 管线
        └ 子串校验：item.sourceExcerpt 必须在 source_text 中（含），否则丢条；全丢 → 整条作废 dismissed
  → 草稿区（checkbox 默认全不勾 + 内容 + 依据）
  → [添加选中] ipc.aiSplitApply(suggestionId, indices)
      apply_split → checklist_add 逐项（title 即内容，≤200 字/上限 50 走既有校验）→ accepted
  → invalidate ["tasks","checklist",id]（徽标+列表即时更新）
```

## 2. Prompt（`SPLIT_SYSTEM_PROMPT`）

```
你是个人工作台的任务拆分助手。根据给定任务生成可执行的检查项。
规则：
1. 只输出 JSON：{"items":[{"title":string,"detail":null,"dueDate":null,"dueTime":null,"ambiguous":true,"sourceExcerpt":string}],"summary":null}
2. title 是一条可勾选的检查项（动词开头，≤50 字），生成 3–8 条；不创建新任务、不带日期提醒。
3. sourceExcerpt 必须是任务原文中的连续片段，说明该项的依据；无依据的项不要生成。
4. 不得改写原任务内容；不确定的宁可不生成。
```

校验（服务端）：`source_text.contains(item.source_excerpt)` 逐条过滤。

## 3. 服务方法

```rust
pub fn request_split(&self, task_id, tasks) -> Result<Option<AISuggestionRecord>>
pub fn apply_split(&self, input: ExtractApplyInput, tasks) -> Result<Vec<ChecklistItem>>
```

- apply 复用 `ExtractApplyInput`（同 extract 语义）；门禁 feature==split、pending；
- checklist_add 失败（如上限）：已加项保留、建议保持 pending + 报错（同 extract 的部分失败语义）。

## 4. 命令

| 命令 | 说明 |
| --- | --- |
| `ai_split_request(taskId)` | 生成草稿 |
| `ai_split_apply(suggestionId, selectedIndices)` | 写入检查项 |

## 5. UI（ChecklistSection 扩展，不新增文件）

草稿状态：`useState<AISuggestionRecord | null>`（从 `aiSuggestionList("split","pending")` 恢复，按 sourceEntityId 过滤）；「AI 拆分」按钮置于检查项标题行右侧。

## 6. 风险

| 风险 | 对策 |
| --- | --- |
| 生成泛泛检查项（"开始做/继续做"） | prompt 动词开头 + 依据约束；用户可全拒 |
| sourceExcerpt 空格/换线不匹配 | 子串校验用原文精确 contains；prompt 要求连续片段 |

## 7. 回滚

无迁移；prompt/服务/命令/UI/docs 独立 commit。
