# Design: v2.0 相关内容建议（切片 4）

## 1. 数据流

```
TaskDetailPanel [相关内容建议] 区
  → ipc.aiRelatedRequest(taskId)
      request_related(task, tasks, search, memories)
        ├ dismiss_pending(Related, taskId)
        ├ candidates = FTS(task.title + notes, [Memory, Clipboard], 12)
        │   − sensitive memories（sanitize 语义）
        │   − 双向已链接实体
        │   − rejected_pairs(taskId)（ledger 历史 dismissed/rejected related 的 sources）
        ├ 空候选 → None
        ├ prompt（候选编号+标题+摘要）→ 模型挑选+理由
        └ 回配：模型输出 title 精确匹配候选 → 丢编造；pending 记录
  → 列表 [关联] aiRelatedConfirm(id, [index]) → links.link("task",…,"related") → accepted
         [不相关] aiRelatedRejectItem(id, index) → dismissed 配对记录 + 剩余项
```

## 2. Prompt（domain，`RELATED_SYSTEM_PROMPT`）

```
你是个人工作台的相关内容推荐助手。给定一个任务和候选内容列表，选出真正相关的条目（最多 5 条）。
规则：
1. 只输出 JSON：{"items":[{"title":string,"detail":string|null,"dueDate":null,"dueTime":null,"ambiguous":true,"sourceExcerpt":string}],"summary":null}
2. title 必须与候选列表中的标题完全一致；sourceExcerpt 必须与候选摘要完全一致。不得编造候选列表之外的条目。
3. detail 用一句话说明相关理由（如「都涉及 Q4 预算」）。
4. 不确定相关的宁可不选。
```

复用 `SuggestedItem` 信封（dueDate/dueTime 恒 null + ambiguous=true，schema 天然放行）。

## 3. 服务方法（AISuggestionService）

```rust
const RELATED_CANDIDATES: i64 = 12;

fn rejected_pair_ids(&self, source_entity_id) -> HashSet<String>   // ledger 查询
pub fn request_related(&self, task_id, tasks, search, memories, links) -> Option<record>
pub fn confirm_related(&self, id, indices, links) -> RelatedConfirmResult { links: Vec<EntityLink>, suggestion }
pub fn reject_related_item(&self, id, index) -> AISuggestionRecord   // 主记录或收口后记录
```

- 回配：`HashMap<候选标题, (entity_type, entity_id)>`；模型 items 标题查表命中才保留；sources = 命中项（entityType/entityId/excerpt）。
- confirm 幂等：link 已存在（同 source/target/kind）则跳过（links.link 内部或预查）；全部成功 → accepted。
- reject_item：插入 dismissed 记录（feature=related, source=task, payload.items=[该条], sources=[该配对]）；主记录剩余 items>0 保持 pending，=0 时 decide（有历史 accepted 配对 → rejected 也行，取 rejected）。
- 过滤已链接：links.list_for_entity("task", id) 的双向 target 集合。

## 4. 命令

| 命令 | 说明 |
| --- | --- |
| `ai_related_request(taskId)` | 生成建议 |
| `ai_related_confirm(suggestionId, indices)` | 批量关联选中 |
| `ai_related_reject_item(suggestionId, index)` | 单条不相关 |

## 5. UI（`RelatedSuggestionsSection.tsx`，TaskDetailPanel 附件区上方）

- pending related 建议（按 sourceEntityId 过滤）列表；无 pending 时显示 [寻找相关内容] 按钮；
- 每项：标题（粗体）+ 理由（muted）+ 摘录（等宽小字）+ [关联][不相关]；
- 处理完显示统计行；错误降级一行文案；
- 关联成功 invalidate ["links"]（详情面板关联计数即时更新）。

## 6. 风险

| 风险 | 对策 |
| --- | --- |
| 模型改写标题导致回配失败 | prompt「完全一致」+ 回配丢弃 + 候选标题短（记忆标题普遍短） |
| FTS 候选质量差 | 候选空/模型全弃 → None → 「未找到相关内容」空态（诚实降级） |
| 重复配对刷 ledger | rejected 配对集合过滤 + confirm 幂等 |

## 7. 回滚

无迁移；prompt/服务/命令/UI/docs 独立 commit。
