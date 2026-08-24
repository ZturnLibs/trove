# Design: v2.0 周期回顾摘要（切片 3）

## 1. 数据流

```
WeeklyReviewPage [生成 AI 摘要]
  → ipc.aiWeeklySummaryRequest()
      commands: state.ai_suggestions.request_weekly_summary(
          &state.weekly_review, &state.tasks, &state.reminders, &state.clipboard)
        ├ dismiss 上一条 pending summary（feature=summary, source="weekly"）
        ├ snapshot() 确定性统计（既有）
        ├ context = 计数 + 各组前 8 条任务标题（剪贴板组按 source_app sanitize，仅标题）
        └ request(Summary, "review", "weekly", ctx)（切片 1 管线）
  → 展示 payload.summary + 数字徽标（对照 snapshot）
  → [重新生成]（重复上述）/ [忽略] aiSuggestionDecide(dismiss)
```

- source_entity_type=`review`、source_entity_id=`weekly`（区分未来 daily/月度）。
- `AIFeature::Summary.prompt_template()` 从 None → 开放（domain 一处改动，服务层短路自动解除）。

## 2. Prompt 契约（domain/ai_suggestion.rs）

```
系统：你是个人工作台的回顾助手。把给定的本周统计数字组织成一段中文小结。
规则：
1. 只输出 JSON：{"summary":string,"items":[]}
2. summary ≤200 字；只陈述数字与事实，可温和提示（如"逾期 3 项，可挑选 1 项处理"）
3. 严禁评价表现、打分、使用"落后/失败/糟糕"等词；不编造数字之外的信息
4. 任务名可提及但不得改动
```

user context（服务层组装，模板无关）：
```
本周统计：收件箱未处理 N；逾期 N；等待/跟进 N；长期未动 N；近7天完成 N； upcoming 周期提醒 N；大体积剪贴板 N。
收件箱示例：A、B、C（最多8个）
逾期示例：…
近7天完成示例：…
```

## 3. 服务方法（AISuggestionService）

```rust
pub fn request_weekly_summary(&self, weekly, tasks, reminders, clipboard)
    -> Result<Option<AISuggestionRecord>, DomainError>
```

- 内部：`dismiss_pending(AIFeature::Summary, "weekly")`（新增小私有方法：UPDATE pending→dismissed）；
- snapshot 各组 items 取 `title`；剪贴板组若 `source_app` 命中排除名单则跳过该条（复用 sanitize 的名单逻辑，不把剪贴板正文放进 context）；
- context items 全部 entity_type=`review`（sensitive 列不存在，sanitize 的 memory 分支自然跳过）。
- `weekly_review_complete` 命令处追加：`ai_suggestions.dismiss_pending(Summary, "weekly")`（失败不阻塞完成）。

## 4. 命令

| 命令 | 说明 |
| --- | --- |
| `ai_weekly_summary_request` | 生成/重新生成，返回 Option<record> |

复用既有 `ai_suggestion_decide`（忽略）。

## 5. UI（WeeklyReviewPage）

页面顶部新增 `WeeklySummaryCard`（本文件内组件，不拆文件）：

- 条件渲染：`settings.ai.mode !== "off" && settings.ai.features.summary`；
- 三态：未生成（按钮）→ 生成中（按钮 loading）→ 已生成（段落 + 徽标行 + 重新生成/忽略）；
- 降级：mutation error → 一行 muted 文案「AI 摘要暂不可用，回顾功能不受影响」；
- 打开页面时若有 pending summary（`aiSuggestionList("summary","pending")` 中 source=weekly）直接展示（刷新不丢）。

## 6. 风险

| 风险 | 对策 |
| --- | --- |
| 模型不守 200 字 | prompt 约束 + UI 段落样式（超长不裁剪，接受） |
| 数字与摘要不符 | 摘要旁始终展示确定性徽标（对照即纠错）；prompt 禁编造 |
| 重复生成刷台账 | dismiss 旧 pending，台账最多一条 active |

## 7. 回滚

无迁移；prompt/服务/命令/UI/docs 独立 commit。
