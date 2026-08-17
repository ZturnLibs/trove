# v1.2 信息捕获余留项

## Goal

收尾 `post-v1` §6 中 v1.2 第二切片后仍未交付的捕获能力，缩小与 CleanShot/Maccy/Obsidian Web Clipper 的差距。

## 余留范围

| 项 | 说明 | 优先级 |
| --- | --- | --- |
| 存储空间管理器 | DB/assets/缩略图占用统计 + 引用安全 GC UI | P1 |
| 截图快速收藏 | 全局快捷键区域截图 → 剪切板/记忆 | P2 |
| 文件引用 | 沙盒书签 + 路径展示 + 失效提示 | P2 |
| 浏览器捕获 | 用户主动扩展：标题/URL/选中文本 | P3 |

## Requirements

1. **存储管理器**：设置或 health 页展示占用；手动触发 GC；不删除有引用的 assets
2. **截图**：快捷键捕获 → 缩略图预览 → 存剪切板历史或记忆；第一版无标注
3. **文件引用**：默认存引用不复制本体；Finder/资源管理器打开；删除引用不删原文件
4. **浏览器扩展**：独立仓库或子目录；仅主动捕获；不记录浏览历史

## Acceptance Criteria

- [ ] 存储数字与 `assets.rs` GC 行为一致
- [ ] 截图 OCR 走现有本地 pipeline
- [ ] 文件引用在 macOS 沙盒下权限可恢复
- [ ] 浏览器扩展不在本仓库阻塞发版（可标记为 follow-up 子切片）

## 复杂度

**Complex** — 必须 `design.md` 拆分 4 项依赖与平台差异；建议分 2–4 个 implement 阶段。

## Notes

- 截图/文件/扩展可拆独立 follow-up 任务
- Wave C 末项；storage 子项可提前到 health-dashboard 之前

## 建议 implement 顺序

1. 存储管理器（解锁 health-dashboard 数据）
2. 截图快速收藏
3. 文件引用
4. 浏览器扩展（可选独立 repo）
