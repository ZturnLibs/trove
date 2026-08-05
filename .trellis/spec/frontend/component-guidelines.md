# Component Guidelines

> How components are built in this project.

---

## Overview

<!--
Document your project's component conventions here.

Questions to answer:
- What component patterns do you use?
- How are props defined?
- How do you handle composition?
- What accessibility standards apply?
-->

(To be filled by the team)

---

## Component Structure

<!-- Standard structure of a component file -->

(To be filled by the team)

---

## Props Conventions

<!-- How props should be defined and typed -->

(To be filled by the team)

---

## Styling Patterns

<!-- How styles are applied (CSS modules, styled-components, Tailwind, etc.) -->

(To be filled by the team)

---

## Accessibility

<!-- A11y requirements and patterns -->

(To be filled by the team)

---

## Common Mistakes

<!-- Component-related mistakes your team has made -->

---

## Branding / App Icons

**品牌标识**：Trove 用宝藏箱 + 字母 T 标记，品牌蓝单色。
- 应用内 Logo 一律使用 `src/components/BrandLogo.tsx`（内联 SVG，`fill="currentColor"`），
  通过 `text-accent` 继承主题色（亮 `#2563eb` / 暗 `#5b8def`），明暗主题自动适配。
- 例：侧边栏 `<BrandLogo className="h-5 w-5 text-accent" />`，Onboarding
  `<BrandLogo className="h-12 w-12 text-accent" />`。

**再生成应用图标**（改 Logo 后必须执行）：

```bash
# 源图为 1024×1024 方形透明 SVG，位于 design/logo.svg
pnpm tauri icon design/logo.svg
rm -rf src-tauri/icons/android src-tauri/icons/ios   # 项目无移动端，删除 CLI 顺带产物
```

- `tauri icon` 会覆盖 `src-tauri/icons/`（icns/ico/各尺寸 PNG），
  `tauri.conf.json` `bundle.icon` 引用的文件名不变；托盘与窗口图标经
  `app.default_window_icon()` 自动生效。
- favicon：`public/logo.svg`，`index.html` 引用 `/logo.svg`。
- **Forbidden**：不要直接改 `src-tauri/icons/` 里的 PNG 产物，一律改 `design/logo.svg` 源图再生成。

**Related**: `design/logo.svg`、`src/components/BrandLogo.tsx`、`src-tauri/icons/`。

---

## Destructive/Confirm Actions

**Gotcha**: Tauri 的 webview（macOS WKWebView / Windows WebView2）不实现
`window.confirm()` 和 `window.alert()`——静默失败并返回假值。任何依赖它们的确认
或提示逻辑在真实应用里永远不执行（按钮"点了没反应"）。

**Convention**: 破坏性/确认类操作一律使用应用内两步内联确认，复用
`src/design-system/patterns/ConfirmButton.tsx`：

```tsx
<ConfirmButton
  confirmLabel="确认删除？"
  onConfirm={() => deleteMutation.mutate()}
  resetKey={task.id} // 切换选中项时自动还原
>
  删除
</ConfirmButton>
```

行为：首次点击进入"确认…？" danger 态，二次点击执行；3 秒未确认自动还原；
`resetKey` 变化时还原。成功/失败提示用内联 state（如 `notice`/`error`）渲染，
不要用 `window.alert()`。

**Forbidden**:

```tsx
// ❌ 在 Tauri webview 中永不生效
if (confirm("确认删除？")) doDelete();

// ✅ 两步内联确认
<ConfirmButton confirmLabel="确认删除？" onConfirm={doDelete}>删除</ConfirmButton>
```

**Related**: `src/design-system/patterns/ConfirmButton.tsx`、`TaskDetailPanel.tsx`、
`MemoryPage.tsx`、`ClipboardPage.tsx`、`SettingsPage.tsx`、`AttachmentsSection.tsx`。
