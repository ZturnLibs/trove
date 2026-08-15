# 个人工作台用户需求全网调研报告

- **产品锚点**：Trove（本地优先个人工作台：任务、提醒、记忆、剪切板）
- **调研日期**：2026-08-08
- **任务**：`.trellis/tasks/08-08-user-feedback-research/`
- **方法**：公开渠道检索 + 主题编码 + 与 Trove 定位对照

---

## 1. 执行摘要

用户对「个人工作台」的真实诉求，**不是**「功能越多越好」，而是围绕五个可量化指标反复出现：

| 指标 | 用户原话倾向 | 跨渠道验证强度 |
| --- | --- | --- |
| **记录成本** | 「打开就能记」「全局快捷键」「不用切窗口」 | ★★★★★ |
| **切换成本** | 「减少 tab 切换」「在一个地方完成上下文」 | ★★★★☆ |
| **找回成本** | 「搜索要快」「OCR 找截图文字」「标签/筛选」 | ★★★★★ |
| **维护成本** | 「每周复盘别像苦差」「别 guilt-trip 逾期」 | ★★★★☆ |
| **可信程度** | 「本地/离线」「数据在我机器上」「提醒要准」 | ★★★★★ |

**与 Trove 定位高度契合的方向**（应加强）：

1. 键盘优先 + 全局浮层快速捕获（Raycast/Things/Mindwtr 共识）
2. 本地优先 + 可预测的数据行为（HelixNotes/Unforget/Maccy 共识）
3. 剪切板 → 任务/记忆 的「捕获闭环」（Obsidian 插件、Maccy OCR 方向）
4. 提醒/通知的**可编辑、可管理、点击落地**（Raycast Reminders、TickTick 反面教材）
5. 聚焦与复盘：单条处理模式、温和逾期、复盘向导（GTD/Everdo 论坛）

**用户明确排斥或 Trove 已决策不做的方向**（保持边界）：

- 团队协作 / 共享任务板（Things/Todoist 用户换用动机之一，但 Trove 定位为个人）
- 重度 AI Agent 编排总控台（2025 中文「个人工作台」热词，与 Trove 边界不同）
- 提前建设移动端 + 云同步账号体系（Trove `post-v1-iteration-design.md` 已排除近期路线）
- 日历双向同步 / 时间块（Todoist/Things 用户痛点，但 Trove 已确认非近期核心）

---

## 2. 调研方法与渠道

### 2.1 渠道清单

| 渠道类型 | 代表来源 | 样本特征 |
| --- | --- | --- |
| **Hacker News Show HN** | HelixNotes #46978621、Unforget #40645743、Amna #23780781 | 开发者/知识工作者，强调 local-first 与 UX 取舍 |
| **Reddit / 社区聚合** | Things 3 替代讨论、TickTick 弃用文 | 任务管理选型、订阅 vs 买断、跨平台 |
| **GitHub Issues** | Raycast extensions、Maccy (p0deje/Maccy) | 可操作的 feature request，带维护者回复 |
| **GTD / 效率论坛** | gettingthingsdone.com、Everdo forum | 复盘流程、心理负担、inbox 处理 |
| **中文效率社区** | 少数派（飞书 AI 工作台、知识管理 2025 版） | Agent 化趋势 vs 表格/文档原生闭环 |
| **产品评测 / 长文** | Nerdynav、Pickuma、DEV (Will Be Done) | 结构化对比 Things/TickTick/Todoist |
| **学术论文 / 开源 README** | Workstream (arXiv)、Mindwtr、will-be-done | 设计原则自述（local-first、零配置） |
| **内部对照** | Trove gap-audit、post-v1-iteration-design | 实现现状与产品承诺 |

### 2.2 编码方法

1. 从每条来源提取**用户行为场景**（而非功能名）
2. 归入 5 大指标（记录/切换/找回/维护/可信）
3. 标注**情感极性**（刚需 / Nice-to-have / 排斥）
4. 与 Trove 模块映射：任务 / 提醒 / 记忆 / 剪切板 / 全局快捷

### 2.3 局限

- 无 Trove 自有用户访谈；结论偏**品类级**而非 Trove 专属
- 中文「个人工作台」2025 热词大量指向 **AI Agent 总控台**，与 Trove 定位需刻意区分
- 部分来源为评测站二次整理 Reddit，已尽量回溯 primary source

---

## 3. 跨渠道需求主题（附证据）

### 3.1 记录成本：「3 秒内捕获，否则就不会记」

**用户声音**

- Raycast 用户希望 Create Reminder 支持 **NLP 自然语言**（`tomorrow 3pm #Work p1`）和 **Cmd+字段跳转**（对标 Things 3）— [Raycast #15993](https://github.com/raycast/extensions/issues/15993)
- Mindwtr 强调 **Capture from anywhere**：全局热键、托盘、Share Sheet — [Mindwtr README](https://github.com/lineCode/Mindwtr)
- Will Be Done 作者：第三次造轮子，因为要 **Vim 键位 + 周视图 + 离线即时写入** — [will-be-done README](https://github.com/will-be-done/will-be-done)
- Amna：任务空间内嵌浏览器/编辑器，**减少「记下了但上下文丢了」** — [HN Amna #23780781](https://news.ycombinator.com/item?id=23780781)

**Trove 映射**

| 现状 | 差距 |
| --- | --- |
| QuickWindow、NL 解析、菜单栏、今日快速添加（v1.1 已交付） | 提醒创建 NLP/字段跳转仍可加强；独立提醒管理弱（内部 P1） |

**优先级建议**：**P1** — 巩固「不打开主窗口也能完整捕获」；对齐 Raycast/Things 的快速录入体验。

---

### 3.2 切换成本：「别让我为一个小动作开三个 App」

**用户声音**

- Raycast Reminders 用户因 **无法附加 URL/打开链接** 被迫回到系统 Reminders — [Raycast #16406](https://github.com/raycast/extensions/issues/16406)、[#18717](https://github.com/raycast/extensions/issues/18717)（注：Apple API 限制；Trove 不受此限）
- Workstream 自述：**Reduced context switching** — PR/Jira/日历一屏，减少浏览器 tab — [arXiv Workstream](https://arxiv.org/abs/2604.17055)
- Pickuma 开发者文：Things 3 **故意不做**复杂过滤，换来说「少在 App 里折腾」— 切换成本也可以是**功能减法**

**Trove 映射**

- Trove 优势：任务+提醒+记忆+剪切板**同一本地库**，无需 Raycast 式多扩展拼装
- 缺口：通知点击无落地（gap-audit P2）；剪切板 directPaste 宣称与实现不符（P2）

**优先级建议**：**P0** — 通知 → 对应提醒/任务；**P0** — 能力宣称与实现一致（directPaste 等），与 §3.5 品牌信任一致。

---

### 3.3 找回成本：「搜不到 = 没存过」

**用户声音**

- HelixNotes：UpNote UX + Obsidian 本地文件；**Tantivy 即时全文搜索**是核心 — [HN #46978621](https://news.ycombinator.com/item?id=46978621)
- Unforget：Keep 迁移后靠 **极快搜索 + #tag 式文本**，文件夹反而不需要 — [HN #40645743](https://news.ycombinator.com/item?id=40645743)
- Maccy：**OCR 搜截图内文字**是 2025 高频需求；用户还要 **从 OCR 结果直接粘贴文本** — [Maccy #992](https://github.com/p0deje/Maccy/issues/992)、[#1063](https://github.com/p0deje/Maccy/issues/1063)
- Maccy 高级用户：**按类型过滤**是留用理由；Pinned 需分组/文件夹 — [#1286](https://github.com/p0deje/Maccy/issues/1286)
- Obsidian vs UpNote：任务管理靠 Todoist；笔记 App **缺提醒**是公认缺口 — [UpNote 对比文](https://getupnote.com/share/notes/wgfQg2Eh3zeRVAOO9vqdUzVQp9a2/4d708608-520d-45f7-9c51-640c7b6451f1)

**Trove 映射**

| 模块 | 用户期望 | Trove 状态 |
| --- | --- | --- |
| 剪切板 | OCR 搜图、按来源/时间筛选 | macOS OCR ✅；Win OCR 缺失 P2；筛选维度少 P3 |
| 记忆 | 搜索、标签筛选、Markdown 渲染 | 搜索/标签/归档 UI 弱 P1；MD 未真渲染 P2 |
| 任务 | 标签浏览、页内搜索 | `task_list_tags` 未接 UI P2 |
| 全局 | 命令面板统一搜四类数据 | QuickWindow 已有，可加强命令分区 |

**优先级建议**：**P1** 记忆搜索/筛选；**P2** 标签体系 UI 贯通（任务+记忆）；**P2** Windows OCR。

---

### 3.4 维护成本：「整理不应比做事更累」

**用户声音**

- Will Be Done：**No angry OVERDUE badges** — 温和 nudge，避免 guilt — [SynidSweet/will-be-done](https://github.com/SynidSweet/will-be-done)
- Everdo 用户：Weekly Review 需要 **一次只看一项** 的 Focus 模式，否则 Someday 列表永远扫一眼过 — [Everdo #4440](https://forum.everdo.net/t/feature-request-focus-during-my-weekly-review/4440)
- GTD 论坛：Weekly Review 成「苦差」因 **inbox 一周才清一次**、项目过多「心理负重」— [GTD #17015](https://forum.gettingthingsdone.com/threads/struggling-with-gtd-reviews.17015/)
- deariary 文：**Reconstruction tax** — 复盘前花 15–20 分钟重建「这周发生了什么」— [deariary 2026-05](https://blog.deariary.com/posts/2026-05-15-todoist-weekly-review-automate-it-with-your-diary)

**Trove 映射**

- `post-v1-iteration-design.md` **v1.3 个人聚焦与复盘** 与 Everdo/GTD 诉求一致：每周回顾 / 专注模式（对应「复盘向导 + 单条处理模式」诉求；v1.3 明确统计只帮助整理，不生成分数或效率结论）
- 温和逾期、跳过/延期已有；缺 **结构化复盘向导** 与 **单条 focus 模式**

**优先级建议**：**P2**（v1.3 路线）— 复盘向导 + 单条处理模式；**P3** — 完成/跳过行为的轻量「周摘要」降低 reconstruction tax。

---

### 3.5 可信程度：「本地优先不是口号」

**用户声音**

- HelixNotes / Unforget：**Firebase/云端 = dealbreaker**；要 plain files + 可选自托管 — HN 讨论
- Maccy：**忽略机密剪贴板类型**、可暂停录制 — [#1335](https://github.com/p0deje/Maccy/issues/1335) 维护者安全模型说明
- TickTick **反面教材**：同步丢任务、电池 drain、Linux syslog 暴涨 — [travelertechie 2025](https://www.travelertechie.com/2025/04/why-i-wont-use-ticktick-anymore.html)
- Things 3 用户换 TickTick 的首要动机之一是 **跨平台**；留 Things 的首要动机是 **买断 + 美观 + 可靠** — [Nerdynav 2025](https://nerdynav.com/ticktick-vs-things-3/)

**Trove 映射**

- Trove **核心差异化**与最强共识重合：SQLite 本地、可导出、离线
- 风险点：提醒调度准确性、数据分页性能、Windows 能力 parity、directPaste 过度承诺

**优先级建议**：**P0 品牌信任** — 修复「宣称 vs 实现」不一致；**P2** — 备份/健康状态 UI（`backup_status` 已存在未接）。

---

### 3.6 键盘优先与「桌面工具感」

**用户声音**

- Things 3：**Best-in-class design** + 键盘/手势 — 多源评测共识
- mach / Will Be Done：**Vim 导航**是留存理由 — [mach](https://github.com/rvcas/mach)
- Raycast：用户期望 Reminders 扩展达到 **Things 级字段快捷键** — [#15993](https://github.com/raycast/extensions/issues/15993)
- Trove 自身文档：`docs/ui-layout-interaction.md` — **桌面工作台，键盘优先** — 与外部声音一致

**优先级建议**：**P2** — 持续补齐快捷键覆盖与可发现性（已有 shortcuts 体系）。

---

### 3.7 片段 / 触发词 / 文本复用

**用户声音**

- Maccy：Pinned snippets 需 **分组/文件夹**，否则换别家 — [#1286](https://github.com/p0deje/Maccy/issues/1286)
- Raycast Snippets：文件夹组织、自动捕获选中文本 — Raycast 社区（规划中）
- Trove v1.1 承诺：**触发词 + 直接粘贴** — gap-audit 标记 **triggerWord 未落地 P2**

**优先级建议**：**P2** — 触发词展开 + 片段分组；与剪切板/记忆快速插入闭环。

---

### 3.8 AI 时代「个人工作台」— 中文语境特殊项

**少数派 / 技术栈 2025 趋势**（与 Trove 边界需区分）

| 中文热词 | 用户要什么 | 与 Trove 关系 |
| --- | --- | --- |
| Agent 总控台 / Skill 调度 | 把多个 AI Skill 编排在一个界面 | **不同品类**；Trove v2.0 是「可控智能辅助」非 Agent IDE |
| 飞书多维表格应用模式 | 表格即应用、低代码搭业务系统 | 偏团队/结构化业务；Trove 不做 CRM |
| 知识管理 + AI | 捕获→整理→检索→输出闭环 | **部分重叠**：记忆+搜索+未来 v2.0 BYOK 建议 |
| Megi 类工具 | 对话变知识树、本地存储 | 重叠「记忆关联」v1.3 方向，非核心 |

**结论**：中文舆论场「个人工作台」语义在 2025 向 **AI 编排** 漂移；Trove 应坚持 **本地事务工作台** 叙事，避免被误读为 Agent 平台。

---

## 4. 竞品维度对照（任务管理品类）

| 维度 | Things 3 | TickTick | Todoist | 用户常见抱怨 | Trove 机会 |
| --- | --- | --- | --- | --- | --- |
| 美观/动效 | ★★★★★ | ★★★☆ | ★★★★ | Things 仅 Apple | 紧凑桌面美学已文档化 |
| 跨平台 | Apple only | 全平台 | 全平台 | Things 无 Win/Web | Win 已有；Linux 后续 |
| 自然语言 | 弱 | 中 | 强 | — | NL 已有，可加强提醒侧 |
| 提醒/通知 | 系统级 | 内置 | 内置 | TickTick 不可靠 | **做好提醒编辑+落地** |
| 日历/时间块 | 只读 | 强 | 中 | 两向 sync 皆弱 | Trove 刻意不做近期 |
| API/自动化 | 无公开 API | Open API | REST 强 | Things 无法 CI 集成 | v1.4 本地 API 候选 |
| 协作 | 无 | 有 | 有 | 非个人用户痛点 | Trove 不做 |
| 定价 | 买断 | 订阅 | 订阅 | 订阅疲劳 | Trove 本地无账号优势 |

---

## 5. Trove 需求优先级矩阵（外部声音 × 内部 gap）

综合本报告与 gap-audit，建议产品 backlog  listening 优先级：

### P0 — 信任与基础体验

| 需求 | 外部证据 | 内部证据 |
| --- | --- | --- |
| 提醒可编辑、独立提醒可删可查 | Raycast 用户被迫回系统 App | gap-audit P1 |
| 能力宣称与实现一致（directPaste 等） | Maccy 以 honest 安全模型获信任 | gap-audit P2 |
| 通知点击落地到具体提醒 | 桌面效率工具通用期望 | gap-audit P2 |

### P1 — 高共识差异化

| 需求 | 外部证据 | 内部证据 |
| --- | --- | --- |
| 记忆搜索 / 标签 / 归档 UI | Unforget/HelixNotes | gap-audit P1 |
| 全局捕获体验打磨（NLP、字段跳转） | Raycast/Things | v1.1 延续 |
| 剪切板→行动闭环（转任务/记忆+OCR） | Maccy OCR、Obsidian 插件 | v1.2 主题 |

### P2 — 体验加深

| 需求 | 外部证据 | 内部证据 |
| --- | --- | --- |
| 触发词 / 片段分组 | Maccy #1286 | gap-audit P2 |
| Windows OCR parity | Maccy 2025 OCR 浪潮 | gap-audit P2 |
| 任务标签筛选 UI | Todoist 标签文化 | gap-audit P2 |
| Markdown 真渲染 | Obsidian/UpNote 期望 | gap-audit P2 |
| 周期提醒 UI（周/月） | TickTick 完整 recurrence | gap-audit P2 |
| 快捷键覆盖与可发现性 | Things 3 键盘/手势共识 | 已有 shortcuts 体系 |
| 备份/健康状态 UI | Maccy 安全模型信任 | `backup_status` 已存在未接 |
| 复盘向导 / 单条 Focus 模式 | Everdo/GTD 论坛 | post-v1 v1.3（每周回顾/专注模式） |

### P3 — v1.3+ 与长期

| 需求 | 外部证据 | 内部证据 |
| --- | --- | --- |
| 轻量周摘要 / 复盘记录降低 reconstruction tax | deariary | post-v1 v1.3 后续 |
| 本地 REST/CLI（开发者自动化） | Todoist API 受众 | post-v1 v1.4 |
| 分页与大数据量性能 | Will Be Done「years of tasks」 | gap-audit P3 |

### 明确低优先级 / 不做（用户有需求但 Trove 不跟）

- 团队协作、共享看板
- 内置 Pomodoro/习惯追踪（TickTick 式 all-in-one 膨胀）
- 重度 AI Agent 并行编排（Orca/VibeKanban 品类）
- 近期移动端 + 强制账号同步
- 日历双向同步与时间块

---

## 6. 建议的后续动作

1. **验证**：用 5–8 个目标用户场景做轻量访谈（开发者、知识工作者、GTD 实践者），检验 P0/P1 排序
2. **指标**：为五指标各设 1 个可测量 proxy（如「捕获→保存中位耗时」「搜索成功率」「提醒误建后 24h 内修正率」）
3. **叙事**：对外材料区分「本地事务工作台」vs「AI Agent 工作台」，避免品类混淆
4. **迭代**：P0 与 gap-audit P1 提醒项合并为下一 sprint 候选

---

## 7. 来源索引

### Hacker News

- HelixNotes — https://news.ycombinator.com/item?id=46978621
- Unforget — https://news.ycombinator.com/item?id=40645743
- Amna — https://news.ycombinator.com/item?id=23780781

### GitHub Issues

- Raycast Apple Reminders NLP — https://github.com/raycast/extensions/issues/15993
- Raycast Reminders URL / 打开链接 — https://github.com/raycast/extensions/issues/16406 , #18717
- Maccy OCR / pinned organization — https://github.com/p0deje/Maccy/issues/992 , #1063 , #1286 , #1335 , #1375

### 论坛 / 长文

- Everdo weekly review focus — https://forum.everdo.net/t/feature-request-focus-during-my-weekly-review/4440
- GTD struggling with reviews — https://forum.gettingthingsdone.com/threads/struggling-with-gtd-reviews.17015/
- deariary reconstruction tax — https://blog.deariary.com/posts/2026-05-15-todoist-weekly-review-automate-it-with-your-diary
- TickTick 弃用 — https://www.travelertechie.com/2025/04/why-i-wont-use-ticktick-anymore.html
- Things vs TickTick — https://nerdynav.com/ticktick-vs-things-3/
- Todoist vs Things — https://www.morgen.so/blog-posts/todoist-vs-things-3
- UpNote 缺提醒对比 — https://getupnote.com/share/notes/wgfQg2Eh3zeRVAOO9vqdUzVQp9a2/4d708608-520d-45f7-9c51-640c7b6451f1
- Pickuma 开发者选型 — https://pickuma.com/for-dev/todoist-vs-ticktick-vs-things-3-task-managers/

### 开源 / 论文

- Will Be Done — https://github.com/will-be-done/will-be-done （「No angry OVERDUE badges」引文见 fork: https://github.com/SynidSweet/will-be-done）
- Mindwtr — https://github.com/lineCode/Mindwtr
- mach — https://github.com/rvcas/mach
- Workstream — https://arxiv.org/abs/2604.17055
- HelixNotes DEV — https://dev.to/lasttyper/why-i-built-helixnotes-a-local-first-markdown-notes-app-3n0f

### 中文社区

- 少数派：飞书 AI 个人效率系统 — https://sspai.com/post/104020
- 少数派：知识管理 2025 — https://sspai.com/post/104783
- 技术栈：个人工作台 Skill — https://jishuzhan.net/article/2084604290919710721

### 内部

- Trove gap-audit — `.trellis/tasks/archive/2026-08/08-05-feature-gap-audit/research/gap-audit.md`
- Trove post-v1 设计 — `docs/post-v1-iteration-design.md`

---

*报告完。如需补充特定渠道（V2EX、Product Hunt、App Store 评论）可开 follow-up 调研切片。*
