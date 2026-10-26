import type {
  ListGroupScope,
  TaskListGroupOverview,
} from "@/ipc/client";

/**
 * 任务分组（分组 → 清单两级）的纯映射函数集合（unit-testable）。
 *
 * 侧边栏与今日页 chips 都从 `TaskListGroupOverview` 出发展示：
 * - 侧边栏任务区行序 = 各分组（可折叠）→ 未分组清单平铺；
 * - 今日页 chips = 全部 + 各分组 + 未分组（有内容才出现）。
 */

export type SidebarListRow = {
  kind: "list";
  id: string;
  name: string;
  groupId: string | null;
  openCount: number;
};

export type SidebarGroupRow = {
  kind: "group";
  id: string;
  name: string;
  openCount: number;
  lists: SidebarListRow[];
};

export type SidebarTaskRow = SidebarGroupRow | SidebarListRow;

/** 侧边栏任务区行（分组在前，未分组清单平铺在后；收件箱独立于本列表）。 */
export function buildSidebarRows(
  overview: TaskListGroupOverview,
): SidebarTaskRow[] {
  const groupRows: SidebarTaskRow[] = overview.groups.map((node) => ({
    kind: "group",
    id: node.group.id,
    name: node.group.name,
    openCount: node.openCount,
    lists: node.lists.map((list) => ({
      kind: "list" as const,
      id: list.id,
      name: list.name,
      groupId: list.groupId,
      openCount: list.openCount,
    })),
  }));
  const ungroupedRows: SidebarTaskRow[] = overview.ungrouped.map((list) => ({
    kind: "list" as const,
    id: list.id,
    name: list.name,
    groupId: null,
    openCount: list.openCount,
  }));
  return [...groupRows, ...ungroupedRows];
}

/** 今日页过滤 chips：全部 + 各分组 + 未分组（未分组 chip 恒展示，收件箱任务归入未分组）。 */
export function buildTodayScopes(overview: TaskListGroupOverview): {
  scope: ListGroupScope | null;
  label: string;
}[] {
  const scopes: { scope: ListGroupScope | null; label: string }[] = [
    { scope: null, label: "全部" },
  ];
  for (const node of overview.groups) {
    scopes.push({
      scope: { kind: "group", groupId: node.group.id },
      label: node.group.name,
    });
  }
  scopes.push({ scope: { kind: "ungrouped" }, label: "未分组" });
  return scopes;
}

/** 从 overview 中查找清单名（含收件箱），找不到时返回回退文案。 */
export function listNameInOverview(
  overview: TaskListGroupOverview | undefined,
  listId: string,
  fallback = "任务",
): string {
  if (!overview) return fallback;
  if (overview.inbox.id === listId) return overview.inbox.name;
  for (const node of overview.groups) {
    const hit = node.lists.find((l) => l.id === listId);
    if (hit) return hit.name;
  }
  const ungrouped = overview.ungrouped.find((l) => l.id === listId);
  return ungrouped?.name ?? fallback;
}
