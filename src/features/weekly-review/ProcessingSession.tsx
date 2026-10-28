import { useCallback, useEffect, useRef, useState } from "react";
import { useMutation, useQueryClient } from "@tanstack/react-query";
import { Button } from "@/design-system/primitives/Button";
import { ipc, type Task } from "@/ipc/client";
import { useRecentActions } from "@/stores/recent-actions";
import {
  processingSummary,
  type ActionKind,
  type ProcessingItem,
  type ProcessingSummary,
} from "./processingSummary";

const ACTIONS: {
  kind: Exclude<ActionKind, "skipped">;
  key: string;
  label: string;
}[] = [
  { kind: "completed", key: "1", label: "完成" },
  { kind: "deferred", key: "2", label: "延期" },
  { kind: "waiting", key: "3", label: "转等待" },
];

function tomorrowIso(): string {
  return new Date(Date.now() + 86_400_000).toISOString();
}

function isTypingTarget(target: EventTarget | null): boolean {
  if (!(target instanceof HTMLElement)) return false;
  return (
    target.tagName === "INPUT" ||
    target.tagName === "TEXTAREA" ||
    target.tagName === "SELECT" ||
    target.isContentEditable
  );
}

export function ProcessingSession({
  items,
  onFinish,
  onCompleteReview,
  onSelectTask,
  onExit,
}: {
  items: ProcessingItem[];
  /** 队列清空时上报最终统计（每次动作后也会随进度更新）。 */
  onFinish: (summary: ProcessingSummary) => void;
  onCompleteReview: () => void;
  onSelectTask?: (taskId: string) => void;
  onExit: () => void;
}) {
  const queryClient = useQueryClient();
  const pushRecentAction = useRecentActions((s) => s.push);
  const [queueIndex, setQueueIndex] = useState(0);
  const [actions, setActions] = useState<ActionKind[]>([]);
  const [notice, setNotice] = useState<string | null>(null);
  const pendingRef = useRef(false);

  const done = queueIndex >= items.length;
  const summary = processingSummary(actions);

  useEffect(() => {
    if (done) onFinish(summary);
    // eslint-disable-next-line react-hooks/exhaustive-deps -- 仅在清空瞬间上报最终统计
  }, [done]);

  const invalidate = useCallback(() => {
    void queryClient.invalidateQueries({ queryKey: ["tasks"] });
    void queryClient.invalidateQueries({ queryKey: ["weekly-review"] });
  }, [queryClient]);

  const completeTask = useMutation({
    mutationFn: (task: Task) => ipc.taskComplete(task.id),
    onError: () => setNotice("操作失败，请重试"),
    onSuccess: (_data, task) => {
      invalidate();
      pushRecentAction({
        label: "完成",
        undo: async () => {
          await ipc.taskUncomplete(task.id);
          void queryClient.invalidateQueries({
            queryKey: ["weekly-review"],
          });
        },
      });
    },
  });

  const deferTask = useMutation({
    mutationFn: (task: Task) => ipc.taskSetDefer(task.id, tomorrowIso()),
    onError: () => setNotice("延期失败（截止早于显示日期等校验未过），可跳过或改用详情面板"),
    onSuccess: () => invalidate(),
  });

  const waitTask = useMutation({
    mutationFn: (task: Task) => ipc.taskSetWaiting(task.id, null, null),
    onError: () => setNotice("转等待失败，请重试"),
    onSuccess: () => invalidate(),
  });

  const advance = useCallback((kind: ActionKind) => {
    setActions((prev) => [...prev, kind]);
    setQueueIndex((prev) => prev + 1);
    setNotice(null);
  }, []);

  const handleAction = useCallback(
    async (kind: ActionKind) => {
      if (done || pendingRef.current) return;
      const item = items[queueIndex];
      if (!item) return;
      if (kind === "skipped") {
        advance("skipped");
        return;
      }
      pendingRef.current = true;
      try {
        // 出牌前校验：任务可能已被其他窗口完结/归档。
        const latest = await ipc.taskGet(item.task.id);
        if (latest.status !== "todo") {
          setNotice(`「${latest.title}」已不在待办状态，已自动跳过`);
          advance("skipped");
          return;
        }
        if (kind === "completed") completeTask.mutate(latest);
        else if (kind === "deferred") deferTask.mutate(latest);
        else waitTask.mutate(latest);
        advance(kind);
      } finally {
        pendingRef.current = false;
      }
    },
    [advance, completeTask, deferTask, done, items, queueIndex, waitTask],
  );

  useEffect(() => {
    if (done) return;
    const onKeyDown = (event: KeyboardEvent) => {
      if (isTypingTarget(event.target)) return;
      if (event.key === "Escape") {
        event.preventDefault();
        onExit();
        return;
      }
      const matched = ACTIONS.find((action) => action.key === event.key);
      if (matched) {
        event.preventDefault();
        void handleAction(matched.kind);
      }
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [done, handleAction, onExit]);

  if (done) {
    return (
      <section className="rounded-[var(--radius-panel)] border border-border bg-surface p-4">
        <h2 className="text-[13px] font-medium">处理完毕</h2>
        <p className="mt-1 text-[12px] text-muted">
          共 {summary.total} 条：完成 {summary.completed} · 延期{" "}
          {summary.deferred} · 转等待 {summary.waiting} · 跳过 {summary.skipped}
        </p>
        <div className="mt-3 flex gap-2">
          <Button size="sm" onClick={onCompleteReview}>
            完成回顾
          </Button>
          <Button size="sm" variant="ghost" onClick={onExit}>
            返回回顾页
          </Button>
        </div>
      </section>
    );
  }

  const item = items[queueIndex];
  const task = item.task;
  const overdueDays =
    task.dueDate != null
      ? Math.max(
          0,
          Math.floor((Date.now() - new Date(task.dueDate).getTime()) / 86_400_000),
        )
      : null;

  return (
    <section className="rounded-[var(--radius-panel)] border border-border bg-surface p-4">
      <div className="flex items-start justify-between gap-2">
        <p className="text-[12px] text-muted">
          第 {queueIndex + 1} / {items.length} 条 · {item.queueLabel}
        </p>
        <Button type="button" size="sm" variant="ghost" onClick={onExit}>
          退出 (Esc)
        </Button>
      </div>
      <h2 className="mt-2 text-[15px] font-medium">{task.title}</h2>
      <p className="mt-1 text-[12px] text-muted">
        {task.listName}
        {overdueDays != null && overdueDays > 0 ? ` · 逾期 ${overdueDays} 天` : ""}
      </p>
      {task.notes ? (
        <p className="mt-2 line-clamp-3 text-[12px] text-muted">{task.notes}</p>
      ) : null}
      {notice ? (
        <p className="mt-2 text-[12px] text-warning">{notice}</p>
      ) : null}
      <div className="mt-3 flex flex-wrap gap-2">
        {ACTIONS.map((action) => (
          <Button
            key={action.kind}
            type="button"
            size="sm"
            onClick={() => void handleAction(action.kind)}
          >
            {action.label}
            <span className="ml-1 text-[11px] text-muted">
              ({action.key})
            </span>
          </Button>
        ))}
        <Button
          type="button"
          size="sm"
          variant="ghost"
          onClick={() => void handleAction("skipped")}
        >
          跳过
          <span className="ml-1 text-[11px] text-muted">(4)</span>
        </Button>
        {onSelectTask ? (
          <Button
            type="button"
            size="sm"
            variant="ghost"
            onClick={() => onSelectTask(task.id)}
          >
            查看详情
          </Button>
        ) : null}
      </div>
    </section>
  );
}
