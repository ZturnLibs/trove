import { useEffect, useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Button } from "@/design-system/primitives/Button";
import { Input } from "@/design-system/primitives/Input";
import {
  ipc,
  type Task,
  type TaskPriority,
  type UpdateTaskInput,
} from "@/ipc/client";

export function TaskDetailPanel({
  task,
  onDeleted,
}: {
  task: Task | null;
  onDeleted?: () => void;
}) {
  const queryClient = useQueryClient();
  const listsQuery = useQuery({
    queryKey: ["task-lists"],
    queryFn: () => ipc.taskListLists(),
  });

  const [draft, setDraft] = useState<UpdateTaskInput | null>(null);
  const [tagText, setTagText] = useState("");
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!task) {
      setDraft(null);
      return;
    }
    setDraft({
      id: task.id,
      title: task.title,
      notes: task.notes,
      priority: task.priority,
      listId: task.listId,
      dueDate: task.dueDate,
      dueTime: task.dueTime,
      tagNames: [...task.tagNames],
    });
    setTagText(task.tagNames.join(", "));
    setError(null);
  }, [task]);

  const saveMutation = useMutation({
    mutationFn: (input: UpdateTaskInput) => ipc.taskUpdate(input),
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: ["tasks"] });
      setError(null);
    },
    onError: (err: Error) => setError(err.message || "保存失败"),
  });

  const completeMutation = useMutation({
    mutationFn: async () => {
      if (!task) return;
      if (task.status === "completed") await ipc.taskUncomplete(task.id);
      else await ipc.taskComplete(task.id);
    },
    onSuccess: () => void queryClient.invalidateQueries({ queryKey: ["tasks"] }),
  });

  const archiveMutation = useMutation({
    mutationFn: () => ipc.taskArchive(task!.id),
    onSuccess: () => void queryClient.invalidateQueries({ queryKey: ["tasks"] }),
  });

  const deleteMutation = useMutation({
    mutationFn: () => ipc.taskDelete(task!.id),
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: ["tasks"] });
      onDeleted?.();
    },
  });

  if (!task || !draft) {
    return (
      <div className="flex h-full items-center justify-center p-6 text-[12px] text-muted">
        选择一项查看详情
      </div>
    );
  }

  const save = () => {
    const tagNames = tagText
      .split(/[,，]/)
      .map((t) => t.trim())
      .filter(Boolean);
    saveMutation.mutate({ ...draft, tagNames });
  };

  return (
    <div className="flex h-full min-h-0 flex-col">
      <div className="flex items-center justify-between border-b border-border px-3 py-2 text-[11px] text-muted">
        <span>任务 · {task.updatedAt.slice(0, 16).replace("T", " ")}</span>
        <span>{task.listName}</span>
      </div>

      <div className="min-h-0 flex-1 space-y-3 overflow-auto p-3">
        <Input
          value={draft.title}
          onChange={(e) => setDraft({ ...draft, title: e.target.value })}
          onBlur={save}
          className="h-9 text-[14px] font-medium"
        />

        <div className="grid grid-cols-2 gap-2">
          <label className="space-y-1 text-[11px] text-muted">
            截止日期
            <Input
              type="date"
              value={draft.dueDate ?? ""}
              onChange={(e) =>
                setDraft({
                  ...draft,
                  dueDate: e.target.value || null,
                })
              }
              onBlur={save}
            />
          </label>
          <label className="space-y-1 text-[11px] text-muted">
            截止时间
            <Input
              type="time"
              value={draft.dueTime ?? ""}
              onChange={(e) =>
                setDraft({
                  ...draft,
                  dueTime: e.target.value || null,
                })
              }
              onBlur={save}
            />
          </label>
          <label className="space-y-1 text-[11px] text-muted">
            清单
            <select
              className="h-8 w-full rounded-[var(--radius-control)] border border-border bg-surface-raised px-2 text-[13px] text-foreground"
              value={draft.listId}
              onChange={(e) => {
                const next = { ...draft, listId: e.target.value };
                setDraft(next);
                saveMutation.mutate({
                  ...next,
                  tagNames: tagText
                    .split(/[,，]/)
                    .map((t) => t.trim())
                    .filter(Boolean),
                });
              }}
            >
              {(listsQuery.data ?? []).map((list) => (
                <option key={list.id} value={list.id}>
                  {list.name}
                </option>
              ))}
            </select>
          </label>
          <label className="space-y-1 text-[11px] text-muted">
            优先级
            <select
              className="h-8 w-full rounded-[var(--radius-control)] border border-border bg-surface-raised px-2 text-[13px] text-foreground"
              value={draft.priority}
              onChange={(e) => {
                const priority = e.target.value as TaskPriority;
                const next = { ...draft, priority };
                setDraft(next);
                saveMutation.mutate({
                  ...next,
                  tagNames: tagText
                    .split(/[,，]/)
                    .map((t) => t.trim())
                    .filter(Boolean),
                });
              }}
            >
              <option value="none">无</option>
              <option value="low">低</option>
              <option value="medium">中</option>
              <option value="high">高</option>
            </select>
          </label>
        </div>

        <label className="block space-y-1 text-[11px] text-muted">
          标签（逗号分隔）
          <Input
            value={tagText}
            onChange={(e) => setTagText(e.target.value)}
            onBlur={save}
            placeholder="工作, 个人"
          />
        </label>

        <label className="block space-y-1 text-[11px] text-muted">
          说明
          <textarea
            value={draft.notes}
            onChange={(e) => setDraft({ ...draft, notes: e.target.value })}
            onBlur={save}
            rows={8}
            className="w-full resize-none rounded-[var(--radius-control)] border border-border bg-surface-raised p-2 text-[13px] text-foreground outline-none focus:ring-2 focus:ring-accent/35"
          />
        </label>

        {error ? <p className="text-[12px] text-danger">{error}</p> : null}
      </div>

      <div className="flex items-center justify-between gap-2 border-t border-border p-3">
        <div className="flex gap-1">
          <Button
            size="sm"
            variant="ghost"
            onClick={() => {
              if (confirm("确认归档此任务？")) archiveMutation.mutate();
            }}
          >
            归档
          </Button>
          <Button
            size="sm"
            variant="ghost"
            onClick={() => {
              if (confirm("确认删除此任务？")) deleteMutation.mutate();
            }}
          >
            删除
          </Button>
        </div>
        <Button
          size="sm"
          onClick={() => completeMutation.mutate()}
          disabled={completeMutation.isPending}
        >
          {task.status === "completed" ? "恢复待办" : "完成"}
        </Button>
      </div>
    </div>
  );
}
