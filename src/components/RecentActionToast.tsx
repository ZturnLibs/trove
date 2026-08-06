import { useEffect, useRef, useState } from "react";
import { useQueryClient } from "@tanstack/react-query";
import { useRecentActions } from "@/stores/recent-actions";
import { Button } from "@/design-system/primitives/Button";

const TOAST_DURATION_MS = 5000;
const ERROR_DURATION_MS = 3000;

export function RecentActionToast() {
  const queryClient = useQueryClient();
  const actions = useRecentActions((s) => s.actions);
  const pop = useRecentActions((s) => s.pop);
  const [error, setError] = useState<string | null>(null);
  const [pending, setPending] = useState(false);
  const timerRef = useRef<number | null>(null);
  const errorTimerRef = useRef<number | null>(null);

  const latest = actions.length > 0 ? actions[actions.length - 1] : null;

  // 5 秒自动消失；新动作（或撤销弹出下一条）时重置计时。
  useEffect(() => {
    if (!latest) return;
    timerRef.current = window.setTimeout(() => pop(latest.id), TOAST_DURATION_MS);
    return () => {
      if (timerRef.current !== null) {
        window.clearTimeout(timerRef.current);
        timerRef.current = null;
      }
    };
  }, [latest, pop]);

  // 撤销失败提示独立于当前 toast 存活，避免被 pop 后无处展示。
  useEffect(() => {
    return () => {
      if (errorTimerRef.current !== null) {
        window.clearTimeout(errorTimerRef.current);
      }
    };
  }, []);

  if (!latest && !error) return null;

  const showError = (message: string) => {
    setError(message);
    if (errorTimerRef.current !== null) {
      window.clearTimeout(errorTimerRef.current);
    }
    errorTimerRef.current = window.setTimeout(
      () => setError(null),
      ERROR_DURATION_MS,
    );
  };

  const handleUndo = async () => {
    if (pending || !latest) return;
    setPending(true);
    try {
      await latest.undo();
      void queryClient.invalidateQueries({ queryKey: ["tasks"] });
      pop(latest.id);
      setError(null);
    } catch (err) {
      // 撤销失败也弹出该条，避免卡住后续动作。
      pop(latest.id);
      showError(err instanceof Error ? err.message : "撤销失败");
    } finally {
      setPending(false);
    }
  };

  return (
    <div className="pointer-events-none fixed bottom-4 left-1/2 z-50 flex w-max max-w-[80vw] -translate-x-1/2 flex-col items-center gap-1">
      {latest ? (
        <div
          aria-live="polite"
          className="pointer-events-auto flex items-center gap-2 rounded-[var(--radius-panel)] border border-border bg-surface-raised px-3 py-2 shadow-lg"
        >
          <span className="truncate text-[13px] text-foreground">
            已{latest.label}
          </span>
          <Button
            size="sm"
            variant="secondary"
            className="shrink-0"
            disabled={pending}
            onClick={() => void handleUndo()}
          >
            {pending ? "撤销中…" : "撤销"}
          </Button>
        </div>
      ) : null}
      {error ? (
        <div
          role="alert"
          className="pointer-events-auto rounded-[var(--radius-panel)] border border-border bg-surface-raised px-3 py-1 text-[11px] text-danger shadow-lg"
        >
          撤销失败：{error}
        </div>
      ) : null}
    </div>
  );
}
