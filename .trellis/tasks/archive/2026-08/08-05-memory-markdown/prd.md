# 记忆 Markdown 渲染（memory-markdown）

## Goal

记忆预览真正渲染 Markdown（标题/列表/粗体/链接/代码块等），兑现输入框「支持基础 Markdown 文本」的承诺；替换当前仅 linkify URL 的纯文本预览。

## 背景（勘察确认）

- MemoryPage 预览目前是 `whitespace-pre-wrap` + `linkify`（MemoryPage.tsx:17-37, 183-186），仅处理 http 链接，无 Markdown 解析。
- `package.json` 无任何 Markdown 依赖。
- 应用 CSP：`script-src 'self'`（禁内联脚本）、`img-src 'self' data:`。
- 记忆为本地用户自己输入的内容；仍需防御 raw HTML（如 `<img onerror=…>`）。

## Requirements

- R1 引入 `marked` 依赖（轻量）。
- R2 新增 `src/components/MarkdownView.tsx`：接收 `markdown: string`，用 marked（GFM + breaks）渲染，**转义/移除 raw HTML**（`renderer.html` 返回转义文本），不引入 `dangerouslySetInnerHTML` 之外的执行路径；输出用 `dangerouslySetInnerHTML` 但内容已消毒。
- R3 MemoryPage 预览模式改用 `MarkdownView` 渲染 `draft.body`（替换 linkify）；无预览时仍显示 textarea。
- R4 样式：标题/列表/粗体/链接/行内代码/代码块有基础样式（Tailwind prose 或手写 CSS），在新窗口不破坏布局。

## Acceptance Criteria

- [ ] AC1 记忆预览能渲染标题、列表、粗体、链接、行内代码、代码块。
- [ ] AC2 记忆正文中的 raw HTML（如 `<script>`、`<img onerror>`）被转义为文本，不执行。
- [ ] AC3 预览与编辑切换行为不变；长内容不破坏布局。
- [ ] AC4 `pnpm typecheck`、`pnpm build`、`pnpm test:unit` 通过。

## Notes

- 轻量任务：PRD + 简要 design。改动为新增依赖 + `MarkdownView.tsx` + MemoryPage 接线。
- 链接点击沿用现有复制行为（或新窗口打开，以实现为准）。
