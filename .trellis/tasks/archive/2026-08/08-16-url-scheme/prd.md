# URL Scheme 生态入口

## Goal

对标 Raycast/Alfred 深度集成与 Todoist API：以最低成本打开本地自动化生态，为 v1.4 CLI/规则引擎埋种子。

## 对标差距

| 竞品能力 | Trove 现状 |
| --- | --- |
| Todoist `todoist://` / REST | 无 |
| Raycast Trove 扩展 | 无官方 scheme |
| Things URL（有限） | 无 |

## Requirements

1. **注册**：`trove://` scheme（`tauri.conf.json`）
2. **动作（第一期）**：
   - `trove://today` / `trove://inbox` / `trove://search?q=`
   - `trove://create?type=task|reminder|memory&title=...`（强制预览确认）
3. **安全**：参数长度/编码白名单；禁止读任意文件或执行 shell
4. **文档**：发布 scheme 清单（docs 或 README 链）
5. **跨平台**：macOS + Windows 行为一致

## Acceptance Criteria

- [ ] 浏览器书签 `trove://create?type=task&title=Test` 弹确认后创建任务
- [ ] 恶意/超长参数被拒绝并记录日志，不 crash
- [ ] 导航类 link 聚焦主窗口并路由正确
- [ ] 与后续 v1.4 统一动作层设计对齐（避免重复 IPC）

## 复杂度

**Complex** — 需 `design.md`（安全模型、与 window_show 交互、create 预览流）。

## Notes

- 设计依据：roadmap §5.5、`post-v1` §8.3
- Wave C；可在 quickwindow-nlp-polish 之后
