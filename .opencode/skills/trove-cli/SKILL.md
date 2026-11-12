---
name: trove-cli
description: "通过 trove-cli 为用户管理 Trove 任务：查询、创建、批量导入（如钉钉聊天转待办）、修改、删除、延期。Use when 用户要求把信息变成任务、查询/修正/回收 Trove 任务，或任何需要操作本地 Trove 数据的自动化流程。"
---

# Trove CLI — Agent 任务管理

本机任务管理应用 Trove 提供 `trove-cli`，Agent 用它完成任务的**闭环**操作：查询 → 创建 → 修正 → 回收。CLI 经运行中的应用落库（不直接碰 SQLite），业务逻辑由应用保证。

## 定位可执行文件

```bash
CLI=/Applications/Trove.app/Contents/MacOS/trove-cli   # 正式版（真实任务数据）
# 开发调试时改用：TROVE_APP=src-tauri/target/debug/trove，数据写入开发实例，互不污染
```

## 铁律（安全）

1. **写操作必须 `--yes`**；没有 `--yes` 不会写库（create 会弹应用内确认，其余直接报错）。
2. **先 `--dry-run --json` 预览**，确认描述无误再真实执行。
3. **`delete` 必须显式 `--children cascade|promote`**——cascade 连子任务一起删，不可逆时优先 `archive`。
4. 批量一次 ≤ 100 条；输出里逐条 `ok`/`error`，**不要**把部分失败当全部失败重跑（会重复导入成功项）。

## 命令速查

```bash
$CLI --json query tasks --status todo --list "工作" --search "关键词" --limit 20   # 查询
$CLI --json query lists                                                          # 清单表(id/名称/待办数)
$CLI --json show <task-uuid>                                                     # 完整详情(notes/标签/父子)
$CLI --json create task --title "…" --notes "…" --due-date 2026-11-13 \
      --list "工作" --priority high --tag 钉钉 --yes                              # 创建(返回完整 task,含 id)
$CLI --json update <id> --title 新标题 --yes                                      # 部分字段更新
$CLI --json reopen <id> --yes              # 已完成→待办(幂等)
$CLI --json archive <id> --yes
$CLI --json delete <id> --children cascade --yes
$CLI --json postpone <id> --days 3 --yes   # 仅待办可延期
$CLI --json defer <id> --at 2026-11-20 --yes   # / --clear 清除
$CLI --json complete <id> --yes
```

## 输出解析

`--json` 输出一个带 `outcome` 标签的对象；创建/更新/重开等返回完整 `Task`（含 `id`、`revision`）。例如：

```json
{ "outcome": "taskCreated", "task": { "id": "…", "title": "…", "status": "todo", … } }
```

`outcome: "rejected"` = 被确认门拒绝（缺 `--yes` 或应用未就绪），配合非 0 退出码与 stderr。

## Recipe：钉钉聊天 → 批量任务

1. 从钉钉消息里提取待办（谁、做什么、截止）。
2. 生成 NDJSON（每行一条，camelCase 字段；`title` 必填）：

```
{"title":"回复 Alice：合同评审意见","notes":"来源：钉钉群【项目组】 11-12 @Alice","dueDate":"2026-11-13","priority":"high","tags":["钉钉","合同"]}
{"title":"跟进供应商报价","notes":"来源：钉钉私聊 Bob 11-12","dueDate":"2026-11-14","tags":["钉钉"]}
```

3. 先 `query lists` 拿到准确清单名 → 批量导入：

```bash
$CLI --json query lists
cat tasks.ndjson | $CLI --json create-batch --list "工作" --yes
```

4. 解析返回 `items[]`：`ok: true` 的 `taskId` 汇报给用户；`ok: false` 的逐条说明原因（常见：清单名不存在、日期格式错）。

## Recipe：修正 / 回收

- 信息变化：`update <id> --title … / --due-date … / --notes …`（只传变化字段，其余不动）。
- 建错了：`delete <id> --children cascade --yes`（有子任务且要保留时用 `promote`）。
- 暂时不做：`defer <id> --at <日期>`（到日期自动回显）或 `postpone --days N`。

## Recipe：巡检

```bash
$CLI --json query overdue --limit 50        # 逾期
$CLI --json query tasks --status todo --due-to $(date +%F)   # 今日及以前截止的待办
```

## 错误处理

| 现象 | 处置 |
| --- | --- |
| `等待应用响应超时` | 应用未启动且启动失败，或正忙；重试一次，仍失败报告用户 |
| `清单「X」不存在；可用清单：…` | 改用列出的名称或 uuid |
| `清单名「X」不唯一` | 用返回的 uuid 精确指定 |
| `该动作需要 confirmed=true` | 命令缺 `--yes` |
| 退出码非 0 | stderr 有原因；批量场景查 `items[].error` |
