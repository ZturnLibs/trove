import type { RecurrenceFrequency, RecurrenceRule } from "@/ipc/client";
import {
  defaultRecurrence,
  recurrenceLabel,
  toggleWeekday,
  withRecurrenceFrequency,
} from "@/lib/recurrence";
import { cn } from "@/lib/cn";

const FREQ_OPTIONS: { value: RecurrenceFrequency; label: string }[] = [
  { value: "daily", label: "每天" },
  { value: "weekdays", label: "工作日" },
  { value: "weekly", label: "每周" },
  { value: "monthly", label: "每月" },
  { value: "everyNDays", label: "每 N 天" },
  { value: "everyNWeeks", label: "每 N 周" },
];

const WEEKDAY_OPTIONS = [
  { value: 1, label: "一" },
  { value: 2, label: "二" },
  { value: 3, label: "三" },
  { value: 4, label: "四" },
  { value: 5, label: "五" },
  { value: 6, label: "六" },
  { value: 7, label: "日" },
] as const;

export type RecurrencePickerProps = {
  value: RecurrenceRule | null;
  onChange: (rule: RecurrenceRule | null) => void;
  disabled?: boolean;
  className?: string;
  compact?: boolean;
};

export function RecurrencePicker({
  value,
  onChange,
  disabled,
  className,
  compact,
}: RecurrencePickerProps) {
  const enabled = value !== null;

  const setEnabled = (next: boolean) => {
    if (!next) {
      onChange(null);
      return;
    }
    onChange(value ?? defaultRecurrence("daily"));
  };

  const patch = (partial: Partial<RecurrenceRule>) => {
    if (!value) return;
    onChange({ ...value, ...partial });
  };

  const setFrequency = (frequency: RecurrenceFrequency) => {
    onChange(withRecurrenceFrequency(value, frequency));
  };

  return (
    <div className={cn("space-y-2", className)}>
      <label className="flex items-center gap-2 text-[12px] text-muted">
        <input
          type="checkbox"
          checked={enabled}
          disabled={disabled}
          onChange={(e) => setEnabled(e.target.checked)}
        />
        重复
      </label>
      {enabled && value ? (
        <div
          className={cn(
            "space-y-2 rounded-[var(--radius-control)] border border-border bg-surface-raised p-2",
            compact && "text-[11px]",
          )}
        >
          <label className="flex flex-col gap-1 text-[11px] text-muted">
            频率
            <select
              className="h-8 rounded-[var(--radius-control)] border border-border bg-surface px-2 text-[13px] text-foreground"
              value={value.frequency}
              disabled={disabled}
              onChange={(e) =>
                setFrequency(e.target.value as RecurrenceFrequency)
              }
            >
              {FREQ_OPTIONS.map((opt) => (
                <option key={opt.value} value={opt.value}>
                  {opt.label}
                </option>
              ))}
            </select>
          </label>

          {value.frequency === "weekly" ||
          value.frequency === "everyNWeeks" ? (
            <div className="space-y-1">
              <span className="text-[11px] text-muted">星期</span>
              <div className="flex flex-wrap gap-1">
                {WEEKDAY_OPTIONS.map((day) => {
                  const active = (value.weekdays ?? []).includes(day.value);
                  return (
                    <button
                      key={day.value}
                      type="button"
                      disabled={disabled}
                      className={cn(
                        "min-w-7 rounded-[var(--radius-control)] border px-1.5 py-0.5 text-[12px]",
                        active
                          ? "border-accent bg-accent/15 text-foreground"
                          : "border-border text-muted hover:bg-row-hover",
                      )}
                      onClick={() => onChange(toggleWeekday(value, day.value))}
                    >
                      {day.label}
                    </button>
                  );
                })}
              </div>
            </div>
          ) : null}

          {value.frequency === "monthly" ? (
            <label className="flex flex-col gap-1 text-[11px] text-muted">
              每月第几天
              <input
                type="number"
                min={1}
                max={31}
                disabled={disabled}
                className="h-8 rounded-[var(--radius-control)] border border-border bg-surface px-2 text-[13px] text-foreground"
                value={value.monthday ?? 1}
                onChange={(e) =>
                  patch({
                    monthday: Math.min(
                      31,
                      Math.max(1, Number(e.target.value) || 1),
                    ),
                  })
                }
              />
            </label>
          ) : null}

          {value.frequency === "everyNDays" ||
          value.frequency === "everyNWeeks" ? (
            <label className="flex flex-col gap-1 text-[11px] text-muted">
              间隔
              <input
                type="number"
                min={1}
                max={365}
                disabled={disabled}
                className="h-8 rounded-[var(--radius-control)] border border-border bg-surface px-2 text-[13px] text-foreground"
                value={value.interval}
                onChange={(e) =>
                  patch({
                    interval: Math.min(
                      365,
                      Math.max(1, Number(e.target.value) || 1),
                    ),
                  })
                }
              />
            </label>
          ) : null}

          <p className="text-[11px] text-muted">{recurrenceLabel(value)}</p>
        </div>
      ) : null}
    </div>
  );
}
