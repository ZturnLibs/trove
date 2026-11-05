import type { QuickAddSubmitKey } from "@/ipc/client";

function isMacPlatform(): boolean {
  if (typeof navigator === "undefined") return false;
  return /mac/i.test(navigator.platform ?? navigator.userAgent ?? "");
}

/**
 * 今日页快速添加的确认键判定（纯函数）。
 * - enter 模式：Enter 或 ⌘/Ctrl+Enter 均提交（行为为既有习惯的超集）
 * - modEnter 模式：仅 ⌘(mac)/Ctrl(其他)+Enter 提交，裸 Enter 不动作
 */
export function shouldSubmitQuickAdd(
  key: string,
  modPressed: boolean,
  mode: QuickAddSubmitKey,
): boolean {
  if (key !== "Enter") return false;
  if (mode === "modEnter") return modPressed;
  return true;
}

/** 提交组合键标签（Enter / ⌘↩ / Ctrl+↩）。 */
export function submitComboLabel(mode: QuickAddSubmitKey): string {
  if (mode === "modEnter") return isMacPlatform() ? "⌘↩" : "Ctrl+↩";
  return "Enter";
}

/** 输入框内的确认方式提示文案（可自定义动词：添加 / 创建…）。 */
export function quickAddSubmitHint(
  mode: QuickAddSubmitKey,
  verb = "添加",
): string {
  return `${submitComboLabel(mode)} ${verb}`;
}
