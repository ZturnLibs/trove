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
- macOS 快捷指令 Actions 映射到同一枚举
