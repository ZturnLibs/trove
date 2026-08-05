# Research: 功能差距审计（Feature Gap Audit）

- **Query**: 对 Trove（Tauri 2 + React + Rust/SQLite 本地优先个人工作台）做功能差距审计，产出分优先级的「待完善/缺失功能」清单
- **Scope**: internal（代码勘察 + 文档/归档 PRD 对照）
- **Date**: 2026-08-05
- **产出**: 本文件（gap-audit.md）

## 审计方法与基线

- 逐模块对比：`docs/README.md`、`docs/development-roadmap.md`、`docs/post-v1-iteration-design.md`、`docs/ui-layout-interaction.md`、`docs/keyboard-shortcuts.md` 中的产品承诺 vs 实际实现。
- 全量搜索 TODO/FIXME/XXX/unimplemented/占位：**无实质待办标记**（命中均为 Todo 状态枚举与 placeholder 属性，无未实现注释）。
- IPC 对照：`src-tauri/src/commands/mod.rs`（70+ 命令） vs `src/ipc/client.ts` vs 前端实际调用点。
- 产品声明：README 声称 v1.0 ~ v1.2（含快捷操作/自然语言/智能列表/模板/片段/图片剪切板/OCR/关联体系）已完成。

### 证据基线（关键命令是否被 UI 调用）

| 命令 | Rust | client.ts | UI 调用 |
|---|---|---|---|
| `reminder_update` | ✅ commands/mod.rs:268, lib.rs:526 | ❌ 无封装 | ❌ 无法编辑任何提醒 |
| `smoke_note_create/list/delete` | ✅ mod.rs:159-171 | ✅ client.ts:431-434 | ❌ 无任何 UI |
| `task_list_tags` | ✅ mod.rs:434 | ✅ client.ts:451 | ❌ 无调用 |
| `task_get` / `memory_get` / `clipboard_get` / `asset_read_thumb` | ✅ | ✅ | ❌ 无调用 |
| `template_preview` | ✅ mod.rs:488 | ✅ client.ts:466 | ❌ 无调用 |
| `backup_status` | ✅ mod.rs:932 | ✅ client.ts:520 | ❌ 无调用（健康页用 app_health） |
| `shortcuts_apply` | ✅ mod.rs:154 | ✅ client.ts:430 | ❌ 无调用 |

---

## 1. 任务（src/features/tasks/、InboxPage、src-tauri/src/application/tasks.rs）

### 现状
- 收件箱/今日/自定义清单/智能列表（明天/未来七天/逾期/高优先级/无日期/最近完成）均可用；优先级、截止日期/时间、标签（详情页逗号分隔输入）、周期任务（NL 解析 + 每天重复）、归档/删除、上下移排序、双击改名、延期、跳过均已有。
- 后端 `TaskQuery` 能力完整（listId/inboxOnly/status/priority/tagId/dueFrom/dueTo/dueNull/completedSince，tasks.rs:524-592）。

### 缺口
1. **[不完整] 标签只在详情面板内维护，无标签筛选/浏览入口** —— 后端 `task_list_tags`（commands/mod.rs:434）与 `TaskQuery.tagId`（tasks.rs:553-556）已就绪，但 TasksPage 只提供清单/状态/优先级筛选（TasksPage.tsx:146-183），`task_list_tags` 全代码库无调用。影响：多标签任务找回困难。
   - 证据：`src-tauri/src/commands/mod.rs:434-436`；`src/features/tasks/TasksPage.tsx:146-183`；`rg task_list_tags` 无 UI 命中。
   - 优先级 **P2**，类型：不完整。
2. **[缺失] 无子任务/检查项/任务分组（项目/看板/日历视图）** —— 数据库无任何 parent/subtask 概念（migrations/0002_tasks.sql 全表字段无），v1.0 范围未承诺，但常见任务应用的基础能力缺失。影响：大型任务无法拆分跟踪。
   - 证据：`src-tauri/migrations/0002_tasks.sql`（tasks 表仅 title/notes/status/priority/list_id/due_*）。
   - 优先级 **P3**，类型：缺失（明确超出 v1.x 主路线，见 post-v1-iteration-design.md §7 专注模式/检查项候选）。
3. **[不完整] 周期任务不可编辑** —— 创建后只能完成/跳过/延期（TaskDetailPanel.tsx:247-256），无修改 recurrence 或结束周期的 UI；`seriesId` 只是标识。影响：录错周期只能删了重建。
   - 证据：`src/design-system/patterns/TaskDetailPanel.tsx:247-256`；`UpdateTaskInput` 无 recurrence 字段（client.ts:143-152）。
   - 优先级 **P3**，类型：不完整。
4. **[不完整] 自定义清单无删除/重命名** —— 命令只有 `task_list_lists` / `task_list_create`（commands/mod.rs:174-181），无 list 删除/改名命令与 UI。影响：误建清单无法清理。
   - 证据：commands/mod.rs:174-181；`rg task_list_delete` 无命中。
   - 优先级 **P3**，类型：缺失。
5. **[不完整] 任务查询无分页/上限** —— `query_tasks` 返回全部匹配行（tasks.rs:524-592 无 LIMIT）；TasksPage 一次性渲染全部。影响：数千任务时列表性能与 IPC 载荷隐患。
   - 证据：tasks.rs:524-592；TasksPage.tsx:48-59。
   - 优先级 **P3**，类型：不完整（性能隐患）。
6. **[不完整] 无任务页内搜索** —— 全局搜索只在 QuickWindow（searchQuery），TasksPage 无标题搜索框。影响：任务页找任务需开全局面板。
   - 证据：TasksPage.tsx 无搜索控件；searchQuery 仅 QuickWindow.tsx:136 调用。
   - 优先级 **P3**，类型：不完整。

---

## 2. 提醒（application/reminders.rs、TodayPage 提醒区、scheduler.rs）

### 现状
- 任务提醒（TaskDetailPanel 内建）、独立提醒（今日页/快速记录/菜单）、贪睡 3 预设、完成、周期提醒、错过补报（reconcile_on_startup）、通知权限横幅、调度器 20s 轮询均可用。后端 `ReminderService.update`（reminders.rs:75-125）完整。

### 缺口
1. **[缺失] 提醒完全不可编辑（后端已就绪，前端未接）** —— `reminder_update` 命令存在并注册（commands/mod.rs:268-286, lib.rs:526），但 client.ts 无封装、UI 无调用。今日页提醒详情只有贪睡/完成（TodayPage.tsx:353-398），任务提醒区只有删除（TaskDetailPanel.tsx:373-379）。影响：录错时间/标题只能删掉重建（独立提醒甚至删不掉，见下条）。
   - 证据：`src-tauri/src/commands/mod.rs:268-286`；`src/ipc/client.ts` 无 reminderUpdate；`rg reminderUpdate` 无 UI 命中。
   - 优先级 **P1**，类型：缺失（IPC 已暴露未接 UI）。
2. **[缺失] 独立提醒无管理入口（不能删除/查看全部）** —— `reminder_delete` 仅被任务提醒区调用（TaskDetailPanel.tsx:327）；独立提醒创建后既不能编辑也不能删除，只能完成/贪睡；没有「全部提醒」列表，无法查看未来提醒。影响：误建独立提醒成为永久残留；无法规划。
   - 证据：TodayPage.tsx:113-128, 353-398；reminderDelete 调用仅 TaskDetailPanel.tsx:327。
   - 优先级 **P1**，类型：缺失。
3. **[不完整] 周期提醒 UI 仅「每天重复」** —— 前端所有提醒创建处只有 daily 复选框（TaskDetailPanel.tsx:309-315、QuickWindow.tsx:349-356、TodayPage.tsx:117-121），weekly/monthly/weekdays 仅 NL 解析可达。影响：无法在 UI 精确设置周期。
   - 证据：`src/design-system/patterns/TaskDetailPanel.tsx:309-315`。
   - 优先级 **P2**，类型：不完整。
4. **[不完整] 系统通知点击无落地** —— scheduler 发送通知后向主窗 emit `reminder://fired`（scheduler.rs:54-64），但前端无任何 listener（rg 仅 Rust 侧命中）。影响：点击通知不会定位到该提醒。
   - 证据：`src-tauri/src/application/scheduler.rs:54-64`；`rg "reminder://fired" src/` 无命中。
   - 优先级 **P2**，类型：不完整。
5. **[不完整] 通知正文为固定文案** —— 通知 body 只写「任务提醒到期/提醒到期」（scheduler.rs:28-32），不含 notes/时间上下文。影响：多提醒时难区分。
   - 证据：scheduler.rs:28-32。
   - 优先级 **P3**，类型：不完整。

---

## 3. 记忆（MemoryPage、application/memories.rs）

### 现状
- 标题/正文、置顶、标签、快速插入标记 + 触发词字段、转任务（保留原记忆 + converted_to 关联）、图片附件区、编辑/删除/复制、仅置顶筛选均可用。后端 `MemoryQuery` 支持 tagId/includeArchived/quickInsertOnly/pinnedOnly（memories.rs:140-175）。

### 缺口
1. **[不完整] Markdown 未真正渲染** —— 输入框提示「支持基础 Markdown 文本…」（MemoryPage.tsx:194），但预览只是 `whitespace-pre-wrap` + URL linkify（MemoryPage.tsx:17-37, 183-186），**无任何 Markdown 解析器**（package.json 无 markdown 依赖）。影响：功能与文案不符。
   - 证据：MemoryPage.tsx:17-37（linkify 仅处理 http 链接）、184-185；package.json dependencies 无 markdown 库。
   - 优先级 **P2**，类型：不完整。
2. **[不完整] 记忆页无搜索/标签筛选/归档视图** —— `MemoryQuery.tagId/includeArchived`（memories.rs:157-162）与全局搜索均可用，但 MemoryPage 只有 pinnedOnly 开关（MemoryPage.tsx:255-261），无搜索框、无标签筛选、无归档入口（`archived` 字段在 UI 完全不可达）。影响：记忆多时仅靠滚动查找；归档字段形同虚设。
   - 证据：MemoryPage.tsx:255-261；`MemoryQuery` 的 tagId/includeArchived 无 UI 传入。
   - 优先级 **P1**，类型：不完整。
3. **[缺失] 触发词（triggerWord）无实际展开/插入功能** —— 触发词可录入（MemoryPage.tsx:170-182）且快速插入记忆会被列出（QuickWindow.tsx:250-259），但**没有**「输入触发词自动展开」或直接粘贴的通道；`triggerWord` 在搜索索引中也不参与。v1.1 承诺「片段短名称/触发词 + 辅助功能直接粘贴」（post-v1-iteration-design.md §5.3 快速文本片段）未落地。
   - 证据：QuickWindow.tsx:249-259（只按 quickInsertOnly 列出，忽略 triggerWord）；search.rs:45（索引仅 title+body）。
   - 优先级 **P2**，类型：缺失。
4. **[不完整] 附件仅支持剪切板图片** —— 附件区只能从剪切板图片历史挑选（AttachmentsSection.tsx:167-169, 附加图片按钮）。文件引用/来源网址（v1.2 承诺，entitylink PRD 明确为「未来扩展」）未实现。
   - 证据：AttachmentsSection.tsx:85-88, 167-169。
   - 优先级 **P2**，类型：缺失（v1.2 后续切片，见排除项说明）。
5. **[不完整] 记忆查询无分页** —— `memory_query` 全量返回（memories.rs:140-175 无 LIMIT）。影响同任务模块。
   - 证据：memories.rs:140-175。
   - 优先级 **P3**，类型：不完整（性能隐患）。

---

## 4. 剪切板（ClipboardPage、application/clipboard.rs）

### 现状
- 文本/图片采集（800ms 轮询）、收藏、搜索（含 OCR 文本 LIKE）、再次复制（写回系统剪贴板）、转任务/转记忆、暂停/恢复、排除应用、保留天数/最大条数、清空非收藏、引用安全清理（enforce_limits + collect_garbage）、来源应用/使用次数展示均可用。图片有缩略图/尺寸/去重。

### 缺口
1. **[不完整] Windows/Linux 无 OCR** —— `recognize_png` 非 macOS 直接返回空文本（ocr.rs:27-34）。影响：Windows 用户图片剪切板搜索只能靠 content 占位（[图片]），v1.2 承诺的「OCR 文本找回截图」在 Windows 不可用。
   - 证据：`src-tauri/src/platform/ocr.rs:27-34`；release.yml 明确构建 Windows 包。
   - 优先级 **P2**，类型：缺失（平台相关）。
2. **[缺失] 直接粘贴（directPaste）为「幻影能力」** —— Settings/Clipboard 展示 directPaste 能力与降级横幅（platform/mod.rs:39-49, ClipboardPage.tsx:282-295），但全代码库**没有**任何实际执行直接粘贴的代码（无 osascript/输入模拟/自动化接口），`clipboard_copy` 只是 write_text/write_image（commands/mod.rs:770-796）。能力状态宣称 macOS/Windows 可用与实际不符。
   - 证据：platform/mod.rs:39-49；commands/mod.rs:770-796；`rg osascript|paste_ 无命中`。
   - 优先级 **P2**，类型：不完整（宣称与实现不符）。
3. **[不完整] 搜索筛选维度少** —— v1.2 搜索增强承诺「按类型、来源应用、时间范围、是否收藏筛选」（post-v1-iteration-design.md §6.3），当前仅 favoritesOnly + 关键词 + kind（clipboard.rs:360-412 与 ClipboardPage.tsx:218-226）。影响：大量历史时按来源/时间找条目困难。
   - 证据：ClipboardPage.tsx:218-226；ClipboardQuery 仅 4 字段（client.ts:374-379）。
   - 优先级 **P3**，类型：不完整。
4. **[缺失] 无截图快速收藏** —— v1.2 承诺「快捷键截取区域直接保存到剪切板/记忆」（post-v1-iteration-design.md §6.3），未实现（无截图命令/服务）。
   - 证据：无 screenshot 相关代码（rg 无命中）；docs 承诺见 post-v1-iteration-design.md §6.3。
   - 优先级 **P3**，类型：缺失（v1.2 未交付项）。
5. **[缺失] 存储占用管理** —— v1.2 承诺存储空间管理器（post-v1-iteration-design.md §6.4），无 UI 查看数据库/资产/缩略图占用；资产 GC 仅在 enforce_limits 内联触发。
   - 证据：assets.rs:189（collect_garbage 存在但无对外命令/UI）；无 storage 统计命令。
   - 优先级 **P3**，类型：缺失。

---

## 5. 全局能力（搜索/快速记录/托盘/快捷键/设置/备份）

### 现状
- 快捷窗口三模式（记录/搜索/剪切板）、托盘锚定弹层、全局快捷键（可配置 + 冲突检测 + 失败提示）、设置页（主题/开机启动/快捷键/模板/剪切板/备份导入导出/权限能力）、启动自动备份、迁移前自动备份、JSON 导出/导入（导入前自动备份）、备份列表/恢复、命令面板模板应用均可用。

### 缺口
1. **[缺失] smoke_notes（快速记录后端）全链路无人使用** —— `smoke_note_create/list/delete` 后端服务 + IPC + client 封装齐备（commands/mod.rs:159-171, app_state.rs 注入 SmokeNoteService），但前端无任何调用；快速记录实际落在 task/reminder/memory。影响：死代码 + 导出包含 smoke_notes 表但 UI 不可见。
   - 证据：`rg smoke src/` 仅 client.ts:431-434；data_port.rs:14 导出 smoke_notes 表。
   - 优先级 **P2**，类型：缺失/遗留（IPC 已暴露未接 UI）。
2. **[缺失] 无撤销/最近操作** —— v1.1 承诺「完成/延期/移动/归档/删除支持短时撤销 + RecentAction」（post-v1-iteration-design.md §5.3），全库无 RecentAction 概念。影响：误操作只能手工反操作。
   - 证据：`rg RecentAction|undo 无命中`（menu_bar.rs:224 的 undo 是系统文本编辑撤销）。
   - 优先级 **P2**，类型：缺失（v1.1 承诺未交付）。
3. **[缺失] 自定义视图（SavedView）未实现** —— v1.1 承诺「保存当前筛选为自定义视图」（post-v1-iteration-design.md §5.3），无 SavedView 表/命令/UI。影响：常用筛选需重复设置。
   - 证据：`rg SavedView 无命中`。
   - 优先级 **P2**，类型：缺失（v1.1 承诺未交付）。
4. **[缺失] 菜单栏今日面板未实现** —— v1.1 承诺托盘/菜单栏展示逾期/今日/下条提醒并可完成/延期（post-v1-iteration-design.md §5.3），当前托盘只有打开/快速记录/剪切板/暂停/设置/退出（lib.rs:65-83）。
   - 证据：lib.rs:65-83（tray menu items）。
   - 优先级 **P3**，类型：缺失（v1.1 承诺未交付）。
5. **[不完整] 命令面板命令与搜索结果未分区** —— v1.1 要求「分区展示」（post-v1-iteration-design.md §5.3），QuickWindow 把命令与命中合并为一个列表（QuickWindow.tsx:150-237）。影响轻微。
   - 证据：QuickWindow.tsx:167-237。
   - 优先级 **P3**，类型：不完整。
6. **[不完整] 模板：无编辑、创建入口仅 2 个硬编码示例、应用前无预览** —— `template_preview` 从未被调用（应用模板直接执行 templateApply，QuickWindow.tsx:216-226, SettingsPage.tsx:379-387）；模板创建 UI 只有「周报/报销」两个按钮（SettingsPage.tsx:319-364），无 reminder/memory 模板入口。v1.1 承诺「模板执行前展示结果」。
   - 证据：SettingsPage.tsx:319-364；QuickWindow.tsx:216-226；`rg templatePreview` 无 UI 命中。
   - 优先级 **P2**，类型：不完整。
7. **[不完整] 帮助菜单全部指向设置页** —— 「快捷键一览」「隐私与数据说明」菜单项都 navigate 到 /settings（menu_bar.rs:397-399），无独立帮助内容。影响：文档入口弱。
   - 证据：menu_bar.rs:397-399。
   - 优先级 **P3**，类型：不完整。

---

## 6. 工程 / 平台

### 现状
- 8 个顺序迁移 + 迁移前自动备份（db/mod.rs:6-21, 94-138）；WAL；release CI 构建 macOS universal + Windows；全局快捷键默认值按平台区分；窗口状态 clamp；单实例；托盘左键弹层锚定；启动备份失败横幅。Rust 单测 32 个。

### 缺口
1. **[不完整] About 对话框仅 macOS** —— About 只存在于 `#[cfg(target_os = "macos")]`（menu_bar.rs:304-334），Windows/Linux 无 About 入口（帮助菜单无「关于」）。
   - 证据：menu_bar.rs:304-334 vs 349-360（非 macOS 菜单无 About）。
   - 优先级 **P2**，类型：不完整。
2. **[不完整] 版本号未随迭代提升** —— package.json / Cargo.toml / tauri.conf.json 均为 0.0.1（release v0.0.1 后 v1.1/v1.2 均完成但未 bump）；app_health 显示 CARGO_PKG_VERSION=0.0.1。影响：用户/诊断无法区分版本，release CI 的 tag 校验将强制 tag 为 0.0.1。
   - 证据：package.json:3；src-tauri/Cargo.toml；src-tauri/tauri.conf.json "version":"0.0.1"；release.yml:22-38（tag 必须等于版本）。
   - 优先级 **P1**，类型：不完整（工程卫生/发布阻塞隐患）。
3. **[不完整] 备份恢复无可读性校验** —— restore 直接以 SQLite backup 覆盖当前库（backup.rs:170-185），未先验证备份文件可打开/可迁移；若备份损坏，会覆盖现有数据（有 pre-restore 备份兜底，但无显式校验与提示）。跨版本要求「迁移前自动备份，并验证备份可读取」（post-v1-iteration-design.md §10）。
   - 证据：backup.rs:170-185；db/mod.rs:126-138（仅迁移前快照，无可读校验）。
   - 优先级 **P1**，类型：不完整（数据安全）。
4. **[不完整] 时区硬编码 Asia/Shanghai** —— NL 解析固定 timezone="Asia/Shanghai"（nl_parse.rs:28），菜单新建提醒硬编码 "Asia/Shanghai"（menu_bar.rs:77），而 UI 其它路径用系统时区。影响：非上海时区用户周期任务/菜单提醒时间偏差。
   - 证据：`src-tauri/src/domain/nl_parse.rs:28`；`src-tauri/src/menu_bar.rs:77`。
   - 优先级 **P2**，类型：不完整（正确性）。
5. **[不完整] 前端测试覆盖几乎为零** —— 仅 1 个测试文件（src/lib/cn.test.ts），vitest 已配置且 CI 跑 `pnpm test:unit`。Rust 32 个测试未覆盖 commands/调度链路。
   - 证据：`find src -name "*.test.*"` → 仅 cn.test.ts；package.json scripts.test:unit。
   - 优先级 **P2**，类型：不完整。
6. **[不完整] 通知/剪切板等关键链路无自动化回归测试** —— 跨版本要求「数据迁移、提醒和自动化规则优先自动化回归测试」（post-v1-iteration-design.md §10），当前 reminders 仅有 2 个 service 测试、无 scheduler 测试。
   - 证据：reminders.rs:566-651（2 测试）；scheduler.rs 无测试。
   - 优先级 **P3**，类型：不完整。

---

## 7. Top 5 优先建议

1. **提醒管理闭环（P1）**：前端接入已有 `reminder_update`（commands/mod.rs:268）+ 增加独立提醒的编辑/删除/「全部提醒」列表（今日页 + 设置或任务页）。理由：后端 100% 就绪、纯前端工作量；当前无法修正录错的提醒时间、独立提醒无法删除是核心体验硬伤；一次修复 2 条 P1 缺口。
2. **任务标签筛选/浏览（P2）**：在 TasksPage 用已有 `task_list_tags` + `TaskQuery.tagId` 增加标签筛选与标签入口（含智能列表同等待遇）。理由：后端就绪、改动小；显著降低多标签任务的找回成本（核心指标）。
3. **记忆页搜索/标签/归档视图（P1）**：MemoryPage 增加搜索框（复用 search_query 或记忆 LIKE）、标签筛选（memory_query tagId）、归档视图（includeArchived + 详情面板归档按钮）。理由：后端就绪、纯前端；`archived` 字段当前完全不可达，是「有数据无 UI」的典型。
4. **版本号维护 + 备份恢复校验（P1）**：将版本 bump 到与迭代同步（0.1.0→…或 1.2.x），restore 前验证备份可打开/校验和并给出明确错误；发布 CI 校验自然生效。理由：数据安全（恢复损坏备份有覆盖风险）+ 发布卫生，成本低。
5. **模板能力补齐（P2）**：应用模板前调用已有 `template_preview` 展示结果并确认；把设置页模板区从 2 个硬编码示例扩展为任务/提醒/记忆通用创建入口（后端 template_create 已支持三类型）。理由：后端就绪、前端工作量可控；兑现 v1.1「模板执行前展示结果」承诺，直接提升模板可用性。

---

## 8. 排除项（明确超出定位，不在建议范围）

- 移动端 App 与云同步/账号体系（post-v1-iteration-design.md §11 条件候选）。
- 日历集成（§11 候选；roadmap 明确「日历规划不进入近期主路线」）。
- 团队协作/绩效/甘特/完整邮件日历（§13 长期边界）。
- 文件引用、浏览器捕获、来源网址、截图快速收藏、存储空间管理器（v1.2 承诺但需新平台能力/插件，列为 P3 或独立立项；entitylink 归档 PRD 明确为「未来扩展」）。
- v1.3（今日重点/推迟显示/等待事项/专注模式/每日收尾/每周回顾）与 v1.4（URL Scheme/CLI/快捷指令/自动化规则/CSV 导入）整块为后续版本规划，本审计仅记录为远期，不列入 Top 5。

## Caveats

- 「直接粘贴为幻影能力」与「About 仅 macOS」基于当前代码静态勘察；若存在未提交的运行时注入或外部脚本，需以实际构建产物复核。
- `reminder_update`、`smoke_note_*`、`template_preview` 等「已暴露未接」判定均通过 `rg` 全库搜索确认无调用；建议实现阶段再跑一次 `rg` 复核。
- 所有性能项（无分页）为隐患级判断，未做基准测试。
