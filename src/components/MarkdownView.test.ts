import { describe, expect, it } from "vitest";
import { escapeHtml, renderMarkdown } from "@/components/MarkdownView";

describe("escapeHtml", () => {
  it("escapes HTML special characters", () => {
    expect(escapeHtml(`<script>alert(1)</script>`)).toBe(
      "&lt;script&gt;alert(1)&lt;/script&gt;",
    );
    expect(escapeHtml(`<img onerror="alert(1)">`)).toBe(
      "&lt;img onerror=&quot;alert(1)&quot;&gt;",
    );
  });
});

describe("renderMarkdown", () => {
  it("renders heading / list / bold / inline code / code block", () => {
    const html = renderMarkdown(`# 标题

- 甲
- 乙

**粗体** 和 \`行内代码\`

\`\`\`
const a = 1;
\`\`\`
`);
    expect(html).toContain("<h1>标题</h1>");
    expect(html).toContain("<ul>");
    expect(html).toContain("<li>甲</li>");
    expect(html).toContain("<strong>粗体</strong>");
    expect(html).toContain("<code>行内代码</code>");
    expect(html).toContain("<pre>");
  });

  it("escapes raw HTML instead of executing it", () => {
    const html = renderMarkdown(`<script>alert(1)</script>

<img src="x" onerror="alert(1)">`);
    expect(html).not.toContain("<script");
    expect(html).not.toContain("<img");
    expect(html).toContain("&lt;script&gt;");
    expect(html).toContain("&lt;img");
  });

  it("only allows http(s)/mailto links and opens them in a new tab", () => {
    const html = renderMarkdown(
      `[site](https://example.com) [bad](javascript:alert(1)) [mail](mailto:a@b.com)`,
    );
    expect(html).toContain(
      '<a href="https://example.com" target="_blank" rel="noreferrer">site</a>',
    );
    expect(html).toContain('href="mailto:a@b.com"');
    expect(html).not.toContain("javascript:");
  });
});
