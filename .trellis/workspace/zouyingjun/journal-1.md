# Journal - zouyingjun (Part 1)

> AI development session journal
> Started: 2026-07-30

---



## Session 1: 今日页底部快速输入任务 & 修复 confirm 失效

**Date**: 2026-08-05
**Task**: 今日页底部快速输入任务 & 修复 confirm 失效
**Branch**: `main`

### Summary

今日页内容区底部新增快速输入任务入口（nlParseCapture 自然语言解析，回车创建，默认截止今天）；修复 Tauri webview 中 window.confirm/alert 失效导致归档/删除不可用，统一改为 ConfirmButton 两步内联确认，并沉淀为前端组件规范

### Git Commits

| Hash | Message |
|------|---------|
| `04c02d2` | (see git log) |
| `aedae3f` | (see git log) |
| `a12eda1` | (see git log) |

### Status

[OK] **Completed**


## Session 2: v1.4 统一动作层并发布 v1.4.0

**Date**: 2026-08-18
**Task**: v1.4 统一动作层并发布 v1.4.0
**Branch**: `feat-improve`

### Summary

完成动作层 / trove-cli / 规则自动化 / 快捷指令查询 / 任务 CSV，合并 PR #10 并发布 v1.4.0；补上 default-run 与 universal lipo 后安装包才出齐。

### Main Changes

- 统一 WorkbenchAction 与 trove-cli、规则自动化、快捷指令 query、任务 CSV 导入导出
- 合并 PR #10，在 main 上 release.sh minor 打出 v1.4.0
- 修复 default-run、universal lipo、beforeBundleCommand 路径后发版流水线转绿
- 把发版与动作层合同写入 .trellis/spec/tauri/ 并归档任务 08-17-v1-4-action-layer

### Git Commits

| Hash | Message |
|------|---------|
| `c2fb1f8` | (see git log) |
| `34577ef` | (see git log) |
| `82796c3` | (see git log) |
| `afdb004` | (see git log) |
| `b9ef347` | (see git log) |
| `a9c894c` | (see git log) |
| `f7f19fa` | (see git log) |
| `4ebd3e3` | (see git log) |
| `f52648d` | (see git log) |
| `8892c69` | (see git log) |
| `bcc5633` | (see git log) |

### Testing

- [OK] cargo test --lib 108/108；PR #10 frontend/rust CI 绿
- [OK] GitHub Release v1.4.0 含 dmg、exe/msi、tar.gz+.sig、latest.json（darwin + windows）

### Status

[OK] **Completed**

### Next Steps

- 如需让 main 也没有活跃任务，把 feat-improve 上的 spec/archive/journal 提交合入 main


## Session 3: v2.0 启动评估与路线图规划

**Date**: 2026-08-19
**Task**: v2.0 启动评估与路线图规划
**Branch**: `feat-improve`

### Summary

对齐 main 的 v1.4.0 后，完成 §9.1 启动门槛评估（门槛1/2 满足、3 冻结数据边界、4 转为切片1 AC），创建父任务 v2-0-ai-assist-roadmap 与切片 1 v2-0-ai-service-boundary 的 PRD，发布 docs/v2-ai-assist-roadmap.md

### Git Commits

| Hash | Message |
|------|---------|
| `3476f53` | (see git log) |

### Status

[OK] **Completed**


## Session 4: v2.0 切片 1：AI 服务边界实现交付

**Date**: 2026-08-21
**Task**: v2.0 切片 1：AI 服务边界实现交付
**Branch**: `feat-improve`

### Summary

实现 0019 迁移、AIConfig/domain 校验、Off+Http provider（key 库外文件隔离）、AISuggestionService 管线（sanitize 红线/结构化校验/审计台账）、7 个 IPC 命令、设置页智能辅助区块与记忆敏感标记、ai-eval 固定样本与离线回归 runner；docs/ai-assist.md 与 privacy 增补。133 lib + 8 eval + 42 前端测试全绿（1 个已知无关 flaky）

### Git Commits

| Hash | Message |
|------|---------|
| `4075ae2` | (see git log) |

### Status

[OK] **Completed**


## Session 5: v2.0 切片 2：长文本提取任务草稿

**Date**: 2026-08-22
**Task**: v2.0 切片 2：长文本提取任务草稿
**Branch**: `feat-improve`

### Summary

交付记忆详情→AI 提取→草稿勾选→批量创建全链路：pending 幂等（不重复打 provider）、apply 门禁与来源引用/EntityLink、ambiguous 日期不猜测写入；ExtractSuggestionsPanel UI；新增 6 服务单测 + 在线评估用例；139 lib + 8 eval + 42 前端测试全绿

### Git Commits

| Hash | Message |
|------|---------|
| `55b177b` | (see git log) |

### Status

[OK] **Completed**


## Session 6: v2.0 切片 3：每周回顾 AI 摘要

**Date**: 2026-08-23
**Task**: v2.0 切片 3：每周回顾 AI 摘要
**Branch**: `feat-improve`

### Summary

交付 weekly_review AI 摘要：Summary prompt 开放（禁评价约束固化）、request_weekly_summary（确定性数字+仅标题上下文、重新生成置 dismissed）、完成回顾清理 pending、WeeklySummaryCard 三态 UI + 数字徽标对照；142 lib + 9 eval 全绿

### Git Commits

| Hash | Message |
|------|---------|
| `14ae648` | (see git log) |

### Status

[OK] **Completed**


## Session 7: v2.0 切片 4：任务相关内容建议

**Date**: 2026-08-23
**Task**: v2.0 切片 4：任务相关内容建议
**Branch**: `feat-improve`

### Summary

交付任务→记忆/剪贴板相关推荐：CJK bigram 候选检索 + 三重过滤 + 模型标题精确回配防编造；逐条 关联（幂等 related 链接）/不相关（拒绝降频）UI；146 lib + 10 eval + 42 前端测试全绿

### Git Commits

| Hash | Message |
|------|---------|
| `7f3627f` | (see git log) |

### Status

[OK] **Completed**


## Session 8: v2.0 切片 5：今日工作建议

**Date**: 2026-08-23
**Task**: v2.0 切片 5：今日工作建议
**Branch**: `feat-improve`

### Summary

交付今日页 AI 工作建议：确定性候选池（排除重点/等待/跳过配对）+ 特征引用理由 + 回配防编造；加入重点走既有撤销链路；跨天自动收口；顺手修复 reject 配对 feature 硬编码 bug；150 lib + 11 eval + 42 前端全绿

### Git Commits

| Hash | Message |
|------|---------|
| `18f7ba8` | (see git log) |

### Status

[OK] **Completed**


## Session 9: v2.0 切片 6：任务检查项数据模型

**Date**: 2026-08-23
**Task**: v2.0 切片 6：任务检查项数据模型
**Branch**: `feat-improve`

### Summary

交付一层检查项模型：0020 迁移、TaskService 五方法（冻结/级联/归一化/上限）、检查项并入任务搜索索引、ChecklistSection UI + TaskRow 进度徽标；为切片 7 AI 任务拆分铺路；153 lib + 11 eval + 42 前端全绿

### Git Commits

| Hash | Message |
|------|---------|
| `af39963` | (see git log) |

### Status

[OK] **Completed**


## Session 10: v2.0 切片 7：AI 任务拆分

**Date**: 2026-08-23
**Task**: v2.0 切片 7：AI 任务拆分
**Branch**: `feat-improve`

### Summary

交付任务→检查项 AI 拆分：依据子串防编造校验、薄任务/完成冻结短路、apply 零任务字段写入；复用切片 6 检查项模型；156 lib + 12 eval + 42 前端全绿

### Git Commits

| Hash | Message |
|------|---------|
| `c55581e` | (see git log) |

### Status

[OK] **Completed**


## Session 11: v2.0 切片 8 语义检索 + 全路线图收官

**Date**: 2026-08-24
**Task**: v2.0 切片 8 语义检索 + 全路线图收官
**Branch**: `feat-improve`

### Summary

交付语义检索（0021 向量索引/embed 批量/排除范围与上限/双列展示/索引管理）并归档全部 8/8 切片与父任务；v2-ai-assist-roadmap 标记已全部交付；164 lib + 13 eval + 42 前端全绿

### Git Commits

| Hash | Message |
|------|---------|
| `83f75aa` | (see git log) |

### Status

[OK] **Completed**


## Session 12: v2.0.0 发版：PR 合并、CI 修复与 Release 流水线

**Date**: 2026-08-24
**Task**: v2.0.0 发版：PR 合并、CI 修复与 Release 流水线
**Branch**: `feat-improve`

### Summary

修复历史 reminder_stats flaky（相对日期化）后 PR #11/#12 CI 双绿合并；scripts/release.sh major 发布 v2.0.0（schema v21），macOS universal + Windows 安装包与更新器 latest.json 全部出齐，Release notes 已挂载

### Git Commits

| Hash | Message |
|------|---------|
| `98c67df` | (see git log) |

### Status

[OK] **Completed**


## Session 13: 任务支持多层嵌套子任务（v2.1 切片）

**Date**: 2026-08-31
**Task**: 任务支持多层嵌套子任务（v2.1 切片）
**Branch**: `allan852/feat-sub-task`

### Summary

为任务实现多层嵌套子任务：parent_id 邻接表 + child_order 兄弟排序、完成聚合（向上自动完成/向下级联，双向，两个全局开关）、删除处置确认（级联/提升）、归档级联、树形列表 + 拖拽归巢、详情面板子任务区、展开状态数据库持久化、设置页开关。

### Main Changes

- 后端：迁移 0022（tasks.parent_id/child_order + task_tree_expanded 表）、TaskService 聚合/归巢/树查询/删除处置/展开状态、6 个新 IPC 命令、AppSettings 两个开关
- 前端：TasksPage 树形渲染 + dnd 归巢与兄弟重排、taskTree.ts 纯函数、TaskDetailPanel SubtaskSection、SettingsPage 子任务区块、TaskRow 深度/chevron/进度徽标
- 测试：12 个后端子任务单测 + 4 个前端树纯函数单测；cargo test --lib 179 通过、typecheck/build/test:unit 通过

### Git Commits

| Hash | Message |
|------|---------|
| `0d71881` | (see git log) |
| `467f184` | (see git log) |
| `bec7517` | (see git log) |

### Testing

- [OK] cargo test --lib: 179 passed
- [OK] pnpm typecheck + build + test:unit: 通过（46 tests）

### Status

[OK] **Completed**

### Next Steps

- 可选：每任务覆盖完成策略、级联改动触发自动化规则、CSV 导入导出 parent 列


## Session 14: 任务分组收尾：AC 核对全绿 + spec 回写 + PR #18

**Date**: 2026-10-26
**Task**: 任务分组收尾：AC 核对全绿 + spec 回写 + PR #18
**Branch**: `feat/task-list-groups`

### Summary

复核 10 条 AC（cargo test 193+13/typecheck/test:unit 53 全绿）；spec 回写 tauri 三态更新语义坑位、frontend state-management 落地约定；推送并确认 PR #18 OPEN

### Git Commits

| Hash | Message |
|------|---------|
| `c985ed0` | (see git log) |

### Status

[OK] **Completed**


## Session 15: 任务看板实现：迁移0024+/board三列拖拽+审查修复

**Date**: 2026-10-26
**Task**: 任务看板实现：迁移0024+/board三列拖拽+审查修复
**Branch**: `feat/task-kanban-board`

### Summary

独立/board看板页落地（阶段列迁移0024、三向拖拽、分组chips、乐观更新）；审查代理3条发现全修复（分流status守卫/UPDATE竞态守卫/15s refetch）；最终门禁 cargo 197+13、typecheck、59 unit 全绿；PR #20

### Git Commits

| Hash | Message |
|------|---------|
| `c334641` | (see git log) |

### Status

[OK] **Completed**


## Session 16: 任务看板：列内新建 + v2.2.0 发布

**Date**: 2026-10-26
**Task**: 任务看板：列内新建 + v2.2.0 发布
**Branch**: `chore/archive-task-kanban-board`

### Summary

看板列内新建（composer+作用域默认清单，61 unit 绿）并入 PR #20；合并后走 chore PR #21 出 v2.2.0（minor），tag 已推、release 工作流构建中；任务归档

### Git Commits

| Hash | Message |
|------|---------|
| `587ed78` | (see git log) |

### Status

[OK] **Completed**


## Session 17: 看板拖拽 DragOverlay 修复 + 587ed78 补合（PR #22）

**Date**: 2026-10-26
**Task**: 看板拖拽 DragOverlay 修复 + 587ed78 补合（PR #22）
**Branch**: `chore/archive-task-kanban-board`

### Summary

用户实测拖拽无跟随/被容器裁剪→改 DragOverlay 模式；发现 PR #20 合并未含 587ed78（列内新建）→ cherry-pick 补合为 PR #22 已合并；v2.2.0 缺该功能，待出 v2.2.1

### Git Commits

| Hash | Message |
|------|---------|
| `bbc0a86` | (see git log) |

### Status

[OK] **Completed**


## Session 18: 周回顾处理模式：四队列单条出牌（PR #30）

**Date**: 2026-10-28
**Task**: 周回顾处理模式：四队列单条出牌（PR #30）
**Branch**: `main`

### Summary

纯前端处理模式：processingSummary 纯函数 + vitest、ProcessingSession 出牌卡（四键+快捷键+Esc、出牌前 status 校验）、页面接入与失效沿用；typecheck/test:unit 全绿；PR #30 已合并

### Git Commits

| Hash | Message |
|------|---------|
| `91f14ec` | (see git log) |

### Status

[OK] **Completed**


## Session 19: 核心流 e2e 冒烟：tauri-driver 10 条用例 CI 全绿（PR #31）

**Date**: 2026-10-29
**Task**: 核心流 e2e 冒烟：tauri-driver 10 条用例 CI 全绿（PR #31）
**Branch**: `main`

### Summary

6 轮 CI 迭代修通 Linux e2e：xvfb（无显示）→ dbus-run-session（AT-SPI 握手）→ custom-protocol（内嵌前端）→ 定位竞态/键入落点/ onBlur 提交 → 页内驻留监听抓编辑框 + reconnect 同目录。事件管线故障期以 workflow_dispatch API 直发验证。最终 run 40753626139：10 passed / 41.3s；lib 199、unit 63 不受影响。spec 新增 tauri/e2e-webdriver.md

### Git Commits

| Hash | Message |
|------|---------|
| `5783bb5` | (see git log) |

### Status

[OK] **Completed**


## Session 20: 启用看板拖拽 e2e（case05/06）——W3C Actions 真实指针（PR #36）

**Date**: 2026-10-29
**Task**: 启用看板拖拽 e2e（case05/06）——W3C Actions 真实指针（PR #36）
**Branch**: `main`

### Summary

case04 实证的 W3C Actions 达成归档任务遗留启用条件：native_drag（Down 后 +14px 越 8px 激活阈值 + duration 插值）+ 看板定位器；case05 跨列、case06 同列排序（页内 JS 读序断言）解除 ignore。CI run 40815077128：10 passed / 48.8s 全绿，仅 case09 深链维持 ignore（待打包注册 scheme）。任务归档

### Git Commits

| Hash | Message |
|------|---------|
| `2a7706e` | (see git log) |

### Status

[OK] **Completed**


## Session 21: 启用 case09 URL Scheme 直达——argv 转发路径（PR #38）

**Date**: 2026-10-29
**Task**: 启用 case09 URL Scheme 直达——argv 转发路径（PR #38）
**Branch**: `main`

### Summary

推翻 ignore 理由：Linux 上 trove:// 经 single-instance argv 转发（handle_trove_url）无需打包注册。case09：收件箱激活 → 二实例 argv trove://today → 今日激活态回归（NavLink bg-row-active）。CI run 40832537410：10 passed / 0 ignored / 50.4s——套件首次零 skip

### Git Commits

| Hash | Message |
|------|---------|
| `59c52b3` | (see git log) |

### Status

[OK] **Completed**


## Session 22: N1 Linux 官方发布通道——release matrix 加 ubuntu（PR #43）

**Date**: 2026-10-29
**Task**: N1 Linux 官方发布通道——release matrix 加 ubuntu（PR #43）
**Branch**: `main`

### Summary

roadmap N1：release publish matrix 加 ubuntu-latest（deps 与 e2e 同源 + libfuse2），产出 .deb/.AppImage 并入 updater latest.json；bundle targets=all 无需改配置。验证点在下一次 tag 发版（PRD 记录）。任务归档

### Git Commits

| Hash | Message |
|------|---------|
| `12b50bc` | (see git log) |

### Status

[OK] **Completed**


## Session 23: N2 前端测试覆盖补强——纯决策内核抽取 + 24 条 vitest（PR #45）

**Date**: 2026-10-29
**Task**: N2 前端测试覆盖补强——纯决策内核抽取 + 24 条 vitest（PR #45）
**Branch**: `main`

### Summary

roadmap N2：boardDrag.resolveBoardDrag（看板拖拽全分支决策）、cardAction.resolveCardAction（出牌前校验）、taskDetailSave（标签解析+载荷组装）三处纯函数抽取，组件只消费决策；vitest 63→87。行为保持由 e2e 兜底：PR CI run 40871464822 三 job 全绿（10 passed / 49.7s，直击改动面）。任务归档

### Git Commits

| Hash | Message |
|------|---------|
| `9240aa9` | (see git log) |

### Status

[OK] **Completed**


## Session 24: N3 统一搜索类型筛选——chips 透传 types（PR #47）

**Date**: 2026-10-29
**Task**: N3 统一搜索类型筛选——chips 透传 types（PR #47）
**Branch**: `main`

### Summary

roadmap N3：快捷搜索此前调 searchQuery 未传 types（协议已有，前端未接）。加筛选 chips（全部/任务/提醒/记忆/剪切板）透传后端，语义匹配按类型客户端过滤，切换重置键盘选中。roadmap N1/N2/N3 全部勾销。typecheck/build/unit 87 绿；PR CI 三 job 全绿。任务归档

### Git Commits

| Hash | Message |
|------|---------|
| `809938f` | (see git log) |

### Status

[OK] **Completed**
