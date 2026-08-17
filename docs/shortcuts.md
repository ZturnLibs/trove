# macOS 快捷指令（v1.4 切片 4）

快捷指令通过 **`trove-cli --json`** 调用与界面 / URL Scheme 相同的 [统一动作层](./action-layer.md)。本切片不捆绑原生 App Intents 扩展；「运行 Shell 脚本」即可拿到结构化 JSON。

## 前置

1. 安装或本地构建 Trove，并保持应用在运行（查询/写入都经单实例转发）。
2. 构建 CLI：

```bash
cd src-tauri
cargo build --release --bin trove-cli
```

3. 在快捷指令里把脚本路径设为 `trove-cli` 绝对路径，或设置环境变量 `TROVE_APP`。

建议脚本开头：

```bash
export PATH="/Applications/Trove.app/Contents/MacOS:$PATH"
# 本地开发可改成 src-tauri/target/release
```

## 首批动作

| 快捷指令意图 | 命令 | 确认 |
| --- | --- | --- |
| 创建任务（弹预览） | `trove-cli create task --title "…"` | 应用内确认 |
| 创建任务（静默） | `trove-cli create task --title "…" --yes` | `--yes` |
| 创建提醒 / 记忆 | `trove-cli create reminder\|memory …` | 同上 |
| 完成任务 | `trove-cli complete <uuid> --yes` | `--yes` |
| 获取今日 | `trove-cli --json query today` | 只读 |
| 获取逾期 | `trove-cli --json query overdue` | 只读 |
| 获取收件箱 | `trove-cli --json query inbox` | 只读 |
| 指定清单 | `trove-cli --json query list <list-uuid>` | 只读 |
| 搜索记忆 | `trove-cli --json query memories "周报"` | 只读 |
| 获取收藏片段 | `trove-cli --json query snippets` | 只读 |

只读查询结果为 `ActionOutcome` JSON（`outcome` 字段区分类型），正文预览截断，避免把完整笔记/剪切板灌进快捷指令日志。

## 推荐快捷指令结构

1. **输入**：快捷指令「文本」或「提问」。
2. **运行 Shell 脚本**：`trove-cli --json query today`（Pass Input: to stdin 可忽略）。
3. **从 JSON 取值**：读取 `data.dueToday` / `items`。
4. **显示结果** 或 **选择列表** 后调用 `trove-cli complete "$id" --yes`。

打开界面而不取数据时，仍可用 URL：

- `trove://today`
- `trove://inbox`
- `trove://search?q=`
- `trove://create?type=task&title=`（强制预览）

## 错误

- 应用未运行：CLI 可能超时（约 15s）或找不到可执行文件。
- 静默写入必须 `--yes`；URL Scheme 入口不会直接写库。
- 失败时 stdout/stderr 只含摘要，不含完整任务/记忆正文。

## 明确不做（本切片）

- 原生 App Intents / `.appex` 扩展
- 快捷指令图库上架
- 任意 Shell 作为规则动作（规则引擎仍只跑已注册动作）

## 参见

- [cli.md](./cli.md)
- [action-layer.md](./action-layer.md)
- [url-scheme.md](./url-scheme.md)
