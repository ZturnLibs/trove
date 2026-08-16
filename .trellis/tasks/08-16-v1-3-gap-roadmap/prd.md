# v1.3 竞品差距补齐路线图

## Goal

将 Trove 与 Things / TickTick / Todoist / Obsidian / Maccy 等功能差距分析，转化为可独立规划、实现、验收的 Trellis 子任务树，按产品路线图优先级推进 v1.3 核心能力与 v1.3.x 差异化切片。

## 背景与依据

- 内部审计：`.trellis/tasks/archive/2026-08/08-05-feature-gap-audit/research/gap-audit.md`（2026-08，部分 P1/P2 已在 8 月批次修复）
- 外部调研：`.trellis/tasks/archive/2026-08/08-08-user-feedback-research/research/user-feedback-report.md`
- 产品/design 依据：`docs/post-v1-iteration-design.md` §7–§8、`docs/next-iteration-roadmap.md`
- 当前基线：Trove v1.2.7，四模块本地合一；v1.2.1 P3 收尾进行中（`08-08-p3-features`）

## 任务地图

| 顺序 | 子任务 | 对标差距 | 目标版本 |
| --- | --- | --- | --- |
| 1 | `08-16-gtd-workflow-states` | Everdo/TickTick 等待/推迟/Someday | v1.3 |
| 2 | `08-16-focus-daily-wrap` | Everdo Focus、每日收尾 | v1.3 |
| 3 | `08-16-weekly-review` | GTD 每周回顾、reconstruction tax | v1.3 |
| 4 | `08-16-clipboard-smart-action` | Maccy OCR→行动、Obsidian 转任务 | v1.3.x |
| 5 | `08-16-health-dashboard` | 本地信任、备份可感知 | v1.3.x |
| 6 | `08-16-memory-wikilinks` | Obsidian 轻量双链 | v1.3.x |
| 7 | `08-16-today-smart-sort` | Todoist Smart Schedule 轻量版 | v1.3.x |
| 8 | `08-16-url-scheme` | Raycast/Todoist API 生态 | v1.4 前置 |
| 9 | `08-16-tray-today-panel` | Things 菜单栏今日视图 | v1.1 余留 |
| 10 | `08-16-capture-remaining` | v1.2 截图/文件/存储管理器 | v1.2 余留 |
| 11 | `08-16-quickwindow-nlp-polish` | Raycast/Things 快速录入 | v1.1 加深 |

## 推荐交付顺序

1. **Wave A（v1.3 主体）**：子任务 1 → 2 → 3（依赖：统一「活跃任务」筛选定义）
2. **Wave B（差异化）**：子任务 5 → 6 → 4 → 7（可并行，5 为 3/7 的数据基础）
3. **Wave C（生态与余留）**：子任务 9 → 11 → 8 → 10

子任务间无硬依赖树约束；上表顺序写入各子任务 `prd.md` 的 Notes，实施时按 Wave 推进。

## 父级验收标准

- [ ] 11 个子任务均具备可执行的 `prd.md` 与明确 AC
- [ ] 复杂子任务（1/2/8/10）在 `task.py start` 前有 `design.md`
- [ ] 每个子任务完成后可独立归档，不影响其他子任务
- [ ] 全部完成后，Trove 在「工作节奏（GTD/复盘）」与「捕获闭环」维度可与 Everdo/Things 局部对标

## 明确不做（父级边界）

- 移动端 + 云同步 + 账号体系
- 团队协作 / 共享看板 / 评论
- 日历双向同步 / 时间块 / 看板视图
- 子任务/检查项（可另开独立任务，不在本子树范围）
- AI Agent 总控台 / 重度自动化编排
- 直接粘贴到前台应用（已有诚实降级，另任务评估）

## 体验极致契约

全子任务遵循 [`quality-bar.md`](./quality-bar.md)：**不极致不退出**。功能「能用」不等于可归档；须通过九维体验门禁与子任务 AC。

## Notes

- 本父任务**不直接改代码**；各子任务为实施单元
- 与进行中的 `08-08-p3-features` 并行：P3 收尾完成后优先启动 Wave A 子任务 1
- 进入任一子任务实现前，需单独 `task.py start <child>` 并经 PRD/design 审阅
