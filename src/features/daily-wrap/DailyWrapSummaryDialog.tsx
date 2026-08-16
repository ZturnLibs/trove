import { Button } from "@/design-system/primitives/Button";
import type { DailyWrapRun } from "@/ipc/client";

export function DailyWrapSummaryDialog({
  open,
  run,
  onClose,
  onStartAgain,
}: {
  open: boolean;
  run: DailyWrapRun | null | undefined;
  onClose: () => void;
  onStartAgain?: () => void;
}) {
  if (!open || !run) return null;

  const summary = run.summary ?? {};
  const decisions =
    (summary.decisions as Record<string, number> | undefined) ?? {};
  const completedTodayCount = summary.completedTodayCount as number | undefined;
  const remindersTodayCount = summary.remindersTodayCount as
    | number
    | undefined;
  const focusProcessRate = summary.focusProcessRate as number | null | undefined;

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center p-4">
      <button
        type="button"
        aria-label="关闭摘要"
        className="absolute inset-0 cursor-default bg-black/40"
        onClick={onClose}
      />
      <div
        className="relative w-full max-w-md rounded-[var(--radius-panel)] border border-border bg-surface p-4 shadow-xl"
        role="dialog"
        aria-modal="true"
        aria-label="今日收尾摘要"
      >
        <h2 className="text-[14px] font-semibold">今日已收尾</h2>
        <p className="mt-1 text-[11px] text-muted">
          {run.wrapDate}
          {run.completedAt
            ? ` · ${run.completedAt.slice(11, 16).replace("T", " ")} 完成`
            : ""}
        </p>

        <dl className="mt-4 grid grid-cols-2 gap-3 text-[12px]">
          {completedTodayCount != null ? (
            <div>
              <dt className="text-muted">今日完成</dt>
              <dd className="text-[18px] font-semibold">{completedTodayCount}</dd>
            </div>
          ) : null}
          {remindersTodayCount != null ? (
            <div>
              <dt className="text-muted">今日提醒</dt>
              <dd className="text-[18px] font-semibold">{remindersTodayCount}</dd>
            </div>
          ) : null}
          {focusProcessRate != null ? (
            <div>
              <dt className="text-muted">重点处理率</dt>
              <dd className="text-[18px] font-semibold">{focusProcessRate}%</dd>
            </div>
          ) : null}
        </dl>

        {Object.keys(decisions).length > 0 ? (
          <ul className="mt-3 space-y-1 text-[11px] text-muted">
            {decisions.keep ? <li>保留 {decisions.keep} 项</li> : null}
            {decisions.defer ? <li>推迟 {decisions.defer} 项</li> : null}
            {decisions.wait ? <li>等待 {decisions.wait} 项</li> : null}
            {decisions.complete ? <li>完成 {decisions.complete} 项</li> : null}
            {decisions.removeFocus ? (
              <li>移出重点 {decisions.removeFocus} 项</li>
            ) : null}
          </ul>
        ) : null}

        <div className="mt-4 flex justify-end gap-2">
          {onStartAgain ? (
            <Button size="sm" variant="secondary" onClick={onStartAgain}>
              再次收尾
            </Button>
          ) : null}
          <Button size="sm" onClick={onClose}>
            关闭
          </Button>
        </div>
      </div>
    </div>
  );
}
