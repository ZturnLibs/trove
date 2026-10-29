import type { UpdateTaskInput } from "@/ipc/client";

/**
 * 详情面板保存语义（纯函数，沿 splitBoardColumns.ts 模式）。
 * onBlur={save} 的提交内核：标签文本解析 + 保存载荷组装。
 */

/** 标签文本 → 标签数组：中英文逗号分隔，逐项 trim，去空。 */
export function parseTagNames(tagText: string): string[] {
  return tagText
    .split(/[,，]/)
    .map((t) => t.trim())
    .filter(Boolean);
}

/** 组装 taskUpdate 载荷：draft 全量透传，标签由文本解析替换。 */
export function buildDetailSaveInput(
  draft: UpdateTaskInput,
  tagText: string,
): UpdateTaskInput {
  return { ...draft, tagNames: parseTagNames(tagText) };
}
