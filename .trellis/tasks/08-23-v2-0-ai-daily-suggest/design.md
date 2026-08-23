# Design: v2.0 每日工作建议（切片 5）

## 1. 数据流

```
TodayPage [今日工作建议（AI）] 卡
  → ipc.aiDailySuggestRequest()
      request_daily_suggest(tasks)
        ├ dismiss_stale_daily_pending()   // created_at 非今日的 pending 全收口
        ├ dismiss_pending(Suggest, "daily")  // 今日旧 pending 也收口（重新生成语义）
        ├ candidates = today_tasks().overdue + due_today
        │   − focus 集 − waiting 集；截 15；每项特征行：
        │   《标题》 截止:2026-08-25 14:00|无 / 优先级:高|中|低|无 / 延期:N / 提醒:09:00
        ├ 空候选 → None
        ├ prompt → 模型挑 1–3 + 理由
        └ 回配（标题精确匹配候选）→ pending（source_entity_id="daily"）
  → [加入重点] 前端 dailyFocusAdd(taskId)（既有撤销栈）+ 项从卡上消失
  → [跳过] aiDailySuggestSkip(suggestionId, index)
      → dismissed 配对记录（feature=suggest, source=daily, items=[该项]）
      → 主记录剩余项清零自动 rejected
  → 再次请求：候选过滤 skipped_pair_ids（当日跳过不再出现）
```

## 2. Prompt（`SUGGEST_SYSTEM_PROMPT`）

```
你是个人工作台的今日规划助手。给定今天的候选任务（含确定性特征），挑出最值得今天聚焦的 1–3 项。
规则：
1. 只输出 JSON：{"items":[{"title":string,"detail":string|null,"dueDate":null,"dueTime":null,"ambiguous":true,"sourceExcerpt":string}],"summary":null}
2. title 必须与候选列表完全一致；sourceExcerpt 必须与该项特征行完全一致。
3. detail 用一句话说明为什么今天做，必须基于特征（如“今天 18:00 截止”“已延期 2 次”“高优先级”）。
4. 严禁编造特征之外的信息；不确定的宁可不选。
```

## 3. 服务方法

```rust
pub fn request_daily_suggest(&self, tasks: &TaskService) -> Result<Option<record>>
fn dismiss_stale_daily_pending(&self)                // created_at < today AND pending → dismissed
pub fn skip_daily_suggest_item(&self, id, index)     // 配对记录 + 收口（复用 reject_related_item 泛化）
fn daily_skipped_ids(&self) -> HashSet<String>       // suggest + source=daily 的 dismissed 配对
```

- `reject_related_item` 泛化：按 feature 校验（related/suggest 均可用），改名 `reject_suggestion_item`（保留旧名 thin wrapper 供既有命令）。
- 特征行格式即候选“摘要”，sourceExcerpt 回配约束生效。
- focus/waiting 排除：`today_tasks()` 已返回 focus/waiting 组 → 取 id 集做差。

## 4. 命令

| 命令 | 说明 |
| --- | --- |
| `ai_daily_suggest_request` | 生成/重新生成 |
| `ai_daily_suggest_skip(suggestionId, index)` | 跳过单项 |

复用 `ai_suggestion_decide`（整卡忽略）。

## 5. UI（`DailySuggestionsCard.tsx`，TodayPage 智能排序区上方）

- pending suggest（source=daily）驱动渲染；无则 [获取今日建议] 按钮（候选空则不渲染卡）；
- 项：标题（粗）+ AI 理由 + 特征徽标（来自 sourceExcerpt 本地解析展示原文）+ [加入重点][跳过]；
- 加入成功 → 该项从 pending 记录中移除（用 skip 通道落 accepted 配对？—— 不：加入即写 focus，建议项通过 reject_suggestion_item 移除但配对记录 status 用 dismissed 会污染“跳过”语义 → 新增 `remove_daily_suggest_item(id, index, accepted: bool)`：配对记录 status=accepted/dismissed，过滤池只读 dismissed）；
- 全部处理完隐藏卡片；错误降级一行。

## 6. 风险

| 风险 | 对策 |
| --- | --- |
| 候选特征缺失（多数任务无日期/优先级） | 特征行仍输出（“无”占位）；模型可全弃 → None 空态 |
| 跨天悬置 | dismiss_stale_daily_pending 每次请求先清 |
| 重复刷配对 | 候选过滤 skipped 集合 |

## 7. 回滚

无迁移；prompt/服务/命令/UI/docs 独立 commit。
