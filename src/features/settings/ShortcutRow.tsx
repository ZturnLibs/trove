import { useEffect, useState } from "react";
import { Button } from "@/design-system/primitives/Button";
import { formatShortcutLabel } from "@/lib/shortcuts";
import { eventToShortcutString } from "@/lib/shortcut-record";
import { cn } from "@/lib/cn";

const LABELS: Record<string, string> = {
  quickCapture: "快速记录",
  search: "统一搜索",
  clipboard: "剪切板浮层",
  focusMain: "聚焦主窗口",
};

export function ShortcutRow({
  id,
  value,
  onChange,
  disabled,
}: {
  id: keyof typeof LABELS;
  value: string;
  onChange: (next: string) => void;
  disabled?: boolean;
}) {
  const [listening, setListening] = useState(false);

  useEffect(() => {
    if (!listening) return;
    const onKeyDown = (event: KeyboardEvent) => {
      event.preventDefault();
      event.stopPropagation();
      if (event.key === "Escape") {
        setListening(false);
        return;
      }
      const next = eventToShortcutString(event);
      if (!next) return;
      onChange(next);
      setListening(false);
    };
    window.addEventListener("keydown", onKeyDown, true);
    return () => window.removeEventListener("keydown", onKeyDown, true);
  }, [listening, onChange]);

  return (
    <div className="flex items-center justify-between gap-3">
      <div className="min-w-0">
        <p className="font-medium">{LABELS[id]}</p>
        <p className="text-[11px] text-muted">
          {formatShortcutLabel(value) || value}
        </p>
      </div>
      <div className="flex items-center gap-2">
        <code
          className={cn(
            "rounded border border-border bg-surface px-2 py-1 font-mono text-[11px]",
            listening && "border-accent text-accent",
          )}
        >
          {listening ? "按下新快捷键…" : value}
        </code>
        <Button
          size="sm"
          variant={listening ? "default" : "secondary"}
          disabled={disabled}
          onClick={() => setListening((v) => !v)}
        >
          {listening ? "取消" : "更改"}
        </Button>
      </div>
    </div>
  );
}
