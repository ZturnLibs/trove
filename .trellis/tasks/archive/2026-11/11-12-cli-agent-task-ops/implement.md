# Implement:trove-cli 完整管理能力

## 执行清单(有序)

### Step 1 domain 层(先行,纯增量)

- [ ] `src-tauri/src/domain/workbench_action.rs`:
  - [ ] 确认/补 `TaskDeleteDisposition` 的 serde derive
  - [ ] 新增 `ActionUpdateTaskInput`、`ActionListHit` 结构
  - [ ] 新增 9 个 `WorkbenchAction` 变体(QueryTasks/GetTask/UpdateTask/ReopenTask/ArchiveTask/DeleteTask/PostponeTask/SetTaskDefer/QueryLists)
  - [ ] 新增对应 `ActionOutcome` 变体(TaskFetched/TaskUpdated/TaskReopened/TaskArchived/TaskDeleted/TaskPostponed/TaskDeferred/ListsQueried;QueryTasks 复用 TasksQueried)
  - [ ] 扩展 `requires_confirmation()` / `is_confirmed()` match 臂
  - [ ] 扩展 `workbench_action_description()` 各新写动作的中文描述(dry-run 输出)
- [ ] 单测(workbench_action.rs tests 模块):新写动作未确认被拒;`UpdateTask` serde round-trip;`QueryTasks` 反序列化含 default query

验证:`cd src-tauri && cargo test --lib`

### Step 2 application 层 dispatch

- [ ] `src-tauri/src/application/workbench_actions.rs`:加 9 个 match 臂,臂体委托给带 `&TaskService` 参数的 `pub(crate) fn`(可直测)
- [ ] `UpdateTask` 合并:get_task → Option 字段合并 + clear_due 优先 → update_task
- [ ] `QueryLists`:list_lists + count_list_todo_tasks 组装 ActionListHit
- [ ] 单测(tempdir Database,沿 tasks.rs 测试模式,tempdir 活过 Database 生命周期):
  - [ ] update 部分字段合并(AC3 读-改-写)
  - [ ] clear_due 清空 due_date+due_time
  - [ ] reopen 幂等 / postpone 非 todo 报错透传(AC5)
  - [ ] delete_tree disposition 透传
  - [ ] QueryLists 输出含收件箱与 open_count

验证:`cargo test --lib`

### Step 3 CLI(bin/trove-cli.rs)

- [ ] `QueryTarget` 加 `Tasks { 过滤 flags }`、`Lists`
- [ ] `Commands` 加 `Show / CreateBatch / Update / Reopen / Archive / Delete / Postpone / Defer`;`Create { kind: Task }` 加 --list/--priority/--tag/--parent/--due-time
- [ ] 清单名解析 helper(QueryLists 转发 → 精确匹配 → 歧义报错列候选)
- [ ] `create-batch`:NDJSON 读取(stdin 或 --file)→ 逐条构造 CreateTask 转发 → 逐条结果数组;上限 100
- [ ] 新 outcome 的人类可读格式化 + `--json` 透传
- [ ] 参数解析单测(现有 CLI 测试模式;若纯 clap 无测试先例,则写 build_action 纯函数测试)

验证:`cargo build --bin trove-cli && cargo test --bin trove-cli`(如无 bin 测试目标则 `cargo test --lib`)

### Step 4 文档 + Skill

- [ ] `docs/cli.md` 重写:命令表、Agent 章节(outcome 解析/`--yes`/自启动)、钉钉批量示例、Raw wire 附录
- [ ] `docs/action-layer.md` 动作表补 9 行
- [ ] `.opencode/skills/trove-cli/SKILL.md`(front-matter + 命令速查 + 3 recipe + 安全约定)

### Step 5 收尾验证(实机冒烟)

- [ ] `cargo test --lib` 全绿
- [ ] 对运行中的正式应用做冒烟:创建「CLI 冒烟-可删除」→ show → update 改标题 → query tasks --search 命中 → delete --children delete → 确认查无(AC1/AC3/AC6 全链路)
- [ ] create-batch 冒烟:3 条 NDJSON(1 条故意缺标题)→ 逐条结果正确(AC2)
- [ ] 对照 AC1–AC8 逐条勾验

## 验证命令

```bash
cd src-tauri
cargo test --lib
cargo build --release --bin trove-cli
/Applications/Trove.app/Contents/MacOS/trove-cli --json --dry-run create task --title t --yes
```

## 风险文件与回滚点

| 文件 | 风险 | 回滚 |
| --- | --- | --- |
| `domain/workbench_action.rs` | match 臂遗漏导致编译错(编译器兜底) | 纯增量,git revert 单文件 |
| `application/workbench_actions.rs` | 合并逻辑写错字段 | 单测 AC3 兜底;revert 单文件 |
| `bin/trove-cli.rs` | clap 参数冲突 | 增量子命令,revert 单文件 |
| `docs/*`、`.opencode/skills/*` | 无运行时风险 | 随意改 |

提交策略:Phase 3.4 单任务一次提交(惯例同近期任务);版本号不动、不发版(release 流程仅 main,extra-binaries-and-release.md)。

## task.py start 前检查

- [ ] implement.jsonl / check.jsonl 已含真实条目(非 _example 种子)
- [ ] prd.md / design.md / implement.md 三件套齐
- [ ] 用户已明确批准本规划总结
