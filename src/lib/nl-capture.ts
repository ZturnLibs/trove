import type { ParsedCapture, TaskPriority } from "@/ipc/client";
import { recurrenceLabel } from "@/lib/recurrence";
import { localTodayString } from "@/lib/defer";

const PRIORITY_LABEL: Record<TaskPriority, string> = {
  none: "无",
  low: "低",
  medium: "中",
  high: "高",
};

/** Build `datetime-local` value from NL parse parts. */
export function buildFireAtFromParsed(
  dueDate: string | null | undefined,
  dueTime: string | null | undefined,
  fallbackDate = localTodayString(),
): string {
  const date = dueDate ?? fallbackDate;
  const time = dueTime ?? "09:00";
  return `${date}T${time}`;
}

export function parseTagsInput(text: string): string[] {
  return [
    ...new Set(
      text
        .split(/[,，]/)
        .map((part) => part.trim())
        .filter(Boolean),
    ),
  ];
}

export function mergeTagNames(
  parsed: string[] | undefined,
  manualInput: string,
): string[] {
  return [...new Set([...(parsed ?? []), ...parseTagsInput(manualInput)])];
}

export function formatParsedHint(parsed: ParsedCapture): string | null {
  const bits = [
    parsed.dueDate ? `日期 ${parsed.dueDate}` : null,
    parsed.dueTime ? `时间 ${parsed.dueTime}` : null,
    parsed.priority !== "none"
      ? `优先级 ${PRIORITY_LABEL[parsed.priority]}`
      : null,
    parsed.tagNames?.length
      ? `标签 ${parsed.tagNames.join("、")}`
      : null,
    parsed.recurrence ? `重复 ${recurrenceLabel(parsed.recurrence)}` : null,
  ].filter(Boolean);
  if (bits.length === 0) return null;
  const suffix = parsed.ambiguousFields.length
    ? "（请确认高亮字段）"
    : "";
  return `识别：${bits.join(" · ")}${suffix}`;
}

export const QUICK_CAPTURE_SYNTAX = [
  { syntax: "明天 / 后天 / 今天", desc: "相对日期" },
  { syntax: "周五 / 下周一", desc: "星期（裸 weekday 会标为待确认）" },
  { syntax: "下午三点 / 15:30", desc: "时间" },
  { syntax: "p1 / p2 / p3 或 !高", desc: "优先级" },
  { syntax: "#标签名", desc: "任务标签，可多个" },
  { syntax: "每天 / 工作日 / 每周五 / 每月1号", desc: "重复规则" },
  { syntax: "⌘1–5", desc: "捕获态跳转：标题 / 日期 / 时间 / 标签 / 优先级" },
] as const;
