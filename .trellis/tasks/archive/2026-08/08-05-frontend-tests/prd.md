# 前端测试覆盖（frontend-tests）

## Goal

为关键前端纯逻辑补充 vitest 单测，提升覆盖率。当前仅 `src/lib/cn.test.ts` 与 `MarkdownView.test.ts` 有测试；本轮覆盖两个快捷键工具函数。

## 背景（勘察确认）

- vitest 配置：`environment: "node"`，include `src/**/*.test.ts`（vite.config.ts:33-36），**无 jsdom**。
- `src/lib/shortcut-record.ts`：`eventToShortcutString(event)` 把 KeyboardEvent 转为存储格式（如 `Command+Shift+Space`）；依赖 `navigator.platform` 判断 Mac，node 下 navigator 未定义时走非 Mac 分支；含 blocked 列表（Command/Ctrl+Q/W/M、Alt+F4 返回 null）。
- `src/lib/shortcuts.ts`：`formatShortcutLabel(raw)` 把存储字符串格式化为 UI 提示（非 Mac 用 `+` 连接与文本，Mac 用 ⌘⌃⌥⇧⌫ 符号拼接）。
- 二者均用 `typeof navigator !== "undefined"` 守卫，可在 node 下直接测非 Mac 分支；Mac 分支用 stub `globalThis.navigator.platform` 测。

## Requirements

- R1 新增 `src/lib/shortcut-record.test.ts`：覆盖 `eventToShortcutString` 非 Mac 分支（Ctrl+Shift+Space 组合、单键、空格→Space、修饰键单独→null、blocked 列表→null）；stub navigator 为 Mac 测 Command 分支。
- R2 新增 `src/lib/shortcuts.test.ts`：覆盖 `formatShortcutLabel`（null/空→""、Command→Ctrl(非Mac)/⌘(Mac)、Ctrl/Alt/Shift 映射、单字符大写、Enter/Esc/Space/Backspace、join 符号差异）。
- R3 不改生产代码；`pnpm test:unit` 全部通过（新增测试数 ≥ 12）。

## Acceptance Criteria

- [ ] AC1 `shortcut-record.test.ts` 覆盖组合键、blocked、修饰键单独、Mac 分支。
- [ ] AC2 `shortcuts.test.ts` 覆盖格式化与 Mac/非 Mac 差异。
- [ ] AC3 生产代码零改动；`pnpm test:unit` 通过（新增用例 ≥ 12）。
- [ ] AC4 `pnpm typecheck` 通过。

## Notes

- 轻量测试任务：PRD-only，不改生产逻辑。
- KeyboardEvent 用 mock 对象（`{ key, ctrlKey, metaKey, altKey, shiftKey } as KeyboardEvent`），因 node 无 DOM。
