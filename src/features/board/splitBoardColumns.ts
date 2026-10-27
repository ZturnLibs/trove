import { arrayMove } from "@dnd-kit/sortable";
import type {
  ListGroupScope,
  Task,
  TaskListGroupOverview,
} from "@/ipc/client";

/**
 * 看板三列的纯分流函数（unit-testable，沿用 listGroupTree.ts 模式）。
 *
 * 输入是两次 task_query 的结果：
 * - todo 查询（status=todo，统一活跃定义已由后端过滤 waiting/deferred）
 *   按客户端 `boardStage` 分流「待办 / 进行中」两列；
 * - completed 查询（completedSince=今天）直接作为「已完成」列。
 *
 * 分组过滤：group 作用域由服务端 `listGroupId` 参数过滤（透传，不在此重复）；
 * ungrouped 作用域服务端无对应参数，这里按 listId → groupId 映射客户端过滤。
 * 各列保持查询返回的既有排序（清单内 sort_order → 创建时间 / 完成时间倒序）。
 */

export type BoardColumns = {
  todo: Task[];
  doing: Task[];
  done: Task[];
};

export function splitBoardColumns(
  todoTasks: Task[],
  completedTasks: Task[],
  scope: ListGroupScope | null,
  groupByListId: ReadonlyMap<string, string | null>,
): BoardColumns {
  const inScope = (task: Task): boolean => {
    if (scope?.kind === "ungrouped") {
      // 未分组 = 清单不属于任何分组（收件箱视为未分组）。
      return groupByListId.get(task.listId) === null;
    }
    // null = 全部；group = 服务端已按 listGroupId 过滤。
    return true;
  };

  const todo: Task[] = [];
  const doing: Task[] = [];
  for (const task of todoTasks) {
    // status 守卫：乐观补丁（完成/取消完成）会把行短暂改成终态，
    // 必须立即将其移出所在列——否则卡片带删除线滞留到 refetch 返回。
    if (task.status !== "todo") continue;
    if (!inScope(task)) continue;
    if (task.boardStage === "doing") doing.push(task);
    else todo.push(task);
  }
  const done = completedTasks.filter(
    (task) => task.status === "completed" && inScope(task),
  );
  return { todo, doing, done };
}

/** listId → groupId 映射（null = 未分组；收件箱恒未分组），供 ungrouped 过滤用。 */
export function buildGroupByListId(
  overview: TaskListGroupOverview,
): Map<string, string | null> {
  const map = new Map<string, string | null>();
  map.set(overview.inbox.id, null);
  for (const node of overview.groups) {
    for (const list of node.lists) {
      map.set(list.id, node.group.id);
    }
  }
  for (const list of overview.ungrouped) {
    map.set(list.id, null);
  }
  return map;
}

/**
 * 列内新建的默认清单：分组作用域 → 该组首个清单（新建任务在当前 chips 过滤下可见）；
 * 空组/全部/未分组 → null（后端默认收件箱）。
 */
export function defaultListIdForScope(
  scope: ListGroupScope | null,
  overview: TaskListGroupOverview,
): string | null {
  if (scope?.kind !== "group") return null;
  const node = overview.groups.find((g) => g.group.id === scope.groupId);
  return node?.lists[0]?.id ?? null;
}

/**
 * 同清单拖动排序：拖拽卡落在同清单卡上时，返回该清单在列内的完整新 id 序列
 * （交给 `task_reorder` 按清单分组重写 sort_order，其他清单不受影响）。
 * 落点是其他清单的卡片 / 找不到卡 / 该清单不足两张 → null（无清单内排序语义，不动作）。
 */
export function sameListReorderIds(
  columnTasks: Task[],
  activeId: string,
  overId: string,
): string[] | null {
  const from = columnTasks.findIndex((t) => t.id === activeId);
  const to = columnTasks.findIndex((t) => t.id === overId);
  if (from === -1 || to === -1 || from === to) return null;
  const moved = columnTasks[from];
  const segment = columnTasks.filter((t) => t.listId === moved.listId);
  if (segment.length < 2) return null;
  const segFrom = segment.findIndex((t) => t.id === activeId);
  const segTo = segment.findIndex((t) => t.id === overId);
  if (segTo === -1) return null;
  return arrayMove(segment, segFrom, segTo).map((t) => t.id);
}
