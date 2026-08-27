import type { Task } from "@/ipc/client";

/**
 * v2.1 subtask tree helpers (pure, unit-testable).
 *
 * The list view renders a forest assembled from a flat `Task[]` returned by
 * `task_query_tree`. Roots are tasks whose `parentId` is null (or whose parent
 * is absent from the set); children are grouped by `parentId` and ordered by
 * `childOrder`.
 */

export type TreeNode = {
  task: Task;
  depth: number;
  /** Direct children, sorted by childOrder. */
  children: TreeNode[];
};

export type TaskForest = {
  /** All roots, sorted by (sortOrder, createdAt). */
  roots: TreeNode[];
  /** taskId -> direct children (sorted by childOrder). */
  childrenOf: Map<string, Task[]>;
  /** taskId -> number of direct children. */
  childCount: Map<string, number>;
  /** taskId -> { done, total } direct-child completion counts. */
  progress: Map<string, { done: number; total: number }>;
};

function sortTasks(a: Task, b: Task): number {
  const byOrder = (a.childOrder ?? 0) - (b.childOrder ?? 0);
  if (byOrder !== 0) return byOrder;
  return a.createdAt < b.createdAt ? -1 : a.createdAt > b.createdAt ? 1 : 0;
}

export function buildForest(tasks: Task[]): TaskForest {
  const byParent = new Map<string | null, Task[]>();
  const byId = new Map<string, Task>();
  for (const t of tasks) {
    byId.set(t.id, t);
    const key = t.parentId;
    const list = byParent.get(key);
    if (list) list.push(t);
    else byParent.set(key, [t]);
  }
  for (const list of byParent.values()) list.sort(sortTasks);

  const childrenOf = new Map<string, Task[]>();
  const childCount = new Map<string, number>();
  const progress = new Map<string, { done: number; total: number }>();
  for (const [parentId, list] of byParent) {
    if (parentId == null) continue;
    childrenOf.set(parentId, list);
    childCount.set(parentId, list.length);
    let done = 0;
    for (const t of list) if (t.status === "completed") done += 1;
    progress.set(parentId, { done, total: list.length });
  }

  const roots = (byParent.get(null) ?? [])
    .filter((t) => byId.has(t.id))
    .sort((a, b) => {
      const byOrder = a.sortOrder - b.sortOrder;
      if (byOrder !== 0) return byOrder;
      return a.createdAt < b.createdAt ? -1 : a.createdAt > b.createdAt ? 1 : 0;
    })
    .map((t) => buildNode(t, 0, byParent));

  // A task whose parent is soft-deleted (absent from the set) is rendered as
  // a root so it never disappears from the tree.
  const orphanRoots: Task[] = [];
  for (const [parentId, list] of byParent) {
    if (parentId != null && !byId.has(parentId)) orphanRoots.push(...list);
  }
  if (orphanRoots.length > 0) {
    orphanRoots.sort((a, b) => a.sortOrder - b.sortOrder);
    roots.push(...orphanRoots.map((t) => buildNode(t, 0, byParent)));
  }

  return { roots, childrenOf, childCount, progress };
}

function buildNode(
  task: Task,
  depth: number,
  byParent: Map<string | null, Task[]>,
): TreeNode {
  const children = (byParent.get(task.id) ?? [])
    .sort(sortTasks)
    .map((c) => buildNode(c, depth + 1, byParent));
  return { task, depth, children };
}

export type VisibleRow = { task: Task; depth: number; hasChildren: boolean };

/**
 * Flatten the forest into the currently visible row list, honoring the
 * expanded set (missing taskId = expanded by default).
 */
export function flattenVisible(
  forest: TaskForest,
  expanded: ReadonlySet<string>,
): VisibleRow[] {
  const out: VisibleRow[] = [];
  const visit = (node: TreeNode) => {
    out.push({
      task: node.task,
      depth: node.depth,
      hasChildren: node.children.length > 0,
    });
    if (node.children.length === 0) return;
    // Backend stores only collapsed rows (default = expanded).
    const collapsed = expanded.has(node.task.id);
    if (!collapsed) {
      for (const child of node.children) visit(child);
    }
  };
  for (const root of forest.roots) visit(root);
  return out;
}

/** Flatten all siblings of a task under the same parent into an id list. */
export function siblingIds(
  forest: TaskForest,
  taskId: string,
): string[] | null {
  for (const list of forest.childrenOf.values()) {
    if (list.some((t) => t.id === taskId)) return list.map((t) => t.id);
  }
  if (forest.roots.some((r) => r.task.id === taskId)) {
    return forest.roots.map((r) => r.task.id);
  }
  return null;
}
