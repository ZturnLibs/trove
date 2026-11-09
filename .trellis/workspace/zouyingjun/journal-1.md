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


## Session 25: v2.4.0 发版——Linux 官方产物首秀（N1 验证通过）

**Date**: 2026-10-29
**Task**: v2.4.0 发版——Linux 官方产物首秀（N1 验证通过）
**Branch**: `main`

### Summary

发版执行：notes（PR #49）+ 版本 bump（PR #50）+ tag v2.4.0。release workflow 4 job 全绿——publish(ubuntu-latest) 首跑即绿，N1 验证通过：Release 资产含 .deb/.AppImage/.rpm（+sig），latest.json 11 平台条目（linux-x86_64×4）。发布体已替换为手写 notes

### Git Commits

| Hash | Message |
|------|---------|
| `1a5d068` | (see git log) |

### Status

[OK] **Completed**


## Session 26: e2e connect 带重试——瞬态连接错误修复（PR #53）+ CI actions Node 24（PR #52）

**Date**: 2026-10-29
**Task**: e2e connect 带重试——瞬态连接错误修复（PR #53）+ CI actions Node 24（PR #52）
**Branch**: `main`

### Summary

维护轮：①CI actions 升 Node 24 大版本（checkout v7/setup-node v7/pnpm-setup v6/upload-artifact v7），deprecation 告警清零（PR #52）；②main run 40888705017 case06 抖动实证 acquire 首连瞬态 hyper SendRequest 即判死 → connect_retry（3 次/250ms）覆盖 acquire+reconnect（PR #53），分支 CI 三 job 全绿

### Git Commits

| Hash | Message |
|------|---------|
| `08f8d89` | (see git log) |

### Status

[OK] **Completed**


## Session 27: 修复剪切板图片不显示——缩略图双重 data URL 前缀（存量缺陷）

**Date**: 2026-11-02
**Task**: 修复剪切板图片不显示——缩略图双重 data URL 前缀（存量缺陷）
**Branch**: `main`

### Summary

用户报告剪切板图片显示不了。定位：后端 thumb_base64 返回带 data:image/png;base64, 前缀的 data URL，而前端全部 6 处消费点统一自行拼同款前缀——双重前缀使 data URL 无效，图片从未渲染成功（v1.2 起存量，全 app 面：剪贴板/统一搜索/附件区/专注关联）。修复后端单点改回裸 base64 + 回归测试（载荷无前缀且解码为 PNG）。lib 200 绿

### Git Commits

| Hash | Message |
|------|---------|
| `ed8dfba` | (see git log) |

### Status

[OK] **Completed**


## Session 28: v2.4.1 发版——剪切板图片显示修复上线

**Date**: 2026-11-02
**Task**: v2.4.1 发版——剪切板图片显示修复上线
**Branch**: `main`

### Summary

发版执行：notes（PR #57）+ 版本 bump（PR #58）+ tag v2.4.1。release workflow 4 job 全绿（含 Linux 第二次验证）；资产全平台 14 件；发布体已替换为手写 notes。上游含 PR #55 剪切板缩略图修复、#53 e2e connect 重试、#52 CI actions Node 24

### Git Commits

| Hash | Message |
|------|---------|
| `532f479` | (see git log) |

### Status

[OK] **Completed**


## Session 29: 修复 macOS 升级后白屏——重启改经 LaunchServices（PR #60）

**Date**: 2026-11-02
**Task**: 修复 macOS 升级后白屏——重启改经 LaunchServices（PR #60）
**Branch**: `main`

### Summary

用户报告 v2.4.1 升级后启动白屏。取证链：bundle 前端 chunk 两版本一致（排除前端包差异）→ 统一日志实证 updater relaunch 实例 launchedByLS=0 且 WebContent 13s 挂起（non visible）→ 代码级根因 tauri process::restart restart_macos_app 直 spawn 二进制绕过 LaunchServices。修复 app_relaunch 命令（延时 400ms 经 LS open + 旧实例退出），updater 改调；机制本机实证（新实例 launchedByLS=1 foreground=1 渲染正常，orca 截图）。lib 200/typecheck/unit 87/build 绿

### Git Commits

| Hash | Message |
|------|---------|
| `7257ecf` | (see git log) |

### Status

[OK] **Completed**


## Session 30: v2.4.2 发版——macOS 升级白屏修复上线

**Date**: 2026-11-02
**Task**: v2.4.2 发版——macOS 升级白屏修复上线
**Branch**: `main`

### Summary

发版执行：notes（PR #62）+ 版本 bump（PR #63）+ tag v2.4.2。release workflow 4 job 全绿（Linux 第三次验证）；资产全平台 14 件；发布体已替换为手写 notes。上游含 PR #60 macOS 升级白屏修复（LaunchServices 重启）

### Git Commits

| Hash | Message |
|------|---------|
| `d654e49` | (see git log) |

### Status

[OK] **Completed**


## Session 31: 今日页快速添加确认键可配置（Enter / ⌘Ctrl+Enter）

**Date**: 2026-11-02
**Task**: 今日页快速添加确认键可配置（Enter / ⌘Ctrl+Enter）
**Branch**: `main`

### Summary

需求实现：AppSettings.quick_add_submit_key（enter|modEnter，默认 enter，JSON 默认值兼容无迁移）；TodayPage composer 按 shouldSubmitQuickAdd 纯函数判定（modEnter 模式裸 Enter 不动作），placeholder 动态提示（Enter 添加 / ⌘↩ 添加 / Ctrl+↩ 添加）；设置页工作节奏区块单选。vitest +8（87→95）、rust 默认值断言补齐（200 绿）。PR CI 三 job 全绿。任务归档

### Git Commits

| Hash | Message |
|------|---------|
| `df843dc` | (see git log) |

### Status

[OK] **Completed**


## Session 32: v2.5.0 发版——快速添加确认键可配置上线

**Date**: 2026-11-02
**Task**: v2.5.0 发版——快速添加确认键可配置上线
**Branch**: `main`

### Summary

发版执行：notes（PR #67）+ 版本 bump（PR #68）+ tag v2.5.0。release workflow 4 job 全绿；资产全平台 14 件；发布体已替换为手写 notes。上游含 PR #65 今日页快速添加确认键可配置（Enter / ⌘Ctrl+Enter + 设置项 + placeholder 提示）

### Git Commits

| Hash | Message |
|------|---------|
| `e887e61` | (see git log) |

### Status

[OK] **Completed**


## Session 33: 设置页左侧分区导航——快速跳转 + scroll-spy（PR #70）

**Date**: 2026-11-02
**Task**: 设置页左侧分区导航——快速跳转 + scroll-spy（PR #70）
**Branch**: `main`

### Summary

设置页 13 分区改两栏：左 sticky 导航（点击 smooth 跳转）+ 右内容自持滚动；scroll-spy 高亮当前分区（滚底强制末项）；AutomationRules/AIAssist 组件区块包 div 挂 settings-* 锚点。orca 截图实证交互（13 项全列/初始高亮/点击跳转+高亮同步）。typecheck/unit 95/build 绿，PR CI 三 job 全绿

### Git Commits

| Hash | Message |
|------|---------|
| `083c50c` | (see git log) |

### Status

[OK] **Completed**


## Session 34: 主窗口初始尺寸不小于屏幕工作区三分之一（PR #72）

**Date**: 2026-11-02
**Task**: 主窗口初始尺寸不小于屏幕工作区三分之一（PR #72）
**Branch**: `main`

### Summary

首次启动（无 .window-state.json）逐维取 max(工作区/3, 1080×720×scale)，物理像素比较；已有持久化状态仍由 window-state 插件恢复。first_launch_size 纯函数 + 4 组边界测试（小屏/HIDPI/超宽/混合），rust 204 绿；PR CI 三 job 全绿。任务完成

### Git Commits

| Hash | Message |
|------|---------|
| `35d7fd6` | (see git log) |

### Status

[OK] **Completed**


## Session 35: UI 原语替换——原生表单控件全量迁移 design-system（PR #74）

**Date**: 2026-11-03
**Task**: UI 原语替换——原生表单控件全量迁移 design-system（PR #74）
**Branch**: `main`

### Summary

新增 Select/Checkbox/Radio/Textarea 四个原语（沿 Input 惯例：forwardRef+cn 基类+用法覆盖），26 文件原生控件全量替换（select 19/checkbox/radio/textarea/裸 input 统一品牌样式与焦点环；file 隐藏输入保留原生；受控与 ref 透传不变）。typecheck/unit 95/build 绿；e2e 三 job 全绿兜底运行时；orca 截图抽查设置页/记忆页无回归

### Git Commits

| Hash | Message |
|------|---------|
| `7d16941` | (see git log) |

### Status

[OK] **Completed**


## Session 36: 看板列内新建确认键与设置同步（PR #79）

**Date**: 2026-11-05
**Task**: 看板列内新建确认键与设置同步（PR #79）
**Branch**: `main`

### Summary

BoardQuickAdd 接收 submitMode（读 quick_add_submit_key，默认 enter）：shouldSubmitQuickAdd 判定提交（modEnter 模式裸 Enter 不动作、Esc 关闭不变）、placeholder 提交键标签按模式与平台显示（标题后 ⌘↩ 创建…）。quickAddSubmit.ts 泛化：submitComboLabel + hint 动词参数。vitest 95→97。e2e 抓到定位器锚定旧 placeholder 文案回归 → 解耦为「标题后」前缀（run 41980493600 三 job 全绿）。任务归档

### Git Commits

| Hash | Message |
|------|---------|
| `5096d37` | (see git log) |

### Status

[OK] **Completed**


## Session 37: v2.6.0 发版——看板列内新建确认键与设置同步上线

**Date**: 2026-11-05
**Task**: v2.6.0 发版——看板列内新建确认键与设置同步上线
**Branch**: `main`

### Summary

发版执行：notes（PR #81）+ 版本 bump（PR #82）+ tag v2.6.0。release workflow 4 job 全绿；资产全平台 14 件；发布体已替换为手写 notes。上游含 PR #79 看板列内新建确认键与设置同步（Enter / ⌘Ctrl+Enter + placeholder 提示）

### Git Commits

| Hash | Message |
|------|---------|
| `eabec13` | (see git log) |

### Status

[OK] **Completed**


## Session 38: 看板拖拽落卡回位动画去除（PR #84）

**Date**: 2026-11-05
**Task**: 看板拖拽落卡回位动画去除（PR #84）
**Branch**: `main`

### Summary

用户报告松手后卡片「跑回」原位再进入新列。动画来源两处：DragOverlay 默认落位动画（浮层滑回拖拽起点，keyframes initial→final）+ 排序容器 transform 回位过渡。修复：dropAnimation={null}（浮层松手处即逝）+ 移除过渡；真实卡片由 stage 乐观补丁即时进新列。orca 合成拖拽无法驱动 macOS WKWebView 的 dnd-kit（驱动器限制）；CI WebKitGTK W3C Actions 拖拽用例全绿兜底

### Git Commits

| Hash | Message |
|------|---------|
| `908cb15` | (see git log) |

### Status

[OK] **Completed**


## Session 39: 任务附件修复——资产 id 纠偏 + 多选批量附加（PR #89）

**Date**: 2026-11-09
**Task**: 任务附件修复——资产 id 纠偏 + 多选批量附加（PR #89）
**Branch**: `main`

### Summary

用户实证：附加图片选中后弹窗关闭但附件不显示。根因：ImagePicker onPick(item.id) 传剪贴板条目 id 当 targetId（应为资产 id）→ 幽灵链接被 entity_link_assets 的 assets.get 跳过。修复 id 来源 + 多选批量附加（勾选角标/计数/已链接隐藏）+ 后端 entity_link_create 资产存在性校验（防幽灵链接入库）+ 链路回归测试（真实资产→link→查询命中+缩略图解析）。rust 205/unit 97 绿；dispatch CI 三 job 全绿（10 passed/51.2s）。附带：e2e job 固定 ubuntu-24.04（webkit2gtk-driver 在 26.04 档案已移除）；ci.yml workflow_dispatch 通道

### Git Commits

| Hash | Message |
|------|---------|
| `b15c79e` | (see git log) |

### Status

[OK] **Completed**
