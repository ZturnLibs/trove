# PRD:完善 trove-cli — Agent 任务管理能力与配套 Skill

## Goal

让外部 Agent(编码助手、脚本、快捷指令)通过 `trove-cli` 完成**闭环**任务管理——以"钉钉聊天记录 → 待办任务"为核心场景:不仅能创建任务,还能查询、修正、回收任务,并配套使用文档与 Agent Skill,使任何 Agent 平台可即取即用。

## Background(已确认事实)

- CLI 走统一动作层:`trove-cli` → `trove-action:{json}` argv → 单实例转发至运行中的应用(`src-tauri/src/lib.rs:510`)→ `cli_protocol::handle_cli_dispatch` 执行 → 临时文件写回 `ActionOutcome`。CLI 不直接读写 SQLite;应用未运行时 CLI 会启动/唤醒应用(workbench-action.md 错误矩阵)。
- 动作枚举 `WorkbenchAction`(`src-tauri/src/domain/workbench_action.rs:37`)现有:Navigate / OpenSearch / CreatePreview / CreateTask / CreateReminder / CreateMemory / CompleteTask / QueryToday / QueryOverdue / QueryInbox / QueryList / SearchMemories / GetSnippets / ExportTasksCsv。serde tag 为 `action`,`ActionOutcome` tag 为 `outcome`(workbench-action.md 契约 2)。
- 写入动作需 `confirmed: true` 且 `requires_confirmation()` / `is_confirmed()` 的 match 臂需同步扩展(workbench_action.rs:253-273);`dry_run: true` 只返回 `DryRun { description }`。
- 应用侧 `TaskService`(`src-tauri/src/application/tasks.rs`)已具备全部后端能力:`query_tasks(TaskQuery)`(:1371,支持 status/list_id/priority/搜索/due 区间/parent_id/limit+offset)、`get_task`(:1351)、`update_task(UpdateTaskInput)`(:403,要求完整字段)、`uncomplete_task`(:674)、`archive_task`(:746)、`delete_task_tree(disposition)`(:815)、`postpone_task(days)`(:1534)、`set_task_defer`(:1561)、`list_lists`(:61)、`count_list_todo_tasks`(:132)。
- `CreateTaskInput`(domain/task.rs:277)已支持 title / notes / priority / list_id / due_date / due_time / tag_names / parent_id;`TaskQuery`(domain/task.rs:303)字段齐备且已派生 serde;CLI 当前 `create` 只暴露 title / notes / due_date(bin/trove-cli.rs:33-42)。
- `ActionOutcome` 已有 `TaskCreated { task: Task }` / `TasksQueried { items, total }` 等变体;`Task`(domain/task.rs:246)整体可序列化,含 tag_names、parent_id、workflow_state。
- 官方安装包内含 `trove-cli`(`Trove.app/Contents/MacOS/trove-cli`);`--app PATH` / `TROVE_APP` 可指向指定实例(正式版 `com.trove.app` vs 开发版 `com.trove.dev` 数据隔离)。
- 项目 skill 约定:`.opencode/skills/<name>/SKILL.md`(带 name/description front-matter,同 trellis-* 技能)。

## Requirements

- R1 通用查询:`query tasks` 支持 `--status / --list / --search / --priority / --due-from / --due-to / --parent / --limit / --offset`,复用 `TaskQuery` 直传,输出 `TasksQueried` JSON
- R2 详情:`show <task-id>` 返回完整 `Task` JSON(notes 全文、tags、父子关系)
- R3 创建增强:`create task` 增加 `--list <名称或uuid> / --priority / --tag <名…> / --parent <id> / --due-time`;清单支持按名称引用
- R4 批量创建:`create-batch` 从 NDJSON 文件或 stdin 一次导入多条任务,逐条返回结果,单条失败不影响其余(上限 100 条/次)
- R5 修改:`update <id>` 支持部分字段(--title/--notes/--due-date/--due-time/--priority/--list/--tag),应用内读-改-写合并;清空截止日需显式 `--clear-due`(规范"清空可选关系需显式标志"坑则)
- R6 生命周期:`reopen <id>`、`archive <id>`、`delete <id> --children delete|archive|promote`、`postpone <id> --days N`、`defer <id> [--at DATE | --clear]`;全部需 `--yes`
- R7 清单发现:`query lists` 输出清单 id/name/kind/openCount(含收件箱),供 Agent 按名解析 uuid
- R8 文档:重写 `docs/cli.md`(Agent 使用章节、钉钉批量示例、Raw 动作 wire 示例);`docs/action-layer.md` 动作表补新行
- R9 Skill:新增 `.opencode/skills/trove-cli/SKILL.md`(命令表、recipe、`--json` outcome 解析约定、`--yes`/`--dry-run` 安全约定、`TROVE_APP` 多实例)

## Key Decisions

- D1 范围 = B「完整管理」(用户 2026-11-12 选定):R1–R9 全做;checklist 增删勾选与子任务树查询(query_tree)不做,留待后续增量
- D2 批量导入在 **CLI 侧循环**现有 `CreateTask` 动作实现,不新增应用侧批量动作——零协议变更、天然逐条隔离失败;代价是 N 次进程间转发(应用运行中,单次转发毫秒级,100 条内可接受)
- D3 清单名解析在 **CLI 侧**完成:先 `QueryLists` 拿清单表,精确匹配名称→uuid;不修改 `CreateTaskInput`(UI 路径零影响);歧义时报错并列出候选
- D4 `UpdateTask` 动作在应用 dispatch 内读-改-写(get_task → 合并 → update_task),`UpdateTaskInput` 要求完整字段的应用层契约不变;清空语义用 `clear_due: bool` 显式标志、空字符串清空 notes
- D5 headless(无应用直连 SQLite)不做:重复任务/提醒/搜索索引/备份逻辑都在应用层,绕过会双写风险;架构约束维持"CLI 不碰 SQLite"
- D6 全部为**增量**变更:不改既有变体签名、不动 `Raw`、不影响 UI/URL 路径

## Acceptance Criteria

- AC1 每个新子命令:`--dry-run --json` 返回可解析描述;实际执行 `--json` 返回 outcome(创建/更新/重开等返回完整 `Task`,含 uuid)
- AC2 `create-batch` 单次调用导入多条,部分失败逐条报错且不影响已成功条目;超过 100 条拒绝执行
- AC3 `update` 只传一个字段时其余字段保持不变(读-改-写回归测试)
- AC4 `delete` 必须显式 `--children`;所有写命令无 `--yes` 不落库(确认门测试)
- AC5 `reopen` 对已完成任务幂等,`postpone` 仅对待办生效(应用层既有行为,CLI 不破坏)
- AC6 清单按名称解析:精确匹配成功;重名/不存在时报错并列出候选清单
- AC7 既有命令行为不变;`Raw` 继续可用;`cargo test --lib` 全绿
- AC8 文档与 Skill 覆盖全部新命令,钉钉示例可复制即用

## Out of Scope

- headless 模式(CLI 直连 SQLite)——见 D5
- checklist 增删勾选、子任务树查询(`query_tree`)、清单/分组 CRUD、标签管理 CRUD——后续增量任务
- 记忆/片段的新写操作
- Windows/Linux CLI 行为差异(lipo/打包契约已在 extra-binaries-and-release.md 覆盖,本次不触碰)
- 交互式 TTY 流程
