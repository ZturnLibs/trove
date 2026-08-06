import { useMemo, useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Clock } from "lucide-react";
import { TaskDetailPanel } from "@/design-system/patterns/TaskDetailPanel";
import { TaskRow } from "@/design-system/patterns/TaskRow";
import { EmptyState } from "@/components/PageScaffold";
import { NotificationPermissionBanner } from "@/components/NotificationPermissionBanner";
import { Button } from "@/design-system/primitives/Button";
import { ConfirmButton } from "@/design-system/patterns/ConfirmButton";
import { Input } from "@/design-system/primitives/Input";
import {
  ipc,
  type Reminder,
  type Task,
  type TodayReminderItem,
  type UpdateReminderInput,
} from "@/ipc/client";
import {
  NewTaskButton,
  SplitTaskLayout,
  TaskGroup,
} from "@/features/tasks/TaskLayout";
import { useDomainInvalidation } from "@/features/tasks/useDomainInvalidation";
import { useTaskRename } from "@/features/tasks/useTaskRename";
import { useRecentActions } from "@/stores/recent-actions";
import { cn } from "@/lib/cn";

function ReminderRow({
  item,
  selected,
  onSelect,
  onComplete,
  onSnooze,
}: {
  item: TodayReminderItem;
  selected?: boolean;
  onSelect: () => void;
  onComplete: () => void;
  onSnooze: (preset: "minutes10" | "hour1" | "tomorrow") => void;
}) {
  const time = item.occurrence.scheduledAt.slice(11, 16);
  return (
    <div
      role="button"
      tabIndex={0}
      onClick={onSelect}
      onKeyDown={(e) => {
        if (e.key === "Enter" || e.key === " ") {
          e.preventDefault();
          onSelect();
        }
      }}
      className={cn(
        "flex min-h-9 items-center gap-2 border-b border-border px-3 py-1.5 text-[13px] hover:bg-row-hover",
        selected && "bg-row-active",
      )}
    >
      <Clock className="h-3.5 w-3.5 shrink-0 text-muted" />
      <div className="min-w-0 flex-1">
        <div className="truncate">{item.reminder.title}</div>
        <div className="text-[11px] text-muted">
          {time}
          {item.reminder.recurrence ? " · 周期" : ""}
          {item.reminder.taskId ? " · 任务提醒" : ""}
        </div>
      </div>
      <Button
        size="sm"
        variant="ghost"
        onClick={(e) => {
          e.stopPropagation();
          onSnooze("minutes10");
        }}
      >
        稍后
      </Button>
      <Button
        size="sm"
        variant="secondary"
        onClick={(e) => {
          e.stopPropagation();
          onComplete();
        }}
      >
        完成
      </Button>
    </div>
  );
}

function ReminderEditForm({
  reminder,
  initialFireAt,
  onSaved,
  onDeleted,
}: {
  reminder: Reminder;
  /** Overrides the default datetime-local value (e.g. an occurrence's scheduledAt). */
  initialFireAt?: string;
  onSaved?: () => void;
  onDeleted?: () => void;
}) {
  const queryClient = useQueryClient();
  const [title, setTitle] = useState(reminder.title);
  const [notes, setNotes] = useState(reminder.notes);
  const [fireAt, setFireAt] = useState(
    (initialFireAt ?? reminder.nextFireAt).slice(0, 16),
  );
  const [enabled, setEnabled] = useState(reminder.enabled);
  const [error, setError] = useState<string | null>(null);

  const updateMutation = useMutation({
    mutationFn: (input: UpdateReminderInput) => ipc.reminderUpdate(input),
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: ["tasks"] });
      void queryClient.invalidateQueries({ queryKey: ["reminders"] });
      setError(null);
      onSaved?.();
    },
    onError: (err: Error) => setError(err.message || "保存失败"),
  });

  const deleteMutation = useMutation({
    mutationFn: () => ipc.reminderDelete(reminder.id),
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: ["tasks"] });
      void queryClient.invalidateQueries({ queryKey: ["reminders"] });
      onDeleted?.();
    },
  });

  const save = () => {
    if (!fireAt) {
      setError("请选择计划时间");
      return;
    }
    const normalized = fireAt.length === 16 ? `${fireAt}:00` : fireAt;
    updateMutation.mutate({
      id: reminder.id,
      title,
      notes,
      fireAt: normalized.replace(" ", "T"),
      recurrence: reminder.recurrence,
      enabled,
      endAt: reminder.endAt,
    });
  };

  return (
    <div className="space-y-2">
      <label className="block space-y-1 text-[11px] text-muted">
        标题
        <Input value={title} onChange={(e) => setTitle(e.target.value)} />
      </label>
      <label className="block space-y-1 text-[11px] text-muted">
        计划时间
        <Input
          type="datetime-local"
          value={fireAt}
          onChange={(e) => setFireAt(e.target.value)}
        />
      </label>
      <label className="block space-y-1 text-[11px] text-muted">
        备注
        <textarea
          value={notes}
          onChange={(e) => setNotes(e.target.value)}
          rows={3}
          className="w-full resize-none rounded-[var(--radius-control)] border border-border bg-surface-raised p-2 text-[13px] text-foreground outline-none focus:ring-2 focus:ring-accent/35"
        />
      </label>
      <label className="flex items-center gap-2 text-[12px] text-muted">
        <input
          type="checkbox"
          checked={enabled}
          onChange={(e) => setEnabled(e.target.checked)}
        />
        启用提醒
      </label>
      {error ? <p className="text-[12px] text-danger">{error}</p> : null}
      <div className="flex flex-wrap gap-2">
        <Button size="sm" onClick={save} disabled={updateMutation.isPending}>
          保存
        </Button>
        <ConfirmButton
          size="sm"
          confirmLabel="确认删除？"
          resetKey={reminder.id}
          disabled={deleteMutation.isPending}
          onConfirm={() => deleteMutation.mutate()}
        >
          删除
        </ConfirmButton>
      </div>
    </div>
  );
}

function AllReminderRow({
  reminder,
  selected,
  editing,
  onSelect,
  onEditToggle,
  onToggleEnabled,
  onDelete,
  onDeleted,
}: {
  reminder: Reminder;
  selected?: boolean;
  editing: boolean;
  onSelect: () => void;
  onEditToggle: () => void;
  onToggleEnabled: () => void;
  onDelete: () => void;
  onDeleted: () => void;
}) {
  return (
    <div>
      <div
        role="button"
        tabIndex={0}
        onClick={onSelect}
        onKeyDown={(e) => {
          if (e.key === "Enter" || e.key === " ") {
            e.preventDefault();
            onSelect();
          }
        }}
        className={cn(
          "flex min-h-9 items-center gap-2 border-b border-border px-3 py-1.5 text-[13px] hover:bg-row-hover",
          selected && "bg-row-active",
        )}
      >
        <Clock className="h-3.5 w-3.5 shrink-0 text-muted" />
        <div className="min-w-0 flex-1">
          <div className="truncate">{reminder.title}</div>
          <div className="text-[11px] text-muted">
            {reminder.nextFireAt.replace("T", " ").slice(0, 16)}
            {reminder.recurrence ? " · 周期" : ""}
            {reminder.taskId ? " · 任务提醒" : ""}
            {!reminder.enabled ? " · 已停用" : ""}
          </div>
        </div>
        <input
          type="checkbox"
          checked={reminder.enabled}
          title={reminder.enabled ? "停用" : "启用"}
          onClick={(e) => e.stopPropagation()}
          onChange={onToggleEnabled}
        />
        <Button
          size="sm"
          variant="ghost"
          onClick={(e) => {
            e.stopPropagation();
            onEditToggle();
          }}
        >
          编辑
        </Button>
        <ConfirmButton
          size="sm"
          confirmLabel="确认删除？"
          resetKey={reminder.id}
          onConfirm={onDelete}
        >
          删除
        </ConfirmButton>
      </div>
      {editing ? (
        <div className="border-b border-border px-3 py-2">
          <ReminderEditForm
            key={reminder.id}
            reminder={reminder}
            onSaved={onEditToggle}
            onDeleted={onDeleted}
          />
        </div>
      ) : null}
    </div>
  );
}

export function TodayPage() {
  useDomainInvalidation();
  const rename = useTaskRename();
  const queryClient = useQueryClient();
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [selectedReminderId, setSelectedReminderId] = useState<string | null>(null);
  const [completedCollapsed, setCompletedCollapsed] = useState(true);
  const [createdId, setCreatedId] = useState<string | null>(null);
  const [quickTitle, setQuickTitle] = useState("");
  const [quickError, setQuickError] = useState<string | null>(null);
  const [showAllReminders, setShowAllReminders] = useState(false);
  const [editingReminder, setEditingReminder] = useState(false);
  const [editingAllId, setEditingAllId] = useState<string | null>(null);

  const todayQuery = useQuery({
    queryKey: ["tasks", "today"],
    queryFn: () => ipc.taskToday(),
  });

  const allRemindersQuery = useQuery({
    queryKey: ["reminders", "all"],
    queryFn: () => ipc.reminderListAll(),
    enabled: showAllReminders,
  });

  const createMutation = useMutation({
    mutationFn: () =>
      ipc.taskCreate({
        title: "新任务",
        dueDate: todayQuery.data?.today,
      }),
    onSuccess: (task) => {
      void queryClient.invalidateQueries({ queryKey: ["tasks"] });
      setSelectedId(task.id);
      setCreatedId(task.id);
      setSelectedReminderId(null);
    },
  });

  const createReminderMutation = useMutation({
    mutationFn: () => {
      const today = todayQuery.data?.today ?? new Date().toISOString().slice(0, 10);
      const fireAt = `${today}T09:00:00`;
      return ipc.reminderCreate({
        title: "新提醒",
        fireAt,
        timezone: Intl.DateTimeFormat().resolvedOptions().timeZone,
      });
    },
    onSuccess: (reminder) => {
      void queryClient.invalidateQueries({ queryKey: ["tasks"] });
      setSelectedReminderId(reminder.id);
      setSelectedId(null);
    },
  });

  const quickAddMutation = useMutation({
    mutationFn: async (text: string) => {
      const parsed = await ipc.nlParseCapture(text);
      const finalTitle = parsed.title.trim() || text;
      const finalDue =
        parsed.dueDate ??
        todayQuery.data?.today ??
        new Date().toISOString().slice(0, 10);
      const finalPriority =
        parsed.priority !== "none" ? parsed.priority : undefined;
      const input = {
        title: finalTitle,
        dueDate: finalDue,
        dueTime: parsed.dueTime,
        priority: finalPriority,
      };
      if (parsed.recurrence) {
        return ipc.taskCreateRecurring(input, parsed.recurrence);
      }
      return ipc.taskCreate(input);
    },
    onSuccess: (task) => {
      void queryClient.invalidateQueries({ queryKey: ["tasks"] });
      setQuickTitle("");
      setQuickError(null);
      setSelectedId(task.id);
      setSelectedReminderId(null);
    },
    onError: (err) => {
      setQuickError(err instanceof Error ? err.message : "创建失败");
    },
  });

  const submitQuickAdd = () => {
    const value = quickTitle.trim();
    if (!value || quickAddMutation.isPending) return;
    quickAddMutation.mutate(value);
  };

  const toggleMutation = useMutation({
    mutationFn: async (task: Task) => {
      if (task.status === "completed") await ipc.taskUncomplete(task.id);
      else await ipc.taskComplete(task.id);
    },
    onSuccess: (_data, task) => {
      void queryClient.invalidateQueries({ queryKey: ["tasks"] });
      const wasCompleted = task.status === "completed";
      useRecentActions.getState().push({
        label: wasCompleted ? "取消完成" : "完成",
        undo: async () => {
          if (wasCompleted) await ipc.taskComplete(task.id);
          else await ipc.taskUncomplete(task.id);
        },
      });
    },
  });

  const reminderComplete = useMutation({
    mutationFn: (occurrenceId: string) => ipc.reminderComplete(occurrenceId),
    onSuccess: () => void queryClient.invalidateQueries({ queryKey: ["tasks"] }),
  });

  const reminderSnooze = useMutation({
    mutationFn: ({
      occurrenceId,
      preset,
    }: {
      occurrenceId: string;
      preset: "minutes10" | "hour1" | "tomorrow";
    }) => ipc.reminderSnooze(occurrenceId, preset),
    onSuccess: () => void queryClient.invalidateQueries({ queryKey: ["tasks"] }),
  });

  const reminderDelete = useMutation({
    mutationFn: (id: string) => ipc.reminderDelete(id),
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: ["tasks"] });
      void queryClient.invalidateQueries({ queryKey: ["reminders"] });
      setSelectedReminderId(null);
      setEditingReminder(false);
      setEditingAllId(null);
    },
  });

  const reminderToggleEnabled = useMutation({
    mutationFn: (reminder: Reminder) =>
      ipc.reminderUpdate({
        id: reminder.id,
        title: reminder.title,
        notes: reminder.notes,
        fireAt: reminder.nextFireAt,
        recurrence: reminder.recurrence,
        enabled: !reminder.enabled,
        endAt: reminder.endAt,
      }),
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: ["tasks"] });
      void queryClient.invalidateQueries({ queryKey: ["reminders"] });
    },
  });

  const selected = useMemo(() => {
    const data = todayQuery.data;
    if (!data || !selectedId) return null;
    return (
      [...data.overdue, ...data.dueToday, ...data.completedToday].find(
        (t) => t.id === selectedId,
      ) ?? null
    );
  }, [todayQuery.data, selectedId]);

  const selectedReminder = useMemo(() => {
    if (!selectedReminderId || !todayQuery.data) return null;
    return (
      todayQuery.data.remindersToday.find(
        (item) => item.reminder.id === selectedReminderId,
      ) ?? null
    );
  }, [todayQuery.data, selectedReminderId]);

  const selectedAllReminder = useMemo(() => {
    if (!selectedReminderId || !showAllReminders) return null;
    return (
      allRemindersQuery.data?.find((r) => r.id === selectedReminderId) ?? null
    );
  }, [selectedReminderId, showAllReminders, allRemindersQuery.data]);

  const toggleAllView = () => {
    const next = !showAllReminders;
    setShowAllReminders(next);
    setSelectedReminderId(null);
    setSelectedId(null);
    setEditingReminder(false);
    setEditingAllId(null);
  };

  const data = todayQuery.data;
  const empty =
    data &&
    data.overdue.length === 0 &&
    data.dueToday.length === 0 &&
    data.completedToday.length === 0 &&
    data.remindersToday.length === 0;

  return (
    <>
      <NotificationPermissionBanner />
      <SplitTaskLayout
      title="今日"
      description={data ? data.today : "加载中…"}
      actions={
        <>
          <Button
            size="sm"
            variant={showAllReminders ? "default" : "secondary"}
            onClick={toggleAllView}
          >
            全部提醒
          </Button>
          <Button
            size="sm"
            variant="secondary"
            onClick={() => createReminderMutation.mutate()}
          >
            新建提醒
          </Button>
          <NewTaskButton onClick={() => createMutation.mutate()} />
        </>
      }
      list={
        showAllReminders ? (
          allRemindersQuery.isLoading ? (
            <div className="p-4 text-[12px] text-muted">加载中…</div>
          ) : (allRemindersQuery.data?.length ?? 0) === 0 ? (
            <EmptyState
              title="暂无提醒"
              body="创建的提醒都会出现在这里，包括未来和已停用的。"
              primaryAction={{
                label: "新建提醒",
                onClick: () => createReminderMutation.mutate(),
              }}
            />
          ) : (
            <div>
              <TaskGroup
                title="全部提醒"
                count={allRemindersQuery.data?.length ?? 0}
              >
                {(allRemindersQuery.data ?? []).map((reminder) => (
                  <AllReminderRow
                    key={reminder.id}
                    reminder={reminder}
                    selected={selectedReminderId === reminder.id}
                    editing={editingAllId === reminder.id}
                    onSelect={() => {
                      setSelectedReminderId(reminder.id);
                      setSelectedId(null);
                      setEditingAllId(null);
                    }}
                    onEditToggle={() =>
                      setEditingAllId((cur) =>
                        cur === reminder.id ? null : reminder.id,
                      )
                    }
                    onToggleEnabled={() =>
                      reminderToggleEnabled.mutate(reminder)
                    }
                    onDelete={() => reminderDelete.mutate(reminder.id)}
                    onDeleted={() => {
                      if (selectedReminderId === reminder.id) {
                        setSelectedReminderId(null);
                      }
                    }}
                  />
                ))}
              </TaskGroup>
            </div>
          )
        ) : todayQuery.isLoading ? (
          <div className="p-4 text-[12px] text-muted">加载中…</div>
        ) : empty ? (
          <EmptyState
            title="今日还没有事项"
            body="给任务加上今天的截止日期，或新建今日提醒。"
            primaryAction={{
              label: "新建任务",
              onClick: () => createMutation.mutate(),
            }}
            secondaryAction={{
              label: "新建提醒",
              onClick: () => createReminderMutation.mutate(),
            }}
            hint="也可用菜单「文件 → 新建任务」或全局快速记录"
          />
        ) : (
          <div>
            <TaskGroup title="逾期" count={data?.overdue.length ?? 0} danger>
              {data?.overdue.map((task) => (
                <TaskRow
                  key={task.id}
                  task={task}
                  overdue
                  selected={selectedId === task.id}
                  onSelect={() => {
                    setSelectedId(task.id);
                    setSelectedReminderId(null);
                  }}
                  onToggleComplete={() => toggleMutation.mutate(task)}
                  onRename={rename}
                />
              ))}
            </TaskGroup>
            <TaskGroup title="今日提醒" count={data?.remindersToday.length ?? 0}>
              {data?.remindersToday.map((item) => (
                <ReminderRow
                  key={item.occurrence.id}
                  item={item}
                  selected={selectedReminderId === item.reminder.id}
                  onSelect={() => {
                    setSelectedReminderId(item.reminder.id);
                    setSelectedId(null);
                  }}
                  onComplete={() => reminderComplete.mutate(item.occurrence.id)}
                  onSnooze={(preset) =>
                    reminderSnooze.mutate({
                      occurrenceId: item.occurrence.id,
                      preset,
                    })
                  }
                />
              ))}
            </TaskGroup>
            <TaskGroup title="今日任务" count={data?.dueToday.length ?? 0}>
              {data?.dueToday.map((task) => (
                <TaskRow
                  key={task.id}
                  task={task}
                  selected={selectedId === task.id}
                  onSelect={() => {
                    setSelectedId(task.id);
                    setSelectedReminderId(null);
                  }}
                  onToggleComplete={() => toggleMutation.mutate(task)}
                  onRename={rename}
                />
              ))}
            </TaskGroup>
            <TaskGroup
              title="今日已完成"
              count={data?.completedToday.length ?? 0}
              collapsed={completedCollapsed}
              onToggle={() => setCompletedCollapsed((v) => !v)}
            >
              {data?.completedToday.map((task) => (
                <TaskRow
                  key={task.id}
                  task={task}
                  selected={selectedId === task.id}
                  onSelect={() => {
                    setSelectedId(task.id);
                    setSelectedReminderId(null);
                  }}
                  onToggleComplete={() => toggleMutation.mutate(task)}
                  onRename={rename}
                />
              ))}
            </TaskGroup>
          </div>
        )
      }
      detail={
        selectedAllReminder ? (
          <div className="flex h-full flex-col overflow-auto p-4">
            <p className="text-[11px] text-muted">提醒</p>
            <h2 className="mt-1 text-[15px] font-semibold">
              {selectedAllReminder.title}
            </h2>
            <p className="mt-2 text-[12px] text-muted">
              下次{" "}
              {selectedAllReminder.nextFireAt.replace("T", " ")}
              {!selectedAllReminder.enabled ? " · 已停用" : ""}
            </p>
            <div className="mt-4">
              <ReminderEditForm
                key={selectedAllReminder.id}
                reminder={selectedAllReminder}
                onDeleted={() => setSelectedReminderId(null)}
              />
            </div>
          </div>
        ) : selectedReminder ? (
          <div className="flex h-full flex-col p-4">
            <p className="text-[11px] text-muted">提醒</p>
            <h2 className="mt-1 text-[15px] font-semibold">
              {selectedReminder.reminder.title}
            </h2>
            <p className="mt-2 text-[12px] text-muted">
              计划时间 {selectedReminder.occurrence.scheduledAt.replace("T", " ")}
            </p>
            {editingReminder ? (
              <div className="mt-4 overflow-auto">
                <ReminderEditForm
                  key={selectedReminder.reminder.id}
                  reminder={selectedReminder.reminder}
                  initialFireAt={selectedReminder.occurrence.scheduledAt}
                  onSaved={() => setEditingReminder(false)}
                  onDeleted={() => {
                    setEditingReminder(false);
                    setSelectedReminderId(null);
                  }}
                />
              </div>
            ) : (
              <>
                {selectedReminder.reminder.notes ? (
                  <p className="mt-3 whitespace-pre-wrap text-[13px]">
                    {selectedReminder.reminder.notes}
                  </p>
                ) : null}
                <div className="mt-auto flex flex-wrap gap-2 border-t border-border pt-3">
                  <Button
                    size="sm"
                    variant="secondary"
                    onClick={() =>
                      reminderSnooze.mutate({
                        occurrenceId: selectedReminder.occurrence.id,
                        preset: "minutes10",
                      })
                    }
                  >
                    10 分钟后
                  </Button>
                  <Button
                    size="sm"
                    variant="secondary"
                    onClick={() =>
                      reminderSnooze.mutate({
                        occurrenceId: selectedReminder.occurrence.id,
                        preset: "hour1",
                      })
                    }
                  >
                    1 小时后
                  </Button>
                  <Button
                    size="sm"
                    variant="secondary"
                    onClick={() =>
                      reminderSnooze.mutate({
                        occurrenceId: selectedReminder.occurrence.id,
                        preset: "tomorrow",
                      })
                    }
                  >
                    明天
                  </Button>
                  <Button
                    size="sm"
                    variant="secondary"
                    onClick={() => setEditingReminder(true)}
                  >
                    编辑
                  </Button>
                  <Button
                    size="sm"
                    onClick={() =>
                      reminderComplete.mutate(selectedReminder.occurrence.id)
                    }
                  >
                    完成
                  </Button>
                  <ConfirmButton
                    size="sm"
                    confirmLabel="确认删除？"
                    resetKey={selectedReminder.reminder.id}
                    disabled={reminderDelete.isPending}
                    onConfirm={() =>
                      reminderDelete.mutate(selectedReminder.reminder.id)
                    }
                  >
                    删除
                  </ConfirmButton>
                </div>
              </>
            )}
          </div>
        ) : (
          <TaskDetailPanel
            task={selected}
            onDeleted={() => setSelectedId(null)}
            focusTitleId={createdId}
          />
        )
      }
      footer={
        <div className="p-2">
          {quickError ? (
            <p className="mb-2 text-[12px] text-danger">{quickError}</p>
          ) : null}
          <div className="flex items-center gap-2">
            <Input
              value={quickTitle}
              onChange={(e) => {
                setQuickTitle(e.target.value);
                setQuickError(null);
              }}
              placeholder="快速添加任务，如：明天下午三点回复客户…"
              onKeyDown={(e) => {
                if (e.nativeEvent.isComposing) return;
                if (e.key === "Enter") {
                  e.preventDefault();
                  submitQuickAdd();
                }
              }}
            />
            <Button
              size="sm"
              className="shrink-0"
              disabled={!quickTitle.trim() || quickAddMutation.isPending}
              onClick={() => submitQuickAdd()}
            >
              添加
            </Button>
          </div>
        </div>
      }
    />
    </>
  );
}
