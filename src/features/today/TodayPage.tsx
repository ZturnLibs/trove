import { useMemo, useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Clock } from "lucide-react";
import { TaskDetailPanel } from "@/design-system/patterns/TaskDetailPanel";
import { TaskRow } from "@/design-system/patterns/TaskRow";
import { EmptyState } from "@/components/PageScaffold";
import { NotificationPermissionBanner } from "@/components/NotificationPermissionBanner";
import { Button } from "@/design-system/primitives/Button";
import { ipc, type TodayReminderItem } from "@/ipc/client";
import {
  NewTaskButton,
  SplitTaskLayout,
  TaskGroup,
} from "@/features/tasks/TaskLayout";
import { useDomainInvalidation } from "@/features/tasks/useDomainInvalidation";
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

export function TodayPage() {
  useDomainInvalidation();
  const queryClient = useQueryClient();
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [selectedReminderId, setSelectedReminderId] = useState<string | null>(null);
  const [completedCollapsed, setCompletedCollapsed] = useState(true);
  const [createdId, setCreatedId] = useState<string | null>(null);

  const todayQuery = useQuery({
    queryKey: ["tasks", "today"],
    queryFn: () => ipc.taskToday(),
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

  const toggleMutation = useMutation({
    mutationFn: async (id: string) => {
      const all = [
        ...(todayQuery.data?.overdue ?? []),
        ...(todayQuery.data?.dueToday ?? []),
        ...(todayQuery.data?.completedToday ?? []),
      ];
      const task = all.find((t) => t.id === id);
      if (!task) return;
      if (task.status === "completed") await ipc.taskUncomplete(id);
      else await ipc.taskComplete(id);
    },
    onSuccess: () => void queryClient.invalidateQueries({ queryKey: ["tasks"] }),
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
            variant="secondary"
            onClick={() => createReminderMutation.mutate()}
          >
            新建提醒
          </Button>
          <NewTaskButton onClick={() => createMutation.mutate()} />
        </>
      }
      list={
        todayQuery.isLoading ? (
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
                  onToggleComplete={() => toggleMutation.mutate(task.id)}
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
                  onToggleComplete={() => toggleMutation.mutate(task.id)}
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
                  onToggleComplete={() => toggleMutation.mutate(task.id)}
                />
              ))}
            </TaskGroup>
          </div>
        )
      }
      detail={
        selectedReminder ? (
          <div className="flex h-full flex-col p-4">
            <p className="text-[11px] text-muted">提醒</p>
            <h2 className="mt-1 text-[15px] font-semibold">
              {selectedReminder.reminder.title}
            </h2>
            <p className="mt-2 text-[12px] text-muted">
              计划时间 {selectedReminder.occurrence.scheduledAt.replace("T", " ")}
            </p>
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
                onClick={() =>
                  reminderComplete.mutate(selectedReminder.occurrence.id)
                }
              >
                完成
              </Button>
            </div>
          </div>
        ) : (
          <TaskDetailPanel
            task={selected}
            onDeleted={() => setSelectedId(null)}
            focusTitleId={createdId}
          />
        )
      }
    />
    </>
  );
}
