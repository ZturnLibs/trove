# Trove URL Scheme (`trove://`)

Trove 注册自定义 URL scheme，供浏览器书签、Raycast/Alfred 脚本、快捷指令等本地自动化调用。

## 注册

- macOS / Windows：安装版应用通过 `tauri-plugin-deep-link` 注册 `trove` scheme
- 开发模式：部分平台需正式安装包后 OS 才会路由到应用

## 动作清单

| URL | 行为 |
| --- | --- |
| `trove://today` | 聚焦主窗口并导航到「今日」 |
| `trove://inbox` | 聚焦主窗口并导航到「收件箱」 |
| `trove://search?q=<词>` | 打开快速窗口并进入搜索，`q` 为可选搜索词 |
| `trove://create?type=task&title=<标题>` | 弹出确认对话框，确认后创建任务 |
| `trove://create?type=reminder&title=<标题>` | 弹出确认对话框，确认后创建提醒 |
| `trove://create?type=memory&title=<标题>` | 弹出确认对话框，确认后创建记忆 |

### `create` 可选参数

| 参数 | 适用 | 说明 |
| --- | --- | --- |
| `title` | 全部 | **必填**，UTF-8，最长 500 字符 |
| `notes` / `body` | 全部 | 备注或正文，最长 5000 字符 |
| `dueDate` / `due` | task | `YYYY-MM-DD` |
| `fireAt` / `fire` | reminder | `YYYY-MM-DDTHH:MM:SS` 或 `YYYY-MM-DD HH:MM`；省略时默认为次日 09:00（本地时区） |

## 示例

```
trove://today
trove://inbox
trove://search?q=发票
trove://create?type=task&title=回复%20Alice
trove://create?type=reminder&title=Standup&fireAt=2026-08-17T09:00:00
trove://create?type=memory&title=Meeting%20notes&body=Discussed%20roadmap
```

## 安全

- 仅接受 `trove://` scheme；拒绝 shell、文件路径或其它 scheme
- URL 总长上限 2048 字符；各参数有独立长度与格式校验
- `create` **必须**经主窗口确认对话框，不会静默写入
- 非法或恶意参数仅记录日志，不会导致应用崩溃

## 与 v1.4 动作层

当前实现：`parse_trove_url` → `WorkbenchAction` → `workbench_action_dispatch` 统一分发。详见 [action-layer.md](./action-layer.md)。

创建预览仍经主窗确认对话框；`url-scheme://pending-create` 事件载荷形状不变。
