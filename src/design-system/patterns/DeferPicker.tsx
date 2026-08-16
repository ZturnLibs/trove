import { useEffect, useState } from "react";
import { Button } from "@/design-system/primitives/Button";
import { Input } from "@/design-system/primitives/Input";
import {
  DEFER_COACH_KEY,
  deferPresets,
  formatDeferHint,
  localTodayString,
} from "@/lib/defer";
import { cn } from "@/lib/cn";

type DeferPickerProps = {
  availableAt: string | null;
  dueDate?: string | null;
  disabled?: boolean;
  compact?: boolean;
  onChange: (availableAt: string | null) => void | Promise<void>;
};

export function DeferPicker({
  availableAt,
  dueDate,
  disabled,
  compact,
  onChange,
}: DeferPickerProps) {
  const [customDate, setCustomDate] = useState(availableAt ?? "");
  const [error, setError] = useState<string | null>(null);
  const [showCoach, setShowCoach] = useState(false);
  const [pending, setPending] = useState(false);

  useEffect(() => {
    setCustomDate(availableAt ?? "");
    setError(null);
  }, [availableAt]);

  useEffect(() => {
    try {
      setShowCoach(localStorage.getItem(DEFER_COACH_KEY) !== "1");
    } catch {
      setShowCoach(false);
    }
  }, []);

  const dismissCoach = () => {
    try {
      localStorage.setItem(DEFER_COACH_KEY, "1");
    } catch {
      /* ignore */
    }
    setShowCoach(false);
  };

  const apply = async (value: string | null) => {
    if (disabled || pending) return;
    if (value && dueDate && value > dueDate) {
      setError(
        "截止日期早于推迟显示日，任务会在此之前到期。请调整截止日期或推迟日。",
      );
      return;
    }
    setError(null);
    setPending(true);
    try {
      await onChange(value);
      if (value) setCustomDate(value);
    } catch (err) {
      setError(err instanceof Error ? err.message : "设置失败");
    } finally {
      setPending(false);
    }
  };

  const presets = deferPresets(localTodayString());

  return (
    <div className={cn("space-y-2", compact && "space-y-1")}>
      {showCoach ? (
        <div className="rounded-[var(--radius-control)] border border-border bg-surface-raised px-2 py-1.5 text-[11px] text-muted">
          <span>
            推迟显示不会修改截止日期；延期则会改变截止日。
          </span>
          <button
            type="button"
            className="ml-2 text-accent hover:underline"
            onClick={dismissCoach}
          >
            知道了
          </button>
        </div>
      ) : null}
      <div className="flex flex-wrap gap-1">
        {presets.map((preset) => (
          <Button
            key={preset.label}
            type="button"
            size="sm"
            variant={
              preset.value === availableAt ||
              (preset.value === null && !availableAt)
                ? "secondary"
                : "ghost"
            }
            disabled={disabled || pending}
            onClick={() => void apply(preset.value)}
          >
            {preset.label}
          </Button>
        ))}
      </div>
      <div className="flex items-center gap-2">
        <Input
          type="date"
          className={cn("h-8", compact ? "text-[11px]" : "text-[12px]")}
          value={customDate}
          disabled={disabled || pending}
          onChange={(e) => setCustomDate(e.target.value)}
        />
        <Button
          type="button"
          size="sm"
          variant="secondary"
          disabled={disabled || pending || !customDate}
          onClick={() => void apply(customDate || null)}
        >
          应用
        </Button>
      </div>
      <p className="text-[11px] text-muted">{formatDeferHint(availableAt)}</p>
      {error ? <p className="text-[11px] text-danger">{error}</p> : null}
    </div>
  );
}
