export function localTodayString(): string {
  const d = new Date();
  const year = d.getFullYear();
  const month = String(d.getMonth() + 1).padStart(2, "0");
  const day = String(d.getDate()).padStart(2, "0");
  return `${year}-${month}-${day}`;
}

export function addDays(dateStr: string, days: number): string {
  const [y, m, d] = dateStr.split("-").map(Number);
  const dt = new Date(y, m - 1, d);
  dt.setDate(dt.getDate() + days);
  const year = dt.getFullYear();
  const month = String(dt.getMonth() + 1).padStart(2, "0");
  const day = String(dt.getDate()).padStart(2, "0");
  return `${year}-${month}-${day}`;
}

export type FollowUpPreset = {
  label: string;
  value: string;
};

export function followUpPresets(today = localTodayString()): FollowUpPreset[] {
  return [
    { label: "今天", value: today },
    { label: "明天", value: addDays(today, 1) },
    { label: "3 天后", value: addDays(today, 3) },
    { label: "下周", value: addDays(today, 7) },
  ];
}

export function formatWaitingHint(
  waitingFor: string | null,
  followUpDate: string | null,
): string {
  if (!waitingFor && !followUpDate) {
    return "等待中的任务不会出现在活跃列表，直到跟进日到期。";
  }
  const parts: string[] = [];
  if (waitingFor) parts.push(`等待：${waitingFor}`);
  if (followUpDate) parts.push(`跟进日 ${followUpDate}`);
  return parts.join(" · ");
}

export function followUpDueWarning(
  dueDate: string | null | undefined,
  followUpDate: string | null,
): string | null {
  if (!dueDate || !followUpDate || followUpDate <= dueDate) return null;
  return "跟进日晚于截止日期，到期时任务仍在等待中。";
}
