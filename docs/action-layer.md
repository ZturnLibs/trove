# 统一动作层（WorkbenchAction）

`v1.4` 引入的内部动作协议，让所有入口（URL Scheme、未来 CLI、快捷指令）共用同一套校验与分发逻辑。

## 动作类型

| 动作 | 说明 | 外部来源 |
| --- | --- | --- |
| `navigate` | 聚焦主窗并路由 | ✅ URL Scheme |
| `openSearch` | 打开 Quick 搜索 | ✅ URL Scheme |
| `createPreview` | 弹出创建确认对话框 | ✅ `trove://create` |
| `createTask` / `createReminder` / `createMemory` | 直接写入 | ❌ 须 `confirmed: true` 且非 UrlScheme |
| `completeTask` | 完成任务 | ❌ 同上 |
| `queryToday` / `queryOverdue` / `queryInbox` / `queryList` | 查询任务摘要 | ✅ CLI / 快捷指令 |
| `queryTasks` | 通用任务查询（状态/清单/搜索/截止区间/父任务/分页） | ✅ CLI / 快捷指令 |
| `getTask` | 任务详情（完整字段） | ✅ CLI / 快捷指令 |
| `queryLists` | 清单清单（id/名称/未完成数） | ✅ CLI / 快捷指令 |
| `updateTask` | 部分字段更新（应用内读-改-写合并） | ❌ 须 `confirmed: true` 且非 UrlScheme |
| `reopenTask` / `archiveTask` | 重开 / 归档 | ❌ 同上 |
| `deleteTask` | 删除任务树（必须显式 disposition：cascade/promote） | ❌ 同上 |
| `postponeTask` | 截止日延后 N 天（仅待办） | ❌ 同上 |
| `setTaskDefer` | 设置 / 清除延期显示日期 | ❌ 同上 |
| `searchMemories` / `getSnippets` | 搜索记忆、收藏文本片段 | ✅ CLI / 快捷指令 |

## 分发选项

```typescript
// ActionDispatchOptions
{
  source: "urlScheme" | "commandPalette" | "cli" | "internal",
  dryRun: boolean  // true 时仅返回描述，不写库
}
```

## IPC

- `workbench_action_dispatch(action, options)` → `ActionOutcome`
- `url_scheme_handle(url)` — 仍可用；内部转为 `WorkbenchAction` 后分发

## 与 URL Scheme 的关系

1. `parse_trove_url` 解析为 `UrlSchemeAction`
2. 转换为 `WorkbenchAction`
3. `WorkbenchActionService::dispatch` 执行（导航/搜索/预览）
4. 创建预览仍 emit `url-scheme://pending-create`，载荷保持 `{ action: "createPreview", ... }` 以兼容现有 UI

详见 [url-scheme.md](./url-scheme.md)。

## 后续（v1.4+）

- ~~本地 CLI~~ → 见 [cli.md](./cli.md)（`trove-cli` 二进制）
- ~~规则自动化~~ → 见 [automation.md](./automation.md)（首版动作直接调用应用服务；后续可收敛到 `WorkbenchAction`）
- ~~macOS 快捷指令~~ → 见 [shortcuts.md](./shortcuts.md)（经 `trove-cli --json` 映射到同一枚举）
