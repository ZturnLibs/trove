import type { RecurrenceFrequency, RecurrenceRule } from "@/ipc/client";

const WEEKDAY_SHORT = ["", "一", "二", "三", "四", "五", "六", "日"] as const;

const FREQ_LABEL: Record<RecurrenceFrequency, string> = {
  daily: "每天",
  weekdays: "工作日",
  weekly: "每周",
  monthly: "每月",
  everyNDays: "每 N 天",
  everyNWeeks: "每 N 周",
};

export function systemTimezone(): string {
  return Intl.DateTimeFormat().resolvedOptions().timeZone;
}

/** ISO weekday 1=Mon … 7=Sun (matches backend). */
export function isoWeekdayFromDate(date = new Date()): number {
  const js = date.getDay();
  return js === 0 ? 7 : js;
}

export function defaultRecurrence(
  frequency: RecurrenceFrequency = "daily",
  refDate = new Date(),
): RecurrenceRule {
  const timezone = systemTimezone();
  const base = {
    version: 1,
    interval: 1,
    timezone,
    endAt: null as string | null,
  };
  switch (frequency) {
    case "weekly":
    case "everyNWeeks":
      return { ...base, frequency, weekdays: [isoWeekdayFromDate(refDate)] };
    case "monthly":
      return { ...base, frequency, monthday: refDate.getDate() };
    default:
      return { ...base, frequency };
  }
}

export function recurrenceLabel(rule: RecurrenceRule): string {
  switch (rule.frequency) {
    case "daily":
      return "每天";
    case "weekdays":
      return "工作日";
    case "weekly": {
      const days = (rule.weekdays ?? [])
        .map((d) => WEEKDAY_SHORT[d] ?? "")
        .filter(Boolean)
        .join("、");
      return days ? `每周 ${days}` : "每周";
    }
    case "monthly":
      return rule.monthday ? `每月 ${rule.monthday} 日` : "每月";
    case "everyNDays":
      return `每 ${rule.interval} 天`;
    case "everyNWeeks": {
      const days = (rule.weekdays ?? [])
        .map((d) => WEEKDAY_SHORT[d] ?? "")
        .filter(Boolean)
        .join("、");
      const interval = rule.interval > 1 ? `${rule.interval} 周` : "周";
      return days ? `每 ${interval} · ${days}` : `每 ${rule.interval} 周`;
    }
    default:
      return FREQ_LABEL[rule.frequency] ?? "周期";
  }
}

export function withRecurrenceFrequency(
  current: RecurrenceRule | null,
  frequency: RecurrenceFrequency,
): RecurrenceRule {
  const next = defaultRecurrence(frequency);
  if (current) {
    next.timezone = current.timezone;
    next.endAt = current.endAt ?? null;
  }
  return next;
}

export function toggleWeekday(
  rule: RecurrenceRule,
  day: number,
): RecurrenceRule {
  const current = rule.weekdays ?? [];
  let next = current.includes(day)
    ? current.filter((d) => d !== day)
    : [...current, day].sort((a, b) => a - b);
  if (next.length === 0) {
    next = [day];
  }
  return { ...rule, weekdays: next };
}

export const RECURRENCE_FREQUENCIES: RecurrenceFrequency[] = [
  "daily",
  "weekdays",
  "weekly",
  "monthly",
  "everyNDays",
  "everyNWeeks",
];
