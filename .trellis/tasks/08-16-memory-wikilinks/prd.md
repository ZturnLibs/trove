# 记忆轻量双链

## Goal

对标 Obsidian `[[wiki link]]` 的轻量版：让记忆从扁平列表升级为可导航网络，复用已有 EntityLink，不做知识图谱可视化。

## 对标差距

| 竞品能力 | Trove 现状 |
| --- | --- |
| Obsidian 双向链接 | EntityLink 有，无 `[[语法]]` |
| 相关笔记推荐 | 无 |
| Megi 知识树 | 刻意不做图谱 |

## Requirements

1. **语法**：记忆正文 `[[标题]]` 保存时解析，建立 `EntityLink(type=mention)` 指向匹配标题的记忆（歧义时用户选择）
2. **反向链接**：记忆详情展示「谁引用了我」
3. **相关记忆**：按共同标签 / EntityLink / 关键词重合推荐 top 5，可一键确认建链
4. **触发词**：被引用时累加 useCount，影响 QuickWindow 排序
5. **边界**：不做块级引用、不做图谱 UI、不做全文 graph 布局

## Acceptance Criteria

- [ ] 编辑 `[[A]]` 后 A 的详情出现反向链接
- [ ] 链接目标不存在时提示创建或取消
- [ ] 删除记忆时链接安全清理（沿用 EntityLink GC 规则）
- [ ] 搜索索引包含 mention 关系（可选增强）

## 复杂度

**Medium** — 解析器 + EntityLink 扩展；PRD 可启动，implement 前确认 mention 类型枚举。

## Notes

- 设计依据：roadmap §5.3、`post-v1` §6.6 边界
- Wave B
