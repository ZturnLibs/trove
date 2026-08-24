# v2.0 可控智能辅助路线图（父任务）

## Goal

依据 `docs/post-v1-iteration-design.md` §9 与启动门槛评估（`research/startup-gate-assessment.md`，四条门槛：1/2 满足、3 本路线图冻结、4 作为切片 1 AC），将 v2.0「可控的智能辅助」拆解为可独立规划、实现、验收的 Trellis 子任务树，并冻结范围、顺序与隐私边界。

本父任务**不直接改业务代码**；实施单元为子任务。父任务交付物：本 PRD、启动门槛评估、`docs/v2-ai-assist-roadmap.md`、切片 1 子任务 PRD。

## 依据

- 设计合同：`docs/post-v1-iteration-design.md` §9（启动条件/产品原则/功能设计/隐私与安全/验收标准）
- 门槛评估：`./research/startup-gate-assessment.md`（2026-08-19）
- 用户证据：`.trellis/tasks/archive/2026-08/08-08-user-feedback-research/research/user-feedback-report.md`
- 能力边界证据：`src-tauri/src/domain/nl_parse.rs`（确定性 strip）、`application/search.rs`（FTS5）、`today_smart_sort`（确定性排序）

## 任务地图

| 顺序 | 子任务 | §9.3 对应特性 | Wave |
| --- | --- | --- | --- |
| 1 | `v2-0-ai-service-boundary` | （基础设施）AI 服务边界 + AISuggestion 数据模型 + provider 抽象 + 设置页授权/敏感范围/审计 | A |
| 2 | `v2-0-ai-extract-tasks` | 复杂信息提取（长文本 → 任务草稿，原文引用 + 逐项确认） | A |
| 3 | `v2-0-ai-related-content` | 相关内容建议（任务 ↔ 记忆/截图/历史任务，确认后写 EntityLink） | B |
| 4 | `v2-0-ai-weekly-summary` | 周期回顾摘要（确定性数字 + 模型组织文字，weekly_review 增强） | B |
| 5 | `v2-0-ai-daily-suggest` | 每日工作建议（今日重点候选 + 理由，today_smart_sort 增强） | B |
| 6 | `checklist-model` | （非 AI 前置）任务检查项数据模型与 UI | C |
| 7 | `v2-0-ai-task-split` | 任务拆分（生成候选检查项，依赖 6） | C |
| 8 | `v2-0-ai-semantic-search` | 语义检索（可重建向量索引，关键词+语义双列展示） | C |

**交付顺序**：Wave A（1→2，地基必须先行）→ Wave B（3/4/5 可并行）→ Wave C（6→7、8 独立）。每个子任务进入实现前单独 `task.py start` 并经 PRD 审阅。

## 全局合同（对所有子任务生效）

1. **AI 可关闭**：provider = off / 本地（Ollama）/ 自定义远程（OpenAI-compatible）；off 或失败时全部 v1.x 功能路径不经过 AI 分支（§9.1 门槛 4）。
2. **建议必带来源**：每条 `AISuggestion` 携带 `SuggestionSource`，UI 可跳转原文（§9.3）。
3. **确认门禁**：任何修改业务数据的建议必须经用户逐项确认，经 v1.4 `WorkbenchActionService` 分发（§9.2）。
4. **敏感红线**（§9.4）：密码管理器来源、标记敏感的记忆、排除应用列表内容**永不发送**；`sanitize_context` 在 provider 调用前执行；prompt/完整输出不进普通日志；审计仅记录提供方/模型/功能/接受与否。
5. **数字不由模型生成**（§9.6）：一切统计数字来自确定性 SQL；模型只组织文字。
6. **派生数据可清空可重建**（§10）：建议历史、向量索引不纳入主备份。
7. **评估样本回归**：切片 1 建立 `fixtures/ai-eval/` 固定样本（日期/任务提取、搜索相关性），后续切片跑同套样本。

## 父级验收标准

- [x] 启动门槛评估完成且结论可追溯（本任务 research/）
- [x] `docs/v2-ai-assist-roadmap.md` 发布并列入 docs/README.md
- [x] 切片 1 PRD 就绪并通过审阅（已交付）
- [x] Wave A 完成后复核门槛 4：off 模式端到端测试覆盖（切片 1 eval_off_provider_end_to_end_writes_nothing + 切片 2 off_mode 测试）
- [x] 8/8 切片完成；§9.6 对照：AI 关闭全功能可用 ✅、建议经确认 ✅、可跳源 ✅、数字不由模型生成 ✅、样本回归（离线 + #[ignore] 在线档）✅、错误建议可拒绝 ✅

## 明确不做（父级边界，继承 §9.7 + v1.3 边界）

- 聊天界面 / 独立 AI 主页
- 未经确认的自动完成/删除/延期/发送
- 绩效评分、行为评价、guilt-trip 文案
- 默认上传全部历史数据；内嵌模型推理（llama.cpp/candle/onnx，v2.x 再评估）
- 移动端 / 云同步 / 团队协作 / 日历双向（长期边界不变）
- AI Agent 总控台、自主操作电脑（§9.7）
