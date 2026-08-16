# Trove 浏览器捕获扩展（Follow-up）

本仓库 **不包含** 浏览器扩展实现。该能力在 `08-16-capture-remaining` 中标记为独立 follow-up，不阻塞桌面端发版。

## 目标（post-v1 §6.3）

- 用户**主动**在浏览器中选中文本 / 页面标题与 URL
- 通过扩展发送到 Trove，创建记忆或剪贴板条目
- **不**记录浏览历史、不后台抓取整页

## 建议实现形态

1. 独立仓库或 `browser-extension/` 子目录（Manifest V3）
2. 与桌面端通信：本地 WebSocket / Native Messaging / 未来的 URL Scheme（`trove://create?type=memory&...`）
3. 发送前必须经用户确认（与 URL Scheme 安全要求一致）

## 桌面端已就绪的对接点

- 记忆创建 IPC：`memory_create`
- 剪贴板文本采集：`clipboard` 轮询（扩展也可写系统剪贴板后由用户「再次复制」）
- EntityLink：可将 URL 作为 `source` 元数据扩展（待专用 `SourceReference` 表）

## 验收（扩展侧）

- [ ] 仅用户点击扩展按钮时捕获
- [ ] 选区 + 标题 + URL 可预览后发送
- [ ] 断网时队列本地重试或明确失败提示
