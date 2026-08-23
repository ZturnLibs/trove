import { useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { ipc, type Task } from "@/ipc/client";
import { Button } from "@/design-system/primitives/Button";
import { ConfirmButton } from "@/design-system/patterns/ConfirmButton";

/**
 * v2.0 slice 6: one-level task checklist. Completed tasks freeze the list to
 * read-only; checking every item never auto-completes the task.
 */
export function ChecklistSection({ task }: { task: Task }) {
  const queryClient = useQueryClient();
  const [draft, setDraft] = useState("");
  const [editingId, setEditingId] = useState<string | null>(null);
  const [editingText, setEditingText] = useState("");

  const frozen = task.status === "completed";

  const checklistQuery = useQuery({
    queryKey: ["tasks", "checklist", task.id],
    queryFn: () => ipc.taskChecklistList(task.id),
  });
  const items = checklistQuery.data?.items ?? [];

  const invalidate = () => {
    void queryClient.invalidateQueries({ queryKey: ["tasks", "checklist", task.id] });
    void queryClient.invalidateQueries({ queryKey: ["tasks"] });
  };

  const addMutation = useMutation({
    mutationFn: (content: string) => ipc.taskChecklistAdd(task.id, content),
    onSuccess: () => {
      setDraft("");
      invalidate();
    },
  });
  const updateMutation = useMutation({
    mutationFn: (input: { id: string; content?: string; checked?: boolean }) =>
      ipc.taskChecklistUpdate(input),
    onSuccess: () => invalidate(),
  });
  const deleteMutation = useMutation({
    mutationFn: (id: string) => ipc.taskChecklistDelete(id),
    onSuccess: () => invalidate(),
  });
  const reorderMutation = useMutation({
    mutationFn: (orderedIds: string[]) => ipc.taskChecklistReorder(task.id, orderedIds),
    onSuccess: () => invalidate(),
  });

  const move = (index: number, delta: -1 | 1) => {
    const next = [...items];
    const target = index + delta;
    if (target < 0 || target >= next.length) return;
    [next[index], next[target]] = [next[target], next[index]];
    reorderMutation.mutate(next.map((i) => i.id));
  };

  return (
    <section className="space-y-2">
      <h3 className="text-[12px] font-semibold">
        检查项
        {items.length > 0 ? (
          <span className="ml-1 text-muted">
            {items.filter((i) => i.checked).length}/{items.length}
          </span>
        ) : null}
      </h3>

      {items.length === 0 && frozen ? (
        <p className="text-[11px] text-muted">无检查项。</p>
      ) : null}

      <ul className="space-y-1">
        {items.map((item, index) => (
          <li key={item.id} className="flex items-center gap-2 text-[12px]">
            <input
              type="checkbox"
              className="mt-0"
              checked={item.checked}
              disabled={frozen || updateMutation.isPending}
              onChange={(e) =>
                updateMutation.mutate({ id: item.id, checked: e.target.checked })
              }
              aria-label={`勾选检查项：${item.content}`}
            />
            {editingId === item.id ? (
              <input
                className="min-w-0 flex-1 rounded border border-border bg-surface px-2 py-1"
                value={editingText}
                autoFocus
                onChange={(e) => setEditingText(e.target.value)}
                onBlur={() => {
                  const next = editingText.trim();
                  if (next && next !== item.content) {
                    updateMutation.mutate({ id: item.id, content: next });
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
                disabled={frozen}
                title={item.content}
                onClick={() => {
                  setEditingId(item.id);
                  setEditingText(item.content);
                }}
              >
                <span className={item.checked ? "text-muted line-through" : ""}>
                  {item.content}
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
                  disabled={index === items.length - 1}
                  onClick={() => move(index, 1)}
                  aria-label="下移"
                >
                  ↓
                </Button>
                <ConfirmButton
                  size="sm"
                  confirmLabel="确认删除"
                  resetKey={item.id}
                  onConfirm={() => deleteMutation.mutate(item.id)}
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
            const content = draft.trim();
            if (content) addMutation.mutate(content);
          }}
        >
          <input
            className="min-w-0 flex-1 rounded border border-border bg-surface px-2 py-1 text-[12px]"
            value={draft}
            placeholder="添加检查项，回车保存"
            onChange={(e) => setDraft(e.target.value)}
            aria-label="新检查项内容"
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
        <p className="text-[11px] text-muted">任务已完成，检查项只读。</p>
      )}

      {addMutation.isError ? (
        <p className="text-[11px] text-warning">{String(addMutation.error)}</p>
      ) : null}
    </section>
  );
}

/** Progress badge for task rows: `2/5` when a checklist exists. */
export function ChecklistBadge({ taskId }: { taskId: string }) {
  const checklistQuery = useQuery({
    queryKey: ["tasks", "checklist", taskId],
    queryFn: () => ipc.taskChecklistList(taskId),
    staleTime: 30_000,
  });
  const total = checklistQuery.data?.total ?? 0;
  if (total === 0) return null;
  const done = checklistQuery.data?.checkedCount ?? 0;
  return (
    <span className="ml-1 rounded border border-border px-1 text-[10px] text-muted">
      {done}/{total}
    </span>
  );
}
