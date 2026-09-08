import { useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { ipc, type Task } from "@/ipc/client";
import { Button } from "@/design-system/primitives/Button";
import { ConfirmButton } from "@/design-system/patterns/ConfirmButton";

/**
 * v2.1 subtasks: direct children of a task, rendered in the detail panel.
 * Children are full tasks; clicking a row opens that child's own detail.
 * Completed/archived parents freeze the section (read-only), matching the
 * checklist convention.
 */
export function SubtaskSection({
  task,
  onOpenTask,
}: {
  task: Task;
  /** Open a child task's own detail panel (switches the selected task). */
  onOpenTask?: (taskId: string) => void;
}) {
  const queryClient = useQueryClient();
  const [draft, setDraft] = useState("");
  const [editingId, setEditingId] = useState<string | null>(null);
  const [editingText, setEditingText] = useState("");
  const [deleteTarget, setDeleteTarget] = useState<Task | null>(null);
  const [deleteDisposition, setDeleteDisposition] = useState<
    "cascade" | "promote" | null
  >(null);

  const frozen = task.status === "completed" || task.status === "archived";

  const childrenQuery = useQuery({
    queryKey: ["tasks", "subtasks", task.id],
    queryFn: () => ipc.taskQuery({ parentId: task.id, limit: 100 }),
    enabled: !!task,
  });
  const children = childrenQuery.data?.items ?? [];
  const total = childrenQuery.data?.total ?? 0;
  const done = children.filter((c) => c.status === "completed").length;

  const invalidate = () => {
    void queryClient.invalidateQueries({
      queryKey: ["tasks", "subtasks", task.id],
    });
    void queryClient.invalidateQueries({ queryKey: ["tasks"] });
    void queryClient.invalidateQueries({ queryKey: ["task-tree-expanded"] });
  };

  const addMutation = useMutation({
    mutationFn: (title: string) =>
      ipc.taskCreate({ title, parentId: task.id, listId: task.listId }),
    onSuccess: () => {
      setDraft("");
      invalidate();
    },
  });

  const toggleMutation = useMutation({
    mutationFn: (child: Task) =>
      child.status === "completed"
        ? ipc.taskUncomplete(child.id)
        : ipc.taskComplete(child.id),
    onSuccess: () => invalidate(),
  });

  const renameMutation = useMutation({
    mutationFn: (input: { id: string; title: string }) =>
      ipc.taskUpdate({
        id: input.id,
        title: input.title,
        notes: "",
        priority: "none",
        listId: task.listId,
        dueDate: null,
        dueTime: null,
        tagNames: [],
      }),
    onSuccess: () => invalidate(),
  });

  const deleteTreeMutation = useMutation({
    mutationFn: (input: { id: string; disposition: "cascade" | "promote" }) =>
      ipc.taskDeleteTree(input.id, input.disposition),
    onSuccess: () => {
      setDeleteTarget(null);
      setDeleteDisposition(null);
      invalidate();
    },
  });

  const reorderMutation = useMutation({
    mutationFn: (orderedIds: string[]) =>
      ipc.taskReorderSubtasks(task.id, orderedIds),
    onSuccess: () => invalidate(),
  });

  const move = (index: number, delta: -1 | 1) => {
    const next = [...children];
    const target = index + delta;
    if (target < 0 || target >= next.length) return;
    [next[index], next[target]] = [next[target], next[index]];
    reorderMutation.mutate(next.map((c) => c.id));
  };

  const startDelete = async (child: Task) => {
    // Only show the disposition dialog when the child has its own children.
    try {
      const grand = await ipc.taskQuery({ parentId: child.id, limit: 1 });
      if ((grand.total ?? 0) === 0) {
        deleteTreeMutation.mutate({ id: child.id, disposition: "cascade" });
        return;
      }
    } catch {
      // Fall back to the dialog on query failure.
    }
    setDeleteTarget(child);
    setDeleteDisposition(null);
  };

  return (
    <section className="space-y-2">
      <div className="flex items-center justify-between">
        <h3 className="text-[12px] font-semibold">
          子任务
          {total > 0 ? (
            <span className="ml-1 text-muted">
              {done}/{total}
            </span>
          ) : null}
        </h3>
      </div>

      {children.length === 0 && frozen ? (
        <p className="text-[11px] text-muted">无子任务。</p>
      ) : null}

      <ul className="space-y-1">
        {children.map((child, index) => (
          <li key={child.id} className="flex items-center gap-2 text-[12px]">
            <input
              type="checkbox"
              className="mt-0"
              checked={child.status === "completed"}
              disabled={frozen || toggleMutation.isPending}
              onChange={() => toggleMutation.mutate(child)}
              aria-label={`勾选完成子任务：${child.title}`}
            />
            {editingId === child.id ? (
              <input
                className="min-w-0 flex-1 rounded border border-border bg-surface px-2 py-1"
                value={editingText}
                autoFocus
                onChange={(e) => setEditingText(e.target.value)}
                onBlur={() => {
                  const next = editingText.trim();
                  if (next && next !== child.title) {
                    renameMutation.mutate({ id: child.id, title: next });
                  }
                  setEditingId(null);
                }}
                onKeyDown={(e) => {
                  if (e.key === "Enter") e.currentTarget.blur();
                  if (e.key === "Escape") setEditingId(null);
                }}
              />
            ) : (
              <button
                type="button"
                className="min-w-0 flex-1 truncate text-left"
                title={child.title}
                disabled={frozen}
                onClick={() => onOpenTask?.(child.id)}
                onDoubleClick={(e) => {
                  e.stopPropagation();
                  if (!frozen) {
                    setEditingId(child.id);
                    setEditingText(child.title);
                  }
                }}
              >
                <span
                  className={
                    child.status === "completed" ? "text-muted line-through" : ""
                  }
                >
                  {child.title}
                </span>
              </button>
            )}
            {!frozen ? (
              <span className="flex shrink-0 items-center gap-0.5">
                <Button
                  size="sm"
                  variant="ghost"
                  disabled={index === 0}
                  onClick={() => move(index, -1)}
                  aria-label="上移"
                >
                  ↑
                </Button>
                <Button
                  size="sm"
                  variant="ghost"
                  disabled={index === children.length - 1}
                  onClick={() => move(index, 1)}
                  aria-label="下移"
                >
                  ↓
                </Button>
                <ConfirmButton
                  size="sm"
                  confirmLabel="确认删除"
                  resetKey={child.id}
                  onConfirm={() => startDelete(child)}
                >
                  删除
                </ConfirmButton>
              </span>
            ) : null}
          </li>
        ))}
      </ul>

      {!frozen ? (
        <form
          className="flex gap-2"
          onSubmit={(e) => {
            e.preventDefault();
            const title = draft.trim();
            if (title) addMutation.mutate(title);
          }}
        >
          <input
            className="min-w-0 flex-1 rounded border border-border bg-surface px-2 py-1 text-[12px]"
            value={draft}
            placeholder="添加子任务，回车保存"
            onChange={(e) => setDraft(e.target.value)}
            aria-label="新子任务标题"
          />
          <Button
            type="submit"
            size="sm"
            variant="secondary"
            disabled={!draft.trim() || addMutation.isPending}
          >
            添加
          </Button>
        </form>
      ) : (
        <p className="text-[11px] text-muted">任务已完成，子任务只读。</p>
      )}

      {addMutation.isError ? (
        <p className="text-[11px] text-warning">{String(addMutation.error)}</p>
      ) : null}

      {deleteTarget ? (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/30 p-4">
          <div
            className="w-full max-w-sm rounded-[var(--radius-panel)] border border-border bg-surface p-4 shadow-lg"
            onClick={(e) => e.stopPropagation()}
          >
            <h3 className="text-[13px] font-medium text-foreground">
              删除子任务「{deleteTarget.title}」
            </h3>
            <p className="mt-2 text-[12px] text-muted">
              该子任务可能还有下级子任务，请选择处理方式：
            </p>
            <div className="mt-4 flex flex-col gap-2">
              <Button
                size="sm"
                variant="secondary"
                disabled={deleteTreeMutation.isPending}
                onClick={() => {
                  setDeleteDisposition("cascade");
                  deleteTreeMutation.mutate({
                    id: deleteTarget.id,
                    disposition: "cascade",
                  });
                }}
              >
                级联删除（含所有下级）
              </Button>
              <Button
                size="sm"
                variant="secondary"
                disabled={deleteTreeMutation.isPending}
                onClick={() => {
                  setDeleteDisposition("promote");
                  deleteTreeMutation.mutate({
                    id: deleteTarget.id,
                    disposition: "promote",
                  });
                }}
              >
                仅删除该子任务（下级提升）
              </Button>
              <Button
                size="sm"
                variant="ghost"
                onClick={() => setDeleteTarget(null)}
              >
                取消
              </Button>
            </div>
            {deleteDisposition === "promote" && deleteTarget ? (
              <p className="mt-2 text-[11px] text-warning">
                子任务的子任务将提升为当前任务的直接子任务。
              </p>
            ) : null}
          </div>
        </div>
      ) : null}
    </section>
  );
}
