# Trove CLI（`trove-cli`）

本地命令行工具，通过运行中的 Trove 应用执行 [统一动作层](./action-layer.md) 动作。

## 构建

```bash
cd src-tauri
cargo build --release --bin trove-cli
```

产物：`target/release/trove-cli`（与 `trove` 主程序同目录）。

## 全局选项

| 选项 | 说明 |
| --- | --- |
| `--dry-run` | 仅本地预览，不启动/唤醒应用 |
| `--json` | 输出 JSON（默认人类可读） |
| `--app PATH` | 指定 Trove 可执行文件；也可用环境变量 `TROVE_APP` |

## 命令

```bash
# 导航（等价 trove://）
trove-cli today
trove-cli inbox
trove-cli search "发票"

# 创建（默认弹出确认对话框，同 trove://create）
trove-cli create task --title "回复 Alice"
trove-cli create reminder --title Standup --fire-at "2026-08-18T09:00:00"

# 直接写入（须 --yes，经 trove-action 协议）
trove-cli create task --title "脚本任务" --yes
trove-cli complete <task-uuid> --yes

# 预览
trove-cli --dry-run today
trove-cli --dry-run --json create task --title "x" --yes

# 查询（需 Trove 在运行；供快捷指令）
trove-cli --json query today
trove-cli --json query overdue --limit 10
trove-cli --json query inbox
trove-cli --json query memories "周报"
trove-cli --json query snippets
```

## 协议

- **URL 类命令**（today / inbox / search / create 无 `--yes`）：向 Trove 进程传递 `trove://…` 参数（单实例转发）。
- **确认写入**（`--yes` / complete）：传递 `trove-action:{json}`，应用在临时文件写入 `ActionOutcome` 响应。

## 要求

- Trove 应用需已安装或 dev 构建可用；mutating 命令需要应用能处理单实例参数。
- CLI **不**直接读写 SQLite 文件。

## 参见

- [action-layer.md](./action-layer.md)
- [url-scheme.md](./url-scheme.md)
