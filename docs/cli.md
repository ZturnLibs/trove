# Trove CLI（`trove-cli`）

本地命令行工具，通过运行中的 Trove 应用执行 [统一动作层](./action-layer.md) 动作。供人使用，也为自动化 Agent（编码助手、脚本、快捷指令）提供完整的任务管理闭环。

## 构建

```bash
cd src-tauri
cargo build --release --bin trove-cli
```

产物：`target/release/trove-cli`（与 `trove` 主程序同目录）。安装包会把 `trove-cli` 放到应用可执行文件旁（macOS：`Trove.app/Contents/MacOS/trove-cli`）。

## 全局选项

| 选项 | 说明 |
| --- | --- |
| `--dry-run` | 仅本地预览，不启动/唤醒应用 |
| `--json` | 输出 JSON（默认人类可读） |
| `--app PATH` | 指定 Trove 可执行文件；也可用环境变量 `TROVE_APP`（可指向 dev 实例实现数据隔离） |

安全约定：**所有写操作必须 `--yes` 才落库**；未加 `--yes` 时 create 走应用内确认、其余写命令直接报错。Agent 应先 `--dry-run --json` 预览再执行。

## 命令

### 导航（等价 trove://）

```bash
trove-cli today
trove-cli inbox
trove-cli search "发票"
```

### 创建

```bash
# 弹出确认对话框，同 trove://create
trove-cli create task --title "回复 Alice"
trove-cli create reminder --title Standup --fire-at "2026-08-18T09:00:00"

# 直接写入（--yes）；任务可指定清单/优先级/标签/父任务/截止时刻
trove-cli create task --title "脚本任务" --yes
trove-cli create task --title "回复 Alice" \
  --list "工作" --priority high --tag 钉钉 --tag 合同 \
  --due-date 2026-11-13 --due-time 18:00 --yes
```

`--list` 支持**清单名称**（精确匹配；重名时报错并列出候选 uuid）或 uuid。

### 批量创建（Agent 高频）

从 NDJSON 文件或 stdin 一次导入，每行一条，逐条隔离失败（上限 100 条/次）：

```bash
trove-cli create-batch --yes --file tasks.ndjson
# 或 cat tasks.ndjson | trove-cli create-batch --list "工作" --yes
```

每行字段（camelCase，`title` 必填）：

```json
{ "title": "回复 Alice：合同评审", "notes": "来源：钉钉群【项目组】", "dueDate": "2026-11-13", "dueTime": "18:00", "priority": "high", "list": "工作", "tags": ["钉钉", "合同"] }
```

JSON 输出：`{ "total": N, "succeeded": X, "failed": Y, "items": [{ "index", "ok", "taskId", "title", "error"? }] }`；有失败项时退出码非 0。

### 查询（需应用在运行；供快捷指令 / Agent）

```bash
trove-cli --json query today
trove-cli --json query overdue --limit 10
trove-cli --json query inbox
trove-cli --json query tasks --status todo --list "工作" --search "合同" \
  --due-from 2026-11-01 --due-to 2026-11-30 --limit 50
trove-cli --json query lists
trove-cli --json query memories "周报"
trove-cli --json query snippets
```

`query tasks` 输出 `TasksQueried`（任务摘要：id/标题/截止/优先级/清单/状态）；需要完整备注、标签、父子关系时用 `show`。

### 详情

```bash
trove-cli --json show <task-uuid>
```

### 修改 / 生命周期（全部需 `--yes`）

```bash
# 部分字段更新：只传需要修改的字段，其余保持不变
trove-cli update <id> --title "新标题" --priority high --yes
trove-cli update <id> --due-date 2026-11-15 --yes
trove-cli update <id> --clear-due --yes            # 清空截止日期与时刻
trove-cli update <id> --notes "" --yes             # 空字符串清空备注
trove-cli update <id> --tag 钉钉 --tag 合同 --yes   # 标签整体替换；不传 --tag 保持不变

trove-cli reopen <id> --yes                        # 已完成 → 待办（幂等）
trove-cli archive <id> --yes
trove-cli delete <id> --children cascade --yes     # 子任务处置必须显式：cascade=一并删除 / promote=提升为顶层
trove-cli postpone <id> --days 3 --yes             # 截止日延后 3 天（仅待办）
trove-cli defer <id> --at 2026-11-20 --yes         # 该日期起重新显示
trove-cli defer <id> --clear --yes                 # 清除延期显示
trove-cli complete <task-uuid> --yes
```

### 导出

```bash
trove-cli export-csv --out ./tasks.csv
trove-cli --json export-csv
```

## Agent 使用约定

1. **解析输出**：`--json` 输出为带 `outcome` 标签的单个 JSON 对象（如 `taskCreated` / `tasksQueried` / `rejected`）。创建/更新/重开等返回完整 `Task` 对象（含 `id`），用于后续 `complete` / `update`。
2. **退出码**：成功 0；被拒绝、超时、部分批量失败均非 0，`stderr` 带原因。
3. **应用自启动**：应用未运行时 CLI 会启动/唤醒它再执行；查询类命令在应用就绪前返回「应用未就绪」。
4. **多实例**：默认路由到 `/Applications/Trove.app`（正式版数据）。开发调试用 `TROVE_APP=<dev 二进制路径>` 或 `--app` 指向构建产物，数据写入开发实例目录，互不污染。
5. **限流**：批量一次 ≤ 100 条；更大规模请分多次调用。

## 钉钉聊天 → 任务（Agent recipe）

Agent 从钉钉群聊提取待办后，生成 NDJSON 并批量入库：

```bash
cat <<'EOF' | trove-cli create-batch --list "工作" --yes
{"title":"回复 Alice：合同评审意见","notes":"来源：钉钉群【项目组】 11-12 消息 @Alice","dueDate":"2026-11-13","priority":"high","tags":["钉钉","合同"]}
{"title":"跟进供应商报价","notes":"来源：钉钉私聊 Bob 11-12","dueDate":"2026-11-14","tags":["钉钉"]}
EOF
```

返回的 `items[].taskId` 可用于完成或修正任务。

## 协议

- **URL 类命令**（today / inbox / search / create 无 `--yes`）：向 Trove 进程传递 `trove://…` 参数（单实例转发）。
- **动作类**（`--yes` / query / show / update / 生命周期）：传递 `trove-action:{json}`，应用在临时文件写入 `ActionOutcome` 响应。
- CLI **不**直接读写 SQLite 文件；业务逻辑（重复任务、提醒、搜索索引、备份）始终在应用层。

## Raw wire 附录（高级）

`trove-cli raw --json '<动作 JSON>'` 可直接派发动作。serde tag：动作用 `"action"`，返回用 `"outcome"`；变体字段为 snake_case。所有写动作需 `"confirmed": true`。

```bash
trove-cli raw --json '{"action":"queryTasks","query":{"status":"todo","search":"合同","limit":10}}'
trove-cli raw --json '{"action":"updateTask","input":{"id":"<uuid>","title":"新标题"},"confirmed":true}'
trove-cli raw --json '{"action":"deleteTask","task_id":"<uuid>","disposition":"cascade","confirmed":true}'
trove-cli raw --json '{"action":"setTaskDefer","task_id":"<uuid>","available_at":"2026-11-20","confirmed":true}'
trove-cli raw --json '{"action":"queryLists"}'
```

新动作一览：`queryTasks` / `getTask` / `queryLists` / `updateTask` / `reopenTask` / `archiveTask` / `deleteTask` / `postponeTask` / `setTaskDefer`。

## 要求

- Trove 应用需已安装或 dev 构建可用；mutating 命令需要应用能处理单实例参数。
- CLI **不**直接读写 SQLite 文件。

## 参见

- [action-layer.md](./action-layer.md)
- [url-scheme.md](./url-scheme.md)
- Agent Skill：`.opencode/skills/trove-cli/SKILL.md`
