import { useEffect, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { Button } from "@/design-system/primitives/Button";
import { Input } from "@/design-system/primitives/Input";
import { ipc, type TaskPriority } from "@/ipc/client";
import { useUiStore, type QuickMode } from "@/stores/ui";
import { cn } from "@/lib/cn";

const modes: { id: QuickMode; label: string }[] = [
  { id: "capture", label: "记录" },
  { id: "search", label: "搜索" },
  { id: "clip", label: "剪切板" },
];

type CaptureType = "task" | "reminder";

export function QuickWindow() {
  const mode = useUiStore((s) => s.quickMode);
  const setQuickMode = useUiStore((s) => s.setQuickMode);
  const [captureType, setCaptureType] = useState<CaptureType>("task");
  const [title, setTitle] = useState("");
  const [dueDate, setDueDate] = useState("");
  const [fireAt, setFireAt] = useState("");
  const [priority, setPriority] = useState<TaskPriority>("none");
  const [daily, setDaily] = useState(false);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const inputRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    let unlisten: (() => void) | undefined;
    void listen<string>("quick://set-mode", (event) => {
      const next = event.payload;
      if (next === "capture" || next === "search" || next === "clip") {
        setQuickMode(next);
      }
    }).then((fn) => {
      unlisten = fn;
    });
    return () => unlisten?.();
  }, [setQuickMode]);

  useEffect(() => {
    inputRef.current?.focus();
    setError(null);
  }, [mode, captureType]);

  const submit = async () => {
    if (mode !== "capture") return;
    const value = title.trim();
    if (!value) return;
    setSaving(true);
    setError(null);
    try {
      if (captureType === "task") {
        await ipc.taskCreate({
          title: value,
          dueDate: dueDate || null,
          priority: priority === "none" ? undefined : priority,
        });
      } else {
        if (!fireAt) throw new Error("请选择提醒时间");
        const normalized = fireAt.length === 16 ? `${fireAt}:00` : fireAt;
        await ipc.reminderCreate({
          title: value,
          fireAt: normalized.replace(" ", "T"),
          timezone: Intl.DateTimeFormat().resolvedOptions().timeZone,
          recurrence: daily
            ? {
                version: 1,
                frequency: "daily",
                interval: 1,
                timezone: Intl.DateTimeFormat().resolvedOptions().timeZone,
              }
            : null,
        });
      }
      setTitle("");
      setDueDate("");
      setFireAt("");
      setPriority("none");
      setDaily(false);
      await ipc.windowHideQuick();
    } catch (err) {
      setError(err instanceof Error ? err.message : "创建失败");
    } finally {
      setSaving(false);
    }
  };

  return (
    <div className="flex h-full flex-col bg-surface text-foreground">
      <div className="flex items-center gap-1 border-b border-border px-2 py-2">
        {modes.map((item) => (
          <Button
            key={item.id}
            size="sm"
            variant={mode === item.id ? "default" : "ghost"}
            onClick={() => setQuickMode(item.id)}
          >
            {item.label}
          </Button>
        ))}
        <div className="flex-1" />
        <Button
          size="sm"
          variant="ghost"
          onClick={() => void ipc.windowHideQuick()}
        >
          Esc
        </Button>
      </div>

      <div className="flex flex-1 flex-col gap-3 p-3">
        {mode === "capture" ? (
          <>
            <div className="flex gap-1 text-[12px]">
              {(["task", "reminder"] as const).map((type) => (
                <button
                  key={type}
                  type="button"
                  className={cn(
                    "rounded-[var(--radius-control)] px-2 py-0.5",
                    captureType === type
                      ? "bg-row-active text-foreground"
                      : "text-muted hover:bg-row-hover",
                  )}
                  onClick={() => setCaptureType(type)}
                >
                  {type === "task" ? "任务" : "提醒"}
                </button>
              ))}
              <span className="px-2 py-0.5 text-muted">记忆 · 阶段 3</span>
            </div>
            <Input
              ref={inputRef}
              value={title}
              onChange={(event) => setTitle(event.target.value)}
              placeholder={
                captureType === "task" ? "快速记录任务…" : "快速记录提醒…"
              }
              onKeyDown={(event) => {
                if (event.key === "Escape") {
                  if (title) setTitle("");
                  else void ipc.windowHideQuick();
                }
                if ((event.metaKey || event.ctrlKey) && event.key === "Enter") {
                  event.preventDefault();
                  void submit();
                }
              }}
            />
            {captureType === "task" ? (
              <div className="grid grid-cols-2 gap-2">
                <label className="space-y-1 text-[11px] text-muted">
                  截止日期
                  <Input
                    type="date"
                    value={dueDate}
                    onChange={(e) => setDueDate(e.target.value)}
                  />
                </label>
                <label className="space-y-1 text-[11px] text-muted">
                  优先级
                  <select
                    className="h-8 w-full rounded-[var(--radius-control)] border border-border bg-surface-raised px-2 text-[13px] text-foreground"
                    value={priority}
                    onChange={(e) => setPriority(e.target.value as TaskPriority)}
                  >
                    <option value="none">无</option>
                    <option value="low">低</option>
                    <option value="medium">中</option>
                    <option value="high">高</option>
                  </select>
                </label>
              </div>
            ) : (
              <div className="grid grid-cols-2 gap-2">
                <label className="space-y-1 text-[11px] text-muted">
                  提醒时间
                  <Input
                    type="datetime-local"
                    value={fireAt}
                    onChange={(e) => setFireAt(e.target.value)}
                  />
                </label>
                <label className="flex items-end gap-2 pb-1 text-[12px] text-muted">
                  <input
                    type="checkbox"
                    checked={daily}
                    onChange={(e) => setDaily(e.target.checked)}
                  />
                  每天重复
                </label>
              </div>
            )}
            {error ? <p className="text-[12px] text-danger">{error}</p> : null}
            <div className="mt-auto flex items-center justify-between text-[11px] text-muted">
              <span>⌘/Ctrl + Enter 创建 · Esc 取消</span>
              <Button
                size="sm"
                disabled={
                  saving ||
                  !title.trim() ||
                  (captureType === "reminder" && !fireAt)
                }
                onClick={() => void submit()}
              >
                {captureType === "task" ? "创建任务" : "创建提醒"}
              </Button>
            </div>
          </>
        ) : (
          <div
            className={cn(
              "flex flex-1 items-center justify-center rounded-[var(--radius-panel)] border border-dashed border-border text-[12px] text-muted",
            )}
          >
            {mode === "search" && "统一搜索将在阶段 3 接入"}
            {mode === "clip" && "剪切板浮层将在阶段 4 接入"}
          </div>
        )}
      </div>
    </div>
  );
}
