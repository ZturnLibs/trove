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
