export function toDateString(d: Date): string {
  const year = d.getFullYear();
  const month = String(d.getMonth() + 1).padStart(2, "0");
  const day = String(d.getDate()).padStart(2, "0");
  return `${year}-${month}-${day}`;
}

export function localTodayString(): string {
  return toDateString(new Date());
}

export function addDays(dateStr: string, days: number): string {
  const [y, m, d] = dateStr.split("-").map(Number);
  const dt = new Date(y, m - 1, d);
  dt.setDate(dt.getDate() + days);
  return toDateString(dt);
}

/** Next Monday strictly after `from` (if Monday, +7 days). */
export function nextMonday(from = localTodayString()): string {
  const [y, m, d] = from.split("-").map(Number);
  const dt = new Date(y, m - 1, d);
  const weekday = dt.getDay();
  const delta = weekday === 0 ? 1 : weekday === 1 ? 7 : 8 - weekday;
  dt.setDate(dt.getDate() + delta);
  return toDateString(dt);
}

export type DeferPreset = { label: string; value: string | null };

export function deferPresets(today = localTodayString()): DeferPreset[] {
  return [
    { label: "取消推迟", value: null },
    { label: "明天", value: addDays(today, 1) },
    { label: "下周一", value: nextMonday(today) },
    { label: "下周（7 天）", value: addDays(today, 7) },
  ];
}

export function formatDeferHint(availableAt: string | null): string {
  if (!availableAt) return "立即在列表中显示";
  return `推迟至 ${availableAt} 再显示`;
}

export const DEFER_COACH_KEY = "trove.coach.defer-vs-postpone";
