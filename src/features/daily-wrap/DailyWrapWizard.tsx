import { useEffect, useRef, useState } from "react";
import { useQueryClient } from "@tanstack/react-query";
import { Button } from "@/design-system/primitives/Button";
import { Input } from "@/design-system/primitives/Input";
import { DeferPicker } from "@/design-system/patterns/DeferPicker";
import { ipc, type Task } from "@/ipc/client";
import { followUpPresets, localTodayString } from "@/lib/waiting";
import { useRecentActions } from "@/stores/recent-actions";
import { cn } from "@/lib/cn";

type InlineAction = "defer" | "wait" | null;

type DecisionCounts = {
  keep: number;
  defer: number;
  wait: number;
  complete: number;
  removeFocus: number;
};

function InlineWaitingForm({
  task,
  disabled,
  onApplied,
  onCancel,
}: {
  task: Task;
  disabled?: boolean;
  onApplied: () => void;
  onCancel: () => void;
}) {
  const queryClient = useQueryClient();
  const [draftFor, setDraftFor] = useState("");
  const [draftFollowUp, setDraftFollowUp] = useState(localTodayString());
  const [pending, setPending] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const apply = async () => {
    const trimmed = draftFor.trim();
    if (!trimmed) {
      setError("请填写等待对象");
      return;
    }
    setPending(true);
    setError(null);
    const prev = {
      workflowState: task.workflowState,
      waitingFor: task.waitingFor,
      followUpDate: task.followUpDate,
    };
    try {
      await ipc.taskSetWaiting(task.id, trimmed, draftFollowUp.trim() || null);
      void queryClient.invalidateQueries({ queryKey: ["tasks"] });
      useRecentActions.getState().push({
        label: `标记等待：${trimmed}`,
        undo: async () => {
          if (prev.workflowState === "waiting") {
            await ipc.taskSetWaiting(
              task.id,
              prev.waitingFor,
              prev.followUpDate,
            );
          } else {
            await ipc.taskClearWaiting(task.id);
          }
          void queryClient.invalidateQueries({ queryKey: ["tasks"] });
        },
      });
      onApplied();
    } catch (err) {
      setError(err instanceof Error ? err.message : "设置失败");
    } finally {
      setPending(false);
    }
  };

  return (
    <div className="mt-2 space-y-2 rounded-[var(--radius-control)] border border-border bg-surface p-2">
      <label className="block space-y-1 text-[11px] text-muted">
        等待对象
        <Input
          value={draftFor}
          disabled={disabled || pending}
          placeholder="例如：张三的回复"
          onChange={(e) => setDraftFor(e.target.value)}
        />
      </label>
      <label className="block space-y-1 text-[11px] text-muted">
        跟进日（可选）
        <div className="flex flex-wrap gap-1">
          {followUpPresets(localTodayString()).map((preset) => (
            <Button
              key={preset.label}
              type="button"
              size="sm"
              variant={draftFollowUp === preset.value ? "secondary" : "ghost"}
              disabled={disabled || pending}
              onClick={() => setDraftFollowUp(preset.value)}
            >
              {preset.label}
            </Button>
          ))}
        </div>
        <Input
          type="date"
          className="mt-1 h-8 text-[12px]"
          value={draftFollowUp}
          disabled={disabled || pending}
          onChange={(e) => setDraftFollowUp(e.target.value)}
        />
      </label>
      {error ? <p className="text-[11px] text-danger">{error}</p> : null}
      <div className="flex gap-2">
        <Button
          type="button"
          size="sm"
          disabled={disabled || pending}
          onClick={() => void apply()}
        >
          确认等待
        </Button>
        <Button type="button" size="sm" variant="ghost" onClick={onCancel}>
          取消
        </Button>
      </div>
    </div>
  );
}

function FocusItemRow({
  task,
  wrapDate,
  disabled,
  onResolved,
  onDecision,
}: {
  task: Task;
  wrapDate: string;
  disabled?: boolean;
  onResolved: () => void;
  onDecision: (kind: keyof DecisionCounts) => void;
}) {
  const queryClient = useQueryClient();
  const [inline, setInline] = useState<InlineAction>(null);
  const [pending, setPending] = useState(false);

  const run = async (fn: () => Promise<void>) => {
    if (pending || disabled) return;
    setPending(true);
    try {
      await fn();
      onResolved();
    } finally {
      setPending(false);
      setInline(null);
    }
  };

  const keep = () => {
    onDecision("keep");
    onResolved();
  };

  const complete = () =>
    run(async () => {
      await ipc.taskComplete(task.id);
      void queryClient.invalidateQueries({ queryKey: ["tasks"] });
      useRecentActions.getState().push({
        label: "完成",
        undo: async () => {
          await ipc.taskUncomplete(task.id);
          void queryClient.invalidateQueries({ queryKey: ["tasks"] });
        },
      });
      onDecision("complete");
    });

  const removeFocus = () =>
    run(async () => {
      await ipc.dailyFocusRemove(task.id, wrapDate);
      void queryClient.invalidateQueries({ queryKey: ["tasks"] });
      useRecentActions.getState().push({
        label: "移出今日重点",
        undo: async () => {
          await ipc.dailyFocusAdd(task.id, wrapDate);
          void queryClient.invalidateQueries({ queryKey: ["tasks"] });
        },
      });
      onDecision("removeFocus");
    });

  return (
    <li className="rounded-[var(--radius-control)] border border-border bg-surface px-3 py-2">
      <div className="flex flex-wrap items-start justify-between gap-2">
        <div className="min-w-0 flex-1">
          <p className="truncate text-[13px] font-medium">{task.title}</p>
          {task.dueDate ? (
            <p className="text-[11px] text-muted">截止 {task.dueDate}</p>
          ) : null}
        </div>
        <div className="flex flex-wrap gap-1">
          <Button
            type="button"
            size="sm"
            variant="ghost"
            disabled={disabled || pending}
            onClick={keep}
          >
            保留
          </Button>
          <Button
            type="button"
            size="sm"
            variant="ghost"
            disabled={disabled || pending}
            onClick={() => setInline(inline === "defer" ? null : "defer")}
          >
            推迟
          </Button>
          <Button
            type="button"
            size="sm"
            variant="ghost"
            disabled={disabled || pending}
            onClick={() => setInline(inline === "wait" ? null : "wait")}
          >
            等待
          </Button>
          <Button
            type="button"
            size="sm"
            variant="ghost"
            disabled={disabled || pending}
            onClick={() => void complete()}
          >
            完成
          </Button>
          <Button
            type="button"
            size="sm"
            variant="ghost"
            disabled={disabled || pending}
            onClick={() => void removeFocus()}
          >
            移出重点
          </Button>
        </div>
      </div>

      {inline === "defer" ? (
        <div className="mt-2 border-t border-border pt-2">
          <DeferPicker
            compact
            availableAt={task.availableAt}
            dueDate={task.dueDate}
            disabled={disabled || pending}
            onChange={async (availableAt) => {
              const prev = task.availableAt;
              await ipc.taskSetDefer(task.id, availableAt);
              void queryClient.invalidateQueries({ queryKey: ["tasks"] });
              useRecentActions.getState().push({
                label: availableAt
                  ? `推迟显示至 ${availableAt}`
                  : "取消推迟显示",
                undo: async () => {
                  await ipc.taskSetDefer(task.id, prev);
                  void queryClient.invalidateQueries({ queryKey: ["tasks"] });
                },
              });
              onDecision("defer");
              onResolved();
              setInline(null);
            }}
          />
        </div>
      ) : null}

      {inline === "wait" ? (
        <InlineWaitingForm
          task={task}
          disabled={disabled || pending}
          onApplied={() => {
            onDecision("wait");
            onResolved();
          }}
          onCancel={() => setInline(null)}
        />
      ) : null}
    </li>
  );
}

const STEP_TITLES = [
  "今日重点未完成",
  "明日预览",
  "收件箱",
  "当日摘要",
] as const;

export function DailyWrapWizard({
  open,
  wrapDate,
  onClose,
  onCompleted,
  onNavigate,
}: {
  open: boolean;
  wrapDate: string;
  onClose: () => void;
  onCompleted: () => void;
  onNavigate?: (path: string) => void;
}) {
  const queryClient = useQueryClient();
  const [step, setStep] = useState(0);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [runId, setRunId] = useState<string | null>(null);
  const [snapshot, setSnapshot] = useState<
    Awaited<ReturnType<typeof ipc.dailyWrapSnapshot>> | null
  >(null);
  const [keptIds, setKeptIds] = useState<Set<string>>(() => new Set());
  const initialUnfinishedRef = useRef(0);
  const decisionsRef = useRef<DecisionCounts>({
    keep: 0,
    defer: 0,
    wait: 0,
    complete: 0,
    removeFocus: 0,
  });
  const [refreshKey, setRefreshKey] = useState(0);

  useEffect(() => {
    if (!open) return;
    setStep(0);
    setError(null);
    setKeptIds(new Set());
    decisionsRef.current = {
      keep: 0,
      defer: 0,
      wait: 0,
      complete: 0,
      removeFocus: 0,
    };

    let cancelled = false;
    setLoading(true);
    void (async () => {
      try {
        const [run, snap] = await Promise.all([
          ipc.dailyWrapStart(wrapDate),
          ipc.dailyWrapSnapshot(wrapDate),
        ]);
        if (cancelled) return;
        setRunId(run.id);
        setSnapshot(snap);
        initialUnfinishedRef.current = snap.unfinishedFocus.length;
      } catch (err) {
        if (!cancelled) {
          setError(err instanceof Error ? err.message : "加载失败");
        }
      } finally {
        if (!cancelled) setLoading(false);
      }
    })();

    return () => {
      cancelled = true;
    };
  }, [open, wrapDate]);

  useEffect(() => {
    if (!open) return;
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key !== "Escape") return;
      const target = event.target;
      if (
        target instanceof HTMLInputElement ||
        target instanceof HTMLTextAreaElement
      ) {
        return;
      }
      event.preventDefault();
      onClose();
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [open, onClose]);

  const refreshSnapshot = async () => {
    const snap = await ipc.dailyWrapSnapshot(wrapDate);
    setSnapshot(snap);
    setRefreshKey((k) => k + 1);
  };

  const visibleFocus = (snapshot?.unfinishedFocus ?? []).filter(
    (t) => !keptIds.has(t.id),
  );

  const focusProcessed =
    initialUnfinishedRef.current - visibleFocus.length;

  const focusRate =
    initialUnfinishedRef.current > 0
      ? Math.round((focusProcessed / initialUnfinishedRef.current) * 100)
      : null;

  const finish = async () => {
    if (!runId || !snapshot) return;
    setLoading(true);
    setError(null);
    try {
      await ipc.dailyWrapComplete(runId, {
        stepsCompleted: 5,
        summary: {
          decisions: { ...decisionsRef.current },
          completedTodayCount: snapshot.completedTodayCount,
          remindersTodayCount: snapshot.remindersTodayCount,
          initialUnfinishedFocus: initialUnfinishedRef.current,
          remainingUnfinishedFocus: visibleFocus.length,
          focusProcessRate: focusRate,
        },
      });
      void queryClient.invalidateQueries({ queryKey: ["tasks"] });
      void queryClient.invalidateQueries({ queryKey: ["daily-wrap"] });
      onCompleted();
    } catch (err) {
      setError(err instanceof Error ? err.message : "收尾失败");
    } finally {
      setLoading(false);
    }
  };

  if (!open) return null;

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center p-4">
      <button
        type="button"
        aria-label="关闭每日收尾"
        className="absolute inset-0 cursor-default bg-black/40"
        onClick={onClose}
      />
      <div
        className="relative flex max-h-[85vh] w-full max-w-lg flex-col rounded-[var(--radius-panel)] border border-border bg-surface shadow-xl"
        role="dialog"
        aria-modal="true"
        aria-label="每日收尾"
      >
        <header className="shrink-0 border-b border-border px-4 py-3">
          <div className="flex items-center justify-between gap-2">
            <h2 className="text-[14px] font-semibold">每日收尾</h2>
            <span className="text-[11px] text-muted">
              第 {step + 1} / {STEP_TITLES.length} 步 · {STEP_TITLES[step]}
            </span>
          </div>
          <div className="mt-2 flex gap-1">
            {STEP_TITLES.map((_, i) => (
              <div
                key={i}
                className={cn(
                  "h-1 flex-1 rounded-full",
                  i <= step ? "bg-accent" : "bg-border",
                )}
              />
            ))}
          </div>
        </header>

        <div className="min-h-0 flex-1 overflow-auto px-4 py-3">
          {loading && !snapshot ? (
            <p className="text-[12px] text-muted">加载中…</p>
          ) : error && !snapshot ? (
            <p className="text-[12px] text-danger">{error}</p>
          ) : snapshot ? (
            <>
              {step === 0 ? (
                <div className="space-y-3">
                  {visibleFocus.length > 0 ? (
                    <p className="text-[12px] text-muted">
                      还有 {visibleFocus.length}{" "}
                      项重点未处理 — 选一个对你最合适的动作
                    </p>
                  ) : (
                    <p className="text-[12px] text-muted">
                      今日重点已全部处理完毕。
                    </p>
                  )}
                  <ul className="space-y-2" key={refreshKey}>
                    {visibleFocus.map((task) => (
                      <FocusItemRow
                        key={task.id}
                        task={task}
                        wrapDate={wrapDate}
                        disabled={loading}
                        onDecision={(kind) => {
                          decisionsRef.current[kind] += 1;
                          if (kind === "keep") {
                            setKeptIds((prev) => new Set(prev).add(task.id));
                          }
                        }}
                        onResolved={() => {
                          void refreshSnapshot();
                        }}
                      />
                    ))}
                  </ul>
                </div>
              ) : null}

              {step === 1 ? (
                <div className="space-y-3">
                  <p className="text-[12px] text-muted">
                    明天到期 {snapshot.tomorrowDue.length} 项（只读预览）
                  </p>
                  {snapshot.tomorrowDue.length === 0 ? (
                    <p className="text-[12px] text-muted">明天暂无到期任务。</p>
                  ) : (
                    <ul className="space-y-1">
                      {snapshot.tomorrowDue.map((task) => (
                        <li
                          key={task.id}
                          className="truncate rounded-[var(--radius-control)] border border-border px-2 py-1.5 text-[12px]"
                        >
                          {task.title}
                          {task.dueTime ? ` · ${task.dueTime}` : ""}
                        </li>
                      ))}
                    </ul>
                  )}
                  {onNavigate ? (
                    <Button
                      type="button"
                      size="sm"
                      variant="secondary"
                      onClick={() => {
                        onClose();
                        onNavigate("/tasks");
                      }}
                    >
                      逐条调整 → 任务
                    </Button>
                  ) : null}
                </div>
              ) : null}

              {step === 2 ? (
                <div className="space-y-3">
                  <p className="text-[12px] text-muted">
                    收件箱待处理 {snapshot.inboxUnprocessed.length} 项
                  </p>
                  {snapshot.inboxUnprocessed.length === 0 ? (
                    <p className="text-[12px] text-muted">收件箱已清空。</p>
                  ) : (
                    <ul className="space-y-1">
                      {snapshot.inboxUnprocessed.slice(0, 8).map((task) => (
                        <li
                          key={task.id}
                          className="truncate rounded-[var(--radius-control)] border border-border px-2 py-1.5 text-[12px]"
                        >
                          {task.title}
                        </li>
                      ))}
                      {snapshot.inboxUnprocessed.length > 8 ? (
                        <li className="text-[11px] text-muted">
                          还有 {snapshot.inboxUnprocessed.length - 8} 项…
                        </li>
                      ) : null}
                    </ul>
                  )}
                  {onNavigate ? (
                    <Button
                      type="button"
                      size="sm"
                      variant="secondary"
                      onClick={() => {
                        onClose();
                        onNavigate("/inbox");
                      }}
                    >
                      进入收件箱
                    </Button>
                  ) : null}
                </div>
              ) : null}

              {step === 3 ? (
                <div className="space-y-4">
                  <p className="text-[12px] text-muted">今日一览</p>
                  <dl className="grid grid-cols-2 gap-3">
                    <SummaryStat
                      label="今日完成"
                      value={snapshot.completedTodayCount}
                    />
                    <SummaryStat
                      label="今日提醒"
                      value={snapshot.remindersTodayCount}
                    />
                    <SummaryStat
                      label="重点未处理"
                      value={visibleFocus.length}
                    />
                    <SummaryStat
                      label="重点处理率"
                      value={focusRate != null ? `${focusRate}%` : "—"}
                    />
                  </dl>
                  <p className="text-[11px] text-muted">
                    点击下方「收尾完成」将记录本次收尾摘要。
                  </p>
                </div>
              ) : null}
            </>
          ) : null}

          {error && snapshot ? (
            <p className="mt-2 text-[12px] text-danger">{error}</p>
          ) : null}
        </div>

        <footer className="flex shrink-0 items-center justify-between gap-2 border-t border-border px-4 py-3">
          <Button type="button" size="sm" variant="ghost" onClick={onClose}>
            退出
          </Button>
          <div className="flex gap-2">
            {step < 3 ? (
              <>
                <Button
                  type="button"
                  size="sm"
                  variant="ghost"
                  disabled={loading}
                  onClick={() => setStep((s) => Math.min(s + 1, 3))}
                >
                  跳过此步
                </Button>
                <Button
                  type="button"
                  size="sm"
                  disabled={loading}
                  onClick={() => setStep((s) => Math.min(s + 1, 3))}
                >
                  下一步
                </Button>
              </>
            ) : (
              <Button
                type="button"
                size="sm"
                disabled={loading || !runId}
                onClick={() => void finish()}
              >
                {loading ? "保存中…" : "收尾完成"}
              </Button>
            )}
          </div>
        </footer>
      </div>
    </div>
  );
}

function SummaryStat({
  label,
  value,
}: {
  label: string;
  value: number | string;
}) {
  return (
    <div className="rounded-[var(--radius-control)] border border-border bg-surface-raised px-3 py-2">
      <dl>
        <dt className="text-[11px] text-muted">{label}</dt>
        <dd className="text-[20px] font-semibold tabular-nums">{value}</dd>
      </dl>
    </div>
  );
}
