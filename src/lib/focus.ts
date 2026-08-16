export const FOCUS_MANY_COACH_KEY = "trove.coach.focus-many";

export function isTaskInFocus(focusIds: Set<string>, taskId: string): boolean {
  return focusIds.has(taskId);
}
