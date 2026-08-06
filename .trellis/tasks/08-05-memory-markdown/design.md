# 技术设计：记忆 Markdown 渲染

## 1. 依赖

```bash
pnpm add marked
```

## 2. MarkdownView 组件

`src/components/MarkdownView.tsx`：

```tsx
import { useMemo } from "react";
import { marked } from "marked";

marked.use({ gfm: true, breaks: true });

export function MarkdownView({ markdown }: { markdown: string }) {
  const html = useMemo(() => {
    const renderer = new marked.Renderer();
    // 防御 raw HTML：把 HTML 节点转义成纯文本，避免执行/样式注入
    renderer.html = (html: string) => escapeHtml(html);
    const tokens = marked.lexer(markdown);
    return marked.parser(tokens, { renderer });
  }, [markdown]);
  return <div className="markdown-body whitespace-pre-wrap break-words text-[13px] leading-relaxed" dangerouslySetInnerHTML={{ __html: html }} />;
}
```

- `renderer.html` 覆盖默认透传，返回 `escapeHtml(html)`（`&<>"'` 转义），使 `<script>`/`<img onerror>` 等以文本展示。
- 链接 `href` 默认仅 http(s)/mailto；marked 链接走默认 renderer.link，可加 `target="_blank" rel="noreferrer"`。

## 3. 样式（styles/app.css 或全局）

`markdown-body` 基础样式（Tailwind v4 用 `@layer` 或普通类）：
- 标题 `h1..h6`：字号/粗细/间距；
- 列表 `ul/ol`：`list-disc`/`list-decimal` + 内边距；
- `strong`/`em`/`a`（`text-accent underline`）/`code`（`bg-row-hover rounded px-1`）/`pre`（`bg-surface-raised border p-2 overflow-auto`）。

## 4. MemoryPage 接线

- 预览分支（`:183-186`）由 `linkify(draft.body)` 改为 `<MarkdownView markdown={draft.body} />`。
- 保留 textarea 编辑分支与 preview 开关；`linkify` 函数可移除（若无其它引用）。

## 5. 回归注意

- CSP `script-src 'self'` 兜底；MarkdownView 已转义 raw HTML，双保险。
- 不改变记忆编辑/保存逻辑。
