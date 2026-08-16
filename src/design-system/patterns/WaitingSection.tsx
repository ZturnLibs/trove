import { useEffect, useState } from "react";
import { Button } from "@/design-system/primitives/Button";
import { Input } from "@/design-system/primitives/Input";
import {
  followUpDueWarning,
  followUpPresets,
  formatWaitingHint,
  localTodayString,
} from "@/lib/waiting";
import { cn } from "@/lib/cn";

type WaitingSectionProps = {
  waitingFor: string | null;
  followUpDate: string | null;
  dueDate?: string | null;
  isWaiting: boolean;
  disabled?: boolean;
  onSetWaiting: (
    waitingFor: string | null,
    followUpDate: string | null,
  ) => void | Promise<void>;
  onClearWaiting: () => void | Promise<void>;
};

export function WaitingSection({
  waitingFor,
  followUpDate,
  dueDate,
  isWaiting,
  disabled,
  onSetWaiting,
  onClearWaiting,
}: WaitingSectionProps) {
  const [draftFor, setDraftFor] = useState(waitingFor ?? "");
  const [draftFollowUp, setDraftFollowUp] = useState(followUpDate ?? "");
  const [editing, setEditing] = useState(false);
  const [pending, setPending] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    setDraftFor(waitingFor ?? "");
    setDraftFollowUp(followUpDate ?? "");
    setEditing(false);
    setError(null);
  }, [waitingFor, followUpDate, isWaiting]);

  const warning = followUpDueWarning(dueDate, draftFollowUp || null);

  const applyWaiting = async () => {
    if (disabled || pending) return;
    const trimmed = draftFor.trim();
    if (!trimmed) {
      setError("请填写等待对象（例如同事姓名或外部依赖）");
      return;
    }
    setPending(true);
    setError(null);
    try {
      await onSetWaiting(trimmed, draftFollowUp.trim() || null);
      setEditing(false);
    } catch (err) {
      setError(err instanceof Error ? err.message : "设置失败");
    } finally {
      setPending(false);
    }
  };

  const endWaiting = async () => {
    if (disabled || pending) return;
    setPending(true);
    setError(null);
    try {
      await onClearWaiting();
    } catch (err) {
      setError(err instanceof Error ? err.message : "操作失败");
    } finally {
      setPending(false);
    }
  };

  const presets = followUpPresets(localTodayString());

  return (
    <div className="space-y-2 border-t border-border pt-3">
      <div className="flex items-center justify-between gap-2">
        <h3 className="text-[11px] font-medium text-muted">等待事项</h3>
        {isWaiting ? (
          <Button
            type="button"
            size="sm"
            variant="ghost"
            disabled={disabled || pending}
            onClick={() => void endWaiting()}
          >
            结束等待
          </Button>
        ) : null}
      </div>

      {!isWaiting ? (
        <Button
          type="button"
          size="sm"
          variant="secondary"
          disabled={disabled || pending}
          onClick={() => {
            setEditing(true);
            setDraftFollowUp((cur) => cur || localTodayString());
          }}
        >
          标记为等待中…
        </Button>
      ) : null}

      {(isWaiting || editing) && (
        <div
          className={cn(
            "space-y-2",
            !isWaiting &&
              "rounded-[var(--radius-control)] border border-border p-2",
          )}
        >
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
              {presets.map((preset) => (
                <Button
                  key={preset.label}
                  type="button"
                  size="sm"
                  variant={
                    draftFollowUp === preset.value ? "secondary" : "ghost"
                  }
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
          {!isWaiting ? (
            <Button
              type="button"
              size="sm"
              disabled={disabled || pending || !draftFor.trim()}
              onClick={() => void applyWaiting()}
            >
              确认等待
            </Button>
          ) : (
            <Button
              type="button"
              size="sm"
              variant="secondary"
              disabled={disabled || pending}
              onClick={() => void applyWaiting()}
            >
              保存
            </Button>
          )}
        </div>
      )}

      <p className="text-[11px] text-muted">
        {formatWaitingHint(
          isWaiting ? waitingFor : draftFor.trim() || null,
          isWaiting ? followUpDate : draftFollowUp.trim() || null,
        )}
      </p>
      {warning ? (
        <p className="text-[11px] text-amber-600 dark:text-amber-400">{warning}</p>
      ) : null}
      {error ? <p className="text-[11px] text-danger">{error}</p> : null}
    </div>
  );
}
