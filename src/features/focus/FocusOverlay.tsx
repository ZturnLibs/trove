import { useCallback, useEffect, useRef, useState } from "react";
import { useQueryClient } from "@tanstack/react-query";
import {
  isPermissionGranted,
  sendNotification,
} from "@tauri-apps/plugin-notification";
import { Button } from "@/design-system/primitives/Button";
import { cn } from "@/lib/cn";
import {
  abandonActiveFocusBestEffort,
  useFocusSession,
} from "@/stores/focus-session";
import { FocusRelatedSection } from "@/features/focus/FocusRelatedSection";
import type { Task } from "@/ipc/client";

const TIMER_PRESETS = [
  { label: "15 分钟", value: 15 },
  { label: "25 分钟", value: 25 },
  { label: "45 分钟", value: 45 },
  { label: "无倒计时", value: null },
] as const;

const priorityLabel: Record<Task["priority"], string> = {
  none: "",
  low: "低",
  medium: "中",
  high: "高",
};

function formatRemaining(ms: number): string {
  const totalSec = Math.max(0, Math.ceil(ms / 1000));
  const min = Math.floor(totalSec / 60);
  const sec = totalSec % 60;
  return `${String(min).padStart(2, "0")}:${String(sec).padStart(2, "0")}`;
}

function useFocusCountdown(
  sessionStartedAt: string,
  plannedMinutes: number | null,
  taskTitle: string,
) {
  const [bonusMs, setBonusMs] = useState(0);
  const [remainingMs, setRemainingMs] = useState<number | null>(null);
  const notifiedRef = useRef(false);
  const rafRef = useRef<number | null>(null);

  const tick = useCallback(() => {
    if (plannedMinutes == null) {
      setRemainingMs(null);
      return;
    }
    const endAt =
      new Date(sessionStartedAt).getTime() +
      plannedMinutes * 60_000 +
      bonusMs;
    const next = endAt - Date.now();
    setRemainingMs(next);

    if (next <= 0 && !notifiedRef.current) {
      notifiedRef.current = true;
      void (async () => {
        try {
          if (await isPermissionGranted()) {
            sendNotification({
              title: "专注时间到",
              body: taskTitle,
            });
          }
        } catch {
          /* ignore */
        }
      })();
    }
  }, [sessionStartedAt, plannedMinutes, bonusMs, taskTitle]);

  useEffect(() => {
    notifiedRef.current = false;
    setBonusMs(0);
  }, [sessionStartedAt, plannedMinutes]);

  useEffect(() => {
    tick();
    const loop = () => {
      tick();
      rafRef.current = requestAnimationFrame(loop);
    };
    rafRef.current = requestAnimationFrame(loop);
    return () => {
      if (rafRef.current !== null) cancelAnimationFrame(rafRef.current);
    };
  }, [tick]);

  const snoozeFiveMinutes = () => {
    notifiedRef.current = false;
    setBonusMs((b) => b + 5 * 60_000);
  };

  return { remainingMs, snoozeFiveMinutes, timedOut: remainingMs !== null && remainingMs <= 0 };
}

export function FocusOverlay() {
  const queryClient = useQueryClient();
  const session = useFocusSession((s) => s.session);
  const task = useFocusSession((s) => s.task);
  const open = useFocusSession((s) => s.open);
  const ending = useFocusSession((s) => s.ending);
  const defaultPlannedMinutes = useFocusSession((s) => s.defaultPlannedMinutes);
  const setDefaultPlannedMinutes = useFocusSession(
    (s) => s.setDefaultPlannedMinutes,
  );
  const endFocus = useFocusSession((s) => s.end);

  const [progressNote, setProgressNote] = useState("");
  const [escConfirm, setEscConfirm] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const plannedMinutes = session?.plannedMinutes ?? defaultPlannedMinutes;

  const { remainingMs, snoozeFiveMinutes, timedOut } = useFocusCountdown(
    session?.startedAt ?? new Date().toISOString(),
    plannedMinutes,
    task?.title ?? "专注任务",
  );

  useEffect(() => {
    if (!open) {
      setProgressNote("");
      setEscConfirm(false);
      setError(null);
    }
  }, [open]);

  useEffect(() => {
    if (!open) return;
    const onBeforeUnload = () => abandonActiveFocusBestEffort();
    window.addEventListener("beforeunload", onBeforeUnload);
    return () => window.removeEventListener("beforeunload", onBeforeUnload);
  }, [open]);

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
      setEscConfirm(true);
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [open]);

  const finish = async (outcome: "completed" | "keptTodo") => {
    setError(null);
    try {
      await endFocus(
        outcome,
        progressNote.trim() ? progressNote.trim() : null,
      );
      void queryClient.invalidateQueries({ queryKey: ["tasks"] });
    } catch (err) {
      setError(err instanceof Error ? err.message : "结束专注失败");
    }
  };

  if (!open || !session || !task) return null;

  const meta = [
    task.listName,
    task.dueDate ? `截止 ${task.dueDate}` : null,
    priorityLabel[task.priority] ? `优先级 ${priorityLabel[task.priority]}` : null,
  ]
    .filter(Boolean)
    .join(" · ");

  return (
    <div
      className="fixed inset-0 z-[60] flex flex-col bg-surface-raised text-foreground"
      role="dialog"
      aria-modal="true"
      aria-label="专注模式"
    >
      <header className="flex shrink-0 items-center justify-between gap-3 border-b border-border px-6 py-3">
        <Button
          size="sm"
          variant="ghost"
          onClick={() => setEscConfirm(true)}
        >
          Esc 退出专注
        </Button>

        <div className="flex items-center gap-2">
          {plannedMinutes != null ? (
            <span
              className={cn(
                "font-mono text-[18px] tabular-nums",
                timedOut && "text-accent",
              )}
            >
              {remainingMs !== null ? formatRemaining(remainingMs) : "--:--"}
            </span>
          ) : (
            <span className="text-[13px] text-muted">无倒计时</span>
          )}
          <select
            className="h-8 rounded-[var(--radius-control)] border border-border bg-surface px-2 text-[12px]"
            value={plannedMinutes ?? ""}
            onChange={(e) => {
              const v = e.target.value;
              setDefaultPlannedMinutes(v === "" ? null : Number(v));
            }}
            title="切换时长仅影响下次开始；当前会话沿用开始时设定"
          >
            {TIMER_PRESETS.map((preset) => (
              <option
                key={preset.label}
                value={preset.value ?? ""}
              >
                {preset.label}
              </option>
            ))}
          </select>
        </div>
      </header>

      <div className="mx-auto flex w-full max-w-2xl min-h-0 flex-1 flex-col gap-6 overflow-auto px-6 py-8">
        <div>
          <h1 className="text-[22px] font-semibold leading-snug">{task.title}</h1>
          {meta ? <p className="mt-1 text-[13px] text-muted">{meta}</p> : null}
        </div>

        {task.notes ? (
          <section className="space-y-2">
            <h2 className="text-[11px] font-medium uppercase tracking-wide text-muted">
              说明
            </h2>
            <p className="whitespace-pre-wrap text-[14px] leading-relaxed">
              {task.notes}
            </p>
            <p className="text-[11px] text-muted">
              如需编辑说明，请先退出专注并在主窗口任务详情中修改。
            </p>
          </section>
        ) : null}

        <section className="space-y-2">
          <h2 className="text-[11px] font-medium uppercase tracking-wide text-muted">
            相关
          </h2>
          <FocusRelatedSection taskId={task.id} />
        </section>

        <label className="block space-y-1 text-[11px] text-muted">
          进展备注（可选）
          <textarea
            value={progressNote}
            onChange={(e) => setProgressNote(e.target.value)}
            rows={3}
            placeholder="记录本次专注的进展…"
            className="w-full resize-none rounded-[var(--radius-control)] border border-border bg-surface p-2 text-[13px] text-foreground outline-none focus:ring-2 focus:ring-accent/35"
          />
        </label>

        {timedOut ? (
          <div className="flex flex-wrap items-center gap-2 rounded-[var(--radius-control)] border border-border bg-surface px-3 py-2 text-[12px]">
            <span>专注时间已到。</span>
            <Button size="sm" variant="secondary" onClick={snoozeFiveMinutes}>
              稍后 5 分钟
            </Button>
          </div>
        ) : null}

        {error ? <p className="text-[12px] text-danger">{error}</p> : null}
      </div>

      <footer className="flex shrink-0 items-center justify-end gap-2 border-t border-border px-6 py-4">
        <Button
          size="sm"
          variant="secondary"
          disabled={ending}
          onClick={() => void finish("keptTodo")}
        >
          保持待办并退出
        </Button>
        <Button
          size="sm"
          disabled={ending}
          onClick={() => void finish("completed")}
        >
          完成任务
        </Button>
      </footer>

      {escConfirm ? (
        <div className="fixed inset-0 z-[70] flex items-center justify-center bg-black/30 p-6">
          <div
            className="w-full max-w-sm rounded-[var(--radius-panel)] border border-border bg-surface p-4 shadow-lg"
            role="alertdialog"
            aria-labelledby="focus-exit-title"
          >
            <h3 id="focus-exit-title" className="text-[14px] font-medium">
              退出专注？
            </h3>
            <p className="mt-2 text-[12px] text-muted">
              任务将保持待办状态，可选填写进展备注。
            </p>
            <div className="mt-4 flex justify-end gap-2">
              <Button size="sm" variant="ghost" onClick={() => setEscConfirm(false)}>
                继续专注
              </Button>
              <Button
                size="sm"
                variant="secondary"
                disabled={ending}
                onClick={() => void finish("keptTodo")}
              >
                保持待办并退出
              </Button>
            </div>
          </div>
        </div>
      ) : null}
    </div>
  );
}
