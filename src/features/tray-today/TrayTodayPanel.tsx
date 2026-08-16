import { useEffect, useMemo, useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { emit, listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import {
  Check,
  ChevronRight,
  Clock,
  ExternalLink,
  MoreHorizontal,
} from "lucide-react";
import { Button } from "@/design-system/primitives/Button";
import { ipc, type Task, type TodayReminderItem } from "@/ipc/client";
import { useDomainInvalidation } from "@/features/tasks/useDomainInvalidation";
import { deferPresets, localTodayString } from "@/lib/defer";
import { cn } from "@/lib/cn";

function formatTime(iso: string): string {
  return iso.slice(11, 16);
}

function nextReminder(items: TodayReminderItem[]): TodayReminderItem | null {
  if (items.length === 0) return null;
  return [...items].sort((a, b) =>
    a.occurrence.scheduledAt.localeCompare(b.occurrence.scheduledAt),
  )[0];
}

function TrayTaskRow({
  task,
  overdue,
  onComplete,
  onDefer,
  onOpen,
}: {
  task: Task;
  overdue?: boolean;
  onComplete: () => void;
  onDefer: (availableAt: string) => void;
  onOpen: () => void;
}) {
  const [menuOpen, setMenuOpen] = useState(false);
  const presets = deferPresets(localTodayString()).filter((p) => p.value);

  return (
    <div className="group flex min-h-9 items-center gap-1.5 border-b border-border px-2 py-1.5 last:border-b-0">
      <button
        type="button"
        aria-label="完成"
        className="flex h-6 w-6 shrink-0 items-center justify-center rounded-[var(--radius-control)] text-muted hover:bg-row-hover hover:text-foreground"
        onClick={onComplete}
      >
        <Check className="h-3.5 w-3.5" />
      </button>
      <div className="min-w-0 flex-1">
        <div
          className={cn(
            "truncate text-[12px]",
            overdue && "text-destructive",
          )}
        >
          {task.title}
        </div>
        {task.dueDate ? (
          <div className="text-[10px] text-muted">截止 {task.dueDate}</div>
        ) : null}
      </div>
      <div className="relative shrink-0">
        <button
          type="button"
          aria-label="更多操作"
          className="flex h-6 w-6 items-center justify-center rounded-[var(--radius-control)] text-muted opacity-0 hover:bg-row-hover group-hover:opacity-100"
          onClick={() => setMenuOpen((v) => !v)}
        >
          <MoreHorizontal className="h-3.5 w-3.5" />
        </button>
        {menuOpen ? (
          <>
            <button
              type="button"
              aria-label="关闭菜单"
              className="fixed inset-0 z-10 cursor-default"
              onClick={() => setMenuOpen(false)}
            />
            <div className="absolute right-0 top-full z-20 mt-0.5 min-w-[7rem] rounded-[var(--radius-control)] border border-border bg-surface py-0.5 shadow-lg">
              {presets.map((preset) => (
                <button
                  key={preset.label}
                  type="button"
                  className="block w-full px-2 py-1 text-left text-[11px] hover:bg-row-hover"
                  onClick={() => {
                    setMenuOpen(false);
                    if (preset.value) onDefer(preset.value);
                  }}
                >
                  推迟 · {preset.label}
                </button>
              ))}
              <button
                type="button"
                className="block w-full px-2 py-1 text-left text-[11px] hover:bg-row-hover"
                onClick={() => {
                  setMenuOpen(false);
                  onOpen();
                }}
              >
                打开详情
              </button>
            </div>
          </>
        ) : null}
      </div>
    </div>
  );
}

function TrayReminderRow({
  item,
  onComplete,
  onOpen,
}: {
  item: TodayReminderItem;
  onComplete: () => void;
  onOpen: () => void;
}) {
  return (
    <div className="group flex min-h-9 items-center gap-1.5 border-b border-border px-2 py-1.5 last:border-b-0">
      <Clock className="h-3.5 w-3.5 shrink-0 text-muted" />
      <div className="min-w-0 flex-1">
        <div className="truncate text-[12px]">{item.reminder.title}</div>
        <div className="text-[10px] text-muted">
          {formatTime(item.occurrence.scheduledAt)}
        </div>
      </div>
      <Button size="sm" variant="ghost" className="h-6 px-1.5 text-[10px]" onClick={onComplete}>
        完成
      </Button>
      <button
        type="button"
        aria-label="打开详情"
        className="flex h-6 w-6 shrink-0 items-center justify-center rounded-[var(--radius-control)] text-muted opacity-0 hover:bg-row-hover group-hover:opacity-100"
        onClick={onOpen}
      >
        <ExternalLink className="h-3 w-3" />
      </button>
    </div>
  );
}

export function TrayTodayPanel() {
  useDomainInvalidation();
  const queryClient = useQueryClient();

  const todayQuery = useQuery({
    queryKey: ["tasks", "today"],
    queryFn: () => ipc.taskToday(),
  });

  useEffect(() => {
    let unlisten: (() => void) | undefined;
    void listen("tray-today://refresh", () => {
      void todayQuery.refetch();
    }).then((fn) => {
      unlisten = fn;
    });
    return () => unlisten?.();
  }, [todayQuery]);

  const completeTaskMutation = useMutation({
    mutationFn: (id: string) => ipc.taskComplete(id),
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: ["tasks", "today"] });
    },
  });

  const deferMutation = useMutation({
    mutationFn: ({ id, availableAt }: { id: string; availableAt: string }) =>
      ipc.taskSetDefer(id, availableAt),
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: ["tasks", "today"] });
    },
  });

  const completeReminderMutation = useMutation({
    mutationFn: (occurrenceId: string) => ipc.reminderComplete(occurrenceId),
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: ["tasks", "today"] });
      void queryClient.invalidateQueries({ queryKey: ["reminders"] });
    },
  });

  const data = todayQuery.data;
  const overdueCount = data?.overdue.length ?? 0;
  const dueCount = data?.dueToday.length ?? 0;
  const reminderCount = data?.remindersToday.length ?? 0;
  const focusCount = data?.focus.length ?? 0;

  const upcoming = useMemo(
    () => nextReminder(data?.remindersToday ?? []),
    [data?.remindersToday],
  );

  const taskRows = useMemo(() => {
    if (!data) return [];
    const seen = new Set<string>();
    const rows: { task: Task; overdue: boolean }[] = [];
    for (const task of data.overdue) {
      if (seen.has(task.id)) continue;
      seen.add(task.id);
      rows.push({ task, overdue: true });
    }
    for (const task of data.dueToday) {
      if (seen.has(task.id)) continue;
      seen.add(task.id);
      rows.push({ task, overdue: false });
    }
    return rows.slice(0, 8);
  }, [data]);

  const reminderRows = useMemo(
    () => (data?.remindersToday ?? []).slice(0, 4),
    [data?.remindersToday],
  );

  const isEmpty =
    overdueCount === 0 &&
    dueCount === 0 &&
    reminderCount === 0 &&
    focusCount === 0;

  async function openTaskInMain(taskId: string) {
    await ipc.windowShowMain();
    await emit("main://navigate", "/today");
    await emit("main://select-task", taskId);
    await getCurrentWindow().hide();
  }

  async function openReminderInMain(reminderId: string) {
    await ipc.windowShowMain();
    await emit("main://navigate", "/today");
    await emit("main://select-reminder", reminderId);
    await getCurrentWindow().hide();
  }

  async function openTodayPage() {
    await ipc.windowShowMain();
    await emit("main://navigate", "/today");
    await getCurrentWindow().hide();
  }

  return (
    <div className="flex h-screen flex-col bg-surface text-foreground">
      <header className="border-b border-border px-3 py-2.5">
        <div className="flex items-center justify-between gap-2">
          <div>
            <h1 className="text-[13px] font-semibold">今日</h1>
            <p className="text-[10px] text-muted">{data?.today ?? localTodayString()}</p>
          </div>
          <button
            type="button"
            className="flex items-center gap-0.5 text-[10px] text-muted hover:text-foreground"
            onClick={() => void openTodayPage()}
          >
            打开主窗口
            <ChevronRight className="h-3 w-3" />
          </button>
        </div>
        <dl className="mt-2 flex flex-wrap gap-x-3 gap-y-0.5 text-[10px] text-muted">
          <div>
            <span className="font-medium text-foreground">{overdueCount}</span> 逾期
          </div>
          <div>
            <span className="font-medium text-foreground">{dueCount}</span> 今日任务
          </div>
          <div>
            <span className="font-medium text-foreground">{focusCount}</span> 重点
          </div>
          <div>
            <span className="font-medium text-foreground">{reminderCount}</span> 提醒
          </div>
        </dl>
        {upcoming ? (
          <div className="mt-2 rounded-[var(--radius-control)] border border-border bg-surface-raised px-2 py-1.5 text-[10px]">
            <span className="text-muted">下一条提醒 · </span>
            <span className="font-medium">{upcoming.reminder.title}</span>
            <span className="text-muted">
              {" "}
              {formatTime(upcoming.occurrence.scheduledAt)}
            </span>
          </div>
        ) : null}
      </header>

      <div className="min-h-0 flex-1 overflow-y-auto">
        {todayQuery.isLoading ? (
          <p className="px-3 py-4 text-[11px] text-muted">加载中…</p>
        ) : isEmpty ? (
          <div className="px-3 py-8 text-center">
            <p className="text-[12px] font-medium">今天没有待办</p>
            <p className="mt-1 text-[11px] text-muted">享受一下空白，或从菜单快速记录</p>
          </div>
        ) : (
          <>
            {taskRows.length > 0 ? (
              <section>
                <h2 className="px-3 py-1.5 text-[10px] font-medium uppercase tracking-wide text-muted">
                  任务
                </h2>
                {taskRows.map(({ task, overdue }) => (
                  <TrayTaskRow
                    key={task.id}
                    task={task}
                    overdue={overdue}
                    onComplete={() => completeTaskMutation.mutate(task.id)}
                    onDefer={(availableAt) =>
                      deferMutation.mutate({ id: task.id, availableAt })
                    }
                    onOpen={() => void openTaskInMain(task.id)}
                  />
                ))}
              </section>
            ) : null}
            {reminderRows.length > 0 ? (
              <section>
                <h2 className="px-3 py-1.5 text-[10px] font-medium uppercase tracking-wide text-muted">
                  提醒
                </h2>
                {reminderRows.map((item) => (
                  <TrayReminderRow
                    key={item.occurrence.id}
                    item={item}
                    onComplete={() =>
                      completeReminderMutation.mutate(item.occurrence.id)
                    }
                    onOpen={() => void openReminderInMain(item.reminder.id)}
                  />
                ))}
              </section>
            ) : null}
          </>
        )}
      </div>
    </div>
  );
}
