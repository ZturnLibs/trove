import { useEffect, useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useNavigate } from "react-router-dom";
import { PageScaffold } from "@/components/PageScaffold";
import { WeeklySummaryCard } from "@/features/weekly-review/WeeklySummaryCard";
import { TaskDetailPanel } from "@/design-system/patterns/TaskDetailPanel";
import { Button } from "@/design-system/primitives/Button";
import {
  ipc,
  type ClipboardItem,
  type Reminder,
  type Task,
  type WeeklyReviewSnapshot,
} from "@/ipc/client";
import { useRecentActions } from "@/stores/recent-actions";
import { useFocusSession } from "@/stores/focus-session";
import { cn } from "@/lib/cn";

function daysSince(iso: string): number {
  const then = new Date(iso);
  const now = new Date();
  return Math.max(
    0,
    Math.floor((now.getTime() - then.getTime()) / 86_400_000),
  );
}

function ReviewCard({
  title,
  count,
  hint,
  children,
  onViewAll,
}: {
  title: string;
  count: number;
  hint?: string;
  children: React.ReactNode;
  onViewAll?: () => void;
}) {
  return (
    <section className="rounded-[var(--radius-panel)] border border-border bg-surface p-3">
      <div className="mb-2 flex items-start justify-between gap-2">
        <div>
          <h2 className="text-[13px] font-medium">
            {title}{" "}
            <span className="text-muted">({count})</span>
          </h2>
          {hint ? <p className="mt-0.5 text-[11px] text-muted">{hint}</p> : null}
        </div>
        {onViewAll && count > 0 ? (
          <Button type="button" size="sm" variant="ghost" onClick={onViewAll}>
            查看全部
          </Button>
        ) : null}
      </div>
      {children}
    </section>
  );
}

function TaskReviewRow({
  task,
  selected,
  onSelect,
  onComplete,
}: {
  task: Task;
  selected: boolean;
  onSelect: () => void;
  onComplete: () => void;
}) {
  return (
    <li
      className={cn(
        "flex items-center gap-2 rounded-[var(--radius-control)] border border-border px-2 py-1.5 text-[12px]",
        selected && "bg-row-active",
      )}
    >
      <button
        type="button"
        className="min-w-0 flex-1 truncate text-left hover:underline"
        onClick={onSelect}
      >
        {task.title}
      </button>
      {task.status === "todo" ? (
        <Button type="button" size="sm" variant="ghost" onClick={onComplete}>
          完成
        </Button>
      ) : null}
    </li>
  );
}

function ReminderReviewRow({ reminder }: { reminder: Reminder }) {
  return (
    <li className="truncate rounded-[var(--radius-control)] border border-border px-2 py-1.5 text-[12px]">
      {reminder.title}
      <span className="ml-2 text-[11px] text-muted">
        {reminder.nextFireAt.slice(0, 16).replace("T", " ")}
      </span>
    </li>
  );
}

function ClipboardReviewRow({
  item,
  onFavorite,
}: {
  item: ClipboardItem;
  onFavorite: () => void;
}) {
  return (
    <li className="flex items-center justify-between gap-2 rounded-[var(--radius-control)] border border-border px-2 py-1.5 text-[12px]">
      <span className="truncate">
        图片
        {item.width && item.height ? ` ${item.width}×${item.height}` : ""}
      </span>
      <Button type="button" size="sm" variant="ghost" onClick={onFavorite}>
        收藏
      </Button>
    </li>
  );
}

export function WeeklyReviewPage() {
  const navigate = useNavigate();
  const queryClient = useQueryClient();
  const startFocus = useFocusSession((s) => s.start);
  const [selectedTaskId, setSelectedTaskId] = useState<string | null>(null);
  const [sessionId, setSessionId] = useState<string | null>(null);

  const lastQuery = useQuery({
    queryKey: ["weekly-review", "last"],
    queryFn: () => ipc.weeklyReviewLastCompleted(),
  });

  const snapshotQuery = useQuery({
    queryKey: ["weekly-review", "snapshot"],
    queryFn: () => ipc.weeklyReviewSnapshot(),
  });

  useEffect(() => {
    void ipc.weeklyReviewStart().then((session) => setSessionId(session.id));
  }, []);

  const completeReview = useMutation({
    mutationFn: async () => {
      if (!sessionId || !snapshotQuery.data) {
        throw new Error("回顾尚未就绪");
      }
      const snap = snapshotQuery.data;
      return ipc.weeklyReviewComplete(sessionId, {
        summary: {
          inboxCount: snap.inboxCount,
          overdueCount: snap.overdueCount,
          waitingFollowUpCount: snap.waitingFollowUpCount,
          staleActiveCount: snap.staleActiveCount,
          completedLast7DaysCount: snap.completedLast7DaysCount,
          upcomingRecurringCount: snap.upcomingRecurringCount,
          largeClipboardCount: snap.largeClipboardCount,
        },
      });
    },
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: ["weekly-review"] });
    },
  });

  const completeTask = useMutation({
    mutationFn: (task: Task) => ipc.taskComplete(task.id),
    onSuccess: (_data, task) => {
      void queryClient.invalidateQueries({ queryKey: ["tasks"] });
      void queryClient.invalidateQueries({ queryKey: ["weekly-review"] });
      useRecentActions.getState().push({
        label: "完成",
        undo: async () => {
          await ipc.taskUncomplete(task.id);
          void queryClient.invalidateQueries({ queryKey: ["weekly-review"] });
        },
      });
    },
  });

  const favoriteClipboard = useMutation({
    mutationFn: (id: string) => ipc.clipboardSetFavorite(id, true),
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: ["weekly-review"] });
      void queryClient.invalidateQueries({ queryKey: ["clipboard"] });
    },
  });

  const selectedTaskQuery = useQuery({
    queryKey: ["tasks", selectedTaskId],
    queryFn: () => ipc.taskGet(selectedTaskId!),
    enabled: !!selectedTaskId,
  });

  const lastCompleted = lastQuery.data?.completedAt;
  const headerHint = lastCompleted
    ? `距上次完成回顾 ${daysSince(lastCompleted)} 天`
    : "首次每周回顾";

  const snap = snapshotQuery.data;

  return (
    <div className="flex h-full min-h-0">
      <div className="min-w-0 flex-1">
        <PageScaffold
          title="每周回顾"
          description={headerHint}
          actions={
            <Button
              size="sm"
              disabled={completeReview.isPending || !sessionId}
              onClick={() => completeReview.mutate()}
            >
              {completeReview.isPending ? "保存中…" : "完成本次回顾"}
            </Button>
          }
        >
          {snapshotQuery.isLoading ? (
            <p className="p-4 text-[12px] text-muted">加载中…</p>
          ) : snap ? (
            <div className="space-y-3 p-4">
              <p className="text-[12px] text-muted">
                以下信号由本地数据聚合，不含效率评分。逐项处理后计数会自动更新。
              </p>
              <WeeklySummaryCard snap={snap} />
              <ReviewCards
                snap={snap}
                selectedTaskId={selectedTaskId}
                onSelectTask={setSelectedTaskId}
                onCompleteTask={(task) => completeTask.mutate(task)}
                onFavoriteClipboard={(id) => favoriteClipboard.mutate(id)}
                onNavigate={navigate}
              />
            </div>
          ) : (
            <p className="p-4 text-[12px] text-danger">加载失败</p>
          )}
        </PageScaffold>
      </div>
      <aside className="w-[360px] shrink-0 border-l border-border bg-surface-raised">
        <TaskDetailPanel
          task={selectedTaskQuery.data ?? null}
          onStartFocus={
            selectedTaskQuery.data?.status === "todo"
              ? () => void startFocus(selectedTaskQuery.data!.id)
              : undefined
          }
        />
      </aside>
    </div>
  );
}

function ReviewCards({
  snap,
  selectedTaskId,
  onSelectTask,
  onCompleteTask,
  onFavoriteClipboard,
  onNavigate,
}: {
  snap: WeeklyReviewSnapshot;
  selectedTaskId: string | null;
  onSelectTask: (id: string) => void;
  onCompleteTask: (task: Task) => void;
  onFavoriteClipboard: (id: string) => void;
  onNavigate: (path: string) => void;
}) {
  const empty = (n: number) =>
    n === 0 ? (
      <p className="text-[11px] text-muted">暂无项</p>
    ) : null;

  return (
    <>
      <ReviewCard
        title="未整理收件箱"
        count={snap.inboxCount}
        hint="需要分类或安排的任务"
        onViewAll={() => onNavigate("/inbox")}
      >
        {snap.inboxCount === 0 ? (
          empty(0)
        ) : (
          <ul className="space-y-1">
            {snap.inboxUnprocessed.map((task) => (
              <TaskReviewRow
                key={task.id}
                task={task}
                selected={selectedTaskId === task.id}
                onSelect={() => onSelectTask(task.id)}
                onComplete={() => onCompleteTask(task)}
              />
            ))}
          </ul>
        )}
      </ReviewCard>

      <ReviewCard
        title="逾期任务"
        count={snap.overdueCount}
        onViewAll={() => onNavigate("/tasks")}
      >
        {snap.overdueCount === 0 ? (
          empty(0)
        ) : (
          <ul className="space-y-1">
            {snap.overdue.map((task) => (
              <TaskReviewRow
                key={task.id}
                task={task}
                selected={selectedTaskId === task.id}
                onSelect={() => onSelectTask(task.id)}
                onComplete={() => onCompleteTask(task)}
              />
            ))}
          </ul>
        )}
      </ReviewCard>

      <ReviewCard
        title="等待跟进"
        count={snap.waitingFollowUpCount}
        onViewAll={() => onNavigate("/today")}
      >
        {snap.waitingFollowUpCount === 0 ? (
          empty(0)
        ) : (
          <ul className="space-y-1">
            {snap.waitingFollowUp.map((task) => (
              <TaskReviewRow
                key={task.id}
                task={task}
                selected={selectedTaskId === task.id}
                onSelect={() => onSelectTask(task.id)}
                onComplete={() => onCompleteTask(task)}
              />
            ))}
          </ul>
        )}
      </ReviewCard>

      <ReviewCard
        title="长期未更新"
        count={snap.staleActiveCount}
        hint="14 天以上未修改的活跃任务"
      >
        {snap.staleActiveCount === 0 ? (
          empty(0)
        ) : (
          <ul className="space-y-1">
            {snap.staleActive.map((task) => (
              <TaskReviewRow
                key={task.id}
                task={task}
                selected={selectedTaskId === task.id}
                onSelect={() => onSelectTask(task.id)}
                onComplete={() => onCompleteTask(task)}
              />
            ))}
          </ul>
        )}
      </ReviewCard>

      <ReviewCard
        title="近 7 天已完成"
        count={snap.completedLast7DaysCount}
      >
        {snap.completedLast7DaysCount === 0 ? (
          empty(0)
        ) : (
          <ul className="space-y-1">
            {snap.completedLast7Days.map((task) => (
              <TaskReviewRow
                key={task.id}
                task={task}
                selected={selectedTaskId === task.id}
                onSelect={() => onSelectTask(task.id)}
                onComplete={() => onCompleteTask(task)}
              />
            ))}
          </ul>
        )}
      </ReviewCard>

      <ReviewCard
        title="即将到来周期提醒"
        count={snap.upcomingRecurringCount}
        onViewAll={() => onNavigate("/today")}
      >
        {snap.upcomingRecurringCount === 0 ? (
          empty(0)
        ) : (
          <ul className="space-y-1">
            {snap.upcomingRecurringReminders.map((reminder) => (
              <ReminderReviewRow key={reminder.id} reminder={reminder} />
            ))}
          </ul>
        )}
      </ReviewCard>

      <ReviewCard
        title="大体积未收藏剪切板"
        count={snap.largeClipboardCount}
        hint="≥ 500 KB 的图片"
        onViewAll={() => onNavigate("/clipboard")}
      >
        {snap.largeClipboardCount === 0 ? (
          empty(0)
        ) : (
          <ul className="space-y-1">
            {snap.largeClipboardItems.map((item) => (
              <ClipboardReviewRow
                key={item.id}
                item={item}
                onFavorite={() => onFavoriteClipboard(item.id)}
              />
            ))}
          </ul>
        )}
      </ReviewCard>
    </>
  );
}
