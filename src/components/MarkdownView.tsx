import { useMemo } from "react";
import { marked, type Renderer, type Tokens } from "marked";

const HTML_ESCAPES: Record<string, string> = {
  "&": "&amp;",
  "<": "&lt;",
  ">": "&gt;",
  '"': "&quot;",
  "'": "&#39;",
};

/** 转义 HTML 特殊字符，把 raw HTML（如 `<script>`、`<img onerror>`）变为可展示的纯文本。 */
export function escapeHtml(input: string): string {
  return input.replace(/[&<>"']/g, (ch) => HTML_ESCAPES[ch]);
}

const SAFE_LINK_SCHEME = /^(https?:|mailto:)/i;

/** 将 Markdown 渲染为已消毒的 HTML：GFM + 自动断行；raw HTML 转义为文本；链接仅放行 http(s)/mailto。 */
export function renderMarkdown(markdown: string): string {
  const renderer = new marked.Renderer();
  renderer.html = ({ text }: Tokens.HTML | Tokens.Tag) => escapeHtml(text);
  renderer.link = function (
    this: Renderer,
    { href, title, tokens }: Tokens.Link,
  ) {
    const text = this.parser.parseInline(tokens);
    if (!SAFE_LINK_SCHEME.test(href)) return text;
    const attrs = [
      `href="${escapeHtml(href)}"`,
      'target="_blank"',
      'rel="noreferrer"',
    ];
    if (title) attrs.push(`title="${escapeHtml(title)}"`);
    return `<a ${attrs.join(" ")}>${text}</a>`;
  };
  return marked.parse(markdown, {
    gfm: true,
    breaks: true,
    renderer,
    async: false,
  });
}

export function MarkdownView({ markdown }: { markdown: string }) {
  const html = useMemo(() => renderMarkdown(markdown), [markdown]);
  return (
    <div
      className="markdown-body whitespace-pre-wrap break-words text-[13px] leading-relaxed"
      dangerouslySetInnerHTML={{ __html: html }}
    />
  );
}
