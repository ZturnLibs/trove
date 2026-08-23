import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import {
  DndContext,
  DragOverlay,
  PointerSensor,
  closestCenter,
  useSensor,
  useSensors,
} from "@dnd-kit/core";
import type { DragEndEvent, DragStartEvent } from "@dnd-kit/core";
import {
  SortableContext,
  arrayMove,
  verticalListSortingStrategy,
} from "@dnd-kit/sortable";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { listen } from "@tauri-apps/api/event";
import { useNavigate } from "react-router-dom";
import { Clock } from "lucide-react";
import { RecurrencePicker } from "@/design-system/patterns/RecurrencePicker";
import { FocusDropZone } from "@/design-system/patterns/FocusDropZone";
import { DailySuggestionsCard } from "@/features/today/DailySuggestionsCard";
import { TaskDetailPanel } from "@/design-system/patterns/TaskDetailPanel";
import { SortableTaskRow, TaskRow } from "@/design-system/patterns/TaskRow";
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
  type TodayTasks,
  type UpdateReminderInput,
  type RecurrenceRule,
} from "@/ipc/client";
import { recurrenceLabel } from "@/lib/recurrence";
import {
  NewTaskButton,
  SplitTaskLayout,
  TaskGroup,
} from "@/features/tasks/TaskLayout";
import { useDomainInvalidation } from "@/features/tasks/useDomainInvalidation";
import { useTaskRename } from "@/features/tasks/useTaskRename";
import { useRecentActions } from "@/stores/recent-actions";
import { useFocusSession } from "@/stores/focus-session";
import { DailyWrapWizard } from "@/features/daily-wrap/DailyWrapWizard";
import { DailyWrapSummaryDialog } from "@/features/daily-wrap/DailyWrapSummaryDialog";
import { cn } from "@/lib/cn";
import { FOCUS_MANY_COACH_KEY } from "@/lib/focus";
import { addDays, localTodayString } from "@/lib/waiting";

type TodayContainerId = "focus" | "due-today";

function findTodayContainer(
  id: string,
  focus: Task[],
  dueToday: Task[],
): TodayContainerId | null {
  if (id === "focus" || focus.some((t) => t.id === id)) return "focus";
  if (id === "due-today" || dueToday.some((t) => t.id === id)) {
    return "due-today";
  }
  return null;
}

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
          {item.reminder.recurrence
            ? ` · ${recurrenceLabel(item.reminder.recurrence)}`
            : ""}
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
  const [recurrence, setRecurrence] = useState<RecurrenceRule | null>(
    reminder.recurrence,
  );
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
      recurrence,
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
      <RecurrencePicker value={recurrence} onChange={setRecurrence} />
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
            {reminder.recurrence
              ? ` · ${recurrenceLabel(reminder.recurrence)}`
              : ""}
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

function WaitingFollowUpRow({
  task,
  today,
  selected,
  onSelect,
  onClearWaiting,
  onContinueWaiting,
  onComplete,
}: {
  task: Task;
  today: string;
  selected?: boolean;
  onSelect: () => void;
  onClearWaiting: () => void;
  onContinueWaiting: () => void;
  onComplete: () => void;
}) {
  const followLabel =
    task.followUpDate === today
      ? "跟进日今天"
      : task.followUpDate
        ? `跟进日 ${task.followUpDate}`
        : null;

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
        "border-b border-border px-3 py-2 text-[13px] hover:bg-row-hover",
        selected && "bg-row-active",
      )}
    >
      <div className="flex items-center gap-2">
        <button
          type="button"
          aria-label="完成"
          className="flex h-4 w-4 shrink-0 items-center justify-center rounded-sm border border-border"
          onClick={(e) => {
            e.stopPropagation();
            onComplete();
          }}
        />
        <div className="min-w-0 flex-1 truncate text-muted">
          <span>⏸ </span>
          {task.title}
        </div>
      </div>
      <div className="mt-1 pl-6 text-[11px] text-muted">
        {task.waitingFor ? `等待：${task.waitingFor}` : "等待中"}
        {followLabel ? ` · ${followLabel}` : ""}
      </div>
      <div className="mt-2 flex flex-wrap gap-1 pl-6">
        <Button
          size="sm"
          variant="ghost"
          onClick={(e) => {
            e.stopPropagation();
            onClearWaiting();
          }}
        >
          结束等待
        </Button>
        <Button
          size="sm"
          variant="ghost"
          onClick={(e) => {
            e.stopPropagation();
            onContinueWaiting();
          }}
        >
          继续等待
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
  const [activeDragId, setActiveDragId] = useState<string | null>(null);
  const suppressClickUntilRef = useRef(0);
  const [quickError, setQuickError] = useState<string | null>(null);
  const [showAllReminders, setShowAllReminders] = useState(false);
  const [editingReminder, setEditingReminder] = useState(false);
  const [editingAllId, setEditingAllId] = useState<string | null>(null);
  const [focusManyDismissed, setFocusManyDismissed] = useState(false);
  const [wrapOpen, setWrapOpen] = useState(false);
  const [wrapSummaryOpen, setWrapSummaryOpen] = useState(false);
  const navigate = useNavigate();

  useEffect(() => {
    try {
      setFocusManyDismissed(localStorage.getItem(FOCUS_MANY_COACH_KEY) === "1");
    } catch {
      setFocusManyDismissed(false);
    }
  }, []);

  useEffect(() => {
    let unlistenTask: (() => void) | undefined;
    let unlistenReminder: (() => void) | undefined;
    void listen<string>("main://select-task", (event) => {
      if (event.payload) {
        setSelectedId(event.payload);
        setSelectedReminderId(null);
      }
    }).then((fn) => {
      unlistenTask = fn;
    });
    void listen<string>("main://select-reminder", (event) => {
      if (event.payload) {
        setSelectedReminderId(event.payload);
        setSelectedId(null);
      }
    }).then((fn) => {
      unlistenReminder = fn;
    });
    return () => {
      unlistenTask?.();
      unlistenReminder?.();
    };
  }, []);

  const sensors = useSensors(
    useSensor(PointerSensor, {
      activationConstraint: { distance: 8 },
    }),
  );

  const startFocus = useFocusSession((s) => s.start);
  const focusStarting = useFocusSession((s) => s.starting);

  const todayQuery = useQuery({
    queryKey: ["tasks", "today"],
    queryFn: () => ipc.taskToday(),
  });

  const settingsQuery = useQuery({
    queryKey: ["settings"],
    queryFn: () => ipc.settingsGet(),
  });

  const sortSuggestionsQuery = useQuery({
    queryKey: ["tasks", "today", "sort-suggestions"],
    queryFn: () => ipc.todaySortSuggestions(),
    enabled: settingsQuery.data?.todaySmartSortEnabled !== false,
  });

  const dueTodayDisplay = useMemo(() => {
    const tasks = todayQuery.data?.dueToday ?? [];
    const suggestions = sortSuggestionsQuery.data;
    if (!suggestions?.enabled || suggestions.suggestions.length === 0) {
      return tasks;
    }
    const rank = new Map(
      suggestions.suggestions.map((s) => [s.taskId, s.rank]),
    );
    return [...tasks].sort(
      (a, b) => (rank.get(a.id) ?? 999) - (rank.get(b.id) ?? 999),
    );
  }, [todayQuery.data?.dueToday, sortSuggestionsQuery.data]);

  const sortReasonById = useMemo(() => {
    const map = new Map<string, string>();
    if (!sortSuggestionsQuery.data?.enabled) return map;
    for (const s of sortSuggestionsQuery.data.suggestions) {
      map.set(s.taskId, s.reason);
    }
    return map;
  }, [sortSuggestionsQuery.data]);

  const sortOrderDiffers = useMemo(() => {
    const suggestions = sortSuggestionsQuery.data?.suggestions ?? [];
    const dueToday = todayQuery.data?.dueToday ?? [];
    if (suggestions.length < 2 || dueToday.length < 2) return false;
    return suggestions.some((s, i) => s.taskId !== dueToday[i]?.id);
  }, [sortSuggestionsQuery.data, todayQuery.data?.dueToday]);

  const completedWrapQuery = useQuery({
    queryKey: ["daily-wrap", "completed", todayQuery.data?.today],
    queryFn: () => ipc.dailyWrapCompletedForDate(todayQuery.data?.today),
    enabled: !!todayQuery.data?.today,
  });

  const focusAddMutation = useMutation({
    mutationFn: (taskId: string) => ipc.dailyFocusAdd(taskId),
    onSuccess: (_data, taskId) => {
      void queryClient.invalidateQueries({ queryKey: ["tasks"] });
      useRecentActions.getState().push({
        label: "加入今日重点",
        undo: async () => {
          await ipc.dailyFocusRemove(taskId);
          void queryClient.invalidateQueries({ queryKey: ["tasks"] });
        },
      });
    },
  });

  const focusRemoveMutation = useMutation({
    mutationFn: (taskId: string) => ipc.dailyFocusRemove(taskId),
    onSuccess: (_data, taskId) => {
      void queryClient.invalidateQueries({ queryKey: ["tasks"] });
      useRecentActions.getState().push({
        label: "移出今日重点",
        undo: async () => {
          await ipc.dailyFocusAdd(taskId);
          void queryClient.invalidateQueries({ queryKey: ["tasks"] });
        },
      });
    },
  });

  const focusReorderMutation = useMutation({
    mutationFn: (orderedIds: string[]) => ipc.dailyFocusReorder(orderedIds),
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: ["tasks", "today"] });
    },
  });

  const carryMutation = useMutation({
    mutationFn: async () => {
      const today = todayQuery.data?.today ?? localTodayString();
      const yesterday = addDays(today, -1);
      return ipc.dailyFocusCarry(yesterday, today);
    },
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: ["tasks"] });
    },
  });

  const dismissCarryMutation = useMutation({
    mutationFn: async () => {
      const settings = await ipc.settingsGet();
      const today = todayQuery.data?.today ?? localTodayString();
      return ipc.settingsSave({
        ...settings,
        lastFocusCarryDismissedDate: today,
      });
    },
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: ["settings"] });
    },
  });

  const reorderMutation = useMutation({
    mutationFn: (orderedIds: string[]) => ipc.taskReorder(orderedIds),
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: ["tasks", "today"] });
      void queryClient.invalidateQueries({ queryKey: ["tasks"] });
      void queryClient.invalidateQueries({
        queryKey: ["tasks", "today", "sort-suggestions"],
      });
    },
  });

  const adoptSortMutation = useMutation({
    mutationFn: () => ipc.taskReorder(dueTodayDisplay.map((t) => t.id)),
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: ["tasks", "today"] });
      void queryClient.invalidateQueries({ queryKey: ["tasks"] });
      void queryClient.invalidateQueries({
        queryKey: ["tasks", "today", "sort-suggestions"],
      });
    },
  });

  const allRemindersQuery = useQuery({
    queryKey: ["reminders", "all"],
    queryFn: () => ipc.reminderListAll(),
    enabled: showAllReminders,
  });

  const toggleFocus = useCallback(
    (taskId: string, inFocus: boolean) => {
      if (inFocus) focusRemoveMutation.mutate(taskId);
      else focusAddMutation.mutate(taskId);
    },
    [focusAddMutation, focusRemoveMutation],
  );

  const handleTodaySelect = (id: string) => {
    if (Date.now() < suppressClickUntilRef.current) return;
    setSelectedId(id);
    setSelectedReminderId(null);
  };

  const handleDragStart = (event: DragStartEvent) => {
    setActiveDragId(String(event.active.id));
  };

  const handleDragEnd = (event: DragEndEvent) => {
    const { active, over } = event;
    setActiveDragId(null);
    suppressClickUntilRef.current = Date.now() + 300;
    if (!over) return;

    const focus = todayQuery.data?.focus ?? [];
    const dueToday = dueTodayDisplay;
    const activeId = String(active.id);
    const overId = String(over.id);
    if (activeId === overId) return;

    const activeContainer = findTodayContainer(activeId, focus, dueToday);
    let overContainer = findTodayContainer(overId, focus, dueToday);
    if (!overContainer && (overId === "focus" || overId === "due-today")) {
      overContainer = overId as TodayContainerId;
    }
    if (!activeContainer || !overContainer) return;

    if (activeContainer !== overContainer) {
      if (overContainer === "focus") {
        focusAddMutation.mutate(activeId);
      } else {
        focusRemoveMutation.mutate(activeId);
      }
      return;
    }

    if (activeContainer === "focus") {
      const oldIndex = focus.findIndex((t) => t.id === activeId);
      const newIndex = focus.findIndex((t) => t.id === overId);
      if (oldIndex < 0 || newIndex < 0) return;
      const orderedIds = arrayMove(
        focus.map((t) => t.id),
        oldIndex,
        newIndex,
      );
      if (orderedIds.join("|") === focus.map((t) => t.id).join("|")) return;
      queryClient.setQueryData<TodayTasks>(["tasks", "today"], (old) => {
        if (!old) return old;
        const order = new Map(orderedIds.map((id, i) => [id, i]));
        return {
          ...old,
          focus: [...old.focus].sort(
            (a, b) =>
              (order.get(a.id) ?? Number.MAX_SAFE_INTEGER) -
              (order.get(b.id) ?? Number.MAX_SAFE_INTEGER),
          ),
        };
      });
      focusReorderMutation.mutate(orderedIds);
      return;
    }

    const oldIndex = dueToday.findIndex((t) => t.id === activeId);
    const newIndex = dueToday.findIndex((t) => t.id === overId);
    if (oldIndex < 0 || newIndex < 0) return;
    const orderedIds = arrayMove(
      dueToday.map((t) => t.id),
      oldIndex,
      newIndex,
    );
    if (orderedIds.join("|") === dueToday.map((t) => t.id).join("|")) return;
    queryClient.setQueryData<TodayTasks>(["tasks", "today"], (old) => {
      if (!old) return old;
      const order = new Map(orderedIds.map((id, i) => [id, i]));
      return {
        ...old,
        dueToday: [...old.dueToday].sort(
          (a, b) =>
            (order.get(a.id) ?? Number.MAX_SAFE_INTEGER) -
            (order.get(b.id) ?? Number.MAX_SAFE_INTEGER),
        ),
      };
    });
    reorderMutation.mutate(orderedIds);
  };

  const handleDragCancel = () => {
    setActiveDragId(null);
    suppressClickUntilRef.current = Date.now() + 300;
  };

  const activeDragTask = useMemo(() => {
    const data = todayQuery.data;
    if (!data || !activeDragId) return null;
    return (
      data.focus.find((t) => t.id === activeDragId) ??
      data.dueToday.find((t) => t.id === activeDragId) ??
      null
    );
  }, [todayQuery.data, activeDragId]);

  const focusIds = useMemo(
    () => new Set((todayQuery.data?.focus ?? []).map((t) => t.id)),
    [todayQuery.data?.focus],
  );

  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      if (!selectedId || showAllReminders) return;
      const target = event.target;
      if (
        target instanceof HTMLInputElement ||
        target instanceof HTMLTextAreaElement ||
        target instanceof HTMLSelectElement
      ) {
        return;
      }

      const data = todayQuery.data;
      const selectedTask = data
        ? [
            ...data.focus,
            ...data.waitingFollowUp,
            ...data.overdue,
            ...data.dueToday,
            ...data.completedToday,
          ].find((t) => t.id === selectedId)
        : null;

      if (event.key === "Enter" && selectedTask?.status === "todo") {
        if (event.metaKey || event.ctrlKey) {
          event.preventDefault();
          void startFocus(selectedId);
          return;
        }
        if (focusIds.has(selectedId)) {
          event.preventDefault();
          void startFocus(selectedId);
          return;
        }
      }

      if (event.key !== "f" && event.key !== "F") return;
      event.preventDefault();
      const inFocus = focusIds.has(selectedId);
      if (event.shiftKey) {
        if (inFocus) toggleFocus(selectedId, true);
      } else if (inFocus) {
        toggleFocus(selectedId, true);
      } else {
        toggleFocus(selectedId, false);
      }
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [
    selectedId,
    showAllReminders,
    focusIds,
    toggleFocus,
    todayQuery.data,
    startFocus,
  ]);

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

  const clearWaitingMutation = useMutation({
    mutationFn: (task: Task) => ipc.taskClearWaiting(task.id),
    onSuccess: (_data, task) => {
      void queryClient.invalidateQueries({ queryKey: ["tasks"] });
      const prev = {
        waitingFor: task.waitingFor,
        followUpDate: task.followUpDate,
      };
      useRecentActions.getState().push({
        label: "结束等待",
        undo: async () => {
          await ipc.taskSetWaiting(
            task.id,
            prev.waitingFor,
            prev.followUpDate,
          );
          void queryClient.invalidateQueries({ queryKey: ["tasks"] });
        },
      });
    },
  });

  const continueWaitingMutation = useMutation({
    mutationFn: (task: Task) => {
      const next = addDays(localTodayString(), 7);
      return ipc.taskSetWaiting(task.id, task.waitingFor, next);
    },
    onSuccess: (_data, task) => {
      void queryClient.invalidateQueries({ queryKey: ["tasks"] });
      const prevDate = task.followUpDate;
      useRecentActions.getState().push({
        label: "继续等待（跟进日 +7 天）",
        undo: async () => {
          await ipc.taskSetWaiting(
            task.id,
            task.waitingFor,
            prevDate,
          );
          void queryClient.invalidateQueries({ queryKey: ["tasks"] });
        },
      });
    },
  });

  const selected = useMemo(() => {
    const data = todayQuery.data;
    if (!data || !selectedId) return null;
    return (
      [
        ...data.focus,
        ...data.waitingFollowUp,
        ...data.overdue,
        ...data.dueToday,
        ...data.completedToday,
      ].find((t) => t.id === selectedId) ?? null
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
  const showCarryBanner =
    !showAllReminders &&
    (data?.focusCarrySuggestions.length ?? 0) > 0 &&
    settingsQuery.data?.lastFocusCarryDismissedDate !== data?.today;
  const empty =
    data &&
    data.focus.length === 0 &&
    data.waitingFollowUp.length === 0 &&
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
          {completedWrapQuery.data ? (
            <Button
              size="sm"
              variant="secondary"
              onClick={() => setWrapSummaryOpen(true)}
            >
              今日已收尾 · 查看摘要
            </Button>
          ) : (
            <Button
              size="sm"
              variant="secondary"
              disabled={!data?.today}
              onClick={() => setWrapOpen(true)}
            >
              每日收尾
            </Button>
          )}
          <Button
            size="sm"
            variant="secondary"
            onClick={() => navigate("/weekly-review")}
          >
            每周回顾
          </Button>
          {selected?.status === "todo" && focusIds.has(selected.id) ? (
            <Button
              size="sm"
              disabled={focusStarting}
              onClick={() => void startFocus(selected.id)}
            >
              专注
            </Button>
          ) : null}
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
            <DailySuggestionsCard />
            <DndContext
              sensors={sensors}
              collisionDetection={closestCenter}
              onDragStart={handleDragStart}
              onDragEnd={handleDragEnd}
              onDragCancel={handleDragCancel}
            >
              {showCarryBanner ? (
                <div className="mx-3 mt-2 flex flex-wrap items-center gap-2 rounded-[var(--radius-control)] border border-border bg-surface-raised px-3 py-2 text-[12px]">
                  <span className="text-muted">
                    {data?.focusCarrySuggestions.length ?? 0}{" "}
                    项昨日重点未完成，是否加入今日？
                  </span>
                  <Button
                    size="sm"
                    onClick={() => carryMutation.mutate()}
                    disabled={carryMutation.isPending}
                  >
                    加入
                  </Button>
                  <Button
                    size="sm"
                    variant="ghost"
                    onClick={() => dismissCarryMutation.mutate()}
                    disabled={dismissCarryMutation.isPending}
                  >
                    忽略
                  </Button>
                </div>
              ) : null}
              <TaskGroup
                title="今日重点"
                count={data?.focus.length ?? 0}
                alwaysShow
              >
                {(data?.focus.length ?? 0) > 5 && !focusManyDismissed ? (
                  <div className="mx-3 mb-1 rounded-[var(--radius-control)] border border-border bg-surface-raised px-2 py-1.5 text-[11px] text-muted">
                    已选 {data?.focus.length ?? 0}{" "}
                    项，聚焦过多可能降低完成率。
                    <button
                      type="button"
                      className="ml-2 text-accent hover:underline"
                      onClick={() => {
                        try {
                          localStorage.setItem(FOCUS_MANY_COACH_KEY, "1");
                        } catch {
                          /* ignore */
                        }
                        setFocusManyDismissed(true);
                      }}
                    >
                      知道了
                    </button>
                  </div>
                ) : null}
                <FocusDropZone id="focus">
                  <SortableContext
                    items={data?.focus.map((t) => t.id) ?? []}
                    strategy={verticalListSortingStrategy}
                  >
                    {(data?.focus.length ?? 0) === 0 ? (
                      <div className="px-3 py-2 text-[11px] text-muted">
                        从下方拖入或按 F 加入今日重点
                      </div>
                    ) : (
                      data?.focus.map((task) => (
                        <SortableTaskRow
                          key={task.id}
                          task={task}
                          inFocus
                          selected={selectedId === task.id}
                          onSelect={() => handleTodaySelect(task.id)}
                          onToggleComplete={() => toggleMutation.mutate(task)}
                          onRename={rename}
                        />
                      ))
                    )}
                  </SortableContext>
                </FocusDropZone>
              </TaskGroup>
              {(data?.waitingFollowUp.length ?? 0) > 0 ? (
                <TaskGroup
                  title="等待跟进"
                  count={data?.waitingFollowUp.length ?? 0}
                >
                  {data?.waitingFollowUp.map((task) => (
                    <WaitingFollowUpRow
                      key={task.id}
                      task={task}
                      today={data?.today ?? localTodayString()}
                      selected={selectedId === task.id}
                      onSelect={() => handleTodaySelect(task.id)}
                      onClearWaiting={() => clearWaitingMutation.mutate(task)}
                      onContinueWaiting={() =>
                        continueWaitingMutation.mutate(task)
                      }
                      onComplete={() => toggleMutation.mutate(task)}
                    />
                  ))}
                </TaskGroup>
              ) : null}
              <TaskGroup title="逾期" count={data?.overdue.length ?? 0} danger>
                {data?.overdue.map((task) => (
                  <TaskRow
                    key={task.id}
                    task={task}
                    overdue
                    inFocus={focusIds.has(task.id)}
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
                title="今日提醒"
                count={data?.remindersToday.length ?? 0}
              >
                {data?.remindersToday.map((item) => (
                  <ReminderRow
                    key={item.occurrence.id}
                    item={item}
                    selected={selectedReminderId === item.reminder.id}
                    onSelect={() => {
                      setSelectedReminderId(item.reminder.id);
                      setSelectedId(null);
                    }}
                    onComplete={() =>
                      reminderComplete.mutate(item.occurrence.id)
                    }
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
                {sortSuggestionsQuery.data?.enabled && sortOrderDiffers ? (
                  <div className="flex flex-wrap items-center justify-between gap-2 border-b border-border bg-surface px-3 py-2 text-[12px]">
                    <span className="text-muted">
                      已按截止、优先级、延期与提醒生成顺序建议（预览中）
                    </span>
                    <Button
                      size="sm"
                      variant="secondary"
                      disabled={adoptSortMutation.isPending}
                      onClick={() => adoptSortMutation.mutate()}
                    >
                      采纳建议顺序
                    </Button>
                  </div>
                ) : null}
                <FocusDropZone id="due-today">
                  <SortableContext
                    items={dueTodayDisplay.map((t) => t.id)}
                    strategy={verticalListSortingStrategy}
                  >
                    {dueTodayDisplay.map((task) => (
                      <SortableTaskRow
                        key={task.id}
                        task={task}
                        inFocus={focusIds.has(task.id)}
                        selected={selectedId === task.id}
                        sortHint={
                          sortSuggestionsQuery.data?.enabled
                            ? sortReasonById.get(task.id)
                            : undefined
                        }
                        onSelect={() => handleTodaySelect(task.id)}
                        onToggleComplete={() => toggleMutation.mutate(task)}
                        onRename={rename}
                      />
                    ))}
                  </SortableContext>
                </FocusDropZone>
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
              <DragOverlay>
                {activeDragTask ? (
                  <div className="rounded-md bg-surface shadow-lg ring-1 ring-border">
                    <TaskRow
                      task={activeDragTask}
                      inFocus={focusIds.has(activeDragTask.id)}
                      onSelect={() => {}}
                      onToggleComplete={() => {}}
                    />
                  </div>
                ) : null}
              </DragOverlay>
            </DndContext>
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
            dailyFocus={
              selected
                ? {
                    inFocus: focusIds.has(selected.id),
                    onToggle: () =>
                      toggleFocus(selected.id, focusIds.has(selected.id)),
                  }
                : undefined
            }
            onStartFocus={
              selected?.status === "todo"
                ? () => void startFocus(selected.id)
                : undefined
            }
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
      {data?.today ? (
        <DailyWrapWizard
          open={wrapOpen}
          wrapDate={data.today}
          onClose={() => setWrapOpen(false)}
          onNavigate={(path) => navigate(path)}
          onCompleted={() => {
            setWrapOpen(false);
            void completedWrapQuery.refetch();
          }}
        />
      ) : null}
      <DailyWrapSummaryDialog
        open={wrapSummaryOpen}
        run={completedWrapQuery.data}
        onClose={() => setWrapSummaryOpen(false)}
        onStartAgain={() => {
          setWrapSummaryOpen(false);
          setWrapOpen(true);
        }}
      />
    </>
  );
}
