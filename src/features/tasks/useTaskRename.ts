import { useCallback } from "react";
import { useQueryClient } from "@tanstack/react-query";
import { ipc, type Task } from "@/ipc/client";

/**
 * Deep-replace the title of the task with `id` wherever it appears in a cached
 * query result. Handles the varied task-list shapes (flat `Task[]`, the nested
 * `TodayTasks` object, etc.) by recursing into arrays and objects and patching
 * any node whose `id` matches and exposes a string `title`. Returns the same
 * reference when nothing changed so React Query can skip unaffected entries.
 */
function patchTaskTitle(data: unknown, id: string, title: string): unknown {
  if (Array.isArray(data)) {
    let changed = false;
    const next = data.map((item) => {
      const patched = patchTaskTitle(item, id, title);
      if (patched !== item) changed = true;
      return patched;
    });
    return changed ? next : data;
  }
  if (data && typeof data === "object") {
    const obj = data as Record<string, unknown>;
    if (obj.id === id && typeof obj.title === "string") {
      return { ...obj, title };
    }
    let changed = false;
    const next: Record<string, unknown> = {};
    for (const [key, value] of Object.entries(obj)) {
      const patched = patchTaskTitle(value, id, title);
      if (patched !== value) changed = true;
      next[key] = patched;
    }
    return changed ? next : data;
  }
  return data;
}

/**
 * Rename a task with an optimistic cache update: every cached task list shows
 * the new title immediately, then the change is persisted. On success the
 * backend's `domain://changed` event triggers a refetch (via `useDomainInvalidation`)
 * that reconciles the cache; on failure we invalidate to revert to server truth.
 *
 * `update_task` rewrites the whole row, so the full task is spread and only the
 * title is changed — this preserves notes / priority / due date / list / tags.
 */
export function useTaskRename() {
  const queryClient = useQueryClient();
  return useCallback(
    async (task: Task, title: string) => {
      const trimmed = title.trim();
      if (!trimmed || trimmed === task.title) return;
      queryClient.setQueriesData(
        { queryKey: ["tasks"] },
        (old: unknown) => patchTaskTitle(old, task.id, trimmed),
      );
      try {
        await ipc.taskUpdate({
          id: task.id,
          title: trimmed,
          notes: task.notes,
          priority: task.priority,
          listId: task.listId,
          dueDate: task.dueDate,
          dueTime: task.dueTime,
          tagNames: task.tagNames,
        });
      } catch (err) {
        // Revert the optimistic update by refetching from the server.
        void queryClient.invalidateQueries({ queryKey: ["tasks"] });
        throw err;
      }
    },
    [queryClient],
  );
}
