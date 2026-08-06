import { useEffect, useRef, useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { AttachmentsSection } from "@/design-system/patterns/AttachmentsSection";
import { Button } from "@/design-system/primitives/Button";
import { ConfirmButton } from "@/design-system/patterns/ConfirmButton";
import { Input } from "@/design-system/primitives/Input";
import {
  ipc,
  type Reminder,
  type Task,
  type TaskPriority,
  type UpdateReminderInput,
  type UpdateTaskInput,
} from "@/ipc/client";
import { useRecentActions } from "@/stores/recent-actions";

export function TaskDetailPanel({
  task,
  onDeleted,
  focusTitleId,
}: {
  task: Task | null;
  onDeleted?: () => void;
  /** When set to the task's id (e.g. right after "新建"), focus + select its title. */
  focusTitleId?: string | null;
}) {
  const queryClient = useQueryClient();
  const listsQuery = useQuery({
    queryKey: ["task-lists"],
    queryFn: () => ipc.taskListLists(),
  });

  const [draft, setDraft] = useState<UpdateTaskInput | null>(null);
  const [tagText, setTagText] = useState("");
  const [error, setError] = useState<string | null>(null);
  const titleRef = useRef<HTMLInputElement>(null);

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

  // Newly created task: focus + select the title so typing replaces "新任务".
  useEffect(() => {
    if (task && focusTitleId && task.id === focusTitleId) {
      requestAnimationFrame(() => {
        titleRef.current?.focus();
        titleRef.current?.select();
      });
    }
  }, [task, focusTitleId]);

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
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: ["tasks"] });
      if (!task) return;
      const wasCompleted = task.status === "completed";
      const taskId = task.id;
      useRecentActions.getState().push({
        label: wasCompleted ? "取消完成" : "完成",
        undo: async () => {
          if (wasCompleted) await ipc.taskComplete(taskId);
          else await ipc.taskUncomplete(taskId);
        },
      });
    },
  });

  const archiveMutation = useMutation({
    mutationFn: () => ipc.taskArchive(task!.id),
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: ["tasks"] });
      if (!task) return;
      const taskId = task.id;
      useRecentActions.getState().push({
        label: "归档",
        undo: async () => {
          await ipc.taskUnarchive(taskId);
        },
      });
    },
  });

  const skipMutation = useMutation({
    mutationFn: () => ipc.taskSkip(task!.id),
    onSuccess: () => void queryClient.invalidateQueries({ queryKey: ["tasks"] }),
  });

  const deleteMutation = useMutation({
    mutationFn: () => ipc.taskDelete(task!.id),
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: ["tasks"] });
      onDeleted?.();
      if (!task) return;
      // 删除撤销为重建（新 id）：快照当前任务字段，撤销时用 taskCreate 重建。
      const snapshot = {
        title: task.title,
        notes: task.notes,
        priority: task.priority,
        listId: task.listId,
        dueDate: task.dueDate,
        dueTime: task.dueTime,
        tagNames: [...task.tagNames],
      };
      useRecentActions.getState().push({
        label: "删除任务（重建）",
        undo: async () => {
          await ipc.taskCreate(snapshot);
        },
      });
    },
  });

  const linksQuery = useQuery({
    queryKey: ["links", "task", task?.id],
    queryFn: () => ipc.entityLinkList("task", task!.id),
    enabled: !!task,
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
          ref={titleRef}
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
            rows={6}
            className="w-full resize-none rounded-[var(--radius-control)] border border-border bg-surface-raised p-2 text-[13px] text-foreground outline-none focus:ring-2 focus:ring-accent/35"
          />
        </label>

        <TaskRemindersSection taskId={task.id} />

        <AttachmentsSection entityType="task" entityId={task.id} />

        {error ? <p className="text-[12px] text-danger">{error}</p> : null}
      </div>

      <div className="flex items-center justify-between gap-2 border-t border-border p-3">
        <div className="flex gap-1">
          {task.seriesId ? (
            <ConfirmButton
              size="sm"
              confirmLabel="确认跳过？"
              onConfirm={() => skipMutation.mutate()}
              resetKey={task.id}
            >
              跳过本次
            </ConfirmButton>
          ) : null}
          <ConfirmButton
            size="sm"
            confirmLabel="确认归档？"
            onConfirm={() => archiveMutation.mutate()}
            resetKey={task.id}
          >
            归档
          </ConfirmButton>
          <ConfirmButton
            size="sm"
            confirmLabel={
              (linksQuery.data?.length ?? 0) > 0
                ? `确认删除？(${(linksQuery.data ?? []).length} 关联)`
                : "确认删除？"
            }
            onConfirm={() => deleteMutation.mutate()}
            resetKey={task.id}
          >
            删除
          </ConfirmButton>
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

function TaskRemindersSection({ taskId }: { taskId: string }) {
  const queryClient = useQueryClient();
  const [fireAt, setFireAt] = useState("");
  const [recurring, setRecurring] = useState(false);
  const [editingId, setEditingId] = useState<string | null>(null);

  const remindersQuery = useQuery({
    queryKey: ["reminders", "task", taskId],
    queryFn: () => ipc.reminderListForTask(taskId),
  });

  const createMutation = useMutation({
    mutationFn: async () => {
      if (!fireAt) throw new Error("请选择提醒时间");
      const normalized = fireAt.length === 16 ? `${fireAt}:00` : fireAt;
      return ipc.reminderCreate({
        title: "任务提醒",
        taskId,
        fireAt: normalized.replace(" ", "T"),
        timezone: Intl.DateTimeFormat().resolvedOptions().timeZone,
        recurrence: recurring
          ? {
              version: 1,
              frequency: "daily",
              interval: 1,
              timezone: Intl.DateTimeFormat().resolvedOptions().timeZone,
            }
          : null,
      });
    },
    onSuccess: () => {
      setFireAt("");
      void queryClient.invalidateQueries({ queryKey: ["reminders"] });
      void queryClient.invalidateQueries({ queryKey: ["tasks"] });
    },
  });

  const deleteMutation = useMutation({
    mutationFn: (id: string) => ipc.reminderDelete(id),
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: ["reminders"] });
      void queryClient.invalidateQueries({ queryKey: ["tasks"] });
    },
  });

  return (
    <section className="space-y-2 border-t border-border pt-3">
      <div className="flex items-center justify-between">
        <h3 className="text-[12px] font-medium">提醒</h3>
        <label className="flex items-center gap-1 text-[11px] text-muted">
          <input
            type="checkbox"
            checked={recurring}
            onChange={(e) => setRecurring(e.target.checked)}
          />
          每天重复
        </label>
      </div>
      <div className="flex gap-2">
        <Input
          type="datetime-local"
          value={fireAt}
          onChange={(e) => setFireAt(e.target.value)}
        />
        <Button
          size="sm"
          variant="secondary"
          disabled={!fireAt || createMutation.isPending}
          onClick={() => createMutation.mutate()}
        >
          添加
        </Button>
      </div>
      <ul className="space-y-1">
        {(remindersQuery.data ?? []).map((reminder) => (
          <li key={reminder.id} className="space-y-1">
            <div className="flex items-center justify-between gap-2 text-[12px]">
              <span className="truncate">
                {reminder.nextFireAt.replace("T", " ")}
                {reminder.recurrence ? " · 周期" : ""}
                {!reminder.enabled ? " · 已停用" : ""}
              </span>
              <div className="flex shrink-0 gap-1">
                <Button
                  size="sm"
                  variant="ghost"
                  onClick={() =>
                    setEditingId((cur) =>
                      cur === reminder.id ? null : reminder.id,
                    )
                  }
                >
                  编辑
                </Button>
                <Button
                  size="sm"
                  variant="ghost"
                  onClick={() => deleteMutation.mutate(reminder.id)}
                >
                  删除
                </Button>
              </div>
            </div>
            {editingId === reminder.id ? (
              <TaskReminderEditRow
                key={reminder.id}
                reminder={reminder}
                onSaved={() => setEditingId(null)}
              />
            ) : null}
          </li>
        ))}
        {(remindersQuery.data?.length ?? 0) === 0 ? (
          <li className="text-[11px] text-muted">暂无提醒</li>
        ) : null}
      </ul>
    </section>
  );
}

function TaskReminderEditRow({
  reminder,
  onSaved,
}: {
  reminder: Reminder;
  onSaved?: () => void;
}) {
  const queryClient = useQueryClient();
  const [fireAt, setFireAt] = useState(reminder.nextFireAt.slice(0, 16));
  const [recurring, setRecurring] = useState(!!reminder.recurrence);
  const [enabled, setEnabled] = useState(reminder.enabled);
  const [error, setError] = useState<string | null>(null);

  const updateMutation = useMutation({
    mutationFn: (input: UpdateReminderInput) => ipc.reminderUpdate(input),
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: ["reminders"] });
      void queryClient.invalidateQueries({ queryKey: ["tasks"] });
      setError(null);
      onSaved?.();
    },
    onError: (err: Error) => setError(err.message || "保存失败"),
  });

  const save = () => {
    if (!fireAt) {
      setError("请选择提醒时间");
      return;
    }
    const normalized = fireAt.length === 16 ? `${fireAt}:00` : fireAt;
    updateMutation.mutate({
      id: reminder.id,
      title: reminder.title,
      notes: reminder.notes,
      fireAt: normalized.replace(" ", "T"),
      recurrence: recurring
        ? (reminder.recurrence ?? {
            version: 1,
            frequency: "daily",
            interval: 1,
            timezone: Intl.DateTimeFormat().resolvedOptions().timeZone,
          })
        : null,
      enabled,
      endAt: reminder.endAt,
    });
  };

  return (
    <div className="space-y-2 border-t border-border pt-2">
      <Input
        type="datetime-local"
        value={fireAt}
        onChange={(e) => setFireAt(e.target.value)}
      />
      <div className="flex items-center gap-3 text-[11px] text-muted">
        <label className="flex items-center gap-1">
          <input
            type="checkbox"
            checked={recurring}
            onChange={(e) => setRecurring(e.target.checked)}
          />
          每天重复
        </label>
        <label className="flex items-center gap-1">
          <input
            type="checkbox"
            checked={enabled}
            onChange={(e) => setEnabled(e.target.checked)}
          />
          启用
        </label>
        <Button
          size="sm"
          variant="secondary"
          className="ml-auto"
          disabled={updateMutation.isPending}
          onClick={save}
        >
          保存
        </Button>
      </div>
      {error ? <p className="text-[12px] text-danger">{error}</p> : null}
    </div>
  );
}
