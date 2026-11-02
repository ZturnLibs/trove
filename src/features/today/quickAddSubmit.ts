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

/** 输入框内的确认方式提示文案。 */
export function quickAddSubmitHint(mode: QuickAddSubmitKey): string {
  if (mode === "modEnter") return isMacPlatform() ? "⌘↩ 添加" : "Ctrl+↩ 添加";
  return "Enter 添加";
}
